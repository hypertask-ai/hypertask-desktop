# Visual parity round 2

## Baseline and interpretation

Baseline PR 50 head e1e2f67663cb0f9a79d90206969b498959690569.
CI https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40318389622.
Downloaded baseline already retained at /tmp/visual-final. Its Windows
memory.json Revision equals that SHA. All screenshots are 1920x1009.
Frozen live reference directory remains read-only.

Original inventory is 91 records: Inbox 30, Board 32, Task 29.
Measured records are Inbox 24, Board 24, Task 23. Twenty-four presence records
are collector omissions. Thirteen presence records are extra synthetic data.
Board column widths depend on column count. Task header, description, comments
and composer bounds depend on different text and comment counts. Removing data
or forcing live-account dimensions onto the dense workload is not a fix.

The source-backed remaining original classes outside the sidebar are the two
Board hover controls and three footer font advances. Additional real defects
are listed separately, never added to the original denominator to claim wins.
The requested 30-record reduction remains an explicit acceptance gate.

## Source-backed correction units

Paths in this table are relative to /home/valentin/projects/hypertask.

| Unit | Web component value | Token/cascade authority | Native proof |
| --- | --- | --- | --- |
| Board selection | SelectionCheckbox defaults 15x15; hidden idle, shown hover/selection; absolute left/top 8; content left padding 8 to 28 | src/components/Common/selection-checkbox.tsx:22,33-55; src/components/PageComponents/Kanban/KanbanTaskComponents/KanbanTaskCard.tsx:123-153; src/components/PageComponents/Kanban/KanbanTaskComponents/TaskDraggableContainer.tsx:66,78,96; src/styles/tailwind-entry.css:10-11 | board_hover_selection_uses_web_overlay_and_reserved_command_slot |
| Board selection colors/check | Purple #4455BB selected; card background; border opacity 60%; 9x7 check path | src/components/Common/selection-checkbox.tsx:45-65; src/components/PageComponents/Kanban/KanbanTaskComponents/KanbanTaskCard.tsx:132-139; tailwind.config.ts:37; src/styles/tailwindThemes/graphite.css:15,55,166 | Same native test; existing click/bulk tests retained |
| Board command | Reserved 14x16 slot, hover-only 16x16 image | src/components/PageComponents/Kanban/KanbanTaskComponents/TaskTopRow.tsx:89-91; src/components/PageComponents/TaskDetail/TaskOptions/HTCButton.tsx:41,56-57; src/styles/tailwind-entry.css:10-11 | Same native test; board_card_command_logo_focuses_clicked_task_and_options_shortcut_opens_palette |

No AI or keyboard shortcut behavior changed. Baselines, fixtures, collectors,
comparison tolerances and existing acceptance assertions are unchanged.

## Hosted unit receipts

First Board unit CI 40322726180 rejected the new regression on all platforms.
The selected state correctly inserted the existing bulk toolbar (35.5px).
The assertion incorrectly compared the selector against the pre-toolbar card.
The correction compares both bounds from the same current frame, retaining
exact 12px/8px offsets, 15px size and painted visibility checks.
All 264 pre-existing desktop tests passed on each platform in that run.
