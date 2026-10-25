"""Package only successful main CI artifacts; publication runs on GitHub."""
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import tomllib

REPO = "hypertask-ai/hypertask-desktop"
ASSETS = (
    ("hypertask-desktop-windows", "hypertask-desktop-windows-x64.exe"),
    ("hypertask-desktop-windows-arm64", "hypertask-desktop-windows-arm64.exe"),
)


def release_identity(event, version):
    run = event["workflow_run"]
    if (event["repository"]["full_name"] != REPO
            or run["head_repository"]["full_name"] != REPO
            or run["event"] != "push" or run["head_branch"] != "main"
            or run["conclusion"] != "success" or run["status"] != "completed"
            or run["path"] != ".github/workflows/ci.yml"):
        raise ValueError("Release requires successful main-push CI in this repository")
    if not re.fullmatch(r"[0-9a-f]{40}", run["head_sha"]):
        raise ValueError("Invalid source SHA")
    if not re.fullmatch(r"\d+\.\d+\.\d+", version) or int(run["run_number"]) <= 0:
        raise ValueError("Invalid version or build sequence")
    return f'v{version}+{run["run_number"]}.{run["head_sha"][:12]}'


def package(event, version, artifacts, output):
    tag = release_identity(event, version)
    sources = [(artifacts / artifact / "ht-desktop.exe", name) for artifact, name in ASSETS]
    for source, _ in sources:
        if not source.is_file() or source.stat().st_size == 0:
            raise ValueError(f"Missing executable: {source}")
        with source.open("rb") as stream:
            if stream.read(2) != b"MZ":
                raise ValueError("Not a Windows executable")
    output.mkdir(parents=True, exist_ok=False)
    hashes = []
    for source, name in sources:
        target = output / name
        shutil.copyfile(source, target)
        with target.open("rb") as stream:
            digest = hashlib.file_digest(stream, "sha256").hexdigest()
        hashes.append(f"{digest}  {name}\n")
    (output / "SHA256SUMS").write_text("".join(hashes), encoding="ascii")
    return tag


def main():
    if os.environ.get("GITHUB_ACTIONS") != "true":
        raise SystemExit("Release packaging runs only on GitHub Actions")
    event = json.loads(Path(os.environ["GITHUB_EVENT_PATH"]).read_text())
    version = tomllib.loads(Path("Cargo.toml").read_text())["workspace"]["package"]["version"]
    tag = package(event, version, Path("release-artifacts"), Path("release-assets"))
    with open(os.environ["GITHUB_OUTPUT"], "a") as stream:
        stream.write(f"tag={tag}\n")
    print(f'UPDATER_RELEASE_PACKAGED {tag} source={event["workflow_run"]["head_sha"]}')


if __name__ == "__main__":
    main()
