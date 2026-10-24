# HTPR-7381 rich-text editor

Contract source: sibling `hypertask` web repository. Native editor values are derived from these files, not a new toolbar/command design.

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

* F593–F598: POST `/api/ai/tiptap-forwardslash`, `src/app/api/ai/tiptap-forwardslash/route.ts:31-36` calls `getCurrentUserFromCookies`; `src/app/api/ai/_lib/editorAi.ts:102-108` uses cookie session. No direct MCP equivalent of the web editor-rewrite contract is substituted.
* F599/F602: POST `/api/tasks/uploadUrl`, `src/lib/storage/uploadViaApi.ts:128-156`; `src/lib/api/task-writes/upload-url.ts:235-240` requires `verifySession` or `getSessionUser`. Finalize is POST `/api/tasks/uploadFinalize`; buffered fallback is POST `/api/tasks/uploadFiles`. No upload/storage credentials are copied into desktop.
* F119 full persisted lifecycle: POST `/api/drafts/deleteDrafts`, `src/components/RTE/useTaskDetailEditorDrafts.tsx:64-91`; `src/pages/api/drafts/deleteDrafts.ts:10-16` calls `getSessionUser`. Existing local discard is not represented as server draft deletion.
* F601: GET `https://api.giphy.com/v1/gifs/{search,trending}`, `src/components/RTE/Components/EmojiGifPicker.tsx:144-169`, requires `NEXT_PUBLIC_GIPHY_API_KEY` (external Giphy API key, **not** MCP auth); no native key/config is available and no fabricated result is shown.
* Task suggestions inside Link modal: GET `/api/tasks/searchByParam`, `src/components/Modals/LinksModal/SetLinkModal.tsx:47-58`, through `taskWriteHandler`/`taskWriteRoute` → `getSessionUser` (`src/lib/api/task-writes/route.ts:32-35,89-91`). Direct URL marks do not require this request.

Video embed itself does **not** need an authenticated endpoint: `items.ts:343-345` and `Extensions/YoutubeTiptap/index.tsx:97-121,130-142` insert local iframe HTML. Native playback parity must not be labeled an auth blocker.

Builds, tests and layout-dump proof run only in GitHub CI. No VPS test copies or owner-laptop access.
