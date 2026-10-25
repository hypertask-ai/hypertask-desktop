# Look9 acceptance checks

Recorded before implementation. Owned paths: layout collector/comparison gates, their source-backed fixtures, focused native rail fixes if measurements require them, CI layout wiring, and look9 evidence. No builds, tests, lint, app launches or test copies on this VPS. No laptop access. All executable acceptance checks run in public GitHub CI; local GitHub queries only retrieve evidence.

1. Baseline. Download the original layout-dumps artifact for 72aeb6fb and the latest available badac636 run. Retain full dense, sparse and reference counts, identifying artifact ID, digest, SHA and run URL. Expect original reference 24 and shared boxes 0; measure rather than assume.
2. Collector. Preserve existing frozen records and comparer tolerances. Extend measurement to all thirteen rail keys on inbox, board and task using only /home/valentin/projects/hypertask/src authority with file:line citations. Hosted browser output must measure the previously missing 24 records, not copy native values or remove controls.
3. Rejection control. In CI, require the collector to reject missing keys and the comparison to reject a real perturbed field under unchanged tolerances. Retain raw measurements and comparison output as layout-dumps artifacts.
4. Visual outcome. Compare unchanged native reference-content dumps with the completed web measurement first. Fix every real mismatch revealed, in focused units. Final reference-content comparison must have no missing/extra keys and zero measured box/style mismatches for inbox, board and task. Report original and expanded counts separately; coverage alone is not a product improvement.
5. Source fidelity. Record every new expected value's web file:line authority and source revision. No new UI, no unrelated fixture changes, no revised thresholds or removed assertions.
6. Final CI. Fetch/rebase origin/main before each push. Draft look9 PR. Exact final SHA must have successful Pstack contract, ubuntu-24.04, windows-2025, macos-15, windows-arm64 and Windows memory receipt and budget jobs. Require the supplied ci-wait-receipt.sh receipt, artifact identity/digests and final layout counts from downloaded CI output.

## Work sequence

- Establish baseline and source-backed collector.
- Prove collector and negative controls through hosted CI.
- Correct each real measured mismatch, if any, then remeasure.
- Self-review source/diff; unavailable cursor-team-kit tools replaced with source review and existing CI.
- Require exact-head final receipt and report measured results.

## Evidence

Pending hosted outputs. No completion claims yet.
