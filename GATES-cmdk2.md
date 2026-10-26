# Command center round 2 acceptance

Scope: inbox, board, task, AI sidebar, shortcuts. Owned paths: command-center assembly/dispatch, focused native tests, docs/cmdk-round2.md, this ledger. Reuse existing operations; no web changes, task creation, quick entry, VPS execution or laptop access.

## Acceptance checks (written before implementation)

1. Audit every historical Blocked, Unsupported and Partial inventory row. Record web declaration, dispatcher/handler, called server route or local state, native capability, and precise missing field/route when omitted. Cite only /home/valentin/projects/hypertask/src for web behavior. Out-of-scope rows remain explicitly excluded. Evidence: docs/cmdk-round2.md.
2. Every newly viable command uses the web label, declaration/group order, context and behavior; reuses native authenticated operations; no inert or invented UI. Evidence: focused native GPUI input tests exercise Ctrl+K selection and observe picker/state/queue effects, including missing-context controls.
3. Preserve blocked create/quick entry (HTPR-7699), unavailable auth contracts and authoritative feature flags. Evidence: source audit plus native omission tests.
4. Exact final pushed SHA passes every required GitHub CI job, native input/layout proof, ARM64, updater, pstack and frozen Windows memory checks. Evidence: read-only receipt and job list from GitHub, never local builds/tests/lint/launches.
5. Draft PR targets main from cmdk2; fetch/rebase before every push; commits end with requested co-author. Evidence: git log, PR URL and exact-head CI URL.

## Throughput checkpoint

- Blocking first steps: source audit and acceptance contract before implementation.
- Independent workstreams: n/a, single helper owns command assembly and native proof.
- Shared mutable state: dedicated cmdk2 worktree; no other helper files touched.
- Smallest safe decomposition: audit, command wiring/tests, exact-head CI correction. Existing Command/CommandAction and Action registry remain the data shape. No new operation layer.

## Execution constraints

Repo pstack skills read. Unbundled cursor-team-kit unavailable; use serial source/diff self-review and GitHub native checks. No independent review claimed.

## Evidence

Pending source audit and GitHub CI.

## Audit and first CI findings

- Historical inventory: 151 unresolved declarations, 145 unique keys. Full row ledger and route/field distinctions: docs/cmdk-round2.md.
- Seven viable native actions selected. Existing CommandAction/Action and durable board/task queues reused. Added only desired inbox card action and nullable parent field to those queues.
- CI f66b77e403926060885fd064d0ef4a78271cda75: https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40279865752. Compile rejection on all three native platforms: E0433, missing Status import in the new test. This is compilation evidence, not behavioral red proof. Import corrected in follow-up.
- Required source limitation: no full native UI claim for child picker, history feed, selected-content AI branch, description-history preview, or arbitrary-task reminders. The audit distinguishes exact missing fields from native implementation gaps. No false API blockers.

## Second source pass

- Selected-comment branch is not API-blocked. Web creates a new chat and inserts an attributed quote (`app/detail/[...slug]/useTaskDetailCommandActions.tsx:97-120`). Reuse native new-chat and quote operations, including both web Comment/AI labels.
- Summaries are normal chat sends, not the cached-summary expansion action. Exact prompts and busy guards are in `hooks/MultiPages/AIChat/useCommentToAiChat.ts:93-129`. Reuse the existing sender with current task context; add native input and actual HTTP-request assertions against a synthetic refusal server. No live model answer claimed.
- This raises viable command keys from seven to ten. The audit preserves remaining complete-child-picker, writer-editor, feed/version and server-contract gaps, rather than calling every MCP primitive equivalent.

- CI 8e60339f053f9f4424d92b77dc390f4a396beec1: https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40281044840. All desktop targets reject serde_json::Map::pointer (E0599). Replaced with typed Map::get then Value::get. No local compiler or test executed.

## Behavioral CI correction

- CI 692f4e23833d10a3e2b9da6834859b3d0501a464: https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40281832093. All three native platforms reach core tests. `command_move_to_inbox_is_desired_state_and_account_bound` rejects its malformed-ack expectation at board_writes.rs:447. Eight sibling board tests pass. Core intentionally retries invalid success responses; desired-state Inbox mutation is safe to retry. Corrected expected Retrying and explicitly require no success refetch on that invalid acknowledgement.
- Added nullable-parent exact-request/restart coverage and summary busy/history draft preservation. Web New badges now distinguish Comment branch from the unbadged AI alias. No acceptance threshold or product behavior weakened.
