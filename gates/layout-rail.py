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
            abs(native[key] - value) > 1.5 if isinstance(value, (int, float)) else native[key] != value
        )}
        records.append({"screen": screen, "key": native["key"], "expected": expected, "native": native, "differences": differences})
receipt = {"sourceRevision": "6f9901764d355aba7bfc76e4e62ff255e0b68b31", "records": records, "differences": sum(len(record["differences"]) for record in records)}
(out / "rail-source-comparison.json").write_text(json.dumps(receipt, indent=2) + "\n")
assert receipt["differences"] == 0, receipt
print(f'LAYOUT_RAIL_SOURCE_OK: {len(records)} previously unsampled records; zero source-backed differences; frozen comparison unchanged', flush=True)
