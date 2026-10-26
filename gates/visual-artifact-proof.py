"""Compare two retained CI captures without changing permanent CI or its oracle."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys

from PIL import Image, ImageChops

ROOT = Path(__file__).resolve().parent.parent
REPO = "hypertask-ai/hypertask-desktop"
SCREENS = ("inbox", "board", "task")


def command(*args):
    return subprocess.check_output(args, cwd=ROOT, text=True)


def api(path):
    return json.loads(command("gh", "api", f"repos/{REPO}/{path}"))


def capture(run_id, destination):
    run = api(f"actions/runs/{run_id}")
    assert run["path"] == ".github/workflows/ci.yml", run
    assert run["status"] == "completed" and run["conclusion"] == "success", run
    assert run["repository"]["full_name"] == REPO and not run["repository"]["private"]
    artifacts = api(f"actions/runs/{run_id}/artifacts?per_page=100")
    matches = [a for a in artifacts["artifacts"] if a["name"] == "windows-visual3"]
    assert len(matches) == 1 and not matches[0]["expired"] and matches[0]["size_in_bytes"] > 0
    artifact = matches[0]
    assert artifact["workflow_run"]["head_sha"] == run["head_sha"]
    assert artifact["created_at"] >= run["run_started_at"]
    command("gh", "run", "download", str(run_id), "-R", REPO, "-n", "windows-visual3", "-D", str(destination))
    receipt = json.loads((destination / "memory.json").read_text(encoding="utf-8-sig"))
    assert receipt["Revision"] == receipt["BuildRevision"] == run["head_sha"]
    assert all((s["ClientWidth"], s["ClientHeight"], s["Dpi"]) == (1920, 1009, 96) for s in receipt["Samples"])
    normalized = destination / "normalized"
    normalized.mkdir()
    for screen in SCREENS:
        data = json.loads((destination / f"{screen}-3.json").read_text(encoding="utf-8-sig"))
        assert (data["width"], data["height"]) == (1920, 1009)
        (normalized / f"{screen}.json").write_text(json.dumps(data), encoding="utf-8")
    transcript = command(sys.executable, str(ROOT / "gates/layout-compare.py"), str(ROOT / "gates/fixtures/layout-reference-live"), str(normalized), "1920")
    (destination / "comparison.txt").write_text(transcript)
    counts = {screen: int(count) for screen, count in re.findall(r"== (inbox|board|task): (\d+) mismatches", transcript)}
    assert set(counts) == set(SCREENS)
    print(f"RUN {run_id} sha={run['head_sha']} artifact={artifact['id']} digest={artifact.get('digest')}\n{transcript}", flush=True)
    return {"run": run, "artifact": artifact, "capture": receipt, "measured_rows": counts}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--before-run", required=True, type=int)
    parser.add_argument("--after-run", required=True, type=int)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    assert os.environ.get("GITHUB_ACTIONS") == "true" and os.environ.get("RUNNER_ENVIRONMENT") == "github-hosted", "Hosted GitHub proof only"
    args.output.mkdir(parents=True, exist_ok=False)
    before = capture(args.before_run, args.output / "before")
    after = capture(args.after_run, args.output / "after")
    for record in (before, after):
        command("git", "fetch", "--no-tags", "--depth=1", "origin", record["run"]["head_sha"])
    old, new = before["run"]["head_sha"], after["run"]["head_sha"]
    protected = [
        "crates/ht-desktop/src/ai*", "crates/ht-desktop/src/keyboard*",
        "crates/ht-desktop/src/shortcuts.rs", "crates/ht-desktop/src/input.rs",
        "crates/ht-desktop/src/shell_shortcuts.json", "crates/ht-desktop/src/command*",
        "crates/ht-desktop/src/shell_commands.rs", "crates/ht-desktop/src/palette*",
        "gates/fixtures", "fixtures", "gates/layout-compare.py",
        "gates/windows-memory-budget.json", "gates/windows-memory-heavy-budget.json",
    ]
    diff = command("git", "diff", old, new, "--", *protected)
    (args.output / "protected.diff").write_text(diff)
    assert not diff, "Protected code, fixture, or threshold changed"
    sidebar = {}
    pixels = {}
    for screen in SCREENS:
        with Image.open(args.output / "before" / f"{screen}.png") as old_image, Image.open(args.output / "after" / f"{screen}.png") as new_image:
            assert old_image.size == new_image.size == (1920, 1009)
            difference = ImageChops.difference(old_image.convert("RGB"), new_image.convert("RGB"))
            pixels[screen] = sum(pixel != (0, 0, 0) for pixel in difference.getdata())
            sidebar[screen] = sum(pixel != (0, 0, 0) for pixel in difference.crop((1500, 0, 1920, 1009)).getdata())
            difference.save(args.output / f"{screen}-difference.png")
            print(f"SIDEBAR {screen} changed_pixels={sidebar[screen]} full_image_changed_pixels={pixels[screen]}", flush=True)
            assert sidebar[screen] == 0, f"Sidebar changed on {screen}"
    inventories = {name: {screen: count + 6 for screen, count in record["measured_rows"].items()} for name, record in (("before", before), ("after", after))}
    reduction = sum(inventories["before"].values()) - sum(inventories["after"].values())
    report = {
        "before": before, "after": after, "original_inventory": inventories,
        "product_row_reduction": reduction, "target_met": reduction >= 20,
        "sidebar_changed_pixels": sidebar, "full_image_changed_pixels": pixels,
        "frozen_reference_sha256": {screen: hashlib.sha256((ROOT / f"gates/fixtures/layout-reference-live/{screen}.json").read_bytes()).hexdigest() for screen in SCREENS},
        "qualification": "All 24 unsampled rail keys and 18 screenshot-only original classes remain counted. A successful comparison receipt does not assert the 20-row target."
    }
    (args.output / "receipt.json").write_text(json.dumps(report, indent=2) + "\n")
    print(f"VISUAL_ARTIFACT_PROOF reduction={reduction} target_met={reduction >= 20} sidebar_pixels=0 protected_diff=empty", flush=True)


if __name__ == "__main__":
    main()
