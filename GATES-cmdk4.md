# Command center round 4 acceptance

Written before implementation. Baseline origin/main 59ae364. Scope is Ctrl+K on inbox, board, task and AI sidebar only. No local builds, tests, lint, test copies, app launches or laptop access.

## Acceptance checks

1. Recheck every unresolved declaration from docs/command-center-parity.md, docs/cmdk-parity.md and docs/cmdk-round2.md against current web AllCommands.ts, commandDispatcher.ts and imports. Record each applicable command's label, desktop icon absence, group, order, shortcut, visibility and outcome. All web values and remaining missing routes/fields carry source file:line citations rooted at /home/valentin/projects/hypertask/src.
2. Compare current main data and writers, including PR 57 if merged. Implement each viable command using the existing typed Command/CommandAction, native operation and queue boundaries. Do not invent web commands or change unrelated layouts.
3. Every changed command gets native Ctrl+K/input tests observing actual state, selected identity, durable payload or visible result. Negative coverage rejects missing/foreign data and unavailable capabilities. Compile and execute these tests only in public GitHub CI.
4. Fetch and rebase origin/main before every push. Commit focused audit/implementation units with requested co-author trailer. Open draft cmdk4 -> main. Required exact-head jobs, artifacts and read-only desktop-ci receipt must succeed. Report each job, exact SHA and URLs; no live authenticated SaaS success inferred from synthetic tests.

## Work sequence

- Ground: read pstack/verification rules, historical ledgers, web declarations/dispatch and native data/writer paths.
- Sketch: preserve typed registry/actions. Compare extending existing palette entrypoints against creating new modal/transport layers; prefer existing entrypoints, without compatibility scaffolding.
- Acceptance and throughput checkpoint before coding.
- Implement and self-review focused command units. GitHub alone verifies executable changes.
- Rebase, draft PR, exact-head CI correction and receipt.

## Throughput checkpoint

- Blocking first steps: source audit and acceptance contract.
- Independent workstreams: other helpers own data fidelity, layouts and shortcuts; no delegated workers authorized here.
- Shared mutable state: dedicated cmdk4 worktree/branch, palette-owned edits only except minimal operation entrypoints.
- Smallest safe decomposition: single owner for coherent registry/context/dispatch/test changes. No parallel writers.

## Principles and verification limits

Prove It Works requires actual named CI output and exact-head receipt. Sequence Verifiable Units separates audit, supported workflows and corrections. Model the Domain preserves existing typed commands/pickers rather than inert registry rows. Repository host adapter replaces unavailable Cursor tooling with serial source/diff self-review. No independent review claimed.

## Evidence

Pending source audit and exact-head GitHub CI.

### Source audit and rejection control

- Current web AST: 299 declarations, 297 fixed registry keys, plus runtime query handover. All 300 existing declaration locations/labels match current source. Rechecked 75 unresolved current declarations and all 151 historical unresolved rows. Full dispositions, current source citations and before/after counts: docs/cmdk-round4.md.
- Initial regression SHA 2448430faabe43ab78134f08626838d56e1ed873: https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40408734872. Linux/Windows/macOS reject cmdk4_remove_subtask_uses_projected_children_without_loading_their_details with `web child projection is sufficient`. Linux reports 297 passed, 1 failed. macOS reports 296 passed, 1 failed. The exact-head receipt correctly refuses this red run; no failure was bypassed.
- The implementation removes the unnecessary child-detail cache prerequisite, not the parent projection/identity validation. Existing account-checked durable BoardWrite dispatch reuses the nullable parent update writer. Real acknowledgement clears child-parent aliases and removes only that child from the parent projection. No invented full child row before acknowledgement. No layout or frozen workload/reference/workflow edits.
- Native keyboard tests cover cold and cached children, exact label/group/hint/order, filtered child identity, acknowledgement and visible remaining picker rows. Negative tests cover malformed/self identities, contradictory cached parent and a changed projection before Enter. Core tests cover durable restart/reopen, exact POST body, wrong account, forbidden writes and wrong/non-detached acknowledgements.
- PR 57 remains open at source-audit completion; its proposed fidelity work does not add a command writer or capability endpoint. Fetch/rebase remains required before every push and completion check.
- Full working fixed keys 84 -> 85; full working declarations 85 -> 86; available fixed keys 90 -> 90. No new web key invented. Existing search/Compose/reminder/notification/fullscreen partial workflows remain partial.
