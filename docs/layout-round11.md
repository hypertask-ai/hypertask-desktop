# Round 11 acceptance and audit

## Acceptance recorded before implementation

1. Download the layout-dumps artifact for main 5e603f44a4f8daeefdab5ff279a75b71f4fb7304. Preserve the complete dense/sparse/reference comparison outputs, artifact ID, digest and run URL.
2. List all 54 dense/sparse records separately, including element, differing property, web/native values, source file:line and real-defect or measurement-artifact classification. Distinguish mismatched content and cardinality from geometry defects; inspect full records, not rounded output alone.
3. Check each apparent artifact against the read-only web source at /home/valentin/projects/hypertask/src. Fix every source-proven real defect in descending visible area. Do not modify frozen references, existing fixtures, comparison tolerances or budgets to lower counts.
4. Each code correction gets a focused hosted regression. Preserve all existing layout/font/input and memory contracts. No builds, tests, lint, app launches, test copies or laptop commands on this VPS.
5. Fetch/rebase origin/main before every push. Deliver a draft look11 PR with exact-head green required CI jobs. Download after artifacts and report unfiltered dense/sparse/reference counts, per-record classifications and any real residuals.

Owned paths: this audit, narrowly necessary ht-desktop layout corrections/regressions, and additive CI evidence only if a genuine defect requires it. Other helpers' workflow, updater and feature work remain untouched.

## Plan

- Establish and classify the immutable baseline.
- Fix source-proven defects largest-first; verify each unit in hosted CI.
- Self-review source and diff; retain unchanged measurement contracts.
- Collect exact-head full-job receipt and final layout artifact.

Unavailable upstream cursor-team-kit tooling is replaced by serial source/diff review and existing hosted checks, per the repository host adapter. No independent-review claim.

## Complete pre-change inventory

Initial artifact: PR 35 head `ad3b00dc488dd24eac0ceecd959db3e15d7fe814`, run https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40246749533. Main confirmation: run https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40249398775 at `5e603f44a4f8daeefdab5ff279a75b71f4fb7304`, artifact `layout-dumps` ID `12619234601`, digest `sha256:14f5f03963e11d26b5c32ebc9b9cb2785d0e6b1badbcfc446edfa6bcc35bf6ae`. Every compared field and all 54 records agree. Only un-compared relative-date text differs on dense/sparse inbox sections.

Web source revision: `003a872014c35eeba1ab21047cdfdf70160e654e`. Every `src/` citation is under `/home/valentin/projects/hypertask/`. Web boxes are captured measurements, not source constants. Citations identify their governing source rules. Native boxes come from CI JSON. Boxes list `(x, y, width, height)` in logical pixels, without rounding. Presence rows include native geometry.

REAL rows mix content differences with styling defects. Correcting those defects need not lower raw totals.

| # | Dataset | Element | Property | Web value | Native value | Classification and cause | Web source |
| ---: | --- | --- | --- | --- | --- | --- | --- |
| 1 | Dense | `inbox.row.0.kind` | x, w | (686.390625, 134.390625, 653.609375, 19.5) | (1019.0, 134.0, 268.0, 19.0) | **Measuring artifact**. Different preceding task title controls remaining width. | `src/components/notifications/NotificationRow.tsx:70-78` |
| 2 | Dense | `inbox.row.0.title` | x, w | (395.390625, 134.390625, 283, 19.5) | (399.0, 134.0, 612.0, 19.0) | **Measuring artifact**. Different task title and preceding ticket width. | `src/components/Common/TaskRowComponents/TaskTitle.tsx:185-208` |
| 3 | Dense | `inbox.row.0.author` | w | (118, 134.390625, 112, 19.5) | (118.0, 134.0, 184.0, 19.0) | **Measuring artifact**. Actual actors differ; width follows names. | `src/components/notifications/NotificationRow.tsx:265-276` |
| 4 | Dense | `inbox.tab.4` | x, w | (528, 24, 69, 32) | (519.0, 24.0, 114.0, 32.0) | **Measuring artifact**. Dense label/count differs; intrinsic width moves subsequent tabs. | `src/components/notifications/inboxSplit/SplitTitle.tsx:32-54` |
| 5 | Dense | `inbox.tab.3` | x, w | (395, 24, 124, 32) | (424.0, 24.0, 86.0, 32.0) | **Measuring artifact**. Dense label/count differs; intrinsic width moves subsequent tabs. | `src/components/notifications/inboxSplit/SplitTitle.tsx:32-54` |
| 6 | Dense | `inbox.tab.1` | w | (223, 24, 72, 32) | (223.0, 24.0, 96.0, 32.0) | **Measuring artifact**. Dense label/count differs; intrinsic width moves subsequent tabs. | `src/components/notifications/inboxSplit/SplitTitle.tsx:32-54` |
| 7 | Dense | `inbox.tab.2` | x, w | (304, 24, 82, 32) | (328.0, 24.0, 87.0, 32.0) | **Measuring artifact**. Dense label/count differs; intrinsic width moves subsequent tabs. | `src/components/notifications/inboxSplit/SplitTitle.tsx:32-54` |
| 8 | Dense | `inbox.tab.5` | presence | Absent from capture | (642.0, 24.0, 114.0, 32.0) | **Measuring artifact**. Six dense splits versus five captured; valid extra data. | `src/components/notifications/inboxSplit/SplitTitle.tsx:32-54` |
| 9 | Dense | `inbox.row.3` | presence | Absent from capture | (118.0, 234.0, 1316.0, 36.0) | **Measuring artifact**. Five dense notifications versus one captured; valid extra data. | `src/components/notifications/NotificationRow.tsx:49-59` |
| 10 | Dense | `inbox.row.2` | presence | Absent from capture | (118.0, 198.0, 1316.0, 36.0) | **Measuring artifact**. Five dense notifications versus one captured; valid extra data. | `src/components/notifications/NotificationRow.tsx:49-59` |
| 11 | Dense | `inbox.row.1` | presence | Absent from capture | (118.0, 162.0, 1316.0, 36.0) | **Measuring artifact**. Five dense notifications versus one captured; valid extra data. | `src/components/notifications/NotificationRow.tsx:49-59` |
| 12 | Dense | `inbox.row.4` | presence | Absent from capture | (118.0, 270.0, 1316.0, 36.0) | **Measuring artifact**. Five dense notifications versus one captured; valid extra data. | `src/components/notifications/NotificationRow.tsx:49-59` |
| 13 | Dense | `inbox.row.0.taskid` | w | (347.390625, 136.390625, 44, 16.5) | (347.0, 137.0, 48.0, 16.0) | **Measuring artifact**. Actual ticket differs; source renders ticket text. | `src/components/Common/TaskRowComponents/TaskTitle.tsx:193-208` |
| 14 | Dense | `board.column.2` | x, w | (1019.40625, 72, 458.8125, 865) | (662.0, 72.0, 280.0, 865.0) | **Measuring artifact**. 13 dense sections versus three captured. Source grid minmax(280px,535px); 280px tracks expected. Extra presence is data. | `src/components/PageComponents/Kanban/KanbanHomepageComponents/Homepage.tsx:922-934` |
| 15 | Dense | `board.column.0` | w | (69.78125, 72, 458.8125, 865) | (70.0, 72.0, 280.0, 865.0) | **Measuring artifact**. 13 dense sections versus three captured. Source grid minmax(280px,535px); 280px tracks expected. Extra presence is data. | `src/components/PageComponents/Kanban/KanbanHomepageComponents/Homepage.tsx:922-934` |
| 16 | Dense | `board.column.1` | x, w | (544.59375, 72, 458.8125, 865) | (366.0, 72.0, 280.0, 865.0) | **Measuring artifact**. 13 dense sections versus three captured. Source grid minmax(280px,535px); 280px tracks expected. Extra presence is data. | `src/components/PageComponents/Kanban/KanbanHomepageComponents/Homepage.tsx:922-934` |
| 17 | Dense | `board.card.0` | w, h | (78.78125, 109, 440.8125, 58) | (79.0, 109.0, 262.0, 126.0) | **REAL (mixed content)**. Native splits priority/date, draft and labels into separate rows. Different content also remains. | `src/components/PageComponents/Kanban/KanbanTaskComponents/TaskTagsRow.tsx:113-114,140-199` |
| 18 | Dense | `board.card.1` | y, w, h | (78.78125, 183, 440.8125, 63.5) | (79.0, 251.0, 262.0, 120.0) | **REAL (mixed content)**. Preceding-card tag stacking shifts this card; own tags also stack. Different content remains. | `src/components/PageComponents/Kanban/KanbanTaskComponents/TaskTopRow.tsx:93-124; src/components/PageComponents/Kanban/KanbanTaskComponents/TaskTagsRow.tsx:113-114` |
| 19 | Dense | `board.column.0.add` | x | (509.59375, 86, 10, 10) | (331.0, 86.0, 10.0, 10.0) | **Measuring artifact**. Control follows narrower dense column; source minimum 280px. | `src/components/PageComponents/Kanban/KanbanHomepageComponents/Homepage.tsx:922-925` |
| 20 | Dense | `board.card.0.title` | y, w, h | (90.78125, 139.5, 211, 19.5) | (91.0, 145.0, 242.0, 38.0) | **Measuring artifact**. Actual title differs; dense 280px track wraps title. | `src/components/PageComponents/Kanban/KanbanTaskComponents/KanbanTaskCard.tsx:266-272` |
| 21 | Dense | `board.title` | w | (96.5, 29.5, 105, 21) | (97.0, 29.0, 83.0, 21.0) | **Measuring artifact**. Actual project title differs; intrinsic width. | `src/components/PageComponents/Kanban/HeaderComponents/TitleKanbanHeader.tsx:26-30` |
| 22 | Dense | `board.count` | x, w | (205.5, 32.5, 7, 16.5) | (184.0, 32.0, 14.0, 16.0) | **Measuring artifact**. Actual count differs; position follows title width. | `src/components/PageComponents/Kanban/HeaderComponents/ViewTabsBar.tsx:192-196` |
| 23 | Dense | `board.card.0.id` | y, w | (90.78125, 117, 44, 16.5) | (91.0, 120.0, 64.0, 16.0) | **Measuring artifact**. Actual ticket differs; dense badge centers ticket vertically. | `src/components/PageComponents/Kanban/KanbanTaskComponents/TaskTopRow.tsx:73-83` |
| 24 | Dense | `board.card.0.badge` | presence | Absent from capture | (241.0, 117.0, 65.0, 22.0) | **Measuring artifact**. Dense task has notification; captured first card does not. | `src/components/PageComponents/Kanban/KanbanTaskComponents/TaskTopRow.tsx:93-124` |
| 25 | Dense | `board.card.0.meta` | presence | Absent from capture | (310.0, 117.0, 23.0, 22.0) | **Measuring artifact**. Dense task has comments; captured first card does not. | `src/components/PageComponents/Kanban/KanbanTaskComponents/TaskTopRow.tsx:129-132` |
| 26 | Dense | `board.card.3` | presence | Absent from capture | (79.0, 523.0, 262.0, 107.0) | **Measuring artifact**. Additional dense tasks; absent capture is data. Shared tag-row correction also applies here. | `src/components/PageComponents/Kanban/KanbanTaskComponents/KanbanTaskCard.tsx:162-203` |
| 27 | Dense | `board.card.4` | presence | Absent from capture | (79.0, 646.0, 262.0, 120.0) | **Measuring artifact**. Additional dense tasks; absent capture is data. Shared tag-row correction also applies here. | `src/components/PageComponents/Kanban/KanbanTaskComponents/KanbanTaskCard.tsx:162-203` |
| 28 | Dense | `board.card.2` | presence | Absent from capture | (79.0, 387.0, 262.0, 120.0) | **Measuring artifact**. Additional dense tasks; absent capture is data. Shared tag-row correction also applies here. | `src/components/PageComponents/Kanban/KanbanTaskComponents/KanbanTaskCard.tsx:162-203` |
| 29 | Dense | `board.card.0.label` | presence | Absent from capture | (91.0, 211.0, 84.0, 16.0) | **REAL (mixed content)**. Dense label is valid data, but native pill uses 16px height, wrong padding/background, and a separate row. Source minimum 20px, padding 2px 6px. | `src/components/PageComponents/Kanban/KanbanTaskComponents/TaskTagsRow.tsx:113-114,186-199; src/components/Labels/LabelWrapper.tsx:13-35` |
| 30 | Dense | `board.column.3` | presence | Absent from capture | (958.0, 72.0, 280.0, 865.0) | **Measuring artifact**. 13 dense sections versus three captured. Source grid minmax(280px,535px); 280px tracks expected. Extra presence is data. | `src/components/PageComponents/Kanban/KanbanHomepageComponents/Homepage.tsx:922-934` |
| 31 | Dense | `task.composer` | y | (270, 347.5, 680, 154) | (270.0, 18739.0, 680.0, 154.0) | **Measuring artifact**. Dense description and 50 comments precede composer; captured task has none. | `src/components/PageComponents/TaskDetail/CommentAndDescription/index.tsx:123-154,208-224,273-280` |
| 32 | Dense | `task.description` | y, h | (290, 165.5, 644, 88) | (290.0, 181.0, 644.0, 359.0) | **Measuring artifact**. Dense populated description versus captured placeholder; y follows header. | `src/components/PageComponents/TaskDetail/CommentAndDescription/DescriptionContainer/InnerHtmlDescription.tsx:33-51` |
| 33 | Dense | `task.title` | h | (288, 36, 796, 37) | (288.0, 36.0, 796.0, 72.0) | **Measuring artifact**. Dense title wraps to two lines; reference fits one. | `src/components/PageComponents/TaskDetail/TopRow/TaskTitle.tsx:72-77,168-174,210` |
| 34 | Dense | `task.id` | y, w | (1089, 234.5, 52, 21) | (1090.0, 250.0, 76.0, 21.0) | **Measuring artifact**. Actual ticket differs; dense y also follows header. | `src/components/PageComponents/TaskDetail/TaskInfoColumn/TaskInfo.tsx:288-307` |
| 35 | Dense | `task.header` | h | (206, 0, 1136, 108.5) | (206.0, 0.0, 1136.0, 124.0) | **Measuring artifact**. Dense two-line title without summary; reference one-line title with summary. Source padding follows summary presence. | `src/components/PageComponents/TaskDetail/TopRow/BaseTitleContainer.tsx:44-52; src/components/PageComponents/TaskDetail/TopRow/TaskDetailTitleContainer.tsx:38-66` |
| 36 | Dense | `task.meta.0` | y | (983, 135.5, 278, 24) | (983.0, 151.0, 278.0, 24.0) | **Measuring artifact**. Rows follow content-dependent header. Dense +15.5px; sizes unchanged. Matched-content rows agree. | `src/components/PageComponents/TaskDetail/TopRow/BaseTitleContainer.tsx:44-52; src/components/PageComponents/TaskDetail/TopRow/TaskDetailTitleContainer.tsx:38-66` |
| 37 | Dense | `task.meta.1` | y | (983, 183.5, 278, 27) | (983.0, 199.0, 278.0, 27.0) | **Measuring artifact**. Rows follow content-dependent header. Dense +15.5px; sizes unchanged. Matched-content rows agree. | `src/components/PageComponents/TaskDetail/TopRow/BaseTitleContainer.tsx:44-52; src/components/PageComponents/TaskDetail/TopRow/TaskDetailTitleContainer.tsx:38-66` |
| 38 | Dense | `task.meta.2` | y | (983, 234.5, 278, 27) | (983.0, 250.0, 278.0, 27.0) | **Measuring artifact**. Rows follow content-dependent header. Dense +15.5px; sizes unchanged. Matched-content rows agree. | `src/components/PageComponents/TaskDetail/TopRow/BaseTitleContainer.tsx:44-52; src/components/PageComponents/TaskDetail/TopRow/TaskDetailTitleContainer.tsx:38-66` |
| 39 | Dense | `task.meta.3` | y | (983, 285.5, 278, 24) | (983.0, 301.0, 278.0, 24.0) | **Measuring artifact**. Rows follow content-dependent header. Dense +15.5px; sizes unchanged. Matched-content rows agree. | `src/components/PageComponents/TaskDetail/TopRow/BaseTitleContainer.tsx:44-52; src/components/PageComponents/TaskDetail/TopRow/TaskDetailTitleContainer.tsx:38-66` |
| 40 | Dense | `task.meta.4` | y | (983, 333.5, 278, 24) | (983.0, 349.0, 278.0, 24.0) | **Measuring artifact**. Rows follow content-dependent header. Dense +15.5px; sizes unchanged. Matched-content rows agree. | `src/components/PageComponents/TaskDetail/TopRow/BaseTitleContainer.tsx:44-52; src/components/PageComponents/TaskDetail/TopRow/TaskDetailTitleContainer.tsx:38-66` |
| 41 | Dense | `task.meta.5` | y | (983, 381.5, 278, 24) | (983.0, 397.0, 278.0, 24.0) | **Measuring artifact**. Rows follow content-dependent header. Dense +15.5px; sizes unchanged. Matched-content rows agree. | `src/components/PageComponents/TaskDetail/TopRow/BaseTitleContainer.tsx:44-52; src/components/PageComponents/TaskDetail/TopRow/TaskDetailTitleContainer.tsx:38-66` |
| 42 | Dense | `task.comment.0` | presence | Absent from capture | (270.0, 598.0, 680.0, 223.0) | **Measuring artifact**. Dense task has comments; captured task has none. | `src/components/PageComponents/TaskDetail/CommentAndDescription/index.tsx:208-224` |
| 43 | Dense | `task.comment.0.body` | presence | Absent from capture | (291.0, 655.0, 638.0, 133.0) | **Measuring artifact**. Dense task has comments; captured task has none. | `src/components/PageComponents/TaskDetail/CommentAndDescription/index.tsx:208-224` |
| 44 | Dense | `task.comment.0.author` | presence | Absent from capture | (291.0, 619.0, 115.0, 24.0) | **Measuring artifact**. Dense task has comments; captured task has none. | `src/components/PageComponents/TaskDetail/CommentAndDescription/index.tsx:208-224` |
| 45 | Sparse | `inbox.row.0.kind` | x, w | (686.390625, 134.390625, 653.609375, 19.5) | (543.0, 134.0, 797.0, 19.0) | **Measuring artifact**. Different preceding task title controls remaining width. | `src/components/notifications/NotificationRow.tsx:70-78` |
| 46 | Sparse | `inbox.row.0.title` | x, w | (395.390625, 134.390625, 283, 19.5) | (392.0, 134.0, 143.0, 19.0) | **Measuring artifact**. Different task title and preceding ticket width. | `src/components/Common/TaskRowComponents/TaskTitle.tsx:185-208` |
| 47 | Sparse | `inbox.row.0.author` | w | (118, 134.390625, 112, 19.5) | (118.0, 134.0, 87.0, 19.0) | **Measuring artifact**. Actual actors differ; width follows names. | `src/components/notifications/NotificationRow.tsx:265-276` |
| 48 | Sparse | `inbox.row.0.taskid` | w | (347.390625, 136.390625, 44, 16.5) | (347.0, 137.0, 41.0, 16.0) | **Measuring artifact**. Actual ticket differs; source renders ticket text. | `src/components/Common/TaskRowComponents/TaskTitle.tsx:193-208` |
| 49 | Sparse | `board.card.0.title` | w | (90.78125, 139.5, 211, 19.5) | (91.0, 139.0, 112.0, 19.0) | **Measuring artifact**. Actual title differs; dense 280px track wraps title. | `src/components/PageComponents/Kanban/KanbanTaskComponents/KanbanTaskCard.tsx:266-272` |
| 50 | Sparse | `board.card.1` | h | (78.78125, 183, 440.8125, 63.5) | (79.0, 182.0, 441.0, 57.0) | **Measuring artifact**. Reference alone has inbox badge; sparse has no notification. Missing 22px badge explains height. | `src/components/PageComponents/Kanban/KanbanTaskComponents/TaskTopRow.tsx:93-124; src/components/PageComponents/Kanban/KanbanTaskComponents/TaskTagsRow.tsx:113-114` |
| 51 | Sparse | `board.count` | x | (205.5, 32.5, 7, 16.5) | (210.0, 32.0, 7.0, 16.0) | **Measuring artifact**. Actual count differs; position follows title width. | `src/components/PageComponents/Kanban/HeaderComponents/ViewTabsBar.tsx:192-196` |
| 52 | Sparse | `board.title` | w | (96.5, 29.5, 105, 21) | (97.0, 29.0, 109.0, 21.0) | **Measuring artifact**. Actual project title differs; intrinsic width. | `src/components/PageComponents/Kanban/HeaderComponents/TitleKanbanHeader.tsx:26-30` |
| 53 | Sparse | `board.card.0.id` | w | (90.78125, 117, 44, 16.5) | (91.0, 117.0, 41.0, 16.0) | **Measuring artifact**. Actual ticket differs; dense badge centers ticket vertically. | `src/components/PageComponents/Kanban/KanbanTaskComponents/TaskTopRow.tsx:73-83` |
| 54 | Sparse | `task.id` | w | (1089, 234.5, 52, 21) | (1090.0, 234.0, 49.0, 21.0) | **Measuring artifact**. Actual ticket differs; dense y also follows header. | `src/components/PageComponents/TaskDetail/TaskInfoColumn/TaskInfo.tsx:288-307` |

Total: 54 records. Dense 13 inbox + 17 board + 14 task = 44. Sparse 4 inbox + 5 board + 1 task = 10. Three dense records include real tag-row defects; 51 reported differences are content/cardinality artifacts. Shared captured style differences: zero. Matched-content comparison: zero, but does not exercise mixed tags.

## Source-backed correction order

1. First two dense cards: one wrapping tags row, 4px item gap, instead of separate priority/date, draft and label rows. Source: `src/components/PageComponents/Kanban/KanbanTaskComponents/TaskTagsRow.tsx:113-114,140-199`. Subsequent cards reuse that renderer.

2. Label pill: minimum height 20px, padding 2px 6px, medium weight, label background and source typography. Source: `src/components/Labels/LabelWrapper.tsx:13-35`, `src/components/Modals/CreateLabel/TaskLabelComponent.tsx:55-74`, `src/styles/tailwindThemes/graphite.css:13,141-143`.

No title, ticket, column, notification, comment or fixture is removed or replaced. Frozen comparisons and tolerances remain unchanged.

## Hosted regression setup

Initial regression run https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40249933805 rejected a malformed new label fixture before reaching layout. Labels require IDs. The fixture is corrected without changing product code or existing assertions; this run is not claimed as product-defect red proof.

Corrected red proof: https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40250714384 at `9ee825517b105a27d494e62b46781a87dd2b3ae3`. Linux, Windows and macOS each report 208 native tests passed and this regression failed. The native `Bug` label starts at y=216.5; `Draft` starts at y=188.5. Both belong to the same source tags row. The label is 16.5px high, normal weight, with a 3px radius. This independently exposes a defect despite the zero reference-content total.

## Implementation and source values

Data shape: one ordered `Vec<AnyElement>` for existing draft, date, priority and label tags, consumed by one wrapping flex row. A local pill builder mirrors the existing web `LabelWrapper`; no new public type, module or API. Moving existing rows inside the source row was chosen over compensating card heights. The frozen reference does not cover mixed tags; changing it would hide that coverage gap rather than repair the product.

| Native value | Web authority under `/home/valentin/projects/hypertask/` |
| --- | --- |
| Tag order, wrapping, 4px gap | `src/components/PageComponents/Kanban/KanbanTaskComponents/TaskTagsRow.tsx:113-114,140-199` |
| Pill 6px horizontal and 2px vertical padding, minimum 20px height, medium weight, leading-none, rounded-sm | `src/components/Labels/LabelWrapper.tsx:13-35` |
| Pill font 11px on Porcelain, Graphite, AMOLED | `src/styles/tailwindThemes/porcelain.css:147-149`, `src/styles/tailwindThemes/graphite.css:141-143`, `src/styles/tailwindThemes/amoled.css:147-149` |
| Dia font 12px and 1px border | `src/components/Labels/LabelWrapper.tsx:24-25`, `src/styles/tailwindThemes/dia.css:153-155` |
| Pill backgrounds #ececef / #363b44 / #2d2d2d / #f1eee7 | `src/styles/tailwindThemes/porcelain.css:22`, `src/styles/tailwindThemes/graphite.css:13`, `src/styles/tailwindThemes/amoled.css:7`, `src/styles/tailwindThemes/dia.css:17` |
| Text inherits theme white-black | `src/components/Labels/LabelWrapper.tsx:36` |
| Draft 11px, medium, 22px line height, green | `src/components/PageComponents/Kanban/KanbanTaskComponents/TaskTagsRow.tsx:140-143` |
| Due calendar 16px, #FB773F | `src/components/Labels/DueDateLabel.tsx:67-84` |
| Priority dot 7px, 4px internal gap; Urgent #F88F9C and High #DE9D6E | `src/components/Modals/TaskPriority/PriorityLabelComponent.tsx:39-68`, `src/lib/constants/constants.ts:85-101` |

The native regression checks all four themes, shared tag-row position, source title-to-tags gap, pill metrics/colors, overflow wrapping with 4px row gap, and no queued mutations. Existing draft and ticket-row regressions remain unchanged. Prove-it-works requires the hosted native measurements. Sequence-verifiable-units keeps the rejected old layout before this correction. Fix-root-causes removes the incorrect separate rows rather than fitting offsets.

## Downloaded after artifact

Product SHA `ed65aed9ca3035ee51ddd4e81d85d29c055a8ca9`, run https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40251319284. Downloaded `layout-dumps` artifact ID `12620393561`; its ZIP SHA-256 matches GitHub's digest `sha256:d7c1ca3272944c130e452e94ab11f3ba3e1ee2d52232f3ac43ba24a40868d0b2`. The before artifact is the exact main artifact recorded above, not a locally generated comparison.

| Dataset | Before inbox / board / task | After inbox / board / task | Before total | After total |
| --- | --- | --- | ---: | ---: |
| Dense | 13 / 17 / 14 | 13 / 17 / 14 | 44 | 44 |
| Sparse | 4 / 5 / 1 | 4 / 5 / 1 | 10 | 10 |
| Matched reference content | 0 / 0 / 0 | 0 / 0 / 0 | 0 | 0 |

Counts are the complete, unfiltered `compare-complete-dense.txt`, `compare-complete-parity.txt` and `compare-complete-reference.txt` from the two hosted artifacts. Original frozen comparisons also remain 68 dense / 34 sparse / 24 reference; those additional 24 are unsampled rail keys. Neither comparison path is replaced or relaxed.

| Original record | Before native | After native | Resolution |
| --- | --- | --- | --- |
| #17 `board.card.0` | x=79, y=109, w=262, h=126 | x=79, y=109, w=262, h=108 | Incorrect tag stacking removed. Narrower source-minimum tracks and actual task content still differ from capture. |
| #18 `board.card.1` | x=79, y=251, w=262, h=120 | x=79, y=233, w=262, h=102 | Prior-card shift and own stacking corrected. Actual card content remains different. |
| #29 `board.card.0.label` | x=91, y=211, w=84, h=16; weight=400; line-height=16.5; bg=#262930; color=#8e9093; radius=3 | x=142, y=189, w=102, h=20; weight=500; line-height=11; bg=#363b44; color=#ffffff; radius=4 | Source pill metrics and shared tags row restored. Valid dense label remains absent from captured first card. |
| #28 `board.card.2` | y=387, h=120 | y=351, h=102 | Same renderer correction; task presence remains an artifact. |
| #26 `board.card.3` | y=523, h=107 | y=469, h=89 | Same renderer correction; task presence remains an artifact. |
| #27 `board.card.4` | y=646, h=120 | y=574, h=102 | Same renderer correction; task presence remains an artifact. |

All three source-proven real records are corrected. Their surviving raw differences are content/cardinality artifacts, not claimed count reductions. The remaining 51 classifications are unchanged. Zero sampled style mismatches and zero matched-content differences are not a claim of complete live-app pixel parity or authenticated actions.

The final documentation descendant gets a separate exact-head full-job receipt on https://github.com/hypertask-ai/hypertask-desktop/pull/36. No VPS build, test, lint, app/test-copy launch or laptop access occurred. Only source inspection, edits, Git, hosted artifact reading and read-only CI receipts ran here.

Hosted macOS log explicitly reports `board_tags_share_one_wrapping_row_with_source_pill_metrics ... ok` and `209 passed; 0 failed`. That test exercises all four themes and wrapped labels. Hosted Windows warm board screenshots were also inspected directly: baseline artifact `windows-memory` ID `12620366459` and corrected artifact ID `12620402984`, both `board-warm.png` at 1920x1009. They show the source tag row replacing stacked priority/labels. No local image-diff test or complete screenshot-equivalence claim is made.

The product SHA's complete receipt reports `DESKTOP_CI_VERIFIED ed65aed9ca3035ee51ddd4e81d85d29c055a8ca9` with eight successful jobs: `ubuntu-24.04`, `windows-2025`, `macos-15`, `windows-arm64`, `Pstack contract`, `updater-ubuntu-24.04`, `updater-windows-2025`, and `Windows memory receipt and budget`. The historical memory comparison matrix is skipped by its unchanged branch condition, not counted as required proof. All eleven required artifacts are nonempty, nonexpired and SHA-bound. The watcher initially hit a shared personal-account rate limit; a later stale shared GET snapshot was rejected. A fresh session-owned cache produced the successful receipt without changing the shared adapter or bypassing any gate.

## Acceptance evidence

| Check | Exact evidence |
| --- | --- |
| 1. Exact-main baseline | Run `40249398775`, artifact `12619234601`; receipt verified main SHA; complete counts 44 / 10 / 0. |
| 2. All 54 records | Numbered inventory #1 through #54 above; original full-precision boxes and source file:line authorities retained. |
| 3. Every source-proven real defect | Records #17, #18 and #29 corrected; five sampled dense cards each lose the incorrect extra 18px; table above retains every resulting box change. |
| 4. Hosted behavior regression and contracts | Run `40250714384` rejects wrong tag-row positions on all three test platforms. Run `40251319284` passes all four theme states and wrapped-label regression, existing strict font/layout/input checks, ARM64 and both memory budgets. |
| 5. Green draft delivery | Product run has all eight required jobs successful. Final documentation descendant's exact-head receipt and freshly downloaded count confirmation are attached to PR 36 before handoff. |
