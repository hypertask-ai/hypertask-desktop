# HTPR-7381: compose and focused comments

Full-sign-in client follow-up: see [prewire-parity.md](prewire-parity.md) for
newly wired portions and native gaps; historical auth blockers below are not
claims that server approval alone completes these rows.

Web contract inspected at `hypertask` commit `6f9901764d355aba7bfc76e4e62ff255e0b68b31`. Web paths below are relative to that repository. No web changes, authenticated application requests, VPS builds/tests/test copies, or owner-laptop access.

## HTPR-7699: native CLI contracts implemented; QA live authorization pending

Contracts re-read at server commit `489e9ea52`: `src/lib/auth/desktopCliTokenUser.ts`,
POST `/api/ai/task-writer`, GET `/api/tasks/createGlobally`, POST
`/api/savedContent/toggleSaved` (comment only), GET `/api/tasks/searchByParam`.
All four accept the verified human CLI bearer only while `htpr-7699-desktop-cli-token`
is enabled for that user. Cookie authentication is not added to desktop.

`compose.rs` implements Ctrl+J, Search/New Task tabs, Tab/Shift+Tab, Enter send,
Shift+Enter newline, board chip and searchable board picker. The actual web writer
body is built by `ht_core::desktop_cli::writer_request`, including `requestKind:
compose-task`; it is not the MCP draft writer or sidebar chat. Streamed HTML and
split JSON are parsed; unavailable writing retains the original note as web does.
The durable queue freezes writer output before retrying idempotent bearer task
creation. Errors preserve the compose note; navigation waits for acknowledgement.

GET defaults is unlocked, **POST createGlobally is not**. Saving therefore uses
existing `/api/mcp/tasks/create` with the returned section. That MCP contract
appends to a column and cannot reproduce the web's returned top `ranking`, custom
field values, or existing-empty-task fill. Those differences remain explicit;
this is not a claim that every flagged compose extension has full parity.

Named CI checks are in `crates/ht-core/tests/desktop_cli.rs`,
`crates/ht-core/tests/http_cli.rs`, and GPUI `compose::tests` / `mentions::tests`.
No builds, tests, test copies, or app launches run on the VPS or owner laptop.

### QA production blocker (not a desktop authentication workaround)

The supplied QA-owner state is user **4036**, `valentin+qa-owner@hypertask.ai`.
Its human CLI exchange succeeds and bearer GET comments for task 60253 on board
7283 returns 200. The unchanged Rust fixture probe returns **401** from defaults,
writer, mentions, star, and pin, so that account does not currently have the new
route authorization. The inherited probe's alternate
flag-QA identity **985** instead gets **404** for that task. No owner account,
board membership change, flag write, or cookie-authenticated contract request is
used to hide this mismatch; no live state was created by the failed seed attempt.

`internal/gates/ai-live-check.mjs --compose <CI compose-live-requests.json>` retains
the storageState → browser consent → one-time code → CLI token exchange flow.
It keeps the token in memory, uses bearer-only native fetch for contracts, and
prints status/short shapes only. Its optional `--seed` creates a temporary QA
comment only after defaults authorizes; pass that ID as CI `qa_comment_id` to
produce unchanged Rust request fixtures, the probe removes that comment after
star/pin cleanup, including on a failed contract. Never use fixture ID 7 as a real QA target without seeding.

## Comments: authorized writes and local behavior

All writes use `Db::enqueue`/the existing durable mutation queue; none bypass it from a GPUI callback.

| Behavior / inventory | Contract | Native / CI coverage |
|---|---|---|
| Reply to a specific comment, F580 / comment part of F127 | `src/hooks/Task Detail/useTaskDetailGlobalStates.ts:508-553`; `src/utils/helperFunctions/TaskDetail/index.ts:86-109`: author mention + `said` + `<blockquote data-quote-reply="true">…</blockquote>`, retaining quoted HTML. A collapsed comment opens first. | `comment_actions.rs`; GPUI `comment_reply_quotes_specific_comment_and_queues_html_not_agent_reply_id`; queue `quoted_reply_create_body_is_html_and_never_uses_agent_only_parent_id`. POST `/api/mcp/comments`, explicit `content_type:"html"`, `kind:"qa"`. `reply_to_comment_id` is agent-only (`src/lib/mcp/operations/comments/operation.ts:520-528`) and is intentionally not used for human quoted replies. |
| Edit own existing comment, F367 | `src/hooks/Task Detail/useTaskDetailGlobalStates.ts:569-595`; bearer PATCH `/api/mcp/comments/[comment_id]`, `validateMcpRouteAuth` alias, ownership at `src/lib/mcp/operations/comments/[comment_id]/operation.ts:56-65,213-224`. | Inline editor, Save/Cancel, Ctrl+Enter, selection/history; GPUI `comment_edit_save_cancel_and_ownership_are_real_desktop_actions`; queue edit, owner validation, restart and invalid-response tests; wire PATCH test. Existing marks/links/media are retained across text edits, not flattened into a new plain-text comment. |
| Delete confirmation, F112 | `src/components/Modals/commands/DeleteCommentById.tsx:53-64`: “Clicking confirm will delete this comment!”, Delete comment / Cancel. Bearer DELETE `/api/mcp/comments/[comment_id]`, same auth helper, `:386-399,460-478`. | Confirmation performs no write until explicit confirm; Escape/cancel leaves the thread intact. GPUI `comment_delete_requires_confirmation_and_escape_is_not_a_write`; queue denial rollback and DELETE wire tests. See permission gap below. |
| Expand one/all, F108/F109 (F639) | O excludes latest comment; Shift+O toggles all; at least the last non-activity comment stays open. `src/hooks/Task Detail/CommentAndDescriptionHooks/useCommentAndDescriptions.ts:525-533,622-629`; `src/utils/helperFunctions/TaskDetail/index.ts:47-62`. Preview limit 150 (`CollapsedCommentPreview.tsx:7`). | GPUI `comment_single_all_expansion_keeps_latest_open_and_does_not_fire_in_editor`; comments displayed chronologically, composer after thread. |
| Reaction picker / fast-like, F123/F124 (F582) | R/L (`src/hooks/Task Detail/CommentAndDescriptionHooks/useCommentAndDescriptions.ts:573-600`); thumbs-up `src/lib/contexts/CommentsContext/index.tsx:237-239`. Picker uses emoji-mart (`src/utils/emojiLoader.js:30-33`); same MIT data bundled, not a fabricated small list. | GPUI `comment_reaction_picker_search_and_fast_like_toggle_the_focused_comment`; full 1,870 base-emoji catalog, search, keyboard/mouse selection, toggling reaction chips, counts and names tooltip. POST `/api/mcp/comments/[comment_id]/reactions`, `validateMcpAuth`, explicit `{emoji,active}` (`src/app/api/mcp/comments/[comment_id]/reactions/route.ts:123-126`; `src/lib/mcp/comments/reactionHandler.ts:122-149`). Explicit state makes retries idempotent. Queue retry/later-optimism test. |
| Line break / undo / redo, F154/F161/F162 | `src/lib/constants/shortcuts.ts:226,233-234`; Tiptap StarterKit `src/components/RTE/Tiptap.ts:135-141`. | Comment, existing-comment and description inputs use a multiline-editor context. Shift+Enter/Enter insert a break, Ctrl+Z/Ctrl+Shift+Z undo/redo, selection and Unicode retained; IME commits grouped; loading/reset discards stale history. AI Enter-to-send is deliberately not rebound. GPUI `comment_and_description_editors_linebreak_undo_redo_do_not_send`, `editor_line_break_undo_redo_keys_preserve_unicode_and_selection`, `editor_ime_composition_is_one_undo_and_loaded_drafts_reset_history`. |

Identity comes from bearer **GET `/api/mcp/user/context`** (`src/lib/mcp/operations/user/context/operation.ts:34-43,182-186`). Identity lookup must not gate unrelated offline queue draining; existing worker regression test remains enabled.

### Remaining auth/permission blockers

* **F115 / F116 (comment star/pin)**: native focused-comment commands and
  Ctrl+Shift+S/P use the authorized `Private`/`Public` web body. Successful 200
  (`saved`) and 201 (`{}` removal) are handled without requiring MCP's `success`
  envelope. The non-idempotent toggle is never automatically replayed after an
  uncertain acknowledgement or expired process lease; denial is surfaced.
* **F587 (@mention search)**: native mixed people/model/agent/task/board/page
  results come from the actual bearer search route, not a cached-only substitute.
  Arrow/Enter and mouse selection retain web notification IDs and linkable task/
  page attributes; headings are skipped, stale results are ignored, Escape does
  not send, and insertion is one rich-editor undo transaction.
* These rows are implemented with CI coverage, **not live-QA Done** until the QA
  identity/flag mismatch above is resolved and the Rust-fixture probe succeeds.

* **Deleting other people's comments**: web POST **`/api/comments/deleteCommentById`** permits an accessible task's comment after **`getSessionUser`** (`src/pages/api/comments/deleteCommentById.ts:13-18,28-36`). MCP DELETE only permits own or configured HyperAI comments (`src/lib/mcp/operations/comments/[comment_id]/operation.ts:460-474`). No ownership bypass is attempted; full web delete permissions remain blocked by the cookie-only route. Native deletion is limited to the bearer route's allowed ownership.

### F578 is a mobile-only inventory affordance

The web **desktop** branch is `!_mbl` at `src/components/PageComponents/TaskDetail/CommentAndDescription/CommentContainer/NewCommentComponent.tsx:444-485`: an always-mounted, auto-growing editor, minimum editable height 80px (`src/styles/tiptap.module.scss:61-64`). The manual resize separator and expanded/collapsed viewport geometry are exclusively in the **mobile** `else` branch (`src/components/PageComponents/TaskDetail/CommentAndDescription/CommentContainer/NewCommentComponent.tsx:490-527`), not desktop. Native desktop now grows with content instead of clipping a fixed-height box. GPUI `desktop_composer_grows_with_content_without_inventing_mobile_resize_controls` checks growth. **F578's mobile expand/collapse/drag UI is not claimed done in a desktop app**; adding a desktop resize button would violate “never add UI the web app does not have.”

## Verification

Only GitHub CI runs builds, workspace/unit tests, GPUI test-context tests, and `xvfb-run … python3 gates/layout-dump.py`. The ignored local `internal/COMPOSE-GATES.md` binds final evidence to the pushed SHA and records status updates in ignored `internal/FEATURES.md`. Draft PR: https://github.com/hypertask-ai/hypertask-desktop/pull/7.

## composer2 follow-up: remaining rows and exact blockers

Web source remains pinned to `6f9901764d355aba7bfc76e4e62ff255e0b68b31`.
All new writes use `Db::enqueue`; GitHub CI is the only runtime oracle.
Previously Done/Blocked rows were inspected, not reimplemented.

### Implemented follow-up contracts

| Rows | Web contract and native CI coverage |
| --- | --- |
| F052, F501 | Bulk Remind and selected-count/Archive/Remind toolbar: `src/components/notifications/inboxSplit/bulk-actions-inbox.tsx:1`, `src/components/notifications/inboxSplit/index.tsx:570-587`. `inbox_row_reminder_targets_clicked_row_cancel_is_not_a_write_and_bulk_is_captured` exercises actual row clicks, cancellation, Shift+H and captured multi-row targets. Default **Regardless** is supported; the unavailable alternate condition is B12 below. |
| F053, F505 | Web search is **local**, not server-wide: `src/app/inbox/Inbox.tsx:236-254,285-292,317-324,839-866`. Inline Ctrl+F filters title/ticket, not comments or unrelated cached tasks; Escape restores the same split. `inbox_real_search_filters_title_ticket_and_escape_restores_split_without_navigation`. |
| F184 | `src/components/Modals/SwipeUnread/SwipeUnread.tsx:77-87,100-176,225-277,350-485`: unread snapshot, embedded task surface, Keep/Archive, Undo, left/right, Backspace/Ctrl+Z, Escape/Ctrl+K flush and caught-up close. `inbox_review.rs` reuses native task detail, defers archive writes until closing, and restores originating screen/split/focus. `unread_review_keeps_archives_undoes_and_flushes_only_decisions_on_escape`. |
| F497 | **Not auth-blocked**: bearer GET `/api/mcp/inbox/list` already returns the web `structuredData` (`src/lib/mcp/operations/inbox/list/operation.ts:145-171`). Compact `data` entries are notification **indices**, not IDs (`src/utils/helperFunctions/inboxHelpers.ts:36-83`). Persist per-account memberships and server tab order; derive counts from currently active rows after queued writes. Same-name project tabs remain distinct. Core `inbox_server_split_memberships_are_durable_account_scoped_and_never_guessed_from_titles`; GPUI `inbox_tabs_use_server_memberships_counts_and_same_name_project_identity`. Only split **management** is B3. |
| F500 | Row/Enter/reply-mode opens enqueue bearer POST `/api/mcp/inbox/seen`; auth is **validateMcpRouteAuth** aliased `validateMcpAuth` (`src/app/api/mcp/inbox/seen/route.ts:2,16,76-98`). Only the opened notification becomes seen; no fabricated unread toggle. Web open/seen: `src/components/notifications/inboxSplit/index.tsx:344,904-918`. `inbox_open_marks_only_target_seen_and_j_k_e_and_tabs_keep_keyboard_flow`; queue `inbox_seen_and_remind_survive_restart_and_send_exact_bearer_contracts`, denial/invalid-response tests. |
| F504 | The actual desktop row has Archive and Remind buttons, **not** an invented context menu/reply button (`src/components/notifications/NotificationRow.tsx:84-107`). Existing Ctrl+R opens reply, Ctrl+K offers applicable task commands (`src/components/notifications/inboxSplit/index.tsx:729-735`). Clock/Archive clicks target their own notification even when another row is focused. |
| F582 | Already implemented on main: reaction chips toggle, show counts and reactors tooltip (`comment_actions.rs:485-500,667-684`); existing picker/fast-like and bearer reaction queue tests cover the behavior. This stale Missing inventory row is reconciled, not reimplemented. |
| F584 | The reachable rendered-comment selection menu contains **Quote only**, not highlight/AI commands (`src/components/PageComponents/TaskDetail/CommentAndDescription/CommentContainer/InnerHTMLComment.tsx:138-150`, `ContextMenu/QuoteButton.tsx:8-29`). A read-only selectable rich viewer preserves selected HTML, marks, author and Unicode; Quote/Enter focus the composer without sending. `selected_comment_quote_preserves_only_selected_html_and_readonly_input_cannot_mutate`, `quote_html_crops_unicode_marks_and_nested_blocks_without_quoting_whole_comment`. |
| F586 | Actual composer Snooze/Remove notification controls: `src/components/PageComponents/TaskDetail/CommentAndDescription/CommentContainer/NewCommentComponent.tsx:630-668,698-709`. Resolve active inbox IDs from SQLite; no comment write, advance only in inbox playlist. `composer_snooze_and_remove_notification_are_real_buttons_without_comment_writes`. Snooze uses bearer POST `/api/mcp/inbox/archive` with `remind_at`, **validateMcpRouteAuth** alias (`src/lib/mcp/operations/inbox/archive/operation.ts:41,76-85,147-179,194-202`), including honest 202 scheduled-delivery fallback and denial rollback. |
| F591 | Editable left-side block grip (`src/components/RTE/Components/DragHandleTiptap.tsx:6-24`, `TiptapEditor.tsx:145-150`). Real mouse drag reorders paragraphs, lists and sibling list items without flattening nested/checklist subtrees. Cross-container drops move enclosing top-level nodes instead of orphaning items. Undo is one transaction. `real_drag_handle_moves_paragraph_below_checklist_and_undo_restores_html`, `drag_paragraph_after_nested_checklist_preserves_marks_and_item_state`, `drag_nested_sibling_items_keeps_their_complete_subtrees_and_rejects_noop`. |
| F592 | Web bubble Ai dropdown has four whole-document operations and five translation languages; it directly replaces returned HTML. It has **no custom prompt or accept/discard/regenerate flow** (`src/components/RTE/Components/AIDropdown.tsx:14-41,96-162`, `src/components/RTE/constants.ts:2-23`). Reuse the existing queued bearer `/api/mcp/ai/improve` equivalent (editor-formatting.md), add Dutch/Spanish/French. `ai_dropdown_click_preserves_selection_and_emits_each_exact_whole_document_command`, `all_web_ai_dropdown_commands_use_same_prompt_contract_through_durable_bearer_queue`; existing all-surface apply/undo and stale-result tests remain enabled. |

### Blocker table (full rows, not pretend desktop success)

Paths are relative to the web repository. An implemented subset does not remove a full-row blocker.

| Ref / rows | Exact route and auth helper | Reason / retained subset |
| --- | --- | --- |
| B17 — F001/F002/F003/F011/F115/F116/F587 live acceptance | The four HTPR-7699 routes use **getDesktopCliTokenUser**, gated by `htpr-7699-desktop-cli-token`. | Native code and CI coverage implemented. QA-owner 4036 has board 7283 access but all four bearer routes return 401; alternate flag-QA 985 gets 404 for the target task. Enable the flag for the board-owning QA account before calling these rows live-QA Done. F011's top ranking/custom-field/empty-task differences remain documented above. |
| B1 — F058 | GET `/api/notifications/markAsUnseen?notificationId=…&seen=1`: `taskWriteHandler` → `taskWriteRoute` → **getSessionUser** (`src/pages/api/notifications/markAsUnseen.ts:1-5`, `src/lib/api/notification-writes/mark-unseen.ts:62-76`, `src/lib/api/task-writes/route.ts:32-35,89-91`). | Bearer POST `/api/mcp/inbox/seen` only sets **true** (`src/app/api/mcp/inbox/seen/route.ts:82-84`). No bearer unread operation exists. U is not a fake local toggle. Mark-read on open works. |
| B2 — F056 | POST `/api/ai/audio-transcript`: **getSessionUser**, `src/app/api/ai/audio-transcript/route.ts:98-100`; caller `src/components/RTE/Components/AudioButton.tsx:406`. | Alt+V cannot honestly deliver dictation with the MCP bearer. No simulated recording/transcript. |
| B3 — F498 | GET/POST `/api/notifications/splits`: `taskWriteHandler` → **getSessionUser**, `src/pages/api/notifications/splits.ts:1-6`, `src/lib/api/notification-writes/splits.ts:31-35,77-81`. | Native reads authoritative tab membership through MCP; cannot persist “Also in Important” settings. |
| B4 — F185/F186/F187/F502/F503 | POST `/api/notifications/inbox-zero/preview`, `/execute`, `/undo`: **getSessionUser**, respectively `src/app/api/notifications/inbox-zero/preview/route.ts:14-16`, `execute/route.ts:61-63`, `undo/route.ts:13-15`. | “Archive all read” and “Archive reactions” are presets of this **server preview/execute/undo** sheet (`src/components/useCommandsState.ts:105-113`), not direct cached-row filters. MCP archive/unarchive does not supply its preview token/rule snapshot/undo operation; no fabricated cleanup sheet. |
| B5 — F057 | GET `/api/tasks/searchOrphans`: **getSessionUser**, `src/lib/api/task-writes/search-orphans.ts:9-12`; POST `/api/tasks/addParent`: `taskWriteRoute` → **getSessionUser**, `src/lib/api/task-writes/add-parent.ts:8-12`, `src/lib/api/task-writes/route.ts:32-35`; empty-result creation uses `/api/tasks/createGlobally` (blocked above). | MCP task update accepts `parent_task_id`, but does not provide the web orphan-search/filter/ranking/create-in-section contract (`src/components/Modals/SubtaskLinkingModal/SubtaskLinking.tsx:45-57,96-109,121-136`). No cached-only substitute. |
| B6 — F059 | POST `/api/savedContent/toggleSaved`: **getSessionUser**, `src/pages/api/savedContent/toggleSaved.ts:14-19`. | Only comment-scoped saved-content was unlocked by HTPR-7699; task star/unstar remains cookie-only. |
| B7 — F062 | GET/POST `/api/tasks/createGlobally`: `taskWriteHandler` / `taskWriteRoute` → **getSessionUser**, `src/pages/api/tasks/createGlobally.ts:110-112`, `src/lib/api/task-writes/route.ts:32-35,89-91`. | GET defaults now accepts human CLI bearers; POST creation still requires the cookie session. MCP task create does not reproduce web top ranking/placement. |
| B8 — F499/F585 | GET/POST `/api/users/preferences`: **loadCurrentUser** → **getSessionUser**, `src/app/api/users/preferences/route.ts:58-75,117-132`, `src/lib/auth/currentUser.ts:13-21`. Inbox drafts: POST `/api/drafts/getUserDrafts`, **getSessionUser**, `src/pages/api/drafts/getUserDrafts.ts:13-17`; draft archive POST `/api/drafts/archiveDraft`, **verifySession**, `src/pages/api/drafts/archiveDraft.ts:13`. | F499 actor/text/metadata/unread subset retained; avatar visibility is explicitly preference-controlled (`NotificationRow.tsx:285-312`) and authoritative draft timeline/lifecycle is not a local fake badge. F585 existing send chords stay; full menu hints/default require unavailable archive-after-comment preference. |
| B9 — F581/F583 | Author hovercard GET `/api/members/personHovercard`: **verifySession**, `src/pages/api/members/personHovercard.ts:45-49`. Overflow pin/star now accepts the flagged human CLI bearer via **getDesktopCliTokenUser** (489e9ea52); AI and task-from-comment routes/auth are A1/A2/A7 in ai-parity.md. | Exact timestamp and focused edit/copy/reply/react/delete remain; pin/star commands are implemented, but broader rows are not relabelled Done. |
| B10 — F588/F590 | GIF GET `https://api.giphy.com/v1/gifs/{search,trending}` requires external **api_key=NEXT_PUBLIC_GIPHY_API_KEY**, `src/components/RTE/Components/EmojiGifPicker.tsx:144-169`. Attachment staging POST `/api/tasks/uploadUrl` and `/api/tasks/uploadFinalize`: **verifySession / getSessionUser**, `src/lib/api/task-writes/upload-url.ts:235-240`, `upload-finalize.ts:210-215`; buffered POST `/api/tasks/n8nUpload`: **verifySession / resolveBetterAuthSession**, `src/pages/api/tasks/n8nUpload.ts:120-126`. | No native GIPHY key; no cancellable bearer editor-upload staging. MCP attachment upload immediately changes published content, so is not substituted. Exact details in editor-formatting.md. |
| B11 — F589 | POST `/api/mcp/comments`, PATCH `/api/mcp/comments/[comment_id]`: **validateMcpRouteAuth**; task update POST `/api/mcp/tasks/update`: **validateMcpAuth**. All call **sanitizeRichHtml**, which strips `<mark>` and color attributes (`src/utils/helperFunctions/sanitizeRichHtml.ts:6-35`; writer locations in editor-formatting.md). Task suggestions GET `/api/tasks/searchByParam` now accepts the flagged human CLI bearer (489e9ea52); rich-format sanitizer restrictions remain. | Existing real keyboard/slash/bubble formatting and link editing retained; full highlight/link-suggestion persistence is blocked, not “fixed” by stripping user formatting. |
| B12 — alternate reminder condition, not claimed | POST `/api/queues/inboxReminder`: **getSessionUser**, `src/pages/api/queues/inboxReminder.ts:27-30` (caller `src/hooks/Inbox/useGlobalFocusHandler.tsx:92-95`). Web “If any activity” is `NewNotification` (`src/lib/constants/constants.ts:381-390`). | MCP `/api/mcp/inbox/archive` hardcodes `invokeCondition: 'DurationComplete'` (`src/lib/mcp/operations/inbox/archive/operation.ts:154,164`). Only web default Regardless is offered. No nonfunctional condition selector. |
| B13 — F651/F652/F653/F654/F655 | GET/POST `/api/agent-sidebar-chats?projectId=…`: **getSessionUser**, `src/app/api/agent-sidebar-chats/route.ts:19-20,70-73`; GET `/api/agent-chat/[sessionId]`: **getSessionUser**, `src/app/api/agent-chat/[sessionId]/route.ts:49-55`; POST `/api/agent-chat/[sessionId]/messages`: **getSessionUser**, `src/app/api/agent-chat/[sessionId]/messages/route.ts:27-35`. Agent @ lookup POST `/api/tasks/searchAll`: `taskWriteHandler/taskWriteRoute` → **getSessionUser** (`src/pages/api/tasks/searchAll.ts:1-5`, `src/lib/api/task-writes/route.ts:32-35`); caller `src/app/agents/chat/useAgentChatComposer.ts:96-103`. | No human-bearer roster/session/transcript/send contract. Agent-token `/api/mcp/chat/pending` is not a human conversation endpoint. Alt+Up/Down, Ctrl+Enter, Ctrl+O and @ cannot be implemented by inventing empty sessions, replies or task context. |
| Desktop exclusion — F578 | No route/auth helper: mobile-only branch at `src/components/PageComponents/TaskDetail/CommentAndDescription/CommentContainer/NewCommentComponent.tsx:490-527`. | Already audited above. Native desktop keeps auto-growing editor; no mobile resize/collapse widget is added. |

F587 native implementation and its live-QA blocker are recorded above; focused-comment F368/F371/F372 retain their pre-existing blockers in ai-parity.md, without redoing their work. Final SHA-bound CI receipts and ignored feature dispositions are recorded in `internal/COMPOSER2-GATES.md` and the public worktree's ignored `internal/FEATURES.md`.

## p2 audit: remaining P1 rows

F639 is a stale inventory gap: `comment_actions.rs` already renders individual
collapsed previews and implements O/Shift+O without closing the latest comment.
CI test `comment_single_all_expansion_keeps_latest_open_and_does_not_fire_in_editor`
is retained, not reimplemented. Web: `src/components/PageComponents/TaskDetail/CommentAndDescription/CommentContainer/CollapsedCommentPreview.tsx:1`.

| Ref / rows | Exact route and auth helper | Reason |
| --- | --- | --- |
| B14 — F638 | POST `/api/tasks/reactToDescription`: `taskWriteHandler` → `taskWriteRoute` → **getSessionUser**, `src/pages/api/tasks/reactToDescription.ts:1-5`, `src/lib/api/task-writes/description-reaction.ts:10-22`, `src/lib/api/task-writes/route.ts:32-35,89-91`. | Description reactions use a Description ID, not a Comment ID. Bearer `/api/mcp/comments/[comment_id]/reactions` only writes comment reactions; it is not a description writer. No fake chip/toggle. |
| B15 — F147/F647 | POST `/api/ai/audio-transcript`: **getSessionUser**, `src/app/api/ai/audio-transcript/route.ts:98-101`. | The improved description microphone is clicked by `src/components/Modals/CreateTaskGloballyModal/TaskTitleModal.tsx:63-69`; it shares ordinary dictation's unavailable route, not the bearer text-improvement endpoint. |
| B16 — F664 | GET/POST `/api/tasks/createGlobally`: `taskWriteHandler` → **getSessionUser**, `src/pages/api/tasks/createGlobally.ts:110-112`, `src/lib/api/task-writes/route.ts:32-35,89-91`. | The row's create/discard-unsaved confirmation consumes the blocked web create-task flow. Existing archive/delete confirmations remain; a standalone pretend create dialog is not parity. |
