"""Hosted-only source mutations: four missing effects, then restored native proof."""
import hashlib
import importlib.util
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "evidence/windows-input"


def main():
    spec = importlib.util.spec_from_file_location("windows_input", ROOT / "gates/windows-input.py")
    suite = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(suite)
    suite.hosted_only()
    source = ROOT / "crates/ht-desktop/src/ui.rs"
    app = ROOT / "target/release/ht-desktop.exe"
    original = source.read_bytes()
    original_digest = hashlib.sha256(app.read_bytes()).hexdigest()
    newline = b"\r\n" if b"\r\n" in original else b"\n"
    marker = b"    fn enqueue(&mut self, write: Write, cx: &mut Context<Self>) -> bool {" + newline
    assert original.count(marker) == 1, "Write entry point changed; review the rejection controls"
    cases = {
        "inbox-archive-notification": "Write::InboxArchive { .. }",
        "board-rename-f2": "Write::TaskTitle { .. }",
        "task-description": "Write::TaskPatch { patch: ht_core::TaskPatch::Description(_), .. }",
        "comment-send-stay": "Write::TaskComment { .. }",
    }
    assert cases.keys() == suite.CASES.keys(), "Every migrated case needs a production-effect break"
    block = "        if let Some(case) = match &write {\n"
    for case, pattern in cases.items():
        block += f'            {pattern} => Some("{case}"),\n'
    block += '''            _ => None,
        } {
            eprintln!("NATIVECI_BROKEN {case}");
            return true;
        }
'''
    OUT.mkdir(parents=True, exist_ok=True)
    proof = {"baseline_exe_sha256": original_digest,
             "source_sha256": hashlib.sha256(original).hexdigest(), "cases": list(cases)}
    with tempfile.TemporaryDirectory(prefix="ht-nativeci-control-") as tmp:
        saved = Path(tmp) / "ht-desktop.exe"
        shutil.copy2(app, saved)
        try:
            source.write_bytes(original.replace(marker, marker + block.encode().replace(b"\n", newline)))
            patch = subprocess.check_output(["git", "diff", "--", str(source)], cwd=ROOT)
            (OUT / "negative-control.patch").write_bytes(patch)
            with (OUT / "negative-build.log").open("w") as log:
                subprocess.run(["cargo", "build", "--release", "-p", "ht-desktop"], cwd=ROOT,
                               stdout=log, stderr=subprocess.STDOUT, check=True)
            proof["broken_exe_sha256"] = hashlib.sha256(app.read_bytes()).hexdigest()
            assert proof["broken_exe_sha256"] != original_digest, "Mutated executable did not change"
            with (OUT / "negative-input.log").open("w") as log:
                subprocess.run([sys.executable, ROOT / "gates/windows-input.py", "--expect-broken",
                                "--out", OUT / "broken"], cwd=ROOT,
                               stdout=log, stderr=subprocess.STDOUT, check=True)
            print((OUT / "negative-input.log").read_text(), end="")
        finally:
            source.write_bytes(original)
            shutil.copy2(saved, app)
            proof["source_restored"] = source.read_bytes() == original
            proof["executable_restored"] = hashlib.sha256(app.read_bytes()).hexdigest() == original_digest
            (OUT / "negative-control.json").write_text(json.dumps(proof, indent=2) + "\n")
        assert proof["source_restored"] and proof["executable_restored"], proof
    assert not saved.exists(), "Temporary control executable survived cleanup"
    with (OUT / "restored-input.log").open("w") as log:
        subprocess.run([sys.executable, ROOT / "gates/windows-input.py", "--out", OUT / "restored"], cwd=ROOT,
                       stdout=log, stderr=subprocess.STDOUT, check=True)
    print((OUT / "restored-input.log").read_text(), end="")
    for case in cases:
        baseline = json.loads((OUT / "baseline" / case / "receipt.json").read_text())
        broken = json.loads((OUT / "broken" / case / "receipt.json").read_text())
        restored = json.loads((OUT / "restored" / case / "receipt.json").read_text())
        assert baseline["passed"] and restored["passed"] and not broken["passed"], case
        assert baseline["revision"] == broken["revision"] == restored["revision"], case
        assert baseline["exe_sha256"] == restored["exe_sha256"] == original_digest, case
        assert broken["exe_sha256"] == proof["broken_exe_sha256"], case
    subprocess.run(["git", "diff", "--exit-code", "--", str(source)], cwd=ROOT, check=True)
    proof["restored_cases_passed"] = list(cases)
    proof["temporary_control_removed"] = True
    (OUT / "negative-control.json").write_text(json.dumps(proof, indent=2) + "\n")
    print("NATIVE_INPUT_CONTROLS_VERIFIED cases=4 source=restored executable=restored cleanup=complete")


if __name__ == "__main__":
    main()
