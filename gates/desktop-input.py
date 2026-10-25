"""GitHub Linux runner: real keyboard archive, presented frames, durable SQLite effect."""
import hashlib
import json
import os
from pathlib import Path
import selectors
import sqlite3
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "evidence/desktop-input"


def main():
    if os.environ.get("GITHUB_ACTIONS") != "true" or os.environ.get("RUNNER_ENVIRONMENT") != "github-hosted":
        raise SystemExit("Run desktop-input only on public GitHub-hosted CI, never the VPS or laptop")
    OUT.mkdir(parents=True, exist_ok=True)
    env = {k: v for k, v in os.environ.items() if k not in ("HT_TOKEN", "GH_TOKEN", "GITHUB_TOKEN")}
    env["WAYLAND_DISPLAY"] = ""
    app = ROOT / "target/release/ht-desktop"
    log = OUT / "app.log"
    with tempfile.TemporaryDirectory(prefix="ht-input-") as tmp:
        db = Path(tmp) / "local.db"
        subprocess.run([ROOT / "target/release/ht-core-cli", "--db", db,
                        "import-fixtures", ROOT / "fixtures/demo"], env=env, check=True)
        with sqlite3.connect(db) as conn:
            before = conn.execute("SELECT id FROM Notification WHERE status='Archive'").fetchall()
        command = [str(app), "--db", str(db), "--offline", "--screen", "inbox",
                   "--board", "15", "--task", "58006", "--theme", "graphite", "--hold-after-ready"]
        with log.open("w") as output, (OUT / "app.stderr").open("w") as errors:
            process = subprocess.Popen(command, env=env, stdout=subprocess.PIPE, stderr=errors)
            try:
                pending = bytearray()

                def wait_line(prefix):
                    deadline = time.monotonic() + 45
                    with selectors.DefaultSelector() as selector:
                        selector.register(process.stdout, selectors.EVENT_READ)
                        while time.monotonic() < deadline:
                            while b"\n" in pending:
                                line, _, rest = pending.partition(b"\n")
                                pending[:] = rest
                                text = line.decode("utf-8", errors="replace")
                                output.write(text + "\n")
                                output.flush()
                                if text.startswith(prefix):
                                    return
                            if not selector.select(timeout=max(0, deadline - time.monotonic())):
                                break
                            chunk = os.read(process.stdout.fileno(), 65536)
                            if not chunk:
                                break
                            pending.extend(chunk)
                    raise AssertionError(f"No {prefix!r} from owned PID {process.pid}; see {log}")

                wait_line("READY")
                window = subprocess.check_output(["xdotool", "search", "--onlyvisible", "--class", "ai.hypertask.desktop"], env=env, text=True).splitlines()
                assert len(window) == 1, f"Expected our sole native window, got {window}"
                window = window[0]
                assert process.poll() is None
                subprocess.run(["xdotool", "sleep", "0.3"], env=env, check=True)
                subprocess.run(["import", "-window", window, str(OUT / "before.png")], env=env, check=True)
                subprocess.run(["xdotool", "windowfocus", "--sync", window, "key", "--clearmodifiers", "e"], env=env, check=True)
                wait_line("QUEUED ")
                with sqlite3.connect(db) as conn:
                    after = conn.execute("SELECT id FROM Notification WHERE status='Archive'").fetchall()
                    changed = set(after) - set(before)
                    assert len(changed) == 1, "One real E key must archive exactly one notification"
                    row_id = changed.pop()[0]
                    assert conn.execute("SELECT archivedAt FROM Notification WHERE id=?", (row_id,)).fetchone()[0]
                    kinds = conn.execute("SELECT kind FROM mutation_queue ORDER BY id").fetchall()
                    assert kinds == [("inbox_archive",)], kinds
                    payload, status, attempts = conn.execute(
                        "SELECT payload_json,status,attempts FROM mutation_queue").fetchone()
                    assert json.loads(payload)["notification_ids"] == [row_id]
                    assert status == "pending" and attempts == 0, "Offline mode must not send the write"
                subprocess.run(["xdotool", "sleep", "0.3"], env=env, check=True)
                subprocess.run(["import", "-window", window, str(OUT / "after.png")], env=env, check=True)
                for name in ("before.png", "after.png"):
                    colors = int(subprocess.check_output(
                        ["identify", "-format", "%k", str(OUT / name)], env=env, text=True))
                    assert colors > 32, f"Blank or unpresented capture: {name} colors={colors}"
                signatures = [subprocess.check_output(
                    ["identify", "-format", "%#", str(OUT / name)], env=env, text=True)
                    for name in ("before.png", "after.png")]
                assert signatures[0] != signatures[1], "Archive must change the presented pixels"
                with sqlite3.connect(db) as conn, sqlite3.connect(OUT / "result.db") as saved:
                    conn.backup(saved)
                subprocess.run(["xdotool", "key", "--clearmodifiers", "2", "Return", "ctrl+m", "ctrl+a"], env=env, check=True)
                subprocess.run(["xdotool", "type", "--clearmodifiers", "--delay", "1", "Native highlighted text"], env=env, check=True)
                subprocess.run(["xdotool", "sleep", "0.3"], env=env, check=True)
                subprocess.run(["import", "-window", window, str(OUT / "editor-plain.png")], env=env, check=True)
                subprocess.run(["xdotool", "key", "--clearmodifiers", "ctrl+a", "ctrl+shift+h", "Right"], env=env, check=True)
                subprocess.run(["xdotool", "sleep", "0.3"], env=env, check=True)
                subprocess.run(["import", "-window", window, str(OUT / "editor-highlight.png")], env=env, check=True)

                def gold_pixels(name):
                    pixels = subprocess.check_output(["convert", str(OUT / name), "-depth", "8", "rgb:-"], env=env)
                    return sum(r >= 245 and 210 <= g <= 235 and 85 <= b <= 120
                               for r, g, b in zip(pixels[0::3], pixels[1::3], pixels[2::3]))

                plain_gold = gold_pixels("editor-plain.png")
                highlighted_gold = gold_pixels("editor-highlight.png")
                assert highlighted_gold - plain_gold >= 500, (
                    f"Real highlighted glyphs need a painted yellow background: before={plain_gold}, after={highlighted_gold}")
                with sqlite3.connect(db) as conn, sqlite3.connect(OUT / "editor.db") as saved:
                    conn.backup(saved)
            finally:
                process.terminate()
                try:
                    process.communicate(timeout=10)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.communicate()
        with sqlite3.connect(db) as conn:
            assert conn.execute("SELECT status FROM Notification WHERE id=?", (row_id,)).fetchone()[0] == "Archive"
        proof = {"revision": os.environ["HT_VERIFY_REVISION"], "exe_sha256": hashlib.sha256(app.read_bytes()).hexdigest(),
                 "action": "native X11 key e", "notification_id": row_id, "queue_kinds": [k[0] for k in kinds],
                 "offline": True, "owned_pid": process.pid, "process_exited": process.poll() is not None,
                 "temporary_db_removed_after_return": str(db),
                 "highlight_pixels": {"plain": plain_gold, "highlighted": highlighted_gold}}
        (OUT / "receipt.json").write_text(json.dumps(proof, indent=2) + "\n")
    assert not db.exists() and process.poll() is not None
    assert all((OUT / name).stat().st_size > 0 for name in ("before.png", "after.png", "result.db", "receipt.json", "app.log"))
    print(f"DESKTOP_INPUT_HIGHLIGHT_VERIFIED plain_gold={plain_gold} highlighted_gold={highlighted_gold} selection=deselected")
    print("DESKTOP_INPUT_ARCHIVE_VERIFIED action=e persisted=Archive queue=inbox_archive cleanup=complete evidence=retained")


if __name__ == "__main__":
    main()
