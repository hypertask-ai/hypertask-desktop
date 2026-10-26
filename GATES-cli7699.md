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

Reuse `desktop_cli::Request`, durable `compose_task` / comment saved mutations, native Compose state and mention results. Refusals must remain errors through transport, queue, and visible UI. Compare exact route authorization before enabling any new creation shortcut. Do not create a second transport or modal framework.

## Evidence

Pending exact-head GitHub CI. Mock transport acceptance is not live owner-account authorization. The owner reports the flag enabled; no production probe is authorized by this CI-only task.

## Source audit and checks to inspect in CI

- Web PR https://github.com/hypertask-ai/hypertask/pull/2028, merged commit `489e9ea5203d85bf9904ac631fc389b382558791`, adds only the four endpoint names in `src/lib/auth/desktopCliTokenUser.ts:7`. Claim validation is `:14-29`; verified user/flag is `:53-77`. Flag definition `src/lib/flags/definitions/htpr-7699-desktop-cli-token.ts:3-10` defaults Owner+QA. Source root `/home/valentin/projects/hypertask`.
- POST writer `src/app/api/ai/task-writer/route.ts:52-61`; GET defaults `src/pages/api/tasks/createGlobally.ts:17-44`; POST comment-only saved `src/pages/api/savedContent/toggleSaved.ts:16-23`; GET mentions `src/pages/api/tasks/searchByParam.ts:5-9` all use that resolver. Existing permission checks remain.
- N/Shift+C's exact POST saves are `src/hooks/MultiPages/useAddDeleteTaskInBoards.tsx:127-148,169-179`. Both remain session-only (`src/lib/api/task-writes/route.ts:35-38,101-106`, global POST dispatch `src/pages/api/tasks/createGlobally.ts:113-115`). No full quick-add UI is enabled.
- `cli_flag_off_and_plan_refusals_keep_server_errors_without_creating_tasks` checks defaults/writer SSE/plan/star/pin refusals, unchanged stored state and no create/replay.
- `writer_body_matches_web_board_scope_without_copying_provider_secrets` checks the complete expected body, including board provider flags and null-team fallback. Web body `src/lib/ai/composeTask.ts:111-128`; provider fields `src/lib/deriveCurrentBoardBilling.ts:32-36`.
- `cli7699_native_ctrlj_mocked_writer_and_flag_off_keep_real_results` drives Ctrl+J, text, Ctrl+Shift+B, Escape, Enter, then drains the real queue against mocked writer/default/create responses. Checks navigation only after acknowledgement and retained note on defaults/writer refusal.
- `cli7699_native_star_pin_mocked_save_remove_and_flag_off_are_authoritative` drives both real comment shortcuts through queue save 200, removal 201 and flag-off 401. Checks exact scope/body, stored saved state and error.
- `cli7699_native_mentions_apply_mocked_http_results_and_refusals` types @QA, captures an actual native HTTP request to a local CI-only mock server, applies its real parsed response, then drives arrow/Enter or Enter/Escape. Checks bearer/no cookie, query/project, skipped headings, task-link HTML, unchanged text on refusal and no send.
- `cli7699_compose_flag_off_paints_the_server_error_and_keeps_the_note` and `cli7699_mention_http_refusal_paints_the_web_empty_result_without_inserting` check rendered nonzero label bounds. Mention HTTP errors map to empty results in the web (`src/components/RTE/MentionData.js:13-41`), rather than invented suggestions.
- Existing retry/restart/account/toggle tests remain active. No borrowed receipt or hand-written success setter is used for the new action-result tests.

Self-review: existing transport/queue/modal ownership retained. Alternative replacement transport/UI rejected. Unavailable cursor-team-kit tools were not claimed. No new explanatory application comments or lint suppressions were added. CI owns every executable check.
