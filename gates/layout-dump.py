import json
import math
import os
from pathlib import Path
import re
import selectors
import sqlite3
import subprocess
import tempfile


ROOT = Path(__file__).resolve().parent.parent
APP = ROOT / "target/release/ht-desktop"
OUT = ROOT / "evidence/layout"
COMMON = {"rail", "main", "rightpanel", "footer", "footer.text"} | {
    f"rail.icon.{i}" for i in range(13)
}
KEYS = {
    "inbox": {
        "inbox.tabs", "inbox.tab.active", "inbox.section", "inbox.row.0.author",
        "inbox.row.0.taskid", "inbox.row.0.title", "inbox.row.0.kind", "inbox.row.0.actions",
    } | {f"inbox.tab.{i}" for i in range(6)} | {f"inbox.row.{i}" for i in range(5)},
    "board": {
        "board.header", "board.title", "board.count", "board.toolbar", "board.column.0.title",
        "board.column.0.count", "board.column.0.add", "board.card.0.id", "board.card.0.title",
        "board.card.0.badge", "board.card.0.label", "board.card.0.meta",
    } | {f"board.column.{i}" for i in range(4)} | {f"board.card.{i}" for i in range(5)},
    "task": {
        "task.header", "task.title", "task.id", "task.description", "task.comment.0",
        "task.comment.0.author", "task.comment.0.body", "task.composer",
    } | {f"task.meta.{i}" for i in range(6)},
}
FIELDS = {
    "key", "x", "y", "w", "h", "color", "bg", "fontSize", "fontWeight", "lineHeight",
    "radius", "borderColor", "text",
}
ENV = {**os.environ, "WAYLAND_DISPLAY": ""}


def validate(path, screen, theme, width=1440, height=900, sparse=False):
    dump = json.loads(path.read_text())
    assert set(dump) == {"screen", "theme", "width", "height", "elements", "missing"}, dump
    assert dump["screen"] == screen and dump["theme"] == theme, dump
    assert dump["width"] == width and dump["height"] == height, dump
    elements = {e["key"]: e for e in dump["elements"]}
    missing = set(dump["missing"])
    assert len(elements) == len(dump["elements"]), "duplicate elements"
    assert len(missing) == len(dump["missing"]), "duplicate missing keys"
    assert not elements.keys() & missing, "present keys marked missing"
    assert (elements.keys() | missing) == COMMON | KEYS[screen], dump
    for element in elements.values():
        assert set(element) == FIELDS, element
        for field in ["x", "y", "w", "h"]:
            assert isinstance(element[field], (int, float)) and math.isfinite(element[field]), element
        assert element["w"] >= 0 and element["h"] >= 0, element
        for field in ["color", "bg", "borderColor"]:
            assert (element[field] is None and field != "color") or re.fullmatch(r"#[0-9a-f]{6}", element[field]), element
        for field in ["fontSize", "fontWeight", "lineHeight", "radius"]:
            assert element[field] is None or isinstance(element[field], (int, float)), element
        assert element["text"] is None or len(element["text"]) <= 40, element
    required = COMMON | KEYS[screen]
    if screen == "inbox":
        assert elements["inbox.tab.active"]["x"] == elements["inbox.tab.0"]["x"]
    if screen == "board" and not sparse:
        assert elements["board.card.0.badge"]["text"] == "In inbox", elements["board.card.0.badge"]
        assert elements["board.card.0.meta"]["text"] == "2", elements["board.card.0.meta"]
    if sparse:
        absent = {
            "inbox": {"inbox.tab.5"} | {f"inbox.row.{i}" for i in range(1, 5)},
            "board": {"board.column.3", "board.card.0.badge", "board.card.0.label", "board.card.0.meta"} | {f"board.card.{i}" for i in range(2, 5)},
            "task": {"task.comment.0", "task.comment.0.author", "task.comment.0.body"},
        }[screen]
        assert missing == absent, missing
        required -= absent
        if screen == "task":
            assert elements["task.composer"]["h"] == 154, elements["task.composer"]
    assert required <= elements.keys(), f"unmeasured native elements: {required - elements.keys()}"
    assert elements["rail"]["x"] == 0 and elements["rail"]["w"] == 48, elements["rail"]
    assert elements["main"]["x"] == 0 and elements["main"]["w"] == width - 420, elements["main"]
    assert elements["rightpanel"]["x"] == width - 420 and elements["rightpanel"]["w"] == 420, elements["rightpanel"]
    for i in range(5):
        assert elements[f"rail.icon.{i}"]["fontSize"] == 16, elements[f"rail.icon.{i}"]
        assert elements[f"rail.icon.{i}"]["lineHeight"] == 24, elements[f"rail.icon.{i}"]
    assert elements["rail.icon.0"]["w"] == 16 and elements["rail.icon.0"]["h"] == 20, elements["rail.icon.0"]
    for i in range(2, 5):
        assert elements[f"rail.icon.{i}"]["y"] == 82 + (i - 2) * 38, elements[f"rail.icon.{i}"]
    assert elements["footer.text"]["text"] == "Command center", elements["footer.text"]
    assert 80 <= elements["footer.text"]["w"] <= 140, elements["footer.text"]
    if theme == "graphite":
        assert 100 <= elements["footer.text"]["w"] <= 110, elements["footer.text"]
    if screen == "board":
        assert elements["board.column.0.add"]["fontSize"] == 16, elements["board.column.0.add"]
        assert elements["board.column.0.add"]["lineHeight"] == 24, elements["board.column.0.add"]
        assert elements["board.card.0.title"]["w"] <= elements["board.card.0"]["w"] - 16, elements["board.card.0.title"]
        assert elements["board.header"]["h"] == 56, elements["board.header"]
        assert elements["board.column.0"]["y"] == 72, elements["board.column.0"]
    if screen == "inbox":
        assert elements["inbox.tabs"]["w"] == min(width - 468, 1440) - 105, elements["inbox.tabs"]
        assert elements["inbox.tab.0"]["h"] == 32, elements["inbox.tab.0"]
        assert elements["inbox.tab.0"]["fontWeight"] == 400, elements["inbox.tab.0"]
    if theme == "graphite":
        assert elements["rail"]["bg"] == "#1a1d21", elements["rail"]
        if screen == "board":
            assert elements["board.title"]["fontSize"] == 14, elements["board.title"]
            assert elements["board.title"]["fontWeight"] == 600, elements["board.title"]
        if screen == "task":
            assert elements["task.title"]["fontSize"] == 24, elements["task.title"]
            assert elements["task.title"]["fontWeight"] == 700, elements["task.title"]
    print(f"LAYOUT {screen}/{theme}: covered={len(elements)} missing={sorted(missing)}", flush=True)
    return dump


OUT.mkdir(parents=True, exist_ok=True)
with tempfile.TemporaryDirectory(prefix="ht-layout-") as tmp:
    db = str(Path(tmp) / "local.db")
    subprocess.run([ROOT / "target/release/ht-core-cli", "--db", db, "import-fixtures", ROOT / "fixtures/demo"], check=True)
    with sqlite3.connect(db) as conn:
        notifications = conn.execute("SELECT n.id,n.raw_json FROM Notification n LEFT JOIN Task t ON t.id=n.taskId WHERE n.status='Normal' AND n.archivedAt IS NULL AND (t.id IS NULL OR t.status='Normal') ORDER BY n.createdAt DESC,n.id DESC LIMIT 5").fetchall()
        assert len(notifications) == 5, "fixture must exercise all five inbox rows"
        for notification_id, raw in notifications:
            notification = json.loads(raw)
            notification["computedSplit"] = "Important"
            conn.execute("UPDATE Notification SET raw_json=? WHERE id=?", (json.dumps(notification), notification_id))
        section_id, section_title = conn.execute("SELECT id,section_title FROM Section WHERE projectId=15 AND deleted=0 AND visibility=1 ORDER BY ranking,id LIMIT 1").fetchone()
        tasks = [json.loads(row[0]) for row in conn.execute("SELECT raw_json FROM Task WHERE projectId=15 AND status='Normal' ORDER BY updatedAt DESC,id ASC")]
        card = next(task for task in tasks if task.get("sectionId") == section_id or (task.get("sectionId") is None and task.get("section") == section_title))
        card["commentCount"] = 2
        card["assigneeCount"] = 0
        card["labels"] = [{"name": "Layout coverage"}]
        conn.execute("UPDATE Task SET raw_json=? WHERE id=?", (json.dumps(card), card["id"]))
        notification_id, raw = notifications[0]
        notification = json.loads(raw)
        notification["computedSplit"] = "Important"
        notification["taskId"] = card["id"]
        conn.execute("UPDATE Notification SET raw_json=?,taskId=? WHERE id=?", (json.dumps(notification), card["id"], notification_id))
    errors = []
    for theme in ["porcelain", "graphite", "amoled", "dia"]:
        for screen in KEYS:
            path = OUT / f"{screen}-{theme}.json"
            args = [str(APP), "--db", db, "--offline", "--screen", screen, "--board", "15", "--task", "58006", "--theme", theme, "--dump-layout", str(path)]
            result = subprocess.run([*args, "--exit-after-ready"], env=ENV, capture_output=True, text=True, timeout=60)
            assert result.returncode == 0 and "READY" in result.stdout, result
            try:
                validate(path, screen, theme)
            except AssertionError as error:
                errors.append(f"{screen}/{theme}: {error}")
                print(errors[-1], flush=True)
    for screen in KEYS:
        path = OUT / f"{screen}.json"
        args = [str(APP), "--db", db, "--offline", "--screen", screen, "--board", "15", "--task", "58006", "--theme", "graphite", "--window-size", "1920x1009", "--dump-layout", str(path), "--exit-after-ready"]
        result = subprocess.run(args, env=ENV, capture_output=True, text=True, timeout=60)
        assert result.returncode == 0 and "READY" in result.stdout, result
        try:
            validate(path, screen, "graphite", 1920, 1009)
        except AssertionError as error:
            errors.append(f"{screen}/graphite/1920: {error}")
            print(errors[-1], flush=True)
    path = OUT / "held.json"
    args = [str(APP), "--db", db, "--offline", "--theme", "graphite", "--dump-layout", str(path)]
    with subprocess.Popen(args, env=ENV, stdout=subprocess.PIPE, stderr=subprocess.PIPE) as app:
        try:
            with selectors.DefaultSelector() as selector:
                selector.register(app.stdout, selectors.EVENT_READ)
                assert selector.select(timeout=60), "no READY from held app"
                assert app.stdout.readline().strip() == b"READY"
            validate(path, "inbox", "graphite")
            assert app.poll() is None, "dump without exit-after-ready exited"
        finally:
            app.terminate()
            app.communicate(timeout=10)
    invalid = subprocess.run([*args[:-1], str(Path(tmp) / "absent/layout.json"), "--exit-after-ready"], env=ENV, capture_output=True, text=True, timeout=60)
    assert invalid.returncode != 0 and "layout dump failed" in invalid.stderr, invalid
    parity_db = str(Path(tmp) / "parity.db")
    subprocess.run([ROOT / "target/release/ht-core-cli", "--db", parity_db, "import-fixtures", ROOT / "gates/fixtures/layout-graphite"], check=True)
    parity_out = OUT / "parity"
    parity_out.mkdir(exist_ok=True)
    for screen in KEYS:
        path = parity_out / f"{screen}.json"
        args = [str(APP), "--db", parity_db, "--offline", "--screen", screen, "--board", "15", "--task", "103", "--theme", "graphite", "--window-size", "1920x1009", "--dump-layout", str(path), "--exit-after-ready"]
        result = subprocess.run(args, env=ENV, capture_output=True, text=True, timeout=60)
        assert result.returncode == 0 and "READY" in result.stdout, result
        validate(path, screen, "graphite", 1920, 1009, sparse=True)
    reference_db = str(Path(tmp) / "reference.db")
    subprocess.run([ROOT / "target/release/ht-core-cli", "--db", reference_db, "import-fixtures", ROOT / "gates/fixtures/layout-reference"], check=True)
    # Relative dates keep the frozen LAST WEEK label independent of the CI date.
    import datetime
    today = datetime.date.today()
    last_thursday = today - datetime.timedelta(days=today.weekday() + 4)
    with sqlite3.connect(reference_db) as conn:
        notification_id, raw = conn.execute("SELECT id,raw_json FROM Notification").fetchone()
        notification = json.loads(raw)
        notification["createdAt"] = f"{last_thursday.isoformat()}T12:00:00Z"
        conn.execute("UPDATE Notification SET raw_json=?,createdAt=? WHERE id=?", (json.dumps(notification), notification["createdAt"], notification_id))
    reference_out = OUT / "reference"
    reference_out.mkdir(exist_ok=True)
    for screen in KEYS:
        path = reference_out / f"{screen}.json"
        args = [str(APP), "--db", reference_db, "--offline", "--screen", screen, "--board", "15", "--task", "103", "--theme", "graphite", "--window-size", "1920x1009", "--dump-layout", str(path), "--exit-after-ready"]
        result = subprocess.run(args, env=ENV, capture_output=True, text=True, timeout=60)
        assert result.returncode == 0 and "READY" in result.stdout, result
        validate(path, screen, "graphite", 1920, 1009, sparse=True)
    assert not errors, "\n".join(errors)
print("LAYOUT_DUMP_OK: all screens, all themes, Graphite 1920x1009, hold, exit and write failure", flush=True)
