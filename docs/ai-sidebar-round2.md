# AI sidebar parity round two

## Contract and baseline

Acceptance precedes implementation in GATES-aisidebar2.md, commit 3f5f108.
Baseline 2a5474e6639a86e97a7b1fcfb3a7c61da5234d63, PR https://github.com/hypertask-ai/hypertask-desktop/pull/56.
Baseline CI https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40391866819.
Baseline windows-visual3 artifact 12686661371, digest sha256:3f8b67fa82a6f90eef3584cbb9a28b4858261f974b17f3a0f72fa16e78ce359f.
Frozen images and JSON are read-only at /home/valentin/projects/hypertask-desktop-wt-public/internal/live-graphite-1920.
All web source paths below are absolute under /home/valentin/projects/hypertask/src.
Rows retain the round-one definition, tolerances and content differences.

## Every remaining row

Each C row applies to all three screens unless explicitly limited below.
C02, C04-C09 and T01-T08 are already closed and are not recounted.
Board C10 and C16 were already matched and are not recounted.

| Row | Screens | Decision and source |
| --- | --- | --- |
| C01 starter stack y | Inbox, Board, Task | Open. Welcome is centered in the remaining transcript, not manually offset. `components/AI_CHAT/WelcomeScreen.tsx:314-315,398-399`. Its residual follows C03. |
| C03 composer top/height | Inbox, Board, Task | Open, coupled to the missing recorder. The real desktop recorder's microphone chooser is explicitly 32px high, `components/RTE/Components/AudioButton.tsx:746-756`. In the actual action row this exceeds the 27.5px model/scope chips, making the complete row 40px including pt8, versus the native 35.5px. `components/AI_CHAT/AiChatComposerActionRow.tsx:74-90,146-152`; composer p8 and input padding8/line21 come from `components/AI_CHAT/AI_Tiptap_Container.tsx:298-305,326-335`, `styles/tiptap.module.scss:48-52,200-204`. An empty reserved recorder slot would change layout without delivering the action. The frozen projection also deliberately models only selectors/send, not the 32px recorder; it expects the current approximately 88px composer. No reference/projection is altered here. |
| C10 selected scope glyph | Inbox, Task | Open selected-state difference. Web chooses ListTodo or SquareKanban from the actual selected scope and the ticket-scope flag, not merely the task's existence. `components/AI_CHAT/AI_Tiptap_Container.tsx:569-580,619-626`; `hooks/MultiPages/AIChat/dockedChatScope.ts:3-4`. No flag state, saved scope or board choice is invented to replace the current native task scope. |
| C11 scope trailing control | Inbox, Board, Task | Open. Web chevron opens Chat context with current/all, optional ticket, and team-grouped boards. `components/AI_CHAT/AI_Tiptap_Container.tsx:631-664`. Complete membership/board labels and saved recent IDs are absent from native Scope. `pages/api/teams/getAllSidebar.ts:7-13` requires the session cookie. Keep the existing honest current/all toggle; do not replace its X with a nonworking or incomplete picker. |
| C12 attachments | Inbox, Board, Task | Open. CLI streaming accepts attachment request arrays, `lib/ai/chatStream/request.ts:20-24,74-77`, and the existing writer creates native sessions/messages, `app/api/ai/chat/stream/route.ts:115-137`. However the writer arguments are only sessionId/messageId/userId/content, `app/api/ai/chat/stream/ensureNativeChatTurn.ts:5-11,73-85`. Missing durable attachment filename/MIME/source/size and attachment records cannot be supplied through that writer. Web attachment persistence/upload is `POST /api/ai-chat/add-message`, `app/api/ai-chat/add-message/route.ts:20-31,55-74`, using loadCurrentUser or nookies_user. `lib/auth/currentUser.ts:17-18` delegates to cookie-only `lib/auth/getSessionUser.ts:21-25,70-79`, not the separate chat-stream CLI fallback. A data-URL-only provider request would lose web attachment persistence. No paperclip is added. |
| C13 recording and microphone chooser | Inbox, Board, Task | Open. `POST /api/ai/audio-transcript` requires `getSessionUser`, `app/api/ai/audio-transcript/route.ts:101-105`, whose implementation does not read Authorization, `lib/auth/getSessionUser.ts:21-25,70-79`. Missing human-CLI transcription authorization, not missing audioFile/projectId request fields. No silent recorder or microphone menu is added. |
| C14 add context | Inbox, Board, Task | Open. The web button focuses the editor end and inserts @, `hooks/MultiPages/AIChat/useAiChatPresentation.ts:47-49`. But the resulting entity picker needs `GET /api/ai/chat/mention?param=...&projectId=...`, `pages/api/ai/chat/mention.ts:19-24`, protected by the same cookie-only getSessionUser. Results include authorized boards, tasks, agents and people with typed IDs, `:121-150`. Existing local task titles and MCP search are not that complete authorized result. A button that merely inserts dead @ text is not context-add parity. |
| C15 selected model | Inbox, Board, Task | Open account/selection difference, not a style defect. Web model label derives from selected option, `components/Global/ModelSelectorDropdown.tsx:511-519`. Availability, billing, BYOK and preferences remain A4 in docs/ai-parity.md. Do not select Haiku just to match a frozen capture. |
| C16 scope label | Inbox, Task | Open selected-state difference; same missing scope/flag/membership state as C10/C11. `components/AI_CHAT/AI_Tiptap_Container.tsx:560-580`. Do not relabel a native task scope as a selected board. |
| C17 prompt text/order | Inbox, Board, Task | Open randomized/server-state difference. `components/AI_CHAT/WelcomeScreen.tsx:96-112,297-301,332-336,398-406`. Local suggestions remain real source-backed actions; no capture-specific fixed strings or changed seed. |
| C18 font/raster painted bounds | Inbox, Board, Task | Open renderer residue, including gutter edges and equal-label glyph paint. Web text source remains `components/AI_CHAT/ChatHeader.tsx:381-426`, `components/Global/ModelSelectorDropdown.tsx:493,511`. No string-specific offsets or global font changes. |
| C19 send stroke | Inbox, Board, Task | Candidate closure. Sidebar-only ai-send uses the same Lucide Send path at stroke1.75 and displayed size16, `components/AI_CHAT/SendMessageButton.tsx:62-65`. The shared send asset remains stroke1.5 and unchanged. Native regression sidebar_send_stroke_matches_web_without_changing_shared_send pins both source path and isolation; pointer/layout regression pins16x16 and existing send/offline behavior. |
| C20 scope sizing/action grouping | Inbox, Board, Task | Candidate closure of the layout property only. Model/scope form a natural-width leading group with gap8, while the send/stop actions form a separate trailing group, preserving right alignment without stretching the scope into the gap. `components/AI_CHAT/AiChatComposerActionRow.tsx:74-90,146-152`; scope insets/icon/label/trailing geometry from `components/AI_CHAT/AI_Tiptap_Container.tsx:599-637`. Missing action presence remains C12-C14; missing scope options remain C11; selected scope text remains C16. Native regression sidebar_action_row_keeps_natural_scope_and_separate_right_actions pins groups, 8px gap, natural label+53px chip, right edge, send16x16, pointer toggle and truthful offline refusal. |
| I01 header title | Inbox only | Open selected conversation difference. `components/AI_CHAT/ChatHeader.tsx:387-426`. No invented board-session mapping or fixed header title. |

The source audit identifies six defensible candidate closures, not twelve.
Potential counts after native and artifact proof are Inbox12, Board9, Task11, total32.
The requested <=26 target remains unmet. C01/C03 would provide six more closures,
but require both a source-backed 32px row and reconciliation with the frozen
selector-only panel projection. No target claim is made by silently changing that
reference or adding an action-sized placeholder. The source-derived microphone
height explains the real web geometry; it is not evidence that transcription works.

The existing native stream writer does remove the old standalone create-session
blocker for plain text. It does not remove the attachment metadata or membership,
mention, microphone and model/preference route boundaries above. Human CLI stream
access itself remains flag-gated, `app/api/ai/_lib/requestUser.ts:66-100`; no live
credentials, writes or flag changes are used as proof.

## Hosted proof

Pending exact-head CI, native regressions, crop measurements and final receipt.
No builds, tests, lint, app execution, local test copies, or laptop access run here.

Frozen reference SHA256:

- inbox.png 8335348d1509e4f5605f35e519264f3c55442e964d6369b5f459487a4fa1020f
- board.png 2b7eae5de107ca11d03d972bf20cc4d970cf0a91254f2271f4590101850910b6
- task.png 2f75683c8acf6688da32e5b4579fbe6e9ab0937c67a63438eba115b2ddcc3143
