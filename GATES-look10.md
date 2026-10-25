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

## Focused source-backed correction

The existing task header branches on summary presence only when rendering the summary, but always uses 8px bottom padding. Web authority at source revision 003a872014c35eeba1ab21047cdfdf70160e654e, src/components/PageComponents/TaskDetail/TopRow/BaseTitleContainer.tsx:35-47, selects pb-3 without a summary and pb-[8px] with a summary. src/styles/tailwind-entry.css:7-8 imports Bootstrap in the first cascade layer. Its important pb-3 rule wins, making the no-summary bottom padding 1rem (16px), not Tailwind's nominal 12px. This is the same imported-utility precedence already retained for mt-3 in gates/fixtures/layout-rail/source.html. Dense task 58006 has no summary. This is a real state-specific spacing defect, even though its long title and comment thread also differ from the frozen live task.

Before changing production code, add a focused native layout regression in the existing task_fields summary_tests module. Require header height to equal 36px top padding plus max(37px minimum title row, measured title height) plus 16px without a summary, for both short and wrapping titles. The 36px is web pt-9 at BaseTitleContainer.tsx:44; the 37px minimum is TaskTitle.tsx:210; the 16px is the imported pb-3 rule selected at BaseTitleContainer.tsx:46 and tailwind-entry.css:7-8. Record the failing hosted output, then change only the existing bottom-padding expression. Preserve summary-present reference geometry, the frozen measurements, all comparison outputs and thresholds. The data shape stays the existing summary presence and measured title/header boxes, with no new product abstraction.

## Baseline

CI https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40243346848, SHA 303d27bb9c62ea3b6c8dd521ea136f5cab1db2fa. Downloaded layout-dumps artifact 12617763359 to /tmp/look10-before. compare-complete-dense.txt reports inbox 13, board 17, task 14, TOTAL 44. compare-complete-parity.txt reports inbox 4, board 5, task 1, TOTAL 10. compare-complete-reference.txt reports zero on every screen. Artifact digest is sha256:a082c1c613ea25ddfefa1aed2fca216609f2d237e1659d7d6dc89bd07e2a7758.

## Hosted red proof

CI https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40246073718 at 6e48862abcaecc242389f025f7c1ec3ded841c23 rejects task_header_without_summary_keeps_web_bottom_spacing on Ubuntu, Windows x64 and macOS. The measured task.header height is 81px, while the source-backed expected height is 89px. Each platform reports 203 passed and this one failed native test. ARM64, Pstack contract and both updater jobs succeed. Missing memory receipts follow the intentionally failing workspace test, not a separate product failure. Full logs and failed receipt are retained in /tmp/look10-red-app-receipt.

## Final proof location

Final exact-head job results, supplied ci-wait-receipt.sh receipt, artifact identities and unfiltered comparison totals are attached to https://github.com/hypertask-ai/hypertask-desktop/pull/35 after hosted verification. The separate audit is docs/layout-round10.md. Self-review preserves the one-expression product change, source-backed native regression, all existing measurement fixtures, comparisons, controls, thresholds and memory budgets. No independent reviewer ran. The unavailable cursor-team-kit review is replaced with source/diff review and hosted checks, per the repository adapter.
