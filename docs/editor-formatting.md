# HTPR-7381 rich-text editor

Contract source: sibling `hypertask` web repository at `6f9901764d355aba7bfc76e4e62ff255e0b68b31`. Native editor values are derived from these files, not a new toolbar/command design.

* `src/components/RTE/Tiptap.ts:135-196`: StarterKit, ordered-list `numbered-list`, task-list `taskList`, multicolor highlight, underline, non-inclusive/autolink/paste links with `_blank`.
* `src/lib/constants/shortcuts.ts:223-243`: editor chords; `:110` Ctrl+B board switching outside editing; `:188` discard.
* `src/components/RTE/useTaskDetailEditorKeyboard.tsx:62-97`: heading overrides, selected/all inline code and JavaScript code-block override.
* `src/components/RTE/Extensions/SlashCommands/SlashCommands.tsx:18-28`: slash anywhere, no required prefix.
* `src/components/RTE/Extensions/SlashCommands/items.ts:339-517`: video URL prompt, create-comment-only emoji, heading/highlight/list/paragraph/mark commands; Code/Code Block move to the document end.
* `src/components/RTE/Extensions/SlashCommands/CommandsList.tsx:92-157`: wrapping up/down, Enter, case-insensitive title/type/alias filtering, selected-index reset.
* `src/components/RTE/Components/EditorFormatButtons.tsx:12-97,196-250`: selection bubble actions, link/unlink, normal text and code commands.
* `src/components/RTE/Components/TiptapBubbleMenu.tsx:39-64`: highlight colors and nonempty paragraph/heading selection visibility.
* `src/styles/tiptap.module.scss:78-166,223-240`: mark/code/heading/list appearance.

## Auth-limited inventory (no pretend handlers)

* F593–F598: the web POST `/api/ai/tiptap-forwardslash` is cookie-only (`src/app/api/ai/tiptap-forwardslash/route.ts:31-36`, `getCurrentUserFromCookies`). However, POST **`/api/mcp/ai/improve`** accepts **`validateMcpRouteAuth`** and uses the same `createPromptForTiptapForwardSlash`, model selection, HTML-block preservation and normalized HTML result (`src/app/api/mcp/ai/improve/route.ts:28-104`; `schema.ts:3-34`). This is an authorized equivalent, not an auth blocker.
* F599/F602: POST `/api/tasks/uploadUrl`, `src/lib/storage/uploadViaApi.ts:128-156`; `src/lib/api/task-writes/upload-url.ts:235-240` requires `verifySession` or `getSessionUser`. Finalize is POST `/api/tasks/uploadFinalize`; buffered fallback is POST `/api/tasks/n8nUpload` (`src/lib/storage/uploadViaApi.ts:169-171`), gated by `verifySession` / `resolveBetterAuthSession` (`src/pages/api/tasks/n8nUpload.ts:120-126`). No upload/storage credentials are copied into desktop.
* F119: the web POST `/api/drafts/deleteDrafts` is cookie-only (`src/pages/api/drafts/deleteDrafts.ts:10-16`, `getSessionUser`), but bearer **GET `/api/mcp/drafts?task_id=…`** and **DELETE `/api/mcp/drafts/[draft_id]`** implement the same own-task/type discard (`src/lib/mcp/operations/drafts/operation.ts:185-269`; `src/lib/mcp/operations/drafts/[draft_id]/operation.ts:179-257`, **`validateMcpRouteAuth`**, ownership checked at `:235-240`). This is not an auth blocker.
* F601: GET `https://api.giphy.com/v1/gifs/{search,trending}`, `src/components/RTE/Components/EmojiGifPicker.tsx:144-169`, requires `NEXT_PUBLIC_GIPHY_API_KEY` (external Giphy API key, **not** MCP auth); no native key/config is available and no fabricated result is shown.
* Task suggestions inside Link modal: GET `/api/tasks/searchByParam`, `src/components/Modals/LinksModal/SetLinkModal.tsx:47-58`, through `taskWriteHandler`/`taskWriteRoute` → `getSessionUser` (`src/lib/api/task-writes/route.ts:32-35,89-91`). Direct URL marks do not require this request.

Video embed itself does **not** need an authenticated endpoint: `items.ts:343-345` and `Extensions/YoutubeTiptap/index.tsx:97-121,130-142` insert local iframe HTML. Native playback parity must not be labeled an auth blocker.

Builds, tests and layout-dump proof run only in GitHub CI. No VPS test copies or owner-laptop access.

MCP POST `/api/mcp/tasks/attachments` does exist and uses `validateMcpAuth` (`src/lib/mcp/operations/tasks/attachments/operation.ts:56-66`). It attaches immediately to an existing saved comment/description (`:138-179`), not to a cancellable unsaved composer draft. It must not be called on slash selection as a fake replacement for upload staging; it would mutate published content before Save.

F153: double-Tab autocomplete has only the shortcut declaration (`src/lib/constants/shortcuts.ts:225`); no corresponding handler or route is registered in the inspected `src/components/RTE` tree. No fabricated completion or guessed AI endpoint is added.

F152 is partial: the URL prompt inserts real local YouTube HTML and can queue an atom-only document. The GPUI app has no iframe/player host, so embedded playback/rendering parity remains blocked by that native capability, **not** bearer authorization. No fake player is rendered.
