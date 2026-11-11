import sys
from pathlib import Path

from PIL import Image, ImageChops

THEMES = ("porcelain", "graphite", "amoled", "dia")
SCREENS = ("inbox", "board", "task")


def percent(live, native):
    left = Image.open(live).convert("RGB")
    right = Image.open(native).convert("RGB")
    if left.size != right.size:
        right = right.resize(left.size)
    diff = ImageChops.difference(left, right)
    pixels = left.size[0] * left.size[1]
    changed = sum(1 for pixel in diff.getdata() if pixel != (0, 0, 0))
    return 100 * changed / pixels


def main():
    live_root = Path(sys.argv[1])
    native_root = Path(sys.argv[2])
    missing = False
    for theme in THEMES:
        for screen in SCREENS:
            live = live_root / theme / f"{screen}.png"
            native = native_root / theme / f"{screen}.png"
            if not live.exists() or not native.exists():
                print(f"PIXEL {theme} {screen} MISSING")
                missing = True
                continue
            score = percent(live, native)
            print(f"PIXEL {theme} {screen} {score:.2f}")
    if missing:
        return 1
    print("PIXEL_SCORES_OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
