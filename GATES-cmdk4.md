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
