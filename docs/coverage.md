# Interaction coverage

This ledger covers inbox, board, task, AI sidebar, and shortcuts. Synthetic CI proves local behavior, not live-server authorization. Other parity rows keep their existing status.

## Proof layers

| Feature | Unit and GPUI | Native OS | Live server |
| --- | --- | --- | --- |
| Inbox | Workspace tests include `inbox_open_marks_only_target_seen_and_j_k_e_and_tabs_keep_keyboard_flow` and row archive/reminder tests in `crates/ht-desktop/src/inbox.rs`. | Existing Linux E smoke retains `desktop-input`. New hosted Windows E case requires the selected row to disappear and exactly its durable archive write. | NOT TESTED by this suite. No owner token or live write. Existing route/auth limits remain. |
| Board | `board_checkbox_selects_without_opening_and_card_click_preserves_selection`, search, bulk actions, and return-selection tests in `crates/ht-desktop/src/board.rs`. | New hosted Windows F2 case replaces the selected card title, stays on Board, and checks its durable title write. Mouse selection, drag, bulk actions, and other native board shortcuts remain NOT TESTED here. | NOT TESTED by this suite. Fixture queue success is not server acceptance. |
| Task description | Workspace editor/queue tests cover HTML, selections, formatting, and rejection behavior. See `docs/editor-formatting.md` and `docs/data-fidelity.md`. | New hosted Windows Ctrl+D, select-all, type, Ctrl+Enter case requires visible saved description and exact HTML queue/storage values. Other native description formatting remains NOT TESTED here. | NOT TESTED by this suite. Existing lossy-editor and authorization limits remain. |
| Task comments | GPUI tests cover draft ownership, send races, focused composer, and rich editing. See `docs/compose-comments.md` and `crates/ht-desktop/src/comment_drafts.rs`. | New hosted Windows Ctrl+M, type, Ctrl+Shift+Enter case requires a displayed comment, empty composer, unchanged task, and exactly one pending comment. Existing Linux highlighted-text pixel check remains. Advanced comment actions remain NOT TESTED here. | NOT TESTED by this suite. No provider key or authenticated comment send. |
| AI sidebar | `sidebar_enter_still_sends_and_shift_enter_is_exactly_one_newline`, offline composer, lazy history, stream ownership, and presentation tests. HTTP/SSE fixtures remain mocks. | NOT TESTED by this Windows suite. Layout and memory captures do not prove input. | NOT TESTED by this suite. Keep `docs/ai-parity.md` blockers. |
| Keyboard shortcuts | `complete_supported_key_map_and_contexts`, modifier guards, shifted punctuation, and feature-specific GPUI tests. | This migration covers E, F2, Ctrl+D, Ctrl+M, Ctrl+A, Enter, Ctrl+Enter, and Ctrl+Shift+Enter only in their tested contexts. Other shortcuts remain NOT TESTED at native OS level. | NOT TESTED by this suite. |

## Migrated cases

Cases originate in `gates/behavior-cases.json`. Expected product behavior comes only from the web source checkout `/home/valentin/projects/hypertask/src`, revision `6f9901764d355aba7bfc76e4e62ff255e0b68b31`.

| Original case | Web contract, relative to web checkout | Visible and durable acceptance | Deliberate break |
| --- | --- | --- | --- |
| `inbox-archive-notification` | `src/app/inbox/Inbox.tsx:1021`; `src/hooks/Inbox/useGlobalFocusHandler.tsx:85-100,279-310`; archive key in `src/lib/configs/inbox.config.ts:72` | Selected row leaves active split. Only the independently selected notification becomes Archive with archivedAt. One pending `inbox_archive`, zero attempts. | Suppress only `Write::InboxArchive` effects. Unchanged native E must fail the visible row/count assertion. |
| `board-rename-f2` | `src/lib/keyboard/taskShortcuts.ts:148`; `src/components/PageComponents/Kanban/KanbanTaskComponents/task.tsx:761` | Saved title appears on the selected card. Stay on Board. One pending `task_title` with the same task ID and text. SQLite title matches. | Suppress only `Write::TaskTitle` effects. Unchanged F2 edit/save must fail the visible title assertion. |
| `task-description` | `src/app/detail/[...slug]/taskDetailEditingKeymap.ts:26-35`; `src/components/RTE/useTaskDetailEditorKeyboard.tsx:121-140`; `src/hooks/Task Detail/CommentAndDescriptionHooks/useSaveContent.ts:500-501` | Editor closes. Saved text appears in description. One pending `task_patch` with exact HTML and `content_type=html`. SQLite matches. | Suppress only description `Write::TaskPatch` effects. Unchanged edit/save must fail the visible description assertion. |
| `comment-send-stay` | `src/app/detail/[...slug]/taskDetailEditingKeymap.ts:72-80`; `src/lib/taskDetailInboxFlow.ts:89-105`; `src/components/RTE/useTaskDetailEditorKeyboard.tsx:121-140` | New comment appears. Composer clears. Same task stays open. One pending `task_comment` with exact HTML and task ID. SQLite holds one matching optimistic comment. | Suppress only `Write::TaskComment` effects. Unchanged send must fail the visible new-comment assertion. |

Synthetic text and demo identities are test data. They are not product defaults. No UI or product behavior changes belong to this migration.

## Evidence and cleanup

`gates/windows-input.py` uses foreground Win32 SendInput, not internal actions. Each case imports `fixtures/demo` into a fresh temporary database. It retains screenshots, measured elements, action logs, SQLite backups, executable digest, and process/database cleanup results. Before/after screenshots must contain presented content and different pixels. The database is checked again after owned-process shutdown.

Baseline proof passed for all four cases at `97d0f04c16d7b80c7ab7165a00246e7b8c3acd0e`.
[Baseline hosted run](https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40238507075) passed all six required jobs. Its receipts retain exact pending writes and successful owned cleanup. Reviewing its images exposed early capture before the saved frame presented. The suite now crosses two further frame boundaries and flushes the desktop compositor before capture.

`windows-input` retains `baseline/`, `broken/`, and `restored/` per-case receipts, screenshots, snapshots, and SQLite backups. `negative-control.patch` records four distinct production write suppressions in one hosted-only build. Every case runs in its own process and database. Each rejection must be `EffectMissing`, with its matching production suppression marker. Startup, typing, focus, database, and cleanup failures cannot pass as rejection proof. Source and executable bytes are restored before all four cases must pass again. `negative-control.json` binds both executable hashes, source restoration, revision agreement, and control cleanup. Final exact-head rejection/restoration proof remains pending until CI completes.

The native Windows result does not certify Intel hardware pixels. Linux screenshots do not certify Windows pixels. Neither proves live authorization. Unmigrated cases remain covered only at their recorded layer.
