# HTPR-7381: remaining-client follow-up

This is a partial continuation of draft PR #17, **not whole-ticket parity**.
The original acceptance inventory is `.unlazy/prewire/GATES.md`; the current
continuation inventory is `.unlazy/prewire/continuation-gates.md` (both local,
uncommitted).
`FEATURES.md` remains uncommitted and carries the final SHA-bound CI receipt.
No application, build, test, test copy, or laptop command ran on the VPS.
Only GitHub Actions is runtime proof; source inspection is not live SaaS QA.

## Additional wired portions

| Portion | Web contract | Native / CI coverage |
| --- | --- | --- |
| AI session create, rename, delete and delete-all; comment branching | `src/utils/api/ai_chat/aiChatHelpers.ts:109-142`: POST `/api/ai-chat/create-session` with optional `taskId,projectId`; POST `/api/ai-chat/update-session` with `sessionId,title`; DELETE `/api/ai-chat/delete-session?delete=<id|ALL>`, no request body. `src/app/detail/[...slug]/useTaskDetailCommandActions.tsx:94-118` creates a session before inserting the focused quote. | `ht-core/src/ai_sessions.rs`, `ht-desktop/src/ai_sessions.rs`, `ai_workspace.rs`, `comment_actions.rs`. Durable mutation queue, account verification, cached optimism, rollback on 401/403, atomic persisted source draft/quote continuation, authoritative server ID/title, late-navigation rejection. Unknown create acknowledgements are never automatically replayed. Core tests `lifecycle_routes_are_durable_optimistic_and_rollback_both_refusals`, `uncertain_create_is_never_replayed_after_restart`, `durable_chat_continuation_is_atomic_account_scoped_and_survives_acknowledgement`; GPUI branch/refusal/recovery/newer-draft tests. The web hooks log/refetch on rejected rename/delete rather than showing a success toast (`usePagedChatHistory.ts:306-339`); native likewise rolls back without a success claim. Delete-all success text appears only after acknowledgement (`generalCommandActions.ts:110-123`). |
| AI team/recent-board scope and entity search | POST `/api/teams/getAllSidebar` with `{userId}`, GET `/api/flags`, GET `/api/ai/chat/mention?param=<query or all>[&projectId=…]`. `AI_Tiptap_Container.tsx:536-685`, `sortBoardsByRecent.ts:3-44`, `MentionData.js:8-26`, `MentionList.tsx:49-101`, `useAiChatState.ts:139-176`, `aiChatSend.ts:161-184`. | `ht-core/src/ai_context.rs`, `ht-desktop/src/ai_context.rs`. Real team IDs, stable team/board recency, twelve recent board IDs, independent docked/fullscreen scopes, server-flag-gated Current ticket, all four mention kinds, server headings, nonselectable error/empty rows, exact mention/context payloads, stale-response discard and explicit context removal. People mentions are inline only, matching the web callback. A denied search displays “Error loading results”, not an empty success. Scope-query refusal retains the previous successful team list. `ht-core/tests/ai_context.rs`, GPUI context tests and wire tests cover acceptance/401/403. Fullscreen history-rail geometry and full AI row parity are not claimed. |
| Configured model catalog and team-scoped preference (subset) | GET `/api/teams/getTeam`, `/api/teams/aiFeatureModels`, `/api/teams/aiProviderSettings`, `/api/teams/byokKeys` with `teamId`; GET `/api/users/preferences`; POST preferences with `{aiModelPreferences:{aiChat:<id>},aiModelPreferencesTeamId:<team>}`. `useAiModelPreference.ts:56-160`, `useTeamAiProviders.ts:34-65`, `useTeamCustomEndpoint.ts:28-67`, `aiModelPreferences.ts:49-119`, `ModelSelectorDropdown.tsx:145-191,594-630`. | `ht-core/src/ai_models.rs`, public catalog/alias JSON extracted from `aiModelOptions.ts`, `preferences.rs`, `ht-desktop/src/ai_models.rs`. Provider filtering, custom-endpoint configuration/GDPR conditions, saved aliases, team/global/feature/board fallback and server-evaluated effective default; no provider secrets or billing data fabricated. Preference writes are durable, optimistic and rolled back on denial. Known-team configuration refusal cannot silently send the old hardcoded picker model. Core/GPUI model tests and real TCP wire fixtures cover each read and the preference write. **Not full A4:** native model/effort submenu presentation, no-current-board last-team resolution, and deployment-specific legacy price mapping remain open. Unknown legacy billing is `None`, not guessed Free/Pro/BYOK. |
| Save-view modes and view management (subset) | `SaveViewModal.tsx:47-79,125-197`: current/default/team/private/reset. POST `/api/projects/views/create-view`, `/update-view`, `/reset-to-default`; POST rename `/delete-rename-view`; DELETE same route with `viewId,projectId` query. Order POST `/update-order`, `/reset-order`, `/set-default-order` (`viewOrder.ts:5-43`). | `ht-core/src/board_views.rs`, `queue.rs`, `ht-desktop/src/board_views.rs`. Actual `userId` on create; queued current/team/private/default/reset actions; name validation; rename; delete confirmation; alphabetical/reset/default order controls; authoritative order stored in web `view_order`; web refusal messages/rollback and preserved name input. Manage views is reachable through its existing Ctrl+K command. `ht-core/tests/board_views.rs`, `save_view_menu_uses_all_five_web_flows_and_keeps_name_on_both_refusals`, and bearer/empty-DELETE-body wire fixtures. **Not the full view row:** see remaining work below. |

All newly introduced HTTP families send the desktop bearer to the web route,
without browser cookies, MCP route substitution, or a sidebar-agent identity.
Tests run in the existing GitHub workspace test jobs; no CI check is bypassed.

## HTPR-7381 continuation: additional native wiring

The following is additional wiring, not a claim that every row is complete.
Current SHA-bound GitHub CI evidence remains in uncommitted `FEATURES.md`.

- Model picker: web provider/family declaration order; nearest-effort family
  selection; separate Instant/Thinking and Light/Standard/High submenus;
  reset-to-plan default; custom model labels; provider/GDPR availability;
  BYOK/premium locking; docked/fullscreen effort-label visibility. No-current-
  board resolution reads actual sidebar teams, chooses the last team only if
  still a member, otherwise the first team. Team-scoped preference mutations
  retain optimistic rollback. Public production legacy billing identifiers are
  now verified, not guessed: see [web-billing-prices.md](web-billing-prices.md).
  Sources: `ModelSelectorDropdown.tsx:303-327,594-771`,
  `AI_Tiptap_Container.tsx:380-391`, `useAiChatModelPreference.ts:16-44`.
- Advanced filter wiring: CreatedBy/UpdatedBy/BlockedByPerson; date presets,
  typed-date payloads and two-month custom range with Back/Confirm; overall
  ANY/ALL and tag/assignee value matching; unread/inbox/staleness/running-timer
  predicates. People and agents use `/api/members/getAllForAssignees` and
  `/api/agents/all`; running timers use `/api/time/running`. Refused rosters do
  not become fake agent choices. Filter edits use full queued unsaved-view
  bodies. Source: `FilterHelperFunctions.ts:19-325`,
  `UpdatedAtRange/index.tsx:75-142,182-235`, `useAgents.ts:31-45,284`.
- Sorting: all fourteen web modes, three-level stack, per-level order/removal,
  duplicate/cap exclusion and searchable choices. Staleness commands preserve
  the secondary stack. Source: `BoardPriorityMode.tsx:29-180,235-244`.
- View rendering: actual current-view column visibility, hide-current-column,
  subtask card-list filtering and personal empty-column visibility, including
  its distinct `updateMode: personal-empty-sections` response. Builtin My Tasks,
  Overdue, Blocked, Agents, current/next cycle and flag-gated Decisions use real
  actor/dependency/cycle/done-column metadata; they are session-local, not fake
  API writes. Saved-view refusal restores the prior builtin without replacing
  a newer selection. Sources: `builtinViews.ts:35-179`,
  `useKanbanViews.ts:157-182,279-368`, `SubtaskHelperFunction.ts:22-57`.
- Expanded create form: title, rich description, board/column, priority, size,
  tags, human/agent assignment and due/start dates, one durable optimistic
  `POST /api/tasks/createGlobally`, no uncertain replay; rejection removes the
  temporary task and preserves all form edits. Bearer identity and real board
  metadata are required online. Source:
  `useCreateTaskModalStates.ts:585-621,634-666`,
  `createTaskGloballycontroller.ts:85-127`.
- Additional actual Ctrl+K handlers: Create task, Sort board, Filters, Filter
  values: match any or all, Sort by time in column, Sort by last comment,
  Manage board columns, Hide board column, Sub-task Settings, and empty-column
  visibility; C / Shift+S / Shift+F preserve editor typing. These are working
  handlers, not a full registry claim.

All newly introduced HTTP behavior has accepting/401/403 mocked CI cases;
local builtin predicates intentionally make no HTTP mutation. Proof does not
include successful live SaaS authorization or whole-app visual equivalence.
The CI suite also retains the prior section-only-cache contract/hydration
regressions; a missing Project row is not an explicitly empty column view.

## Still required native work

These remain missing implementation, not just server authorization:

- Full Ctrl+K: remaining navigation/settings/preferences effects, board/team
  administration, uploads, billing/provider controls, tutorial lifecycle,
  templates/relations/recurrence/time and unsupported agent families. Neither
  hidden commands nor opening a generic page counts as implementation.
- Views: archived-task hydration/rendering; smart splits; view-tab hide/empty
  preferences and drag order; complete column reordering/layout management;
  nested subtask card-property rendering (list visibility modes are wired).
  Natural-language date search is not the web's full Sugar parser; timer reads
  are loaded for filter activation, not a full realtime timer subscription.
- Create: flagged empty-ticket fill, typed-board routing and remaining writer
  feature flags, attachment staging/finalization, full custom-date calendar in
  the create form, and source-comment/parent/duplicate expansion paths.
- Full picker/settings handoff: team-targeted native Plans/API Keys settings
  are not implemented; external web links cannot set the browser's persisted
  selectedSettingsTeamId. That gap must not be hidden behind a full-row Done.
  Full web-vs-native visual acceptance is not claimed by method-level GPUI
  tests or the existing generic layout contract.
- Untouched AI rows still include fullscreen history rail, attachments,
  agent-side conversations and the boundaries in `ai-parity.md`.

Session lifecycle changes remain serialized and stream-addressed by server
session ID. No mocked 200 is proof of live SaaS authorization.
