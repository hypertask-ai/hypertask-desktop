"""GitHub-only release packaging and trust-boundary regressions."""
import copy
import hashlib
import importlib.util
import os
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("release", Path(__file__).with_name("updater-release.py"))
release = importlib.util.module_from_spec(spec)
spec.loader.exec_module(release)


class ReleaseTests(unittest.TestCase):
    def event(self):
        return {"repository": {"full_name": release.REPO}, "workflow_run": {
            "head_repository": {"full_name": release.REPO}, "event": "push", "head_branch": "main",
            "status": "completed", "conclusion": "success", "path": ".github/workflows/ci.yml",
            "head_sha": "abcdef123456" + "0" * 28, "run_number": 42,
        }}

    def test_only_successful_main_push_is_releasable(self):
        self.assertEqual(release.release_identity(self.event(), "0.1.0"), "v0.1.0+42.abcdef123456")
        for key, value in [("event", "pull_request"), ("event", "workflow_dispatch"),
                           ("head_branch", "updater"), ("conclusion", "failure"),
                           ("conclusion", "cancelled"), ("status", "in_progress"),
                           ("path", ".github/workflows/other.yml"),
                           ("head_repository", {"full_name": "attacker/repo"}),
                           ("head_sha", "bad"), ("run_number", 0)]:
            event = copy.deepcopy(self.event())
            event["workflow_run"][key] = value
            with self.subTest(key=key, value=value), self.assertRaises(ValueError):
                release.release_identity(event, "0.1.0")
        print("UPDATER_RELEASE_SOURCE_VERIFIED")

    def test_both_architectures_and_manifest_match_real_files(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            inputs = root / "inputs"
            for index, (artifact, _) in enumerate(release.ASSETS):
                source = inputs / artifact / "ht-desktop.exe"
                source.parent.mkdir(parents=True)
                source.write_bytes(b"MZ" + bytes([index]) * 1234)
            output = root / "output"
            tag = release.package(self.event(), "0.1.0", inputs, output)
            self.assertEqual(tag, "v0.1.0+42.abcdef123456")
            lines = (output / "SHA256SUMS").read_text().splitlines()
            self.assertEqual(len(lines), 2)
            for line, (artifact, asset) in zip(lines, release.ASSETS):
                digest, name = line.split("  ")
                self.assertEqual(name, asset)
                self.assertEqual((output / asset).read_bytes(), (inputs / artifact / "ht-desktop.exe").read_bytes())
                self.assertEqual(digest, hashlib.sha256((output / asset).read_bytes()).hexdigest())
            (inputs / release.ASSETS[1][0] / "ht-desktop.exe").unlink()
            with self.assertRaises(ValueError):
                release.package(self.event(), "0.1.0", inputs, root / "partial")
            self.assertFalse((root / "partial").exists())
        print("UPDATER_RELEASE_ASSETS_AND_SHA256_VERIFIED")


if __name__ == "__main__":
    if os.environ.get("GITHUB_ACTIONS") != "true":
        raise SystemExit("Run only on GitHub Actions")
    unittest.main(verbosity=2)
