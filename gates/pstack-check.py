"""CI-only integrity, discovery, model-role, and feature-map contract."""
import hashlib
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parent.parent


def check(root):
    vendor = root / ".claude/pstack"
    manifest = json.loads((vendor / "UPSTREAM.json").read_text())
    assert re.fullmatch(r"[0-9a-f]{40}", manifest["revision"]), "Pin an upstream commit"
    assert manifest["license"] == "MIT"
    copied = {str(p.relative_to(vendor)) for folder in ("skills", "docs", "agents")
              for p in (vendor / folder).rglob("*") if p.is_file()} | {"LICENSE", "README.md"}
    assert copied == set(manifest["files"]), "Upstream inventory drift: update copied trees and manifest together"
    for name, digest in manifest["files"].items():
        assert list(hashlib.sha256((vendor / name).read_bytes()).digest()) == digest, f"Upstream changed: {name}"
    assert "MIT License" in (vendor / "LICENSE").read_text()
    skills = sorted((vendor / "skills").glob("*/SKILL.md"))
    for skill in skills:
        exposed = root / ".claude/skills" / skill.parent.name
        assert exposed.resolve() == skill.parent.resolve(), f"Expose complete skill directory: {skill.parent.name}"
        assert (exposed / "SKILL.md").read_bytes() == skill.read_bytes()
        assert re.search(rf"^name: {re.escape(skill.parent.name)}$", skill.read_text(), re.M), str(skill)
    models = (root / ".claude/rules/pstack-models.md").read_text()
    setup = (vendor / "skills/setup-pstack/SKILL.md").read_text()
    roles = re.findall(r"^([^#\n]+): (?:grok-|claude-).+$", setup, re.M)
    assert roles, "Upstream setup role parser found no roles"
    for role in roles:
        assert re.search(rf"^{re.escape(role)}: gpt-6\.1-sol high(?:, gpt-6\.1-sol high)*$", models, re.M), f"Codex high role missing: {role}"
    for path in ("AGENTS.md", ".claude/CLAUDE.md", ".claude/rules/pstack-host.md"):
        assert (root / path).is_file(), f"Missing host entrypoint: {path}"
    for path in ("docs/how-we-ship.md", "docs/pstack-review.md", "gates/desktop-input.py",
                 "gates/desktop-ci-receipt.py"):
        assert (root / path).is_file(), f"Missing verification deliverable: {path}"
    assert "@../AGENTS.md" in (root / ".claude/CLAUDE.md").read_text()
    assert ".claude/pstack/skills/poteto-mode/SKILL.md" in (root / "AGENTS.md").read_text()
    feature_dir = root / ".claude/skills/verify-desktop/features"
    index = (feature_dir / "README.md").read_text()
    entries = set(re.findall(r"\]\(\./([^/)]+\.md)\)", index))
    assert entries == {p.name for p in feature_dir.glob("*.md") if p.name != "README.md"}, "Feature index drift"
    assert 3 <= len(entries) <= 5, "Keep the initial map focused"
    for name in entries:
        text = (feature_dir / name).read_text()
        assert re.findall(r"^## (.+)$", text, re.M) == [
            "Sub-features", "How to get to it (user POV)", "Driving it with GitHub CI", "Gotchas"], name
        assert "Preconditions:" in text and "Proof:" in text, name
    skill = (feature_dir.parent / "SKILL.md").read_text()
    assert re.search(r"^name: verify-desktop$", skill, re.M)
    assert re.search(r"^description: .+", skill, re.M)
    for section in ("Launch", "Doctor", "Drive", "Evidence", "Cleanup", "Helpers"):
        assert f"## {section}" in skill, section
    for path in re.findall(r"<!-- source: ([^ ]+) -->", index):
        assert (root / path).exists(), f"Feature source disappeared: {path}"
    print(f"PSTACK_INTEGRITY_OK files={len(copied)} skills={len(skills)} roles={len(roles)} features={len(entries)}")


if __name__ == "__main__":
    check(ROOT)
