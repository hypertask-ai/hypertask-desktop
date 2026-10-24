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
The CLI login issues an MCP-audience token (`app/api/cli/token-exchange/route.ts:79`,
`lib/mcp/auth/verifyJwt.ts:441-474`). An OAuth bearer is not an interchangeable
MCP token, and switching token exchange alone does not unlock cookie history.

| Ref | Exact route(s) and auth boundary | Consequence |
| --- | --- | --- |
| A1 | `POST /api/ai/chat/stream`, `POST /api/ai/chat/cancel`: `getAiRequestUser`, `app/api/ai/_lib/requestUser.ts:13-34`, first cookies, otherwise `hasOAuthAccessTokenClaims` before `validateMcpAuth`. An MCP-audience personal token fails the OAuth claim prefilter. | F368 summary generation, F613 attachment delivery, F617 successful streaming/cancellation and all live AI sends remain blocked. Preserve the existing visible server refusals; do not manufacture answers or mark EOF as completion. Tool progress is the server's `status`, not a fabricated tool-event schema (`lib/ai/chatStream/types.ts:12`). |
| A2 | `GET /api/ai-chat/all-sessions[?compat=htpr-6924&limit=30&cursor=…]`, transcript `?compat=htpr-6924&sessionId=…`: `loadCurrentUser`, `app/api/ai-chat/all-sessions/route.ts:119-127` -> `getSessionUser`, `lib/auth/currentUser.ts:13-21`; no MCP bearer path. `POST /api/ai-chat/create-session`: `getSessionUser`, `app/api/ai-chat/create-session/route.ts:29`. `POST /api/ai-chat/add-message`: `loadCurrentUser`. | F614 cross-device/paged history and F372 durable new-chat branch are blocked. Local new-chat UUIDs in the existing client are not claimed as server-created sessions. The legacy all-sessions response can contain every transcript; it must remain lazy/off the UI thread. |
| A3 | `POST /api/ai-chat/update-session` `{sessionId,title}`: `loadCurrentUser` or legacy `isValidUser(nookies_user)`, `app/api/ai-chat/update-session/route.ts:14-31,60`. `DELETE /api/ai-chat/delete-session?delete=<uuid or ALL>`: same auth, `app/api/ai-chat/delete-session/route.ts:29-46,54-67`. | F615 rename/delete/delete-all are blocked; no local-only rename or fake server deletion is offered. |
| A4 | `GET /api/teams/getTeam?teamId=…`: `verifySession(req.cookies[SESSION_COOKIE])`, `pages/api/teams/getTeam.ts:11`. `GET /api/teams/aiFeatureModels?teamId=…`, `/api/teams/aiProviderSettings?teamId=…`, `/api/teams/byokKeys?teamId=…`: `getSessionUser`, respectively `pages/api/teams/aiFeatureModels.ts:125`, `aiProviderSettings.ts:63`, `byokKeys.ts:58`. `GET/POST /api/users/preferences`: `loadCurrentUser` / `getCurrentUserFromCookies`, `app/api/users/preferences/route.ts:60-75,117-132`. | F609 configured-model/provider/plan filtering, search of the allowed set and web-scoped model preferences cannot be completed. Retain main's explicit small model subset and exact request IDs, not an invented globally allowed catalog. |
| A5 | `GET /api/ai/chat/mention?param=…&projectId=…`: `getSessionUser`, `pages/api/ai/chat/mention.ts:21`. Scope team/board data uses `GET /api/teams/getAllSidebar`: `verifySession`, `pages/api/teams/getAllSidebar.ts:8`. | F612 add-context entity search is blocked. F611's complete team-grouped/recent-board scope picker cannot be synthesized from MCP board names without board/team membership. Feature-gated current-ticket scope must not be assumed enabled. |
| A6 | `GET/POST /api/agent-sidebar-chats?projectId=…`: `getSessionUser`, `app/api/agent-sidebar-chats/route.ts:20,71`, plus same-origin mutation validation. MCP `/chat/pending` is agent-token-only, not a human conversation chooser (`app/api/mcp/chat/pending/route.ts:35-42`). | F620 board-agent roster, chooser and conversation creation are blocked; MCP agent identity summaries are not sidebar chat sessions. |
| A7 | `POST /api/tasks/createGlobally`: `getSessionUser`, `pages/api/tasks/createGlobally.ts:5,29-31`; modern create-global service uses `taskWriteRoute` -> `getSessionUser`, `lib/api/task-writes/route.ts:33`. Web passes `createTaskFromComment` and adds source backlinks (`app/detail/[...slug]/useTaskDetailCommentActions.tsx:35-64`, `lib/api/task-writes/create-global-effects.ts:376-416`). MCP create task deliberately sets `createTaskFromComment: undefined`, `lib/mcp/tasks/services.ts:594`. | F371 full comment-to-task conversion is blocked by the missing bearer source-comment/backlink contract. A plain new task is not substituted for the web action. F618 response-to-task also needs the web's `position: "top"` contract (`hooks/MultiPages/AIChat/useAiChatPresentation.ts:160-168`); MCP create validates no position/ranking and always ranks after the last task (`lib/mcp/tasks/services.ts:488-493`). A differently placed plain task is not substituted; response-copy works but full row parity is blocked. |
| A8 | Comment pin/star uses `POST /api/savedContent/toggleSaved` (see `hooks/Task Detail/useStarAndPin.ts:37-50`, `lib/constants/APIRouteConstants.ts`), `getSessionUser`, `pages/api/savedContent/toggleSaved.ts:14-19`. MCP `PATCH /api/mcp/comments/<id>` accepts text/content_type/mentions, not pin/star visibility (`lib/mcp/operations/comments/[comment_id]/operation.ts:103`). | F583 is partial: ellipsis targets the exact focused comment and opens implemented edit/copy/reply/react/delete commands; pin/star and blocked AI/task commands are omitted rather than inert or routed to a different write. |
| A9 | `GET /api/members/personHovercard?projectId=…&kind=user\|agent&id=…`: `verifySession(req.cookies[SESSION_COOKIE])`, `pages/api/members/personHovercard.ts:45-49`; client `hooks/MultiPages/usePersonHovercard.ts:16-27`. | F581 author/profile/agent hovercard metadata is blocked. The exact timestamp tooltip is implemented independently; no guessed identity/profile UI is added. |

## Implemented behavior and inventory disposition

| Rows | Disposition | Native path / web evidence |
| --- | --- | --- |
| F227, F607 | Implemented local pin/unpin; pin explicitly opens and clears suppression, close unpins; presentation persists across navigation/restart. | `ai_presentation.rs`, `ai_workspace.rs`; `components/AI_CHAT/ChatHeader.tsx:434-453`, `components/commandDispatcher.ts:732-741`, `store/index.ts:331-375`. |
| F228 | Fullscreen command opens the existing conversation without changing the workspace return target. Fullscreen server history rail is A2, not claimed. | `ai_workspace.rs`; `components/commandDispatcher.ts:742-749`, `lib/aiChatDisplayMode.ts:14-27`. |
| F606 | Implemented resize edge, persisted width, 340px minimum, min(900, floor(viewport × .85)) maximum. | `ai_presentation.rs`; `lib/configs/style.config.ts:1-15`, `components/AI_CHAT/AiChatSidebarResizeHandle.tsx:29-56`. |
| F608 | Local sidebar/floating/fullscreen transitions preserve conversation/draft; close suppresses automatic reopening, floating minimize preserves the chat. Fullscreen history/rename is A2/A3; not full row parity. | `ai_workspace.rs`, `ai_sidebar.rs`; `components/AI_CHAT/ChatHeader.tsx:455-515,582-601`, `components/AI_CHAT/ChatWindow.tsx:16-52`, `components/AI_CHAT/AI_Chat_Layout.tsx:126-133`. |
| F609 | Partial retained main model/request implementation; full configured picker A4. | `ai_sidebar.rs`, `ht-core/src/ai_chat.rs`; `lib/aiModelOptions.ts:246-523`. |
| F610, F616 | Existing scope/request and local starter-suggestion paths retained and regression-tested; live requests remain A1. Generated task suggestions use cookie-only `POST /api/ai/task-questions`, `getSessionUser` (`app/api/ai/task-questions/route.ts:60`). | `Scope::load`, `ht-core/src/ai_chat.rs`; `hooks/MultiPages/AIChat/aiChatSend.ts:159-194`, `components/AI_CHAT/WelcomeScreen.tsx:24-72`. |
| F611, F612 | Full scope/add-context pickers blocked A5; do not invent ungrouped board/team relationships or a flag state. | `components/AI_CHAT/AI_Tiptap_Container.tsx:540-685`. |
| F613 | End-to-end attachments/screenshot delivery blocked A1; no decorative attachment button added. | `hooks/MultiPages/AIChat/aiChatSend.ts:100-151,189-193`. |
| F614 | Existing lazy background legacy history is retained; paged/durable history blocked A2. | `ai_sidebar.rs`; `components/AI_CHAT/ChatHeader.tsx:148-162,220-290`. |
| F615 | Blocked A3. | No fake rename/delete UI. |
| F617 | Existing incremental SSE/status/markdown/error/cancel paths retained; successful live parity blocked A1. | `ai_sidebar.rs`, `ht-core/src/ai_chat.rs`. |
| F618 | Plain-text response copy (web clipboard fallback) implemented. Full row blocked A7: web top-position create-task contract unavailable; response branch/add-to-comment controls are **not in the inspected web MessageItem**, so are not invented. | `components/AI_CHAT/MessageItem.tsx:159-227`; `hooks/MultiPages/AIChat/useAiChatPresentation.ts:119-168`. |
| F619 | Implemented 4px bottom threshold, preserve user position while streaming, scroll-to-latest button. | `ai_sidebar.rs`; `hooks/MultiPages/AIChat/useAiChatPresentation.ts:103-125`, `components/AI_CHAT/MessageList.tsx:71-91`. |
| F620 | Blocked A6. | No invented human-to-agent token conversion. |
| F368, F372 | Blocked A1/A2. | Summary submits quote + “Summarize this comment concisely.”; branch awaits server session creation (`hooks/MultiPages/AIChat/useCommentToAiChat.ts:90-108`, `app/detail/[...slug]/useTaskDetailCommandActions.tsx:94-118`). |
| F369, F370, F373 | Implemented focused content/plain-text clipboard fallback, URL with displayed comment-index fragment (not database ID), quote insertion into the existing AI composer without sending or creating a session. | `comment_actions.rs`; `app/detail/[...slug]/useTaskDetailCommentActions.tsx:85-134`, `hooks/MultiPages/AIChat/useCommentToAiChat.ts:78-87`, `utils/helperFunctions/TaskDetail/index.ts:88-109`. |
| F371 | Blocked A7. | No partial plain-task substitute. |
| F579 | Per-task/user durable local restore and 750ms debounced queued bearer autosave; server drafts cached separately from published comments. Discard queues a delete; successful comment acknowledgement queues draft cleanup, failed sends restore drafts, newer edits are not cleared. | `comment_drafts.rs`, `ht-core/src/comment_draft.rs`; `components/RTE/useTaskDetailEditorDrafts.tsx:75-161,179-181`. Bearer `POST/GET /api/mcp/drafts`, `DELETE /api/mcp/drafts/<id>` use `validateMcpRouteAuth` (`lib/mcp/operations/drafts/operation.ts:53-67,87-89,123-157,194-204`; `lib/mcp/operations/drafts/[draft_id]/operation.ts:179-200,237-265`). |
| F581 | Exact local date/time tooltip implemented. Full inventory row also asks for author/profile navigation, blocked A9; do not relabel that broader row Done. | `comment_actions.rs`; `components/PageComponents/TaskDetail/CommentAndDescription/CommentContainer/CommentOptions.tsx:144-165`, `utils/helperFunctions/helperFunctions.ts:660-685`. |
| F583 | Existing ellipsis-to-focused-command path extended with working copies; full row blocked A1/A2/A7/A8. | `comment_actions.rs`; `components/PageComponents/TaskDetail/CommentAndDescription/CommentContainer/CommentOptions.tsx:117-127`. |

## CI evidence

The draft PR is <https://github.com/hypertask-ai/hypertask-desktop/pull/9>.
Final SHA-bound results are recorded in the PR and the local acceptance ledger;
auth-blocked rows remain blocked even when their pure client-side tests pass.
