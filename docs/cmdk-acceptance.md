# Command center acceptance checks

## Checks written before implementation

1. Inventory every web registry command in `docs/cmdk-parity.md`, with source file:line, native support, and explicit unsupported or out-of-scope reasons. No invented commands.
2. GPUI command-list tests assert the complete supported key, label, shortcut, and group sequence for Inbox, Board, Task, comment focus, archive state, and bulk selection. Missing or reordered commands fail. Prove the initial regression fails in GitHub CI, then passes after the fix.
3. GPUI keyboard input opens Ctrl+K, filters commands, and executes supported AI sidebar actions. Assert visible state and command usage, not only registry presence.
4. Match web desktop row and modal geometry, header search icon, no mobile-only row icons, keycaps, group headings, and New badges. Cite web values. Geometry assertions run in GPUI CI. Full screenshot identity remains unproven without a web/native paired capture.
5. Fetch and rebase before every push. Finish with exact-head GitHub CI, all six required jobs, nonexpired proof artifacts, and the read-only `desktop-ci-receipt.py` receipt. No local build/test/lint/app launch. No laptop access.

## Work sequence

- Read registry, dispatch, native commands, and verification map.
- Commit inventory and regression tests. Observe CI failure.
- Keep the existing typed `Command` and `CommandAction` data shapes. Order supported commands once after context-specific producers. Reuse existing native action paths.
- Push the smallest focused fix. Inspect every CI job and receipt.

## Throughput checkpoint

- Blocking first steps: source inventory and acceptance checks.
- Independent workstreams: other helpers own keys2/nativeci/look8/prewire. This branch owns command-center metadata, assembly, rendering, focused GPUI tests, and these docs.
- Shared mutable state: dedicated cmdk worktree. Only minimal action visibility changes outside command-center files.
- Smallest safe decomposition: one owner. Registry order and context assembly need one consistent review. No delegated workers.

## Verification limits

Unavailable Cursor built-ins use serial self-review instead. Prove It Works requires actual CI results. Sequence Verifiable Units requires red/green commits. Model the Domain preserves typed actions rather than adding inert UI rows. Remote authorization gaps stay explicit.

## Source-driven design

`command_center.rs` produces supported base metadata. `board.rs` and
`comment_actions.rs` add context-specific actions. `interactions.rs` currently
appends these after its base sort, splitting groups and losing web order.
The final assembly must order all producers before inserting board navigation
and recommendations. Usage must sort within groups, not just recommendations.

Two serial alternatives were considered. A second complete web registry would
repeat labels/actions and introduce unsupported rows. A final stable group/key
ordering table preserves the existing typed commands and action handlers.
The latter is the smaller fix. No new transport or feature screen is needed.

Native task reminders only target existing inbox notifications. They cannot
stand in for the web's arbitrary task reminder. That row remains blocked.
