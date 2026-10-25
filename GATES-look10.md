# Look10 acceptance checks

Recorded before implementation. Base 303d27bb9c62ea3b6c8dd521ea136f5cab1db2fa (look9). Owned paths: focused native layout corrections, additive source-backed layout regression coverage, and look10 evidence. No VPS builds/tests/lint/app launches/test copies. No laptop access. All executable verification runs in public GitHub CI.

1. Retrieve baseline CI 40243346848 layout-dumps artifact. Preserve unchanged full comparisons: dense 44, sparse 10, content-matched reference 0. Record artifact identity and digest.
2. Trace every reported difference to native and /home/valentin/projects/hypertask/src file:line authority. Distinguish real geometry/style defects from different fixture content, absent data, and extra controls. Never change frozen measurements, fixtures, comparer tolerances or remove controls to lower counts.
3. Fix only source-proven layout defects. Add focused behavior-level coverage if a defect exists. Verify each focused unit through GitHub CI before further implementation. If no source-supported correction exists, report that directly rather than inventing dimensions or matching content.
4. Preserve inbox, board, task, AI sidebar and keyboard scope. Preserve auth blockers and partial coverage claims. Self-review only; no independent reviewer or unavailable cursor-team-kit claims.
5. Fetch/rebase origin/main before each push; rebase after look9 merges. Push look10; open draft PR to main. Require exact-head GitHub CI output and supplied ci-wait-receipt.sh receipt. Every required job must succeed; historical memory-baseline skip is not required.
6. Download exact-head layout-dumps; report before/after dense, sparse and content-matched reference counts with CI URLs, SHA, artifact IDs/digests, and result of every job. A lower count caused only by changed fixtures or content is not a layout improvement.

## Work sequence

- Baseline established from hosted native/browser outputs.
- Audit differences against source, preserving baseline.
- Correct real defects only; CI-verifiable unit per correction.
- Self-review source/diff; exact-head receipt and artifact comparison.

## Baseline

CI https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40243346848, SHA 303d27bb9c62ea3b6c8dd521ea136f5cab1db2fa. Downloaded layout-dumps artifact 12617763359 to /tmp/look10-before. compare-complete-dense.txt reports inbox 13, board 17, task 14, TOTAL 44. compare-complete-parity.txt reports inbox 4, board 5, task 1, TOTAL 10. compare-complete-reference.txt reports zero on every screen. Digest and final results pending read-only GitHub receipt.
