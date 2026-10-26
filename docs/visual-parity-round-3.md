# Visual parity round 3

## Acceptance checks written before implementation

All executable checks run in public GitHub CI only. No VPS builds, tests,
lint, app launches, test copies, or laptop access. Frozen web references and
comparison tolerances remain unchanged. Scope is inbox, board, task; AI sidebar
must remain pixel-identical and keyboard shortcut code must remain unchanged.

1. Baseline is round 2 SHA 3d57a0d87e58c1cdeeee11b6f800ab5569bdf58f.
   Original remaining inventory is Inbox 30, Board 30, Task 29, total 89.
   Retain every original row identity. Final total must be at most 64.
   Evidence: exact-head hosted Windows screenshots/layout versus the same
   frozen web boxes, plus original non-measured class inventory.
2. Board columns use the web count-dependent rule, not fixed 280px.
   Native layout tests cover three columns and overflowing column counts,
   card padding, content-dependent height, title and command reservation.
3. Task header/title use web sizing and wrapping; description follows actual
   content, metadata follows the header, composer follows content/comments.
   Native layout tests pin each source value and finite composer position.
4. Inbox tabs use web labels, gaps and widths. Notification row author/title/
   kind sizing follows source. Native layout tests pin these bounds.
5. Capture fixture matches frozen reference content where recoverable.
   Keep dense demo/heavy fixtures and interaction tests unchanged. Never hide
   actual UI. Record every capture-only data change and its reference evidence.
6. AI sidebar x1500..1919 has zero changed pixels on all three screenshots
   versus round 2. No keyboard shortcut code diff. No reference/threshold diff.
7. Each fix includes web component and token file:line citations. Named native
   tests must pass on Ubuntu, Windows and macOS. Every required exact-head CI
   job succeeds, including ARM64 and both memory receipts. Supplied read-only
   CI waiter must emit DESKTOP_CI_VERIFIED with retained artifact evidence.

## Execution units

1. Source and fixture inventory; board geometry and capture content.
2. Task header, description, composer geometry.
3. Inbox tabs and row sizing.
4. Exact-head hosted screenshots, original-row reconciliation, sidebar diff,
   full CI receipt, and remaining-defect fixes until all checks hold.

Principle Prove It Works selects hosted artifacts over compilation claims.
Principle Sequence Verifiable Units selects component commits and CI brackets.
Self-review only. Unavailable Cursor-only tools use repo-native checks instead.

## Source inventory and data diagnosis

Round 2 dense demo contains 13 board columns and 50 task comments, not the
three columns and zero comments in the frozen web sample. Its first four
columns happen to be the collector's maximum, not the board's total count.
The existing native rule already computes 97% of available width, 16px gaps,
and 280..535px columns. At thirteen columns it correctly reaches 280px.
At three columns its prior hosted reference dump is already 459px wide.
The dense title is different and legitimately wraps; the dense description
contains text and legitimately exceeds the empty-description minimum. The
composer follows all 50 comments, explaining the apparent enormous offset.
Do not truncate text, delete comments, hide columns, or force a fixed composer
position to reproduce an unrelated account's screenshot.

Visual capture now uses gates/fixtures/visual3, independently of unchanged
memory workloads. Both the round 2 executable and current executable receive
the identical fixture in public Windows CI. Geometry reduction from aligning
content is reported separately from genuine product corrections. Sidebar
identity is checked on identical-content before/after captures, and unchanged
dense captures remain independently comparable with the round 2 artifact.

| Layout contract | Web component | Token/cascade authority | Native test |
| --- | --- | --- | --- |
| 97% board width, 16px gap, count-dependent minmax(280,535) grid | src/components/PageComponents/Kanban/KanbanHomepageComponents/Homepage.tsx:915-917 | src/styles/tailwind-entry.css:10-11; src/components/PageComponents/Kanban/KanbanSectionComponents/section.tsx:427 | board_web_grid_uses_column_count_and_cards_follow_content |
| Card 4px left border, 8px padding, 6px gap; content height | src/components/PageComponents/Kanban/KanbanTaskComponents/TaskDraggableContainer.tsx:66,96; TaskTopRow.tsx:69-91 | src/styles/tailwind-entry.css:10-11; src/components/PageComponents/Kanban/KanbanTaskComponents/KanbanTaskCard.tsx:266-274 | same test |
| Tabs 32px high, 15px right padding, 4px inner gaps | src/components/notifications/inboxSplit/SplitTitle.tsx:32-52 | src/styles/tailwind-entry.css:10-11; src/components/notifications/inboxSplit/SplitTitle.tsx:36; tailwind.config.ts:167-178 | inbox_web_tabs_and_intrinsic_row_widths_follow_reference_content |
| Row 15% author reserve, 32px outer gaps, intrinsic title, remaining kind | src/components/notifications/NotificationRow.tsx:56-83; src/components/Common/TaskRowComponents/TaskTitle.tsx:156-176,195-210 | src/styles/tailwind-entry.css:10-11; tailwind.config.ts:167-178 | same test |

All web paths are relative to /home/valentin/projects/hypertask.

## First hosted rejection

CI 40343475286 ran all 279 pre-existing desktop tests successfully on each
platform and rejected the two new tests. GPUI TestAppContext supplies
NoopTextSystem, not the executable's platform font shaping. Important 1 thus
measures 102px in this mock, rather than the actual font's 96px. New GPUI tests
now pin content, source spacing, count-dependent column widths and flow.
Exact font-dependent boxes are pinned by visual3-proof.py against real hosted
Windows layout dumps using the unchanged frozen 1.5px comparison. No existing
test or comparer tolerance changed.

The third column starts at 1020 in presented integer coordinates because each
459px column and 16px gap is quantized. The new test checks adjacent presented
columns' exact 16px gap instead of assuming independent rounding of the third
continuous coordinate. The actual source grid equation remains unchanged.
