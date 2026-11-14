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

`gates/windows-input.py` uses foreground Win32 SendInput, not internal actions. Each case imports `fixtures/demo` into a fresh temporary database. It retains screenshots, measured elements, action logs, SQLite backups, executable digest, and process/database cleanup results. Before/after native PrintWindow screenshots must contain presented content and different pixels. This is the existing Windows memory capture method. Desktop copies and per-method color counts survive alongside them for capture diagnosis. The database is checked again after owned-process shutdown.

Baseline proof passed for all four cases at `97d0f04c16d7b80c7ab7165a00246e7b8c3acd0e`.
[Baseline hosted run](https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40238507075) passed all six required jobs. Its receipts retain exact pending writes and successful owned cleanup. Reviewing its images exposed early capture before the saved frame presented. The suite now crosses two further frame boundaries and flushes the desktop compositor before capture.

`windows-input` retains `baseline/`, `broken/`, and `restored/` per-case receipts, screenshots, snapshots, and SQLite backups. `negative-control.patch` records four distinct production write suppressions in one hosted-only build. Every case runs in its own process and database. The gate requires native key acknowledgement and two further frames before checking these synchronous write effects. Missing-effect controls must retain an empty mutation queue. This suite is not an idle-render soak test. Each rejection must be `EffectMissing`, with its matching production suppression marker. Startup, typing, focus, database, and cleanup failures cannot pass as rejection proof. Source and executable bytes are restored before all four cases must pass again. `negative-control.json` binds both executable hashes, source restoration, revision agreement, and control cleanup. Latest exact-head proof is recorded on [draft PR checks](https://github.com/hypertask-ai/hypertask-desktop/pull/31/checks). Accept only completed green CI with `NATIVE_INPUT_CONTROLS_VERIFIED` and all four restored cases.

The native Windows result does not certify Intel hardware pixels. Linux screenshots do not certify Windows pixels. Neither proves live authorization. Unmigrated cases remain covered only at their recorded layer.

## HTPR-7381 acceptance jobs

The Bench workflow measures three budgets outside the app's own log line.

- Cold start: `gates/cold-start.mjs` runs Hyperfine on macOS and writes `evidence/cold-start/hyperfine.json`. The median must be under 300 ms.
- Input to frame: `gates/presentmon-frame.ps1` sends operating-system keystrokes on Windows while PresentMon 2.6 records. `gates/presentmon-score.py` reads `MsAllInputToPhotonLatency` and `MsBetweenPresents`. Input-to-photon p95 must be under 16 ms.
- Memory: `gates/memory.mjs` builds the input benchmark and keeps the 100 MB cap.
- Live readback: `gates/live-readback.py` writes every supported edit through `ht-core-cli write-json` on QA Sandbox board 7049, reads each one back with its own API GET, and prints `KIND_PASS <kind>` or `KIND_FAIL <kind>`. It then proves reopening three ways: the same database in a new process, a resync of that database, and a fresh database, printing `REOPEN_PASS` or `REOPEN_FAIL` per edit. It deletes its test tasks, restores the board title, removes its test column and unarchives the inbox row before it exits. It prints `LIVE_READBACK_OK` only when every kind and every reopen check passes. The web repo workflow `desktop-live-readback.yml` runs it from production with the QA login at a pinned desktop commit.

## Supported edits and exclusions

Every edit below is reachable from the Inbox, Board or Task detail screens and goes through `Db::enqueue` and `drain_once`.

| Edit | Write kind | Live check |
| --- | --- | --- |
| Rename task | `task_title` | Task GET title |
| Move task to a column | `task_section`, `board` `MoveTask` | Task GET column |
| Description | `task_patch` `Description` | Task GET description |
| Priority, size, due date | `task_patch` `Priority`, `Estimate`, `DueDate` | Task GET fields |
| Labels | `board` `CreateTaskLabel`, `task_patch` `Labels` | Task GET labels |
| Assignees, assign to me | `task_patch` `Assignees`, `board` `AssignToMe` | Task GET assignees |
| Parent and remove subtask | `task_patch` `Parent`, `board` `RemoveSubtask` | Task GET parent |
| Archive and restore task | `task_patch` `Status` | Task GET status |
| Star task | `board` `StarTask` | Task GET saved flag |
| Waiting on | `board` `WaitingOn` | Task GET waiting user |
| Timer | `board` `ToggleTimer` | Running timer GET |
| Move to inbox | `board` `MoveToInbox` | Inbox list GET |
| Mark inbox row seen | `inbox_seen` | Inbox list GET |
| Remind or archive inbox row | `inbox_remind`, `inbox_archive` | Inbox list GET, then unarchive |
| Send, edit, react, save, delete comment | `task_comment`, `comment_action` | Comments GET, saved content GET |
| Comment draft save and discard | `comment_draft`, `discard_draft` | Drafts GET |
| Compose task | `compose_task` | Task GET |
| Add, rename, delete column | `board` `AddColumn`, `RenameColumn`, `DeleteColumn` | Sections GET |
| Rename board | `board` `RenameBoard` | Projects GET, then restored |

Excluded, with the reason:

- `board` `ArchiveBoard`: the API cannot create a throwaway board, and archiving the shared sandbox would break design QA.
- `board` `AssignAgent`: needs a second agent identity that the QA login does not own.
- `shell` writes (profile, webhooks, skills, project instructions, members): not on the Inbox, Board or Task screens.
- `editor_improve`: calls the AI writer, which is not a stored edit.

Theme captures for Porcelain, Graphite, AMOLED and Dia land in `evidence/theme-pixels`. Light and Graphite 2 are not desktop themes, so this job does not score them. Pixel scores against the live app run only when `gates/fixtures/live-theme-pixels/<theme>/<screen>.png` is present.
