# HTPR-7381 AI and focused-comment parity

Scope: F227/F228, F368-F373, F579/F581/F583, F606-F620. Web paths below
are relative to `hypertask/src`. This is not a claim of complete AI parity.
Proof is GitHub CI only: workspace request/queue unit tests, GPUI TestAppContext
interactions, and `gates/layout-dump.py` under xvfb. No VPS compilation/tests,
Windows laptop access, cookie copying, provider credentials, or invented API
successes are permitted.

## Authentication and response-contract blockers

The existing native AI client is retained. It uses bearer-only requests, never
`x-hypertask-sidebar-project`; history HTTP/JSON and transcript preparation stay
on worker threads and start only on explicit history interaction. Existing
stream/history refusal behavior is not evidence of successful authentication.
Round two inspected `/home/valentin/projects/hypertask/src` directly. The web now
has a human CLI bearer path gated by `htpr-7684-desktop-ai-chat-bearer`
(`app/api/ai/_lib/requestUser.ts:66-100`). It refuses agent, management,
team-scoped tokens and sidebar-project headers; flag Off/errors refuse access.
The definition defaults to OWNER_AND_QA
(`lib/flags/definitions/htpr-7684-desktop-ai-chat-bearer.ts:3-9`), not proof of
this account's deployed flag state. No flag was flipped or live request made.
A1/A2 remain **blocked for live proof**, not an assertion that human CLI bearer
support is absent. OAuth and MCP token audiences remain different.

| Ref | Exact route(s) and auth boundary | Consequence |
| --- | --- | --- |
| A1 | `POST /api/ai/chat/stream`, `POST /api/ai/chat/cancel`: cookie/OAuth `getAiRequestUser`, then human CLI `getCliBearerChatUser` (`app/api/ai/chat/stream/route.ts:32-34`, `app/api/ai/chat/cancel/route.ts:19`, `app/api/ai/_lib/requestUser.ts:66-100`). | Conditional bearer path exists; deployed flag and successful account stream/cancel remain unproven. Preserve real refusals and unfinished-stream errors. Successful live sends, attachments and server tool effects remain blocked; no flag changes or simulated provider success count as live proof. |
| A2 | `GET /api/ai-chat/all-sessions` now accepts flag-gated human CLI bearer (`app/api/ai-chat/all-sessions/route.ts:119-125`); paged compatibility also requires `HTPR_6924_REST_COMPAT_FLAG` (`:138-149`). `POST /api/ai-chat/create-session` still calls `getSessionUser` (`app/api/ai-chat/create-session/route.ts:27-33`); add-message remains cookie-bound. | Existing lazy history client retained; live legacy/paged transcript proof and durable new sessions remain blocked. Local new-chat state is not server session creation. Server timestamps/attachments/agent attribution and task previous threads are not claimed complete. |
| A3 | `POST /api/ai-chat/update-session` `{sessionId,title}`: `loadCurrentUser` or legacy `isValidUser(nookies_user)`, `app/api/ai-chat/update-session/route.ts:14-31,60`. `DELETE /api/ai-chat/delete-session?delete=<uuid or ALL>`: same auth, `app/api/ai-chat/delete-session/route.ts:29-46,54-67`. | F615 rename/delete/delete-all are blocked; no local-only rename or fake server deletion is offered. |
| A4 | `GET /api/teams/getTeam?teamId=…`: `verifySession(req.cookies[SESSION_COOKIE])`, `pages/api/teams/getTeam.ts:11`. `GET /api/teams/aiFeatureModels?teamId=…`, `/api/teams/aiProviderSettings?teamId=…`, `/api/teams/byokKeys?teamId=…`: `getSessionUser`, respectively `pages/api/teams/aiFeatureModels.ts:125`, `aiProviderSettings.ts:63`, `byokKeys.ts:58`. `GET/POST /api/users/preferences`: `loadCurrentUser` / `getCurrentUserFromCookies`, `app/api/users/preferences/route.ts:60-75,117-132`. | F609 configured-model/provider/plan filtering, search of the allowed set and web-scoped model preferences cannot be completed. Retain main's explicit small model subset and exact request IDs, not an invented globally allowed catalog. |
| A5 | `GET /api/ai/chat/mention?param=…&projectId=…`: `getSessionUser`, `pages/api/ai/chat/mention.ts:21`. Scope team/board data uses `POST /api/teams/getAllSidebar`: `verifySession`, `pages/api/teams/getAllSidebar.ts:8`. | F612 add-context entity search is blocked. F611's complete team-grouped/recent-board scope picker cannot be synthesized from MCP board names without board/team membership. Feature-gated current-ticket scope must not be assumed enabled. |
| A6 | `GET/POST /api/agent-sidebar-chats?projectId=…`: `getSessionUser`, `app/api/agent-sidebar-chats/route.ts:20,71`, plus `browserMutationIsSameOrigin`, `app/api/agent-sidebar-chats/route.ts:73`. MCP `/chat/pending` is agent-token-only, not a human conversation chooser (`app/api/mcp/chat/pending/route.ts:35-42`). | F620 board-agent roster, chooser and conversation creation are blocked; MCP agent identity summaries are not sidebar chat sessions. |
| A7 | `POST /api/tasks/createGlobally`: `taskWriteHandler`, `pages/api/tasks/createGlobally.ts:110-112` -> `getSessionUser`, `lib/api/task-writes/route.ts:89`; create-global service also uses `taskWriteRoute` -> `getSessionUser`, `lib/api/task-writes/route.ts:33`. Web passes `createTaskFromComment` and adds source backlinks (`app/detail/[...slug]/useTaskDetailCommentActions.tsx:35-64`, `lib/api/task-writes/create-global-effects.ts:376-416`). MCP create task deliberately sets `createTaskFromComment: undefined`, `lib/mcp/tasks/services.ts:594`. | F371 full comment-to-task conversion is blocked by the missing bearer source-comment/backlink contract. A plain new task is not substituted for the web action. F618 response-to-task also needs the web's `position: "top"` contract (`hooks/MultiPages/AIChat/useAiChatPresentation.ts:160-168`); MCP create validates no position/ranking and always ranks after the last task (`lib/mcp/tasks/services.ts:488-493`). A differently placed plain task is not substituted; response-copy works but full row parity is blocked. |
| A8 | Comment pin/star uses `POST /api/savedContent/toggleSaved` (see `hooks/Task Detail/useStarAndPin.ts:37-50`, `lib/constants/APIRouteConstants.ts`), `getSessionUser`, `pages/api/savedContent/toggleSaved.ts:14-19`. MCP `PATCH /api/mcp/comments/<id>` accepts text/content_type/mentions, not pin/star visibility (`lib/mcp/operations/comments/[comment_id]/operation.ts:103`). | F583 is partial: ellipsis targets the exact focused comment and opens implemented edit/copy/reply/react/delete commands; pin/star and blocked AI/task commands are omitted rather than inert or routed to a different write. |
| A9 | `GET /api/members/personHovercard?projectId=…&kind=user\|agent&id=…`: `verifySession(req.cookies[SESSION_COOKIE])`, `pages/api/members/personHovercard.ts:45-49`; client `hooks/MultiPages/usePersonHovercard.ts:16-27`. | F581 author/profile/agent hovercard metadata is blocked. The exact timestamp tooltip is implemented independently; no guessed identity/profile UI is added. |

## Round-two sidebar behavior inventory

One row per inspected desktop-web behavior. All web citations below are relative
to `/home/valentin/projects/hypertask/src`, not screenshots or guessed values.
`match` means the named local native behavior, not full pixel equivalence or
live account authorization. `differs`/`missing` rows below remain explicitly
blocked by the original auth/flag boundary. Generic rich-editor fidelity remains
owned by `docs/editor-formatting.md`, not a new AI-only toolbar.

| Web behavior | Web source file:line | Audited native | Current native status and proof |
| --- | --- | --- | --- |
| Default right panel is 420px. | `lib/configs/style.config.ts:1-2` | match | **match**; `web_resize_bounds_and_persisted_pin_suppression_and_mode`, `ai_toggle_shortcut_closes_from_composer_and_reopens_same_draft`. |
| Resize uses 340px minimum and min(900, floor(viewport × .85)) maximum; width persists. | `lib/configs/style.config.ts:4-15`, `components/AI_CHAT/AiChatSidebarResizeHandle.tsx:29-56` | match | **match**; `ai_resize_mouse_drag_clamps_and_persists_after_reopen`. |
| Sidebar header is 48px, px-2, pt-4. | `components/AI_CHAT/ChatHeader.tsx:381-385` | differs | **match**; `sidebar_web_header_welcome_composer_and_tall_input_geometry`. |
| Pin opens chat, clears suppression; close unpins and suppresses reopen. | `components/AI_CHAT/ChatHeader.tsx:459-479`, `hooks/MultiPages/AIChat/aiChatKeyboard.ts:31-38` | match | **match**; `ai_header_pin_modes_close_and_commands_are_real_mouse_paths`. |
| Sidebar/floating/fullscreen preserve conversation; minimized floating restores or closes. | `components/AI_CHAT/ChatHeader.tsx:481-529,586-599`, `components/AI_CHAT/ChatWindow.tsx:16-59` | match | **match**; existing presentation and pointer tests. Fullscreen history remains A2. |
| Title/history menu uses server conversations and configured names. | `components/AI_CHAT/ChatHeader.tsx:387-426`, `hooks/MultiPages/AIChat/useSessionAndChatHistory.ts:55-74` | differs | **differs, blocked A2/A6**; retain local title/history, no fake roster or durable session. |
| History loads/paginates, groups previous seven days/older, selects transcript. | `components/AI_CHAT/ChatHeader.tsx:158-175`, `hooks/MultiPages/AIChat/usePagedChatHistory.ts:1-110` | differs | **differs, blocked A2**; lazy background legacy history retained; `history_is_lazy_background_results_are_polled_without_an_active_turn`. |
| New-chat button and Ctrl/Cmd+Shift+O/J create a session and focus input. | `components/AI_CHAT/ChatHeader.tsx:429-441`, `hooks/MultiPages/AIChat/aiChatKeyboard.ts:203-213`, `hooks/MultiPages/AIChat/useAiChatSessions.ts:35-54` | differs | **differs, blocked A2** for durable creation; local reset/focus tested by `keyboard_reset_platform_aliases_create_distinct_chats_and_clear_the_draft`. |
| Rename, delete and delete-all change server history. | `components/AI_CHAT/ChatHeader.tsx:531-582`, `app/api/ai-chat/delete-session/route.ts:29-67` | missing | **missing, blocked A3**; no local fake server mutation. |
| Ctrl/Cmd+Shift+/ toggles chat even from its composer. | `hooks/MultiPages/AIChat/aiChatKeyboard.ts:128-137` | differs | **match**; composer close and workspace reopen, including consumed-Shift Ctrl+?, tested by `ai_toggle_shortcut_closes_from_composer_and_reopens_same_draft`. |
| Ctrl+Q opens/focuses; toggles back only in desktop sidebar, not floating/fullscreen. | `hooks/MultiPages/AIChat/aiChatKeyboard.ts:83-114,139-200` | differs | **match**; `idle_escape_keeps_composer_and_ctrl_q_only_returns_from_sidebar`; existing workspace focus tests. |
| Idle Escape does not blur the composer; typing Escape cancels. | `hooks/MultiPages/AIChat/aiChatKeyboard.ts:79-82` | differs | **match** locally; idle key test; transport cancel tests remain synthetic, live cancel A1. |
| Placeholder reads “Focus with CTRL + Q, @ for context”. | `hooks/MultiPages/AIChat/useAiTiptap.ts:72-79` | match | **match**; existing TextInput placeholder, no invented empty-state prose. |
| Rich composer uses existing marks; AI slash menu contains skills, not formatting commands. | `hooks/MultiPages/AIChat/useAiTiptap.ts:60-93`, `components/RTE/Extensions/SlashCommands/items.ts:125-126` | differs | **match** for shared native rich marks/skills-only surface; `ai_composer_rich_marks_keep_enter_send_and_skills_only` plus existing `ai_skills` native key tests. No new toolbar. |
| Composer has p-2 rounded-lg; outer p-2; input scrolls up to 380px. | `components/AI_CHAT/AI_Tiptap_Container.tsx:261,298-335`, `styles/tiptap.module.scss:45-53` | differs | **match** for geometry; `sidebar_web_header_welcome_composer_and_tall_input_geometry`. |
| Enter sends nonempty text; Shift+Enter inserts one newline; whitespace sends nothing. | `hooks/MultiPages/AIChat/aiChatKeyboard.ts:63-78`, `hooks/MultiPages/AIChat/aiChatSend.ts:82-92` | match | **match**; `sidebar_enter_still_sends_and_shift_enter_is_exactly_one_newline`, rich-key test; offline drafts retained. |
| Main model, providers, reasoning options and preferences follow team availability/BYOK. | `components/AI_CHAT/AI_Tiptap_Container.tsx:400-428`, `lib/aiModelOptions.ts:246-523` | differs | **differs, blocked A4**; retained small main model picker, no invented team configuration. |
| Default request includes current board/task/inbox surface context. | `hooks/MultiPages/AIChat/aiChatSend.ts:118-123,213-245` | match | **match** locally; `scope_matches_web_board_task_and_inbox_rules`, request fixture tests, summary pointer request test. Live A1. |
| Team/recent-board/current-ticket scope picker and @ context search use server membership/flags. | `components/AI_CHAT/AI_Tiptap_Container.tsx:540-685` | differs | **differs, blocked A5**; existing current/all scope retained; no guessed team groups or enabled ticket flag. |
| Attachments/screenshots have previews, removal, upload and request delivery. | `components/AI_CHAT/AI_Tiptap_Container.tsx:310-321,514-526`, `hooks/MultiPages/AIChat/aiChatSend.ts:143-176` | missing | **missing, blocked A1**; no decorative upload controls. |
| Dictation and Ctrl/Cmd+Shift+D use authenticated audio transcription. | `components/AI_CHAT/AI_Tiptap_Container.tsx:218-241,430-486`, `hooks/MultiPages/AIChat/aiChatKeyboard.ts:119-124`, `app/api/ai/audio-transcript/route.ts:1-75` | missing | **missing, blocked shell B19**; preserve original audio cookie boundary, no fake recorder. |
| Empty welcome centers four source-backed starter prompts, px-10 py-4, gap-2. | `components/AI_CHAT/WelcomeScreen.tsx:79-93,295-329,397-411` | differs | **match** locally; geometry/pointer test and `suggestions_use_full_web_pools_without_duplicates_or_invented_counts`. |
| Ticket welcome includes “Summarize the current task” sending `/i Summarize the current task`. | `components/AI_CHAT/taskSummaryAction.ts:1-7`, `components/AI_CHAT/WelcomeScreen.tsx:304-317` | missing | **match** locally; `task_summary_mouse_action_sends_exact_web_prompt_and_context` captures real native pointer → worker HTTP request. Mock refusal is not live summary success (A1). |
| Starter pools include six ticket/nine general prompts and current board counts, choosing four. | `components/AI_CHAT/WelcomeScreen.tsx:46-93,176-264` | differs | **match**; full source-backed pools sampled without duplicates; core pool regression and native suggestion click. |
| Generated task questions, Ask all and previous threads use server task data. | `components/AI_CHAT/WelcomeScreen.tsx:151-175,280-294,330-360`, `app/api/ai/task-questions/route.ts:70-74` | missing | **missing, blocked A1/A2**; generated questions remain cookie-bound; no invented suggestions from a provider. |
| Flag-gated edit-task/refinement followup/board-builder welcome changes session workflow. | `components/AI_CHAT/WelcomeScreen.tsx:130-150,298-305,367-394` | missing | **missing, blocked A1/A2 and shell B26**; preserve F621-F623/F646/F648 blockers and unproven flag state. |
| Message list is scrollable with p-2 and space-y-2; human messages right, assistant left. | `components/AI_CHAT/MessageList.tsx:101-109`, `components/AI_CHAT/MessageItem.tsx:44-79` | differs | **match** locally; transcript row geometry and native copy/edit tests; no whole-screen pixel claim. |
| Human HTML/plain content displays as rich text, not literal tags. | `components/AI_CHAT/MessageItem.tsx:300-312,347-355` | differs | **match** for shared native renderer; `human_message_copy_and_edit_are_real_mouse_actions`. |
| Assistant markdown/HTML, Unicode, links and blocks render incrementally. | `components/AI_CHAT/MessageItem.tsx:300-312`, `utils/helperFunctions/normalizeAssistantHtml.ts:1-35` | match | **match** for existing renderer subset; `markdown_replies_use_existing_rich_text_with_unicode_and_blocks`, stream-content tests. Generic rich-document limits remain in editor ledger. |
| Delivered human copy/edit/retry controls act on the exact message. | `components/AI_CHAT/MessageItem.tsx:167-191`, `hooks/MultiPages/AIChat/aiChatKeyboard.ts:49-61` | missing | **match** locally; `human_message_copy_and_edit_are_real_mouse_actions`, `retry_mouse_action_preserves_real_offline_refusal`. Successful retry remains A1. |
| Delivered assistant copy uses rendered plain-text clipboard fallback. | `components/AI_CHAT/MessageItem.tsx:209-216`, `hooks/MultiPages/AIChat/useAiChatPresentation.ts:132-161` | match | **match**; `response_copy_mouse_action_copies_rendered_unicode_text`. |
| Create-task-from-response uses the web top-position creation contract. | `components/AI_CHAT/MessageItem.tsx:217-222`, `hooks/MultiPages/AIChat/useAiChatPresentation.ts:164-200` | missing | **missing, blocked A7**; no substitute MCP task without required web effects. |
| Delivered message metadata includes relative timestamp and agent attribution/attachments. | `components/AI_CHAT/MessageItem.tsx:61-67,86-104`, `utils/generateTime.ts:1-30` | missing | **missing, blocked A2/A6** for complete server transcript metadata; no invented timestamps/agent identities. |
| Copy/actions are absent while an assistant message streams. | `components/AI_CHAT/MessageItem.tsx:86-104` | differs | **match**; `streaming_status_stays_in_message_and_copy_waits_for_done`. |
| Busy composer Enter/send queues a followup; Stop remains separate; queued items can be removed. | `hooks/MultiPages/AIChat/aiChatSend.ts:44-77`, `components/AI_CHAT/AI_Tiptap_Container.tsx:273-277,490-511`, `components/Common/QueuedMessagesStrip.tsx:1-46` | missing | **match** for unflagged in-memory queue; `busy_composer_queues_with_enter_and_mouse_removes_without_stopping`; no fake provider output. |
| Queue drains after the current reply; session switch/new chat clears fallback queue. | `hooks/MultiPages/AIChat/useAiChatSessions.ts:25-43,99-133` | missing | **match** locally; queue drain/native action tests; live deliveries A1. |
| Flag-gated durable per-user/session queue survives reload and concurrent delivery. | `hooks/MultiPages/AIChat/useAiChatState.ts:96-106`, `hooks/MultiPages/AIChat/useAiChatSessions.ts:102-119` | missing | **missing, blocked A1/A2 and unproven flag state**; no claim that an in-memory queue equals server/session persistence. |
| Status/typing/cursor stays with the active assistant row, not the composer. | `components/AI_CHAT/MessageItem.tsx:237-270,353-361`, `components/AI_CHAT/TypingIndicator.tsx:3-29` | differs | **match** for native status placement and typing/cursor state; `streaming_status_stays_in_message_and_copy_waits_for_done`. CSS animation pixels not claimed. |
| Stream content/title/status/errors update their original session after navigation. | `hooks/MultiPages/AIChat/aiChatStream.ts:108-162` | match | **match** locally; `late_stream_updates_stay_with_the_original_chat_and_preserve_errors`; live A1. |
| Cancelled done keeps partial content or “Stream cancelled.”, not a false failure. | `hooks/MultiPages/AIChat/aiChatStream.ts:174-193` | differs | **match**; `cancelled_done_is_not_a_failure_and_refusal_survives_eof`. |
| Refusals remain visible; unfinished EOF cannot pretend completion. | `hooks/MultiPages/AIChat/aiChatStream.ts:141-163,277-306`, `app/api/ai/_lib/requestUser.ts:93-96` | differs | **match** for truthful error preservation; cancellation/EOF test and existing transport refusal tests. Exact web toast/error presentation remains live A1. |
| Scrolled-away reading position survives new chunks and returning to a chat. | `components/AI_CHAT/MessageList.tsx:71-100` | differs | **match**; `incoming_stream_keeps_user_scroll_position_until_latest_is_clicked`, `switching_local_chats_restores_each_transcript_scroll`. |
| Follow when within 48px; latest indicator uses 4px; latest button only when not typing. | `components/AI_CHAT/MessageList.tsx:95-100`, `hooks/MultiPages/AIChat/useAiChatPresentation.ts:103-129`, `components/AI_CHAT/AI_Tiptap_Container.tsx:270-272` | differs | **match** locally; native scroll/status tests. |
| Board-agent chooser loads roster and creates/selects authenticated conversations. | `components/AI_CHAT/AI_Chat_Sidebar.tsx:63-100,258-279` | missing | **missing, blocked A6**; no MCP agent summary masquerades as a sidebar chat. |

## Retained focused-comment inventory

These earlier rows are not widened by the sidebar pass.

| Rows | Disposition | Native path / web evidence |
| --- | --- | --- |
| F368, F372 | Blocked A1/A2. | Summary submits quote + “Summarize this comment concisely.”; branch awaits server session creation (`hooks/MultiPages/AIChat/useCommentToAiChat.ts:90-108`, `app/detail/[...slug]/useTaskDetailCommandActions.tsx:94-118`). |
| F369, F370, F373 | Implemented focused content/plain-text clipboard fallback, URL with displayed comment-index fragment (not database ID), quote insertion into the existing AI composer without sending or creating a session. | `comment_actions.rs`; `app/detail/[...slug]/useTaskDetailCommentActions.tsx:85-134`, `hooks/MultiPages/AIChat/useCommentToAiChat.ts:78-87`, `utils/helperFunctions/TaskDetail/index.ts:88-109`. |
| F371 | Blocked A7. | No partial plain-task substitute. |
| F579 | Per-task/user durable local restore and 750ms debounced queued bearer autosave; server drafts cached separately from published comments. Discard queues a delete; successful comment acknowledgement queues draft cleanup, failed sends restore drafts, newer edits are not cleared. | `comment_drafts.rs`, `ht-core/src/comment_draft.rs`; `components/RTE/useTaskDetailEditorDrafts.tsx:75-161,179-181`. Bearer `POST/GET /api/mcp/drafts`, `DELETE /api/mcp/drafts/<id>` use `validateMcpRouteAuth` (`lib/mcp/operations/drafts/operation.ts:53-67,87-89,123-157,194-204`; `lib/mcp/operations/drafts/[draft_id]/operation.ts:179-200,237-265`). |
| F581 | Exact local date/time tooltip implemented. Full inventory row also asks for author/profile navigation, blocked A9; do not relabel that broader row Done. | `comment_actions.rs`; `components/PageComponents/TaskDetail/CommentAndDescription/CommentContainer/CommentOptions.tsx:144-165`, `utils/helperFunctions/helperFunctions.ts:660-685`. |
| F583 | Existing ellipsis-to-focused-command path extended with working copies; full row blocked A1/A2/A7/A8. | `comment_actions.rs`; `components/PageComponents/TaskDetail/CommentAndDescription/CommentContainer/CommentOptions.tsx:117-127`. |

## CI evidence

Earlier-round evidence is <https://github.com/hypertask-ai/hypertask-desktop/pull/9>.
Round-two exact-head evidence is recorded in the draft PR and `GATES-aiside2.md`.
Auth/flag-blocked rows remain blocked even when native fixture tests pass.
No VPS builds/tests, test copies, owner-laptop commands or live flag changes ran.

## p2 audit: stale remaining P1 classifications

Full-row blockers also cover F228 (fullscreen history, A2), F621/F622/F623 and
F646/F648 (web task-writer/draft refinement, shell B26 and A1/A2), F644 (successful
edit/retry requests, A1), and F645 (assistant-to-task creation, A7). Existing
presentation, copy and error behavior is retained, not counted as full-row Done.
No MCP-audience token is exchanged for a browser/OAuth session.
