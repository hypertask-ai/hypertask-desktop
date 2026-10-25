"""Additional web-source rail coverage. The frozen live comparer stays unchanged."""
import json
import os
from pathlib import Path

if os.environ.get("GITHUB_ACTIONS") != "true":
    raise SystemExit("Rail layout proof runs only on GitHub Actions")

root = Path(__file__).resolve().parent.parent
out = root / "evidence/layout"
records = []
# AppShellRail.tsx:79,191,226-285,303-360; graphite.css:53-55.
for screen in ("inbox", "board", "task"):
    dump = json.loads((out / "reference" / f"{screen}.json").read_text())
    elements = {element["key"]: element for element in dump["elements"]}
    table = elements["rail.icon.4"]
    for index in range(5, 13):
        native = elements[f"rail.icon.{index}"]
        expected = {
            "x": table["x"], "w": 36, "h": 34, "radius": 5,
            "color": "#8e9093", "bg": None,
            "y": table["y"] + (index - 4) * (34 + 4) if index < 8 else dump["height"] - 8 - 4 - 34 - (12 - index) * (34 + 4),
        }
        differences = {key: {"expected": value, "native": native[key]} for key, value in expected.items() if (
            abs(native[key] - value) > (1.5 if key in ("x", "y", "w", "h") else 0.5) if isinstance(value, (int, float)) else native[key] != value
        )}
        records.append({"screen": screen, "key": native["key"], "expected": expected, "native": native, "differences": differences})
receipt = {"sourceRevision": "6f9901764d355aba7bfc76e4e62ff255e0b68b31", "records": records, "differences": sum(len(record["differences"]) for record in records)}
(out / "rail-source-comparison.json").write_text(json.dumps(receipt, indent=2) + "\n")
assert receipt["differences"] == 0, receipt
print(f'LAYOUT_RAIL_SOURCE_OK: {len(records)} previously unsampled records; zero source-backed differences; frozen comparison unchanged', flush=True)


def pointer_differences(record):
    return {
        key: {"expected": expected, "native": native}
        for key, expected, native in (
            ("color", "#ffffff", record["element"]["color"]),
            ("bg", "#3a3d45", record["element"]["bg"]),
            ("paintedHoverBackground", True, record["paintedHoverBackground"]),
        ) if expected != native
    }


before = json.loads((root / "gates/fixtures/layout-rail/hover-before.json").read_text())
current = json.loads((out / "rail-pointer-states.json").read_text())
for records in (before["records"], current):
    assert len(records) == 24
    assert {(record["expanded"], record["element"]["key"]) for record in records} == {
        (expanded, f"rail.icon.{index}") for expanded in (False, True) for index in range(1, 13)
    }
old_failures = [record for record in before["records"] if pointer_differences(record)]
assert len(old_failures) == 22, before
for record in old_failures:
    print(f'LAYOUT_OLD_RAIL_HOVER_REJECTED {before["revision"]} {record["element"]["key"]} expanded={record["expanded"]}', flush=True)
new_failures = [record for record in current if pointer_differences(record)]
hover = {"beforeRevision": before["revision"], "beforeFailedStates": len(old_failures), "afterFailedStates": len(new_failures), "beforeFieldDifferences": sum(len(pointer_differences(record)) for record in before["records"]), "afterFieldDifferences": sum(len(pointer_differences(record)) for record in current)}
(out / "rail-hover-comparison.json").write_text(json.dumps(hover, indent=2) + "\n")
assert not new_failures, new_failures
for record in current:
    if record["expanded"]:
        assert record["label"]["color"] == "#8e9093", record
print(f'LAYOUT_RAIL_HOVER_OK: {hover["beforeFailedStates"]} -> {hover["afterFailedStates"]} failed pointer states; {hover["beforeFieldDifferences"]} -> {hover["afterFieldDifferences"]} field differences', flush=True)
