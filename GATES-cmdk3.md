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

Pending source audit and exact-head GitHub CI.

## Source audit and first CI correction

- Fresh TypeScript AST audit identifies 299 declarations, 297 fixed registry keys, and the runtime query handover. Dynamic Boards are separate. docs/command-center-parity.md records expressions, effective Task/Time slots, final visibility filters, dispatch and exact B/N contracts.
- Existing Command/CommandAction and native operations remain the shape. New keys: Follow system, search query handover, cached-child removal. Plain Search now opens main's existing search screen. Five existing board-form commands gain loaded Inbox/Task contexts. No screen layout was edited.
- CI https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40349200748 at 6e8295e36d4e81a8e24f301c0dbd8eef66d04e60 rejects E0624 on Linux, Windows, macOS and ARM64. navigate_search was shell-private. Widen only that existing entrypoint to crate::ui for palette dispatch. No local compiler/test execution.
- Source review also preserves precise limits: cold child snapshots, full document-search engine, focused-writer flags, session-only Inbox Zero/unread/reminder routes, complete history/version fields. No falsely invented route blocker on nullable parent writes or CLI Compose.
- Eight focused native tests now cover query handover across three surfaces, ranking placement, theme persistence, child selection/missing data, four-theme geometry/type/colors, five forms across three surfaces, missing catalog and foreign-column rejection. Existing transport/queue tests remain required.
