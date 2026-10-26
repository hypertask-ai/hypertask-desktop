"""Hosted Windows visual proof, preserving the frozen comparer and workloads."""
import json
import os
from pathlib import Path
import re
import subprocess
import sys

from PIL import Image, ImageChops

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "evidence/visual3-proof"
BASE = "3d57a0d87e58c1cdeeee11b6f800ab5569bdf58f"
HEAD = os.environ.get("HT_VERIFY_REVISION")
if (os.environ.get("GITHUB_ACTIONS") != "true"
        or os.environ.get("RUNNER_ENVIRONMENT") != "github-hosted" or os.name != "nt"):
    raise SystemExit("Visual proof runs only on public GitHub-hosted Windows")
assert HEAD and HEAD != BASE
OUT.mkdir(parents=True, exist_ok=True)


def compare(name, directory):
    normalized = OUT / name
    normalized.mkdir()
    for screen in ("inbox", "board", "task"):
        data = json.loads((directory / f"{screen}-3.json").read_text(encoding="utf-8-sig"))
        assert (data["width"], data["height"]) == (1920, 1009)
        (normalized / f"{screen}.json").write_text(json.dumps(data), encoding="utf-8")
    result = subprocess.run([
        sys.executable, str(ROOT / "gates/layout-compare.py"),
        str(ROOT / "gates/fixtures/layout-reference-live"), str(normalized), "1920",
    ], check=True, capture_output=True, text=True)
    print(f"CAPTURE {name}\n{result.stdout}", flush=True)
    (OUT / f"{name}.txt").write_text(result.stdout, encoding="utf-8")
    counts = {screen: int(count) for screen, count in re.findall(r"== (inbox|board|task): (\d+) mismatches", result.stdout)}
    assert set(counts) == {"inbox", "board", "task"}
    return counts


def unchanged_sidebar(before, after, label):
    counts = {}
    for screen in ("inbox", "board", "task"):
        with Image.open(before / f"{screen}.png") as old, Image.open(after / f"{screen}.png") as new:
            assert old.size == new.size == (1920, 1009)
            difference = ImageChops.difference(
                old.convert("RGB").crop((1500, 0, 1920, 1009)),
                new.convert("RGB").crop((1500, 0, 1920, 1009)),
            )
            counts[screen] = sum(pixel != (0, 0, 0) for pixel in difference.getdata())
            assert counts[screen] == 0, f"{label}/{screen}: changed sidebar pixels {counts[screen]}"
            print(f"VISUAL3_SIDEBAR {label} {screen} changed_pixels={counts[screen]}", flush=True)
    return counts


paths = {name: ROOT / "evidence" / directory for name, directory in {
    "round2_dense": "visual3-round2-dense", "current_dense": "windows-memory",
    "round2_matched": "windows-visual3-before", "current_matched": "windows-visual3",
}.items()}
for name, directory in paths.items():
    receipt = json.loads((directory / "memory.json").read_text(encoding="utf-8-sig"))
    expected = BASE if name.startswith("round2") else HEAD
    assert receipt["Revision"] == expected, (name, receipt)
    # The capture harness records its checkout, not the downloaded binary's build.
    assert receipt["BuildRevision"] == (HEAD if name == "round2_matched" else expected), (name, receipt)
receipts = {name: json.loads((directory / "memory.json").read_text(encoding="utf-8-sig")) for name, directory in paths.items()}
assert receipts["round2_matched"]["ExeSHA256"] == receipts["round2_dense"]["ExeSHA256"]
assert receipts["current_matched"]["ExeSHA256"] == receipts["current_dense"]["ExeSHA256"]
counts = {name: compare(name, directory) for name, directory in paths.items()}
assert counts["round2_dense"] == {"inbox": 24, "board": 24, "task": 23}, counts
reduction = sum(counts["round2_dense"].values()) - sum(counts["current_matched"].values())
assert reduction >= 25, f"Only {reduction} fewer original measured rows"
sidebars = {
    "dense": unchanged_sidebar(paths["round2_dense"], paths["current_dense"], "dense"),
    "matched": unchanged_sidebar(paths["round2_matched"], paths["current_matched"], "matched"),
}

# Each bound comes from the existing frozen reference; its 1.5px limit is unchanged.
for screen, keys in {
    "board": ["board.column.0", "board.column.1", "board.column.2", "board.card.0", "board.card.1", "board.card.0.title"],
    "task": ["task.header", "task.title", "task.description", "task.composer"] + [f"task.meta.{i}" for i in range(6)],
    "inbox": [f"inbox.tab.{i}" for i in range(5)] + [f"inbox.row.0.{key}" for key in ("title", "kind", "author")],
}.items():
    live = {e["key"]: e for e in json.loads((ROOT / f"gates/fixtures/layout-reference-live/{screen}.json").read_text())["elements"]}
    native = {e["key"]: e for e in json.loads((OUT / f"current_matched/{screen}.json").read_text())["elements"]}
    for key in keys:
        delta = max(abs(native[key][field] - live[key][field]) for field in ("x", "y", "w", "h"))
        assert delta <= 1.5, (key, delta, native[key], live[key])
        print(f"VISUAL3_NATIVE_BOX {key} max_delta={delta}", flush=True)

subprocess.run(["git", "fetch", "--no-tags", "--depth=1", "origin", BASE], cwd=ROOT, check=True)
protected = [
    "crates/ht-desktop/src/ai*", "crates/ht-desktop/src/keyboard*",
    "crates/ht-desktop/src/shortcuts.rs", "crates/ht-desktop/src/input.rs",
    "crates/ht-desktop/src/shell_shortcuts.json", "gates/layout-compare.py",
    "gates/fixtures/layout-reference-live", "fixtures/demo",
    "gates/windows-memory-budget.json", "gates/windows-memory-heavy-budget.json",
]
subprocess.run(["git", "diff", "--exit-code", BASE, HEAD, "--", *protected], cwd=ROOT, check=True)
report = {"sha": HEAD, "baseline": BASE, "measured_rows": counts, "reduction_including_content_alignment": reduction,
          "sidebar_changed_pixels": sidebars,
          "original_inventory": {name: {screen: count + 6 for screen, count in rows.items()} for name, rows in counts.items()},
          "qualification": "Matched-content reduction is not a product fix; identical-fixture before/after counts stay separate."}
(OUT / "receipt.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
print(f"VISUAL3_PROVEN reduction={reduction} sidebar_pixels=0 protected_code_unchanged", flush=True)
