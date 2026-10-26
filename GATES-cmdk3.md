# Command center round 3 acceptance

Written before implementation. Scope: palette on inbox, board, task and AI sidebar. Dedicated cmdk3 worktree. No board/task/inbox layout changes. No local builds, tests, lint, test copies, app launches or laptop access.

## Acceptance checks

1. Re-read current web AllCommands.ts, commandDispatcher.ts and imported declarations/handlers. Inventory every applicable declaration with label, icon, group, order, shortcut, visibility and effect. Cite only /home/valentin/projects/hypertask/src file:line. Record baseline and final native status in docs/command-center-parity.md. Exclusions and missing routes/fields stay explicit.
2. Match supported palette metadata and web desktop geometry/tokens before adding behavior. Native tests assert complete context lists, row geometry, grouping and shortcuts. Do not add web-absent commands.
3. Implement commands where main already supplies the data and writer. Reuse Command/CommandAction and existing operations. Every behavior change gets native input or observable-state coverage. Unsupported commands name exact missing route/field or native prerequisite with citations, not invented authorization blockers.
4. GitHub CI alone compiles and executes proof. Exact final pushed SHA must pass every required job and the read-only desktop-ci receipt, including native input, layout, ARM64, updater, pstack and frozen Windows memory artifacts.
5. Fetch/rebase origin/main before each push. Commit focused units with requested co-author trailer. Open draft PR cmdk3 -> main. Record exact SHA, PR/run URLs, job conclusions, test names and receipt.

## Work sequence and throughput

- Source audit and acceptance contract first; palette metadata next; viable behavior and native tests next; exact-head CI corrections last.
- Existing typed Command rows and Action dispatch remain the data shape. No new command transport or modal framework.
- One helper, serial source/diff review. No subagents requested. Other helpers own unrelated layouts.
- Prove It Works selects actual GitHub test output and receipt, not compilation guesses. Sequence Verifiable Units keeps audit, metadata and behaviors separately reviewable.
- Unbundled Cursor tooling is unavailable; use repository host adapter and serial review. Do not claim independent review or live SaaS success from synthetic tests.

## Evidence

Source audit complete. Verified implementation snapshot below. Final pushed-head receipt and job evidence live on draft PR https://github.com/hypertask-ai/hypertask-desktop/pull/54.

## Source audit and first CI correction

- Fresh TypeScript AST audit identifies 299 declarations, 297 fixed registry keys, and the runtime query handover. Dynamic Boards are separate. docs/command-center-parity.md records expressions, effective Task/Time slots, final visibility filters, dispatch and exact B/N contracts.
- Existing Command/CommandAction and native operations remain the shape. New keys: Follow system, search query handover, cached-child removal. Plain Search now opens main's existing search screen. Five existing board-form commands gain loaded Inbox/Task contexts. No screen layout was edited.
- CI https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40349200748 at 6e8295e36d4e81a8e24f301c0dbd8eef66d04e60 rejects E0624 on Linux, Windows, macOS and ARM64. navigate_search was shell-private. Widen only that existing entrypoint to crate::ui for palette dispatch. No local compiler/test execution.
- Source review also preserves precise limits: cold child snapshots, full document-search engine, focused-writer flags, session-only Inbox Zero/unread/reminder routes, complete history/version fields. No falsely invented route blocker on nullable parent writes or CLI Compose.
- Eight focused native tests now cover query handover across three surfaces, ranking placement, theme persistence, child selection/missing data, four-theme geometry/type/colors, five forms across three surfaces, missing catalog and foreign-column rejection. Existing transport/queue tests remain required.

- CI https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40350494342 at 9508530759a2035da33dfb2ff0f5391865fdf9f7 rejects E0004: the existing nested generic field-write match must explicitly exclude the new child-removal variant already handled above it. Added that variant to the existing unreachable handled-action arm, retaining exhaustive dispatch.
- Serial source review corrected the test's ambiguous Rename board search selection: web prefix scoring also ranks Rename board column. Native keyboard tests now select the exact visible row with arrows. No production ranking weakened. Query input line height now 21px, and palette search glyph uses its own web 1.75-stroke asset; unrelated screen icons remain unchanged.

- CI https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40351290376 at 642dfb3c5b50395da42ba987fcd6ba9826261d36 runs all eight new native tests successfully on Windows/macOS; macOS reports 286 passed, one failed. The retained fresh_command_search_fetches_and_keyboard_opens_uncached_task test still expects the old picker loading element. Update only its loading-element assertion to the existing Search screen and retain its actual server fetch, keyboard Enter, uncached detail/catalog/draft loading and final task assertions. Preserve Enter result opening through the existing search route, with no pending-request or blank-query activation.
- User-account GitHub quota expired during gh run watch. The read-only scoped GH_REAL adapter from round 2 reuses the installed wrapper's child_env in memory and a 120-second watch interval. No token printed/stored and no global configuration changed. Required waiter remains the entrypoint; exact-head receipt remains mandatory.

## Verified implementation snapshot

- SHA ebbe642f300852a1ddf40f8845f81616f829b2ee, run attempt 1: https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40353373314. Exact-head waiter exits 0 and prints DESKTOP_CI_VERIFIED for that SHA. The following follow-up changes only documentation; its final-head receipt must also succeed before handoff.
- Linux, Windows and macOS each report `287 passed; 0 failed`. All eight cmdk3 tests and `fresh_command_search_fetches_and_keyboard_opens_uncached_task` pass on all three. Linux additionally prints `DESKTOP_INPUT_ARCHIVE_VERIFIED action=e persisted=Archive queue=inbox_archive cleanup=complete evidence=retained`.
- Success jobs: Pstack contract, ubuntu-24.04, windows-2025, macos-15, windows-arm64, updater-ubuntu-24.04, updater-windows-2025, Windows memory receipt and budget. Optional historical memory-reference matrix is skipped by its existing branch condition. No required job skipped.
- Read-only receipt retains 13 nonexpired artifacts. Required proof includes desktop-input 12664669499, windows-input 12667185751, layout-dumps 12664329163, web-source-fonts 12664737276, pstack-contract 12665196497, windows-memory 12665419284 and windows-memory-heavy 12667315328, plus all four platform binaries. Full artifact SHA-256 digests are in the waiter receipt and final PR evidence. No local workload or laptop access.
- Native fixed-key availability 87 -> 90. Newly available query handover, Follow system and cached-child removal. Five existing board forms gain loaded Inbox/Task contexts. Child removal remains partial for cold snapshots; exact document search, focused writer flags and B/N contracts remain open, not falsely Done.
- Source/diff self-review only; no independent reviewer claimed. Prove It Works selected real GitHub input/test/receipt output. Sequence Verifiable Units kept acceptance, inventory, behavior and CI corrections separately reviewable.
