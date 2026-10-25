import json
import sys
from pathlib import Path

live_dir, native_dir = Path(sys.argv[1]), Path(sys.argv[2])
LOGICAL_WIDTH = int(sys.argv[3]) if len(sys.argv) > 3 else 1280
BOX = ("x", "y", "w", "h")
STYLE = ("color", "bg", "fontSize", "fontWeight", "lineHeight", "radius", "borderColor")


def load(path, logical_width):
    data = json.loads(path.read_text())
    scale = data["width"] / logical_width
    elements = {}
    for e in data["elements"]:
        e = dict(e)
        for k in BOX:
            e[k] = e[k] / scale
        elements[e["key"]] = e
    return data, elements


def norm(v):
    return v.lower() if isinstance(v, str) else v


total = 0
for screen in ("inbox", "board", "task"):
    live_data, live = load(live_dir / f"{screen}.json", LOGICAL_WIDTH)
    _, native = load(native_dir / f"{screen}.json", LOGICAL_WIDTH)
    rows = []
    for key, l in live.items():
        n = native.get(key)
        if n is None:
            rows.append((999, key, "missing in app"))
            continue
        box = max(abs(l[k] - n[k]) for k in BOX)
        if box > 1.5:
            rows.append((box, key, "box live " + " ".join(f"{l[k]:.0f}" for k in BOX) + " | app " + " ".join(f"{n[k]:.0f}" for k in BOX)))
        for k in STYLE:
            lv, nv = norm(l.get(k)), norm(n.get(k))
            if lv is None or nv is None:
                continue
            if isinstance(lv, (int, float)) and isinstance(nv, (int, float)):
                if abs(lv - nv) > 0.5:
                    rows.append((50, key, f"{k} live {lv} | app {nv}"))
            elif lv != nv:
                rows.append((50, key, f"{k} live {lv} | app {nv}"))
    for key in native.keys() - live.keys():
        rows.append((10, key, "only in app"))
    rows.sort(key=lambda r: -r[0])
    total += len(rows)
    print(f"== {screen}: {len(rows)} mismatches")
    for _, key, msg in rows:
        print(f"  {key}: {msg}")
print(f"TOTAL {total}")
