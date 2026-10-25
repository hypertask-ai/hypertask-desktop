"""Hosted Windows input, visible effects, durable writes, and owned cleanup."""
import argparse
from contextlib import closing
import ctypes as C
from ctypes import wintypes as W
import hashlib
import json
import os
from pathlib import Path
import sqlite3
import subprocess
import struct
import tempfile
import time

ROOT = Path(__file__).resolve().parent.parent
CASES = {
    "inbox-archive-notification": ("inbox", None, None),
    "board-rename-f2": ("board", "F2", "Native CI renamed task"),
    "task-description": ("task", "Ctrl+D", "Native CI description"),
    "comment-send-stay": ("task", "Ctrl+M", "Native CI comment"),
}


def hosted_only():
    if (os.environ.get("GITHUB_ACTIONS") != "true"
            or os.environ.get("RUNNER_ENVIRONMENT") != "github-hosted" or os.name != "nt"):
        raise SystemExit("Run only on public GitHub-hosted Windows, never VPS or laptop")


class EffectMissing(AssertionError):
    pass


def wait_for(read, predicate, message, error=AssertionError):
    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        value = read()
        if predicate(value):
            return value
        time.sleep(0.05)
    raise error(message)


class NativeWindow:
    def __init__(self, process):
        from PIL import ImageGrab
        self.grab = ImageGrab.grab
        self.process = process
        self.api = C.WinDLL("user32", use_last_error=True)
        self.api.GetForegroundWindow.restype = W.HWND
        self.api.GetWindowThreadProcessId.argtypes = [W.HWND, C.POINTER(W.DWORD)]
        self.api.IsWindowVisible.argtypes = [W.HWND]
        self.api.GetClassNameW.argtypes = [W.HWND, W.LPWSTR, C.c_int]
        self.api.ShowWindow.argtypes = [W.HWND, C.c_int]
        self.api.SetWindowPos.argtypes = [W.HWND, W.HWND, C.c_int, C.c_int, C.c_int, C.c_int, W.UINT]
        self.api.SetForegroundWindow.argtypes = [W.HWND]
        self.api.GetClientRect.argtypes = [W.HWND, C.POINTER(W.RECT)]
        self.api.ClientToScreen.argtypes = [W.HWND, C.POINTER(W.POINT)]
        self.api.PostMessageW.argtypes = [W.HWND, W.UINT, W.WPARAM, W.LPARAM]
        callback_type = C.WINFUNCTYPE(W.BOOL, W.HWND, W.LPARAM)
        self.api.EnumWindows.argtypes = [callback_type, W.LPARAM]

        def find():
            assert process.poll() is None, "Owned process exited during window discovery"
            found = []

            @callback_type
            def visit(hwnd, _):
                owner = W.DWORD()
                self.api.GetWindowThreadProcessId(hwnd, C.byref(owner))
                name = C.create_unicode_buffer(256)
                self.api.GetClassNameW(hwnd, name, len(name))
                if owner.value == process.pid and name.value == "Zed::Window":
                    found.append(hwnd)
                return True

            self.api.EnumWindows(visit, 0)
            return found

        windows = wait_for(find, lambda value: len(value) == 1, "No unique owned HWND")
        self.hwnd = windows[0]
        self.api.ShowWindow(self.hwnd, 9)
        assert self.api.SetWindowPos(self.hwnd, 0, 0, 0, 1440, 940, 0x0040), "Cannot size owned window"
        assert self.api.SetForegroundWindow(self.hwnd), "Cannot focus owned window"
        wait_for(self.api.GetForegroundWindow, lambda h: h == self.hwnd, "Owned HWND lacks foreground focus")

    def send(self, vk=0, scan=0, flags=0):
        class Keyboard(C.Structure):
            _fields_ = [("vk", W.WORD), ("scan", W.WORD), ("flags", W.DWORD),
                        ("time", W.DWORD), ("extra", C.c_size_t)]

        class Mouse(C.Structure):
            _fields_ = [("x", W.LONG), ("y", W.LONG), ("data", W.DWORD),
                        ("flags", W.DWORD), ("time", W.DWORD), ("extra", C.c_size_t)]

        class Payload(C.Union):
            _fields_ = [("keyboard", Keyboard), ("mouse", Mouse)]

        class Input(C.Structure):
            _fields_ = [("kind", W.DWORD), ("payload", Payload)]

        assert self.process.poll() is None, "Owned process exited before input"
        assert self.api.GetForegroundWindow() == self.hwnd, "Input target lost foreground focus"
        event = Input(1, Payload(keyboard=Keyboard(vk, scan, flags, 0, 0)))
        assert self.api.SendInput(1, C.byref(event), C.sizeof(event)) == 1, "SendInput failed"

    def key(self, chord):
        keys = {"Ctrl": 0x11, "Shift": 0x10, "Enter": 0x0D, "F2": 0x71}
        codes = [keys.get(part, ord(part) if len(part) == 1 else 0) for part in chord.split("+")]
        assert all(codes), f"Unknown chord {chord}"
        try:
            for code in codes:
                self.send(vk=code)
        finally:
            for code in reversed(codes):
                self.send(vk=code, flags=2)

    def type(self, text):
        for char in text:
            self.send(scan=ord(char), flags=4)
            self.send(scan=ord(char), flags=6)

    def capture(self, path):
        assert C.WinDLL("dwmapi").DwmFlush() == 0, "Desktop compositor did not flush"
        rect, point = W.RECT(), W.POINT()
        assert self.api.GetClientRect(self.hwnd, C.byref(rect))
        assert self.api.ClientToScreen(self.hwnd, C.byref(point))
        image = self.grab(bbox=(point.x, point.y, point.x + rect.right, point.y + rect.bottom))
        image.save(path)
        assert len(image.getcolors(image.width * image.height) or []) > 32, "Blank/unpresented screenshot"
        return hashlib.sha256(image.tobytes()).hexdigest()


def snapshot(path):
    try:
        return json.loads(path.read_text())
    except (FileNotFoundError, json.JSONDecodeError, PermissionError):
        return None


def presented_frames(behavior):
    # A snapshot is written before its frame presents. Cross two more frame boundaries.
    for _ in range(2):
        previous = behavior.stat().st_mtime_ns
        wait_for(lambda: behavior.stat().st_mtime_ns, lambda stamp: stamp > previous,
                 "No fresh native presentation frames")


def visible(data, key, text=None):
    if not data:
        return False
    return any(e["key"] == key and e["w"] > 0 and e["h"] > 0
               and e["x"] >= 0 and 0 <= e["y"] < 850
               and (text is None or e.get("text", "").strip() == text) for e in data["elements"])


def verify_database(db, case, task_id, notification_id, text):
    with closing(sqlite3.connect(db)) as conn, conn:
        rows = conn.execute("SELECT kind,payload_json,status,attempts FROM mutation_queue ORDER BY id").fetchall()
        assert len(rows) == 1, f"Expected exactly one write, got {rows}"
        kind, raw, status, attempts = rows[0]
        payload = json.loads(raw)
        assert status == "pending" and attempts == 0, "Offline input must not send requests"
        if case == "inbox-archive-notification":
            assert kind == "inbox_archive" and payload["notification_ids"] == [notification_id], rows
            status, archived = conn.execute("SELECT status,archivedAt FROM Notification WHERE id=?", (notification_id,)).fetchone()
            assert status == "Archive" and archived
        else:
            assert payload["task_id"] == task_id, rows
            if case == "board-rename-f2":
                assert kind == "task_title" and payload["title"] == text, rows
                assert conn.execute("SELECT title FROM Task WHERE id=?", (task_id,)).fetchone()[0] == text
            elif case == "task-description":
                assert kind == "task_patch" and payload["description"] == f"<p>{text}</p>", rows
                assert payload["content_type"] == "html", rows
                assert json.loads(conn.execute("SELECT raw_json FROM Task WHERE id=?", (task_id,)).fetchone()[0])["description"] == f"<p>{text}</p>"
            else:
                assert kind == "task_comment" and payload["text"] == f"<p>{text}</p>", rows
                comments = [json.loads(row[0]) for row in conn.execute("SELECT raw_json FROM Comment WHERE taskId=?", (task_id,))]
                assert sum(row.get("text") == f"<p>{text}</p>" for row in comments) == 1, comments
        return {"kind": kind, "payload": payload, "status": "pending", "attempts": attempts}


def run_case(case, app, core, out, env):
    screen, edit_key, text = CASES[case]
    out.mkdir(parents=True)
    result = {"case": case, "revision": os.environ["HT_VERIFY_REVISION"], "actions": [],
              "exe_sha256": hashlib.sha256(app.read_bytes()).hexdigest(), "passed": False}
    with tempfile.TemporaryDirectory(prefix="ht-nativeci-") as tmp:
        db = Path(tmp) / "local.db"
        dump = out / "layout.json"
        behavior = dump.with_suffix(".behavior.json")
        subprocess.run([core, "--db", db, "import-fixtures", ROOT / "fixtures/demo"], env=env, check=True)
        with closing(sqlite3.connect(db)) as conn, conn:
            conn.execute("DELETE FROM Comment WHERE taskId=58006")
            conn.execute("UPDATE Task SET raw_json=json_set(raw_json,'$.description','<p>Native CI original</p>','$.totalComments',0) WHERE id=58006")
        process = None
        window = None
        try:
            with (out / "stdout.txt").open("w") as stdout, (out / "stderr.txt").open("w") as stderr:
                command = [str(app), "--db", str(db), "--offline", "--background", "--screen", screen,
                           "--board", "15", "--task", "58006", "--theme", "graphite", "--dump-layout", str(dump)]
                result["command"] = command
                process = subprocess.Popen(command, env=env, stdout=stdout, stderr=stderr)
                result["owned_pid"] = process.pid
                window = NativeWindow(process)
                before = wait_for(lambda: snapshot(behavior), lambda s: s and s["state"]["screen"] == screen,
                                  "No presented starting screen")
                (out / "before.json").write_text(json.dumps(before, indent=2))
                task_id = before["state"]["task_id"]
                result["task_id"] = task_id
                assert task_id is not None
                presented_frames(behavior)
                first_hash = window.capture(out / "before.png")
                notification_id = None

                def key(chord):
                    result["actions"].append({"key": chord})
                    window.key(chord)

                if edit_key:
                    key(edit_key)
                    field, mode = ("title_text", "title") if screen == "board" else (("description_text", "description") if case == "task-description" else ("comment_text", "comment"))
                    wait_for(lambda: snapshot(behavior), lambda s: s and s["state"]["editing"] == mode,
                             f"{edit_key} did not open editor")
                    key("Ctrl+A")
                    result["actions"].append({"type": text})
                    window.type(text)
                    wait_for(lambda: snapshot(behavior), lambda s: s and s["state"][field] == text,
                             "Native typed text did not reach focused editor")
                    key("Enter" if screen == "board" else ("Ctrl+Enter" if case == "task-description" else "Ctrl+Shift+Enter"))
                else:
                    with closing(sqlite3.connect(db)) as conn, conn:
                        tabs = json.loads(conn.execute("SELECT value FROM kv WHERE key='inbox-splits:0'").fetchone()[0])
                        members = tabs[before["state"]["split"]]["ids"]
                        placeholders = ",".join("?" for _ in members)
                        expected = conn.execute(
                            f"SELECT n.id FROM Notification n LEFT JOIN Task t ON t.id=n.taskId "
                            f"WHERE n.status='Normal' AND n.archivedAt IS NULL AND (t.id IS NULL OR t.status='Normal') "
                            f"AND n.id IN ({placeholders}) ORDER BY n.createdAt DESC,n.id DESC", members).fetchall()
                        notification_id = expected[before["state"]["selected"]][0]
                        original = {row[0] for row in conn.execute("SELECT id FROM Notification WHERE status='Archive'")}
                    key("E")

                def effect(s):
                    if not s or s["state"]["screen"] != screen:
                        return False
                    state = s["state"]
                    if case == "inbox-archive-notification":
                        return state["inbox_count"] == before["state"]["inbox_count"] - 1 and state["selection_count"] == before["state"]["selection_count"] - 1
                    if state["editing"] != "none" or state["task_id"] != task_id:
                        return False
                    if case == "board-rename-f2":
                        return visible(s, "board.card.0.title", text)
                    if case == "task-description":
                        return visible(s, "task.description", text)
                    return state["comment_text"] == "" and visible(s, "task.comment.0.body", text)

                after = wait_for(lambda: snapshot(behavior), effect, f"{case}: visible effect missing", EffectMissing)
                (out / "after.json").write_text(json.dumps(after, indent=2))
                presented_frames(behavior)
                assert window.capture(out / "after.png") != first_hash, "Visible effect must change presented pixels"
                if case == "inbox-archive-notification":
                    with closing(sqlite3.connect(db)) as conn, conn:
                        changed = {row[0] for row in conn.execute("SELECT id FROM Notification WHERE status='Archive'")} - original
                    assert changed == {notification_id}, "One key must archive only the selected notification"
                result["mutation"] = verify_database(db, case, task_id, notification_id, text)
                result["notification_id"] = notification_id
                result["passed"] = True
        except Exception as error:
            result["error_type"] = type(error).__name__
            result["error"] = str(error)
            if window and process.poll() is None:
                window.capture(out / "failure.png")
        finally:
            if process:
                if process.poll() is None and window:
                    window.api.PostMessageW(window.hwnd, 0x0010, 0, 0)
                try:
                    process.wait(timeout=10)
                    result["graceful_exit"] = process.returncode == 0
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait()
                    result["graceful_exit"] = False
                result["process_exited"] = process.poll() is not None
            with closing(sqlite3.connect(db)) as conn, closing(sqlite3.connect(out / "result.db")) as saved:
                conn.backup(saved)
            if result["passed"]:
                verify_database(out / "result.db", case, task_id, notification_id, text)
    result["temporary_db_removed"] = not db.exists()
    (out / "receipt.json").write_text(json.dumps(result, indent=2) + "\n")
    assert result["temporary_db_removed"] and result.get("process_exited") and result.get("graceful_exit"), result
    return result


def main():
    hosted_only()
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", type=Path, default=ROOT / "evidence/windows-input/baseline")
    parser.add_argument("--expect-broken", action="store_true")
    args = parser.parse_args()
    api = C.WinDLL("user32")
    api.SetProcessDPIAware()
    display = C.create_string_buffer(220)
    struct.pack_into("H", display, 68, 220)
    assert api.EnumDisplaySettingsW(None, -1, display), "Cannot read runner display mode"
    requested = C.create_string_buffer(display.raw, 220)
    struct.pack_into("I", requested, 72, 0x180000)
    struct.pack_into("II", requested, 172, 1920, 1080)
    assert api.ChangeDisplaySettingsW(requested, 2) == 0, "Runner cannot provide 1920x1080"
    assert api.ChangeDisplaySettingsW(requested, 0) == 0, "Cannot set isolated runner display"
    app = ROOT / "target/release/ht-desktop.exe"
    core = ROOT / "target/release/ht-core-cli.exe"
    env = {k: v for k, v in os.environ.items() if k not in ("HT_TOKEN", "GH_TOKEN", "GITHUB_TOKEN")}
    try:
        results = [run_case(case, app, core, args.out / case, env) for case in CASES]
    finally:
        restored = api.ChangeDisplaySettingsW(display, 0) == 0
        args.out.mkdir(parents=True, exist_ok=True)
        (args.out / "display.json").write_text(json.dumps({"requested": [1920, 1080], "restored": restored}) + "\n")
        assert restored, "Cannot restore runner display"
    for result in results:
        if args.expect_broken:
            assert not result["passed"] and result.get("error_type") == "EffectMissing", result
            assert f'NATIVECI_BROKEN {result["case"]}' in (args.out / result["case"] / "stderr.txt").read_text(), result
            print(f'NATIVE_INPUT_REJECTED {result["case"]} reason=visible_effect_missing cleanup=complete')
        else:
            assert result["passed"], result
            print(f'NATIVE_INPUT_VERIFIED {result["case"]} persisted=true cleanup=complete')


if __name__ == "__main__":
    main()
