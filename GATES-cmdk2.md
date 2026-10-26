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
