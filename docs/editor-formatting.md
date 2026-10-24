# HTPR-7381 rich-text editor

Web contract pinned at `6f9901764d355aba7bfc76e4e62ff255e0b68b31` in sibling `hypertask`. No VPS builds/tests/test copies and no owner-laptop access. Proof is exclusively GitHub CI (workspace tests, GPUI test contexts, Linux xvfb layout dump).

## Contract

* `src/components/RTE/Tiptap.ts:135-196`: StarterKit, ordered-list `numbered-list`, task-list `taskList`, multicolor highlight, underline, non-inclusive/autolink/paste links with `_blank`.
* `src/lib/constants/shortcuts.ts:223-243`: editor chords; `:109` Ctrl+B switches boards outside editing; `:188` discard.
* `src/components/RTE/useTaskDetailEditorKeyboard.tsx:62-97`: heading overrides, selected/all inline code and JavaScript code-block override; `:113-117` discard maps create-comment to Comment and other rich-editor modes to Description.
* `src/components/RTE/Extensions/SlashCommands/SlashCommands.tsx:18-28`: slash anywhere, no required prefix.
* `src/components/RTE/Extensions/SlashCommands/items.ts:66-320,339-517`: AI command values, video URL prompt, create-comment-only emoji, heading/highlight/list/paragraph/mark commands; Code/Code Block move to the document end.
* `src/components/RTE/Extensions/SlashCommands/CommandsList.tsx:92-157`: wrapping up/down, Enter, case-insensitive title/type/alias filtering, selected-index reset.
* `src/components/RTE/Components/EditorFormatButtons.tsx:12-97,196-250`: selection bubble link/unlink, marks, normal text and code commands.
* `src/components/RTE/Components/TiptapBubbleMenu.tsx:39-64,107-120`: selection-only bubble and exact highlight colors.
* `src/components/RTE/Components/EmojiGifPicker.tsx:248-256`; `src/components/RTE/Extensions/LazyEmoji.ts:157-166,425-483`: selected emoji serializes as the named emoji span, not an invented attachment.
* `src/components/RTE/useTaskDetailEditorWriter.tsx:49-85`: URL normalization, extending an existing link and unsetting highlight.
* `src/styles/tiptap.module.scss:78-166,223-277`; `src/styles/globals.scss:214-216`: mark/code/heading/list/quote styles.
* `src/components/RTE/Extensions/ReplyBlockquote.ts:61-100`: Enter leaves a reply quote rather than editing the quoted content.

## Inventory and CI acceptance map

The 35 requested P1 rows are reconciled below. A passing insertion test is not evidence of an implemented video player or upload workflow.

| Rows | Native behavior / CI tests |
|---|---|
| F119 | `discard_is_surface_scoped_and_queues_the_authorized_server_lifecycle`, `cached_web_drafts_load_in_their_own_editor_and_discard_keeps_published_content`; core `discard_resolves_fresh_own_ids_after_restart_and_preserves_other_draft_type`, `discard_denial_restores_cached_draft_and_never_deletes_another_account`. Published HTML is never replaced by a discarded draft. |
| F151 | `slash_filter_navigation_dismissal_modes_and_emoji_insert`, `slash_and_bubble_formatting_work_with_real_pointer_events`: real typed slash, filtering, wrapping navigation, Escape, Enter and measured mouse selection. |
| F152 | **Partial / blocked rendering** — URL prompt and local YouTube HTML insertion are real (`unsupported_web_services_are_not_inert_slash_commands_and_video_html_is_real`), including atom-only saves, but the app has no iframe/player host. No fake player. No authenticated endpoint is required for the web insertion (`items.ts:343-345`; `Extensions/YoutubeTiptap/index.tsx:97-121,130-142`); this is **not** an auth blocker. |
| F153 | **Blocked contract** — only `src/lib/constants/shortcuts.ts:225` declares double-Tab autocomplete. There is no registered handler or route in the inspected RTE source. No guessed completion endpoint or fake text is added. Route/auth helper: none registered. |
| F154, F161, F162 | Existing multiline/IME tests plus `slash_format_is_one_undo_transaction_and_history_never_reopens_a_dialog`; document tests cover hard breaks versus paragraph/list splits, empty-list/quote exit and code newlines. |
| F155–F160, F163–F172 | `every_format_keyboard_and_slash_choice_is_a_real_editor_action` exercises each chord and supported formatting slash choice; `rich_editor_real_keys_marks_caret_history_and_slash` covers caret marks and selection history; `rich_html_all_three_editor_surfaces_use_real_keys_and_the_durable_queue` covers create/edit/description and the painted Ctrl+B board switcher outside editors. |
| F593–F598 | `all_six_ai_slash_commands_queue_apply_and_undo_without_publishing_on_all_surfaces`; core `all_six_web_ai_commands_use_same_prompt_contract_through_durable_bearer_queue`, `ai_denial_and_invalid_success_never_pretend_to_improve_or_publish_content`. Results remain local until Save. |
| F599, F602 | **Blocked upload staging** — exact routes/auth below; the bearer attachment endpoint immediately changes published content and is not substituted for a cancellable editor upload. |
| F600 | `slash_filter_navigation_dismissal_modes_and_emoji_insert`: bundled web catalog, search and real named-emoji HTML insertion, create-comment-only visibility. |
| F601 | **Blocked external credential** — exact GIPHY route/key below. No fabricated search results. |
| F603, F604 | Formatting keyboard/slash test plus `task_checkboxes_list_enter_and_mixed_highlight_colors_are_real_editor_transactions`; task/ordered list class, data attributes, checked state and Enter behavior have document round-trip tests. |
| Links (explicit task scope) | `links_autolink_paste_selection_palette_and_dialog_are_rich_transactions`, `clicking_a_link_uses_web_open_on_click_without_shell_shortcuts`; direct URL marks, validation, paste-on-selection, unlink and HTML round trips. **Task suggestion lookup remains blocked on the exact cookie-only search contract below.** |

Every saved formatting family additionally passes `every_web_format_round_trips_queue_restart_and_existing_bearer_wire_contracts` in `crates/ht-core/tests/editor_html.rs`: real durable enqueue/reopen/drain and exact POST/PATCH HTML bodies, without rewriting the markup. `editor_document.rs` also tests Unicode edit invariants, nested marks, partial list/quote lifting, list conversion, code selection replacement and preservation of unrelated media/quote attributes. `rich_text.rs` checks rendered headings, custom highlights, code, links, numbering and task markers; GPUI tests measure editor font sizes/gutters and click actual painted controls.

## Authorized writes, not false auth blockers

* **AI**: web POST `/api/ai/tiptap-forwardslash` is cookie-only (`src/app/api/ai/tiptap-forwardslash/route.ts:31-36`, `getCurrentUserFromCookies`). Bearer **POST `/api/mcp/ai/improve`**, **`validateMcpRouteAuth`**, uses the **same** `createPromptForTiptapForwardSlash`, model selection, HTML-block preservation and normalized result (`src/app/api/mcp/ai/improve/route.ts:28-104`; `schema.ts:3-34`). All six web command values are queued to this equivalent, not blocked or replaced with another prompt. Its 60-second server allowance fits a 70-second request timeout and the existing 120-second queue lease. Denial is an error, never an invented result.
* **Discard**: web POST `/api/drafts/deleteDrafts` uses `getSessionUser` (`src/pages/api/drafts/deleteDrafts.ts:10-16`). Bearer **GET `/api/mcp/drafts?task_id=…`** and **DELETE `/api/mcp/drafts/[draft_id]`**, **`validateMcpRouteAuth`**, provide own-draft/type deletion (`src/lib/mcp/operations/drafts/operation.ts:185-269`; `src/lib/mcp/operations/drafts/[draft_id]/operation.ts:179-257`, ownership at `:235-240`). IDs are resolved at drain time, not guessed from stale cache. Draft cache and optimistic flags roll back on denial. Successful publication queues matching draft cleanup only **after** the server accepts the comment/description; a denied Save cannot delete the draft.

All external writes run through `crates/ht-core/src/queue.rs`. AI results and pending editor identity/source are durable local records. Results cannot overwrite another task, another comment, a discarded/saved editor, or newer typing. No provider keys or cookie credentials are copied into native.

## Exact remaining blockers

* **F599/F602**: POST **`/api/tasks/uploadUrl`** (`src/lib/storage/uploadViaApi.ts:128-156`) requires **`verifySession` / `getSessionUser`** (`src/lib/api/task-writes/upload-url.ts:235-240`). POST **`/api/tasks/uploadFinalize`** uses the same helpers (`src/lib/api/task-writes/upload-finalize.ts:210-215`). Buffered POST **`/api/tasks/n8nUpload`** (`src/lib/storage/uploadViaApi.ts:169-171`) uses **`verifySession` / `resolveBetterAuthSession`** (`src/pages/api/tasks/n8nUpload.ts:120-126`). Bearer POST **`/api/mcp/tasks/attachments`** does exist, using **`validateMcpAuth`** (`src/lib/mcp/operations/tasks/attachments/operation.ts:56-66`), but it immediately attaches to an existing saved comment/description (`:138-179`). Calling that on slash selection would mutate published content before Save/Cancel and is not staging parity.
* **F601**: GET **`https://api.giphy.com/v1/gifs/{search,trending}`**, `src/components/RTE/Components/EmojiGifPicker.tsx:144-169`, requires **`api_key=NEXT_PUBLIC_GIPHY_API_KEY`**. This is an external API-key parameter, not a Hypertask MCP auth helper. No native key/config is available; no key is invented or extracted from owner credentials.
* **Task-link suggestions**: web GET **`/api/tasks/searchByParam`**, `src/components/Modals/LinksModal/SetLinkModal.tsx:47-58`, uses **`taskWriteHandler` / `taskWriteRoute` → `getSessionUser`** (`src/lib/api/task-writes/route.ts:32-35,89-91`). Its task subset comes from the mixed mention search/ranking (`src/utils/controllers/tasks/taskSearchByParam.ts:109-181,194-212`). Bearer GET `/api/mcp/tasks/search` exists (`validateMcpRouteAuth`, `src/lib/mcp/operations/tasks/search/operation.ts:65-101`) but is a different full-text/operator/result-ranking contract; it is not mislabeled as the web task-link lookup. Direct external/relative URL links work without this request.

The five blocked inventory rows and the remaining task-link lookup are explicitly not claimed as full web-editor parity. A green CI run verifies implemented behavior, not these unavailable capabilities.
