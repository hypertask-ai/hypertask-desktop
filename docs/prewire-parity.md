# HTPR-7381: full-sign-in client prewire

This draft is **not whole-ticket parity**. HTPR-7699's server authorization is
approved, not deployed for every route. Existing blocker ledgers remain useful
inventories; “cookie-only” is no longer a reason to omit native work. This file
records exactly which portions now have client wiring and which still need it.
No VPS builds, test execution, test copies, app launches, or laptop access.
Runtime evidence comes only from the draft PR's GitHub Actions runs.

## Wired paths (GitHub CI receipt required before calling ready)

| Requested portion | Actual web route/body | Native behavior and CI tests |
| --- | --- | --- |
| Ranked card drag/drop, p2 B1 | PUT `/api/tasks/moveTask`, `{section_title,sectionId,taskId,ranking,projectId}`. Web `src/utils/api/global/apiHelpers/moveTaskHandler.ts:6-24`, `src/utils/generateRank.ts:12-59`. | Real GPUI drag targets cards/column space, numeric rank between neighbors, durable optimistic section/rank, restart and denial rollback. Native reads order by rank. `ranked_web_drop_is_optimistic_durable_and_rolls_back_both_denials`, `denied_ranked_move_does_not_revert_newer_title_and_cannot_stack_stale_rollbacks`, `real_card_drag_to_another_column_queues_ranked_web_move_without_opening_task`. Keyboard moves still use the existing MCP subset; do not relabel all p2 B1 rows. |
| Inline top task creation, p2 B3 | POST `/api/tasks/create`, or POST `/api/tasks/createGlobally` when active view supplies defaults. Web `src/hooks/MultiPages/useAddDeleteTaskInBoards.tsx:95-192`, `src/utils/helperFunctions/Views/NewTaskViewDefaults.ts:8-32`. | Existing column + opens inline title entry. Exact user/board/section/ranking/index and filter defaults; durable pending card disappears on denial, title stays editable. No navigation/drag on placeholder IDs. `inline_create_uses_web_route_body_active_filters_ranking_and_rolls_back_denial`, `column_add_is_real_inline_input_and_denial_keeps_title_without_phantom_card`. |
| Compose save, shell B26/compose B7 | GET defaults, POST `/api/ai/task-writer`, POST `/api/tasks/createGlobally`; **no MCP create substitution**. Web `src/lib/ai/composeTask.ts:225-279`. | Preserve returned top ranking/section, description, writer due date/labels/custom fields and view defaults. Accept `{newTask}`, not MCP's envelope. Denial retains the original note and shows the web's create failure. `compose_writer_defaults_and_create_are_real_durable_queue_operations`, `compose_web_create_refusal_never_reports_success_or_creates_a_task`, `writer_custom_fields_survive_json_and_html_without_leaking_markers_into_description`. Flagged empty-task filling, typed-ticket column routing and expanded create-modal controls remain incomplete. |
| Switcher team groups and favorite drag/order, shell B2/B17 | **POST**, not GET, `/api/teams/getAllSidebar` with `{userId}`; GET `/api/favorites/getFavorites`; PUT `/api/favorites/reorderFavorites` with `{favorites:[{projectId}]}`. Web `src/utils/api/Homepage/index.ts:17-31`, `src/components/sidebars/leftSidebar.tsx:94-152,378-448`, `src/lib/teamSwitcherOrder.ts:5-29`. | Rail and Ctrl+B share authoritative string team IDs, group ordering, favorite slots (0 is tenth), search, Recent, queued replacement and rollback. No guessed board/team join. `switcher_reads_actual_post_teams_and_get_favorites_and_never_guesses_on_denial`, `favorite_replacement_is_durable_optimistic_and_rolls_back_401_and_403`, `ctrl_b_groups_authoritative_teams_and_favorites_and_queues_real_reordering`. Ten-slot Manage Favorites, favorite collapse and favorite keyboard-slot actions remain incomplete. |
| Core filters/sort/saved views, p2 B2/B4 | POST `/api/projects/boardTasks` `{projectId,userId}` is the current web active-board read; POST `/api/projects/views/unsaved-view` sends full settings plus `baseViewId`; saved-view switch/create use `/switch-view`, `/create-view`. Web `src/utils/api/Homepage/index.ts:105-146`, `src/hooks/Homepage/Views/useKanbanViews.ts:163-249`. | UI supports core Priority/Size/Tags/Assignees filters, primary sorts/order, named-view tabs and private Save view. Full settings preserve inherited/null fields; denied writes restore view state. Queue contracts additionally cover `/update-view`, `/update-order`, `/set-default-order`, `/reset-order`, `/reset-to-default`, but those additional contracts are **not claims of reachable complete UI**. `unsaved_sort_and_filter_use_full_web_body_and_rollback_refusals_after_restart`, `saved_view_and_order_routes_accept_the_web_response_shapes_or_refuse_without_success`, `sort_picker_queues_full_settings_and_refusal_rolls_back_without_closing_picker`. |
| Paged AI history, AI A2/F614 subset | GET `/api/ai-chat/all-sessions?compat=htpr-6924&limit=30[&cursor=…]`; transcript `?compat=htpr-6924&sessionId=…`. Web `src/utils/api/ai_chat/aiChatHelpers.ts:90-103`, `src/hooks/MultiPages/AIChat/usePagedChatHistory.ts:52-67,104-131`. | Lazy worker-thread page/transcript reads, real cursors, deduplication, legacy full-transcript response when supplied by server, no empty-chat success after denial, stale transcript intent discarded on new-chat/account changes. `paged_history_and_transcripts_use_web_queries_with_bearer_and_preserve_denials`, `history_pages_append_without_duplicates_and_denied_transcripts_do_not_open_empty_chats`. |

HTTP fixture tests in `ht-core/tests/http_cli.rs` verify bearer headers, the real
web paths, unchanged JSON, and 200/401/403 responses for all newly wired route
families. A refused gated web route does not revoke a valid desktop MCP login;
MCP `/user/context` refusals still use the existing sign-in invalidation path.

Web creation has no idempotency implementation in
`src/lib/api/task-writes/route.ts:27-105`. Sending an Idempotency-Key header alone
cannot make replay safe. Native queue records an attempted compose create and
never automatically resends an uncertain create, including after restart.

## Still blocked by missing native work, not merely server authentication

- **Full board filter/view rows:** advanced date/actor/staleness/timer filters,
  filter match controls, sorting stacks, archived-task hydration, full subtask
  rendering, column visibility/order UI, built-in tabs, smart splits,
  rename/delete/default/reset/order management. Core route tests are not full-row
  implementation. Human assignee filter metadata still needs the web member
  shape rather than assuming every MCP member has the web `uid` discriminator.
- **Full Ctrl+K (shell B1/B9/B20/B21):** durable preference controls, expanded
  board settings, attachment staging/finalization, and tutorial lifecycle/step
  execution have not been implemented here. Existing supported commands remain;
  no inert “full list”, successful upload, or locally completed tour is added.
- **AI A2/A3:** explicit server create/rename/delete/delete-all and durable
  comment-to-chat branching still need native queued handlers. Existing local
  new-chat behavior is not claimed as successful server session creation.
- **AI A4/A5:** configured model/provider/plan availability, durable model
  preferences, and full team/recent-board scope/entity search are not implemented.
  Existing model/context controls remain a subset. Never infer feature flags,
  provider credentials, billing eligibility, or board/team associations.
- **All untouched blocker rows** in shell/AI/p2/compose ledgers retain their
  existing partial/blocker classifications. They are not made ready by approval
  of server work or by another row's passing mocked tests.

No live sign-in acceptance is claimed. All full-login live writes still depend
on HTPR-7699; a green CI receipt proves the mocked client contracts, not SaaS QA.
