"""CI-only whole-box comparison of unchanged and content-matched layout fixtures."""
import json
import os
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "evidence/layout"
LIVE = ROOT / "gates/fixtures/layout-reference-live"
BOX = ("x", "y", "w", "h")

if os.environ.get("GITHUB_ACTIONS") != "true":
    raise SystemExit("Reference layout comparisons run only on GitHub Actions")

report = {}
for name, directory in (("dense", OUT), ("parity", OUT / "parity"), ("reference", OUT / "reference")):
    result = subprocess.run([sys.executable, str(ROOT / "gates/layout-compare.py"), str(LIVE), str(directory), "1920"], check=True, capture_output=True, text=True)
    (OUT / f"compare-{name}.txt").write_text(result.stdout)
    print(f"FIXTURE {name}\n{result.stdout}", flush=True)
    boxes = []
    for screen in ("inbox", "board", "task"):
        live = json.loads((LIVE / f"{screen}.json").read_text())
        native = json.loads((directory / f"{screen}.json").read_text())
        assert (native["width"], native["height"]) == (1920, 1009), native
        elements = {e["key"]: e for e in native["elements"]}
        for element in live["elements"]:
            other = elements[element["key"]]
            delta = max(abs(element[k] - other[k]) for k in BOX)
            if delta > 1.5:
                boxes.append({"screen": screen, "key": element["key"], "area": max(element["w"] * element["h"], other["w"] * other["h"]), "max_delta": delta, "live": {k: element[k] for k in BOX}, "native": {k: other[k] for k in BOX}})
    boxes.sort(key=lambda e: -e["area"])
    report[name] = {"differences": int(result.stdout.strip().splitlines()[-1].split()[1]), "boxes_by_area": boxes}
    for box in boxes:
        print(f'BOX_BY_AREA {name} {box["key"]} area={box["area"]:.2f} max_delta={box["max_delta"]:.4f}', flush=True)
(OUT / "reference-comparison.json").write_text(json.dumps(report, indent=2) + "\n")
print("LAYOUT_REFERENCE_MEASURED", flush=True)
