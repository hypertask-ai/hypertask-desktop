"""Inspect public GitHub CI for an exact SHA. Never build, test, or drive locally."""
import argparse
import json
import os
import re
import subprocess

REPO = "hypertask-ai/hypertask-desktop"
REQUIRED_JOBS = (
    "Pstack contract", "ubuntu-24.04", "windows-2025", "macos-15",
    "windows-arm64", "Windows memory receipt and budget",
)
REQUIRED_ARTIFACTS = (
    "pstack-contract", "desktop-input", "layout-dumps", "web-source-fonts",
    "windows-memory", "windows-memory-heavy", "hypertask-desktop-windows",
    "hypertask-desktop-linux", "hypertask-desktop-mac-arm64",
    "hypertask-desktop-windows-arm64",
)


def validate(sha, run, jobs, artifacts):
    if run["head_sha"] != sha:
        raise ValueError("Stale SHA: verify the current pushed head, not origin/main or an earlier run")
    if run["path"] != ".github/workflows/ci.yml":
        raise ValueError("Use the desktop CI workflow, not a similarly named workflow")
    if run["status"] != "completed" or run["conclusion"] != "success":
        raise ValueError("CI is not completed success: inspect --log-failed and fix before shipping")
    for name in REQUIRED_JOBS:
        matches = [j for j in jobs if j["name"] == name]
        if len(matches) != 1 or matches[0]["conclusion"] != "success":
            raise ValueError(f"Required job is absent, skipped, or failed: {name}")
    for name in REQUIRED_ARTIFACTS:
        matches = [a for a in artifacts if a["name"] == name and not a["expired"]]
        if len(matches) != 1 or matches[0]["size_in_bytes"] <= 0:
            raise ValueError(f"Missing, empty, duplicate, or expired proof artifact: {name}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("sha", help="full current pushed commit SHA")
    parser.add_argument("--run", type=int, help="inspect a specific run instead of the newest matching CI")
    args = parser.parse_args()
    if not re.fullmatch(r"[0-9a-f]{40}", args.sha):
        parser.error("pass a full 40-character commit SHA")
    env = {k: v for k, v in os.environ.items() if k not in ("GH_TOKEN", "GITHUB_TOKEN")}

    def api(path):
        return json.loads(subprocess.check_output(
            ["/usr/bin/gh", "api", f"repos/{REPO}/{path}"], env=env, text=True))

    if args.run:
        run = api(f"actions/runs/{args.run}")
    else:
        runs = api(f"actions/runs?head_sha={args.sha}&per_page=100")["workflow_runs"]
        runs = [r for r in runs if r["path"] == ".github/workflows/ci.yml" and r["head_sha"] == args.sha]
        if not runs:
            raise SystemExit("No CI run for this SHA. Push and open/update its PR first.")
        run = max(runs, key=lambda r: r["id"])
    run_id = run["id"]
    jobs = api(f"actions/runs/{run_id}/jobs?per_page=100")["jobs"]
    artifacts = api(f"actions/runs/{run_id}/artifacts?per_page=100")["artifacts"]
    validate(args.sha, run, jobs, artifacts)
    print(f'CI_RUN {run_id} attempt={run["run_attempt"]} {run["html_url"]}')
    for job in jobs:
        print(f'CI_JOB {job["name"]}: {job["conclusion"]}')
    for artifact in artifacts:
        print(f'CI_ARTIFACT {artifact["name"]} id={artifact["id"]} digest={artifact.get("digest", "unavailable")}')
    print(f'DESKTOP_CI_VERIFIED {args.sha} {run["html_url"]}')


if __name__ == "__main__":
    main()
