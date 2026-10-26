# Desktop verification map

Preconditions: a draft PR in hypertask-ai/hypertask-desktop, exact pushed SHA,
public GitHub-hosted runners, synthetic fixtures only. Read ../SKILL.md for
launch, doctor and teardown. No VPS or laptop drive commands are authorized.

- [Inbox archive](./inbox.md) has native X11 input and durable SQLite proof.
- [Board and task editing](./editing.md) maps existing GPUI/queue tests and layouts.
- [AI and rich comments](./ai-comments.md) separates local behavior from auth blockers.
- [Presentation and memory](./presentation.md) maps typography/layout and frozen budgets.

<!-- source: crates/ht-desktop/src/input.rs -->
<!-- source: crates/ht-desktop/src/ui.rs -->
<!-- source: crates/ht-core/src/queue.rs -->
<!-- source: .github/workflows/ci.yml -->
<!-- source: docs/shell-parity.md -->
<!-- source: docs/p2-parity.md -->
<!-- source: docs/ai-parity.md -->
<!-- source: docs/compose-comments.md -->
<!-- source: docs/editor-formatting.md -->
<!-- source: docs/data-fidelity.md -->
<!-- source: docs/layout-parity.md -->
<!-- source: docs/memory-ci.md -->
<!-- source: docs/memory-heavy.md -->

This is a focused initial map, not the full feature inventory. A path without
real native driving remains a coverage gap, even when its unit tests pass.
Maintenance checks index equality, source links, actual CI steps and artifacts.

- [Silent Windows updates](./updater.md) maps release packaging and supervised swaps.

Hosted Windows migrations and rejection evidence: [coverage ledger](../../../../docs/coverage.md).
