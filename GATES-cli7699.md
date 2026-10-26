# CLI7699 acceptance checks

Written before implementation. Scope: inbox, board, task, AI sidebar, keyboard shortcuts. Proof runs only on public GitHub CI. No VPS builds/tests/lint/app launches, test copies, laptop access, cookie copying, live writes, or flag changes.

## Checks

1. Read merged web commit 489e9ea5203d85bf9904ac631fc389b382558791 and current `/home/valentin/projects/hypertask/src` source. Cite exact flag, human CLI resolver, methods and routes. Distinguish GET defaults from POST creation and plain quick entry.
2. Ctrl+J uses the web compose/writer request and native web-equivalent controls. A mocked response proves actual native shortcut, request, saved result and retained draft on refusal. Preserve pre-existing implemented behavior rather than duplicate it.
3. Focused comment star/pin sends exact Private/Public body. Mock success and removal, flag-off refusal, uncertain response and account identity protection. No fake success or unsafe toggle retries.
4. Comment @mention search sends actual authorized search parameters. Native input and mocked mixed results prove selection, HTML/notification identity, stale result handling, and a visible server refusal.
5. N/Shift+C and full-create routes are enabled only if merged web code accepts human CLI auth on the exact write. Otherwise retain the blocker, with exact method/route/auth citations; no replacement UI.
6. Update `docs/compose-comments.md`, `docs/keyboard-parity.md`, and `docs/cmdk-round2.md` with precise implemented/partial/blocked status and source citations. No broad Done claims or invented UI.
7. Fetch/rebase origin/main before every push. Open requested draft PR. Exact-head CI must show every required job successful and nonexpired proof artifacts, including native input and both Windows memory receipts. Inspect feature test output and collect read-only CI receipt.

## Plan / throughput checkpoint

- Blocking first steps: source/auth audit, existing native flow audit, acceptance checks before edits.
- Independent workstreams: serial implementation and self-review; other helpers own separate worktrees. No new delegates authorized.
- Shared mutable state: only this worktree/branch; reuse existing queue and modal state. Rebase before every push.
- Smallest safe decomposition: preserve already-implemented contracts; repair missing behavior/tests, then refresh stale ledgers. Source/diff self-review replaces unavailable cursor-team-kit controls, never a claimed independent review.

## Data shape / design

Reuse `desktop_cli::Request`, durable `compose_create` / comment saved mutations, native Compose state and mention results. Refusals must remain errors through transport, queue, and visible UI. Compare exact route authorization before enabling any new creation shortcut. Do not create a second transport or modal framework.

## Evidence

Pending exact-head GitHub CI. Mock transport acceptance is not live owner-account authorization. The owner reports the flag enabled; no production probe is authorized by this CI-only task.
