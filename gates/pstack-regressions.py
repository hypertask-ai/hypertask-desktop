"""Run on GitHub only. Negative controls for real desktop receipt/discovery traps."""
import copy
import importlib.util
from pathlib import Path
import shutil
import tempfile
import unittest

ROOT = Path(__file__).resolve().parent.parent


def load(name):
    spec = importlib.util.spec_from_file_location(name, ROOT / "gates" / f"{name}.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


receipt = load("desktop-ci-receipt")
integration = load("pstack-check")


class ReceiptRegression(unittest.TestCase):
    def setUp(self):
        self.sha = "a" * 40
        self.run = {"head_sha": self.sha, "path": ".github/workflows/ci.yml",
                    "status": "completed", "conclusion": "success"}
        self.jobs = [{"name": n, "conclusion": "success"} for n in receipt.REQUIRED_JOBS]
        self.artifacts = [{"name": n, "expired": False, "size_in_bytes": 1} for n in receipt.REQUIRED_ARTIFACTS]

    def test_valid_receipt(self):
        receipt.validate(self.sha, self.run, self.jobs, self.artifacts)

    def test_wrong_head(self):
        with self.assertRaisesRegex(ValueError, "Stale SHA"):
            receipt.validate("b" * 40, self.run, self.jobs, self.artifacts)

    def test_three_platforms_are_not_full_ci(self):
        for name in ("windows-arm64", "Windows memory receipt and budget"):
            with self.subTest(name=name):
                jobs = copy.deepcopy(self.jobs)
                next(j for j in jobs if j["name"] == name)["conclusion"] = "failure"
                with self.assertRaisesRegex(ValueError, name):
                    receipt.validate(self.sha, self.run, jobs, self.artifacts)

    def test_workflow_failure_is_not_hidden_by_green_platforms(self):
        self.run["conclusion"] = "failure"
        with self.assertRaisesRegex(ValueError, "not completed success"):
            receipt.validate(self.sha, self.run, self.jobs, self.artifacts)

    def test_skipped_memory_and_expired_artifacts(self):
        jobs = [j for j in self.jobs if j["name"] != "Windows memory receipt and budget"]
        with self.assertRaisesRegex(ValueError, "Windows memory"):
            receipt.validate(self.sha, self.run, jobs, self.artifacts)
        self.artifacts[0]["expired"] = True
        with self.assertRaisesRegex(ValueError, "expired proof artifact"):
            receipt.validate(self.sha, self.run, self.jobs, self.artifacts)

    def test_other_workflow_cannot_supply_proof(self):
        self.run["path"] = ".github/workflows/bench.yml"
        with self.assertRaisesRegex(ValueError, "desktop CI workflow"):
            receipt.validate(self.sha, self.run, self.jobs, self.artifacts)


class IntegrationRegression(unittest.TestCase):
    def test_current_and_broken_skill_discovery(self):
        integration.check(ROOT)
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            shutil.copytree(ROOT / ".claude", root / ".claude", symlinks=True)
            shutil.copyfile(ROOT / "AGENTS.md", root / "AGENTS.md")
            shutil.copytree(ROOT / "docs", root / "docs")
            for path in ("crates", "gates", ".github", "fixtures"):
                (root / path).symlink_to(ROOT / path, target_is_directory=True)
            integration.check(root)
            (root / ".claude/skills/poteto-mode").unlink()
            with self.assertRaisesRegex(AssertionError, "Expose complete skill directory"):
                integration.check(root)
            print("PSTACK_BROKEN_DISCOVERY_CONTROL_REJECTED")

    def test_claude_fallback_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            shutil.copytree(ROOT / ".claude", root / ".claude", symlinks=True)
            models = root / ".claude/rules/pstack-models.md"
            models.write_text(models.read_text().replace("bug-fix: gpt-6.1-sol high", "bug-fix: opus high"))
            with self.assertRaisesRegex(AssertionError, "Codex high role missing: bug-fix"):
                integration.check(root)
            print("PSTACK_CLAUDE_FALLBACK_CONTROL_REJECTED")


if __name__ == "__main__":
    unittest.main(verbosity=2)
