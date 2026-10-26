# Visual parity round 4

## Acceptance checks, before implementation

1. Baseline is main 2ceb32559117 and its public CI windows-visual3 capture, with unchanged gates/fixtures/visual3. Retain all original ledger identities. Starting inventories are Inbox 14, Board 14, Task 15, total 43. Require at least 20 fewer rows from product changes alone.
2. Compare exact-head hosted Windows screenshots and native layout dumps with the read-only frozen /home/valentin/projects/hypertask-desktop-wt-public/internal/live-graphite-1920 references. Preserve gates/layout-compare.py and every threshold. No fixture, reference, hiding, or content shortcuts.
3. Every correction cites component and token file:line from /home/valentin/projects/hypertask/src. Each correction adds or updates a native layout regression pinning the source value. Unfixable rows retain identity and exact source-backed reason.
4. AI sidebar crop x1500..1919 must have zero changed pixels on Inbox, Board, Task versus main capture. No AI sidebar, keyboard shortcut, or command center code changes.
5. Verify only on public GitHub-hosted runners. No VPS builds, tests, lint, app launches, test copies, or laptop access. One-off comparisons remain separate from permanent CI and post their results on the draft PR.
6. Fetch and rebase origin/main before each push. Every required job and artifact must verify against the final exact HEAD through the supplied CI receipt script. Report job conclusions, test names, capture deltas, SHA, PR and CI URLs.

## Execution units

1. Establish immutable main artifact identity and enumerate all remaining row identities.
2. Fix largest source-backed layout group and native regression; commit and obtain hosted proof.
3. Continue source-backed groups with hosted before/after checks until reduction reaches 20, or document exact blockers without dropping rows.
4. Exact-head full CI receipt, unchanged-sidebar image proof, protected-path diff, and PR evidence.

Principle Prove It Works chooses actual hosted captures and executable native regressions. Principle Sequence Verifiable Units chooses component commits and exact-head hosted verification. Self-review only; unavailable Cursor-only tools use repository-native evidence instead.

Owned files are this document, inbox/board/task portions of crates/ht-desktop/src/ui.rs and their native layout tests, plus narrowly required source-backed main-content styling. AI/sidebar, shortcuts, command center, fixtures, references, thresholds and other helpers' files are excluded.
