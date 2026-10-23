import json
import math
import os
from pathlib import Path
import re
import selectors
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


def validate(path, screen, theme):
    dump = json.loads(path.read_text())
    assert set(dump) == {"screen", "theme", "width", "height", "elements", "missing"}, dump
    assert dump["screen"] == screen and dump["theme"] == theme, dump
    assert dump["width"] == 1440 and dump["height"] == 900, dump
    elements = {e["key"]: e for e in dump["elements"]}
    missing = set(dump["missing"])
    assert len(elements) == len(dump["elements"]), "duplicate elements"
    assert len(missing) == len(dump["missing"]), "duplicate missing keys"
    assert not elements.keys() & missing, "present keys marked missing"
    assert elements.keys() | missing == COMMON | KEYS[screen], dump
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
    required = COMMON - {"rightpanel"} | KEYS[screen]
    if screen == "inbox":
        required -= {f"inbox.tab.{i}" for i in range(2, 6)}
        assert missing >= {"rightpanel"}, dump
        assert elements["inbox.tab.active"]["x"] == elements["inbox.tab.0"]["x"]
    if screen == "board":
        required -= {"board.card.0.badge", "board.card.0.label", "board.card.0.meta"}
    assert required <= elements.keys(), f"unmeasured native elements: {required - elements.keys()}"
    assert elements["rail"]["x"] == 0 and elements["rail"]["w"] == 48, elements["rail"]
    assert elements["main"]["x"] == 48, elements["main"]
    if theme == "graphite":
        assert elements["rail"]["bg"] == "#1a1d21", elements["rail"]
        if screen == "board":
            assert elements["board.title"]["fontSize"] == 14, elements["board.title"]
            assert elements["board.title"]["fontWeight"] == 700, elements["board.title"]
        if screen == "task":
            assert elements["task.title"]["fontSize"] == 24, elements["task.title"]
            assert elements["task.title"]["fontWeight"] == 700, elements["task.title"]
    print(f"LAYOUT {screen}/{theme}: covered={len(elements)} missing={sorted(missing)}", flush=True)
    return dump


OUT.mkdir(parents=True, exist_ok=True)
with tempfile.TemporaryDirectory(prefix="ht-layout-") as tmp:
    db = str(Path(tmp) / "local.db")
    subprocess.run([ROOT / "target/release/ht-core-cli", "--db", db, "import-fixtures", ROOT / "fixtures/demo"], check=True)
    for theme in ["porcelain", "graphite", "amoled", "dia"]:
        for screen in KEYS:
            path = OUT / f"{screen}-{theme}.json"
            args = [str(APP), "--db", db, "--offline", "--screen", screen, "--board", "15", "--task", "58006", "--theme", theme, "--dump-layout", str(path)]
            result = subprocess.run([*args, "--exit-after-ready"], env=ENV, capture_output=True, text=True, timeout=60)
            assert result.returncode == 0 and "READY" in result.stdout, result
            validate(path, screen, theme)
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
print("LAYOUT_DUMP_OK: all screens, all themes, hold, exit and write failure", flush=True)
