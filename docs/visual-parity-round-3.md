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

CI 40346041401 passed the inbox regression and all existing tests, then
rejected the board card height. Self-review found a fixture error: assigning
the unshown second notification to QABO-7 added a real In inbox badge there.
The second notification now has no task, preserving the reference's badge on
QABO-5 only. The unsupported guessed assignee was also removed. No UI is hidden.
The old-binary capture records the current harness checkout in BuildRevision;
its actual executable SHA-256 must match the original round 2 dense receipt.
The current capture must similarly match the unchanged current dense binary.

## Windows typography correction

CI 40347917535 passed both new GPUI tests on all three platforms, plus both
unchanged Windows memory workloads. Real captures reduced measured rows
71 to 39 (Inbox 24 to 18, Board 24 to 12, Task 23 to 9). Dense and matched
sidebar comparisons each returned zero changed pixels for all three screens.
The strict new capture gate correctly rejected remaining Windows inbox widths.

DirectWrite uses fractional design advances for these unhinted source fonts.
Linux already uses FreeType normal pixel advances. For example the source font
requires Important 70px, Agents 46px, and an 11px counter 7px, but the Windows
layout gives Important's complete tab 93px rather than 96px. The same cause
shortens the reference task title from 283px to 276px, moving the kind column.

Windows Graphite main content now opts into the existing Linux measurement
policy using an internal IBM Plex Sans Web family alias. It selects the same
embedded IBM Plex Sans face, not a replacement font. FreeType normal advances
come from the actual DirectWrite-selected font bytes and glyph IDs; shaping
adjustments retain the pixel grid. Fixed-pitch text, zero-advance marks, color
emoji and unopted text keep their existing behavior. The AI sibling does not
opt in. No string-specific widths, padding compensation or global font changes.

Web authority: src/lib/fonts/ibmPlexSans.ts:3-9;
src/styles/tailwindThemes/graphite.css:169-172;
src/styles/tailwind-entry.css:7-11. Font sizes and spacing remain those of
SplitTitle.tsx and TaskTitle.tsx cited above. Existing CI FreeType and Chromium
font probes independently pin the same source advances. New Windows backend
test web_pixel_advances_preserve_unopted_text pins 106/70/46/7 advances and
checks unchanged original-font glyph IDs, positions and byte indices afterward.
The hosted capture gate still requires every requested source box within the
original 1.5px tolerance, and exact zero sidebar pixel changes.

## Verified Windows unit and task correction

Source-metrics unit SHA 9b7955631708d7d492a5e2a46ca6c906a40f2b27 passed
CI https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40353410678.
All eight applicable jobs succeeded, including ARM64 and both memory budgets.
Historical memory comparison kept its original skipped branch condition.
The supplied receipt emitted DESKTOP_CI_VERIFIED. Account quota exhaustion
required the installed shared reader's existing auth and a process-local
60-second watch adapter; no shared tools, credentials or production state changed.

| Comparison | Inbox | Board | Task | Total |
| --- | ---: | ---: | ---: | ---: |
| Original round 2 dense measured records | 24 | 24 | 23 | 71 |
| Same dense content after typography | 21 | 23 | 23 | 67 |
| Round 2 executable, matched capture content | 18 | 12 | 9 | 39 |
| New executable, same matched content | 8 | 8 | 9 | 25 |

This is 32 content-alignment rows plus 14 same-content product rows, not 46
product fixes. Original remaining inventories include six additional original
screenshot-only records per screen: 30/30/29 to 14/14/15, total 89 to 43.
All 24 unsampled rail keys remain counted, not silently removed. The Task
footer remains an open measured row. All original sidebar classes stay open.
Dense and matched sidebar x1500..1919 comparisons each returned zero changed
pixels on Inbox, Board and Task. The protected-code/reference diff was empty.

The remaining source-backed Task header correction replaces the summary's
21px reserve with its 13px text's 19.5px line box, and gives the title itself
the source textarea's 37px minimum. Text continues wrapping naturally; the
unrelated long dense title is never clipped or shortened. The existing
no-summary 16px Bootstrap bottom spacing remains unchanged.

| Task value | Web component | Token/cascade authority | Native proof |
| --- | --- | --- | --- |
| Title 24px, naturally wrapping, minimum 37px | src/components/PageComponents/TaskDetail/TopRow/TaskTitle.tsx:43-49,168-173,210 | src/styles/tailwind-entry.css:7-11; TaskTitle.tsx:46,210 | task_web_header_title_description_meta_and_composer_flow |
| Summary 13px / 19.5px, 8px row spacing and bottom padding | src/components/PageComponents/TaskDetail/TopRow/TaskSummary.tsx:30-40; BaseTitleContainer.tsx:44,51-52 | src/styles/tailwind-entry.css:10-11; tailwind.config.ts:172 | same test; strict hosted old/new header and title check |
| Empty description 80px minimum + 8px top margin, content may grow | src/components/PageComponents/TaskDetail/CommentAndDescription/DescriptionContainer/StaticDescriptionShell.tsx:65-75 | src/styles/tiptap.module.scss:62-64; src/styles/tailwind-entry.css:10-11 | same test, including twenty published paragraphs |
| Composer follows actual comments; never an absolute account-specific offset | src/components/PageComponents/TaskDetail/CommentAndDescription/index.tsx:274-280 | src/components/PageComponents/TaskDetail/CommentAndDescription/BaseCommentAndDescriptionContainer.tsx:18-26; src/styles/tailwind-entry.css:10-11 | same test, including two actual comments and exact 16px native flow gap |

All acceptance constraints remain unchanged. Final exact-head receipt follows
in the PR evidence comment, avoiding a self-referential documentation commit.
