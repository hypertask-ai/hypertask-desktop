# AI sidebar round two acceptance

Owned paths: `docs/ai-parity.md`, `crates/ht-desktop/src/ai_sidebar.rs`,
`ai_message_tests.rs`, `ai_presentation.rs`, `ai_workspace.rs`, and narrowly
necessary AI-only core/shortcut code. Other helpers own unrelated surfaces.

## Checks written before implementation

1. Inventory every inspected web sidebar behavior in the existing ledger.
   Each row cites `/home/valentin/projects/hypertask/src` file:line evidence,
   labels native match/differs/missing, names native proof or retained blocker.
   Existing A1-A9 remain blocked unless current route/auth source disproves them.
2. Fix every unblocked differs/missing row. Add native behavior tests per fix,
   including real GPUI keys/pointers for interactive changes. Test observable
   geometry, clipboard, draft/focus, stream/error results, not source strings.
3. Preserve 420px default, source-derived resize bounds, chat/draft continuity,
   multiline input, server refusals, incomplete-stream errors and scroll safety.
   No invented provider success, browser cookies, ungrounded UI or flag state.
4. GitHub current-head workspace tests run all new regressions on hosted runners.
   Inspect named test results in Linux CI logs. No local builds/tests/lint,
   no test copies and no owner-laptop commands.
5. Fetch/rebase main before every push. Small co-authored commits on aiside2;
   draft PR targets main. All required current-head jobs and retained artifacts
   verified using the owner's once-per-run wait/receipt command. Report each
   job, SHA, PR/run links; synthetic proof never clears live-auth blockers.

## Execution plan

- Blocking first steps: source audit and acceptance ledger, then native fixes.
- Independent workstreams: n/a, single owner; other helpers use other worktrees.
- Shared mutable state: AI-only files; no shared working tree or credentials.
- Smallest safe decomposition: retain Chat/ActiveTurn/Presentation models;
  source-backed UI/input corrections plus focused regression tests.
- Review: serial self-review; no independent reviewer claimed. Unbundled
  cursor-team-kit checks unavailable; use source/diff review and hosted CI.
- Proof principle: exact-head CI receipt, never local execution.
- Sequence principle: inventory, focused implementation/tests, CI repair only.

## Evidence

Pending source audit, implementation and current-head hosted CI.
