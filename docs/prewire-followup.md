# HTPR-7381: remaining-client follow-up

This is a partial continuation of draft PR #17, **not whole-ticket parity**.
The acceptance inventory is `.unlazy/prewire/GATES.md` (local, uncommitted).
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

## Still required native work

These remain missing implementation, not just server authorization:

- Full advanced views: date/actor/staleness/timer filters and match controls,
  sorting-stack controls and remaining comparators; built-in tabs; archived
  hydration, subtask rendering, column visibility/order UI; smart splits;
  view-tab hide/empty preferences, drag order and complete layout management.
- Full create flows: expanded task modal, flagged empty-ticket fill and typed
  column routing, full attachment staging/finalization and writer-feature flags.
- Full Ctrl+K handlers: the remaining web declarations are not implemented by
  opening a generic settings page. Durable preferences and their actual render
  effects, expanded board/team settings, uploads, billing/provider controls,
  tutorial lifecycle/step execution and other unsupported command families
  remain open. No inert full-list registry is exposed.
- Full configured model picker: model/effort submenu presentation and the web's
  no-current-board last-team resolution remain native work. Legacy Stripe price
  IDs are deployment configuration absent from the inspected source; billing
  eligibility must not be guessed. The server still authorizes actual sends.
- Full AI rows still include fullscreen history rail, attachments, agent-side
  conversations and the untouched boundaries in `ai-parity.md`.

Session lifecycle changes are serialized. Streams are addressed by their server
session ID, including optimistic deletion snapshots, so a rejected deletion or
new-chat request cannot discard newer streamed text or redirect it to another
chat. Full-login live acceptance still needs the server rollout and live QA;
a mocked 200 is not proof of SaaS access.
