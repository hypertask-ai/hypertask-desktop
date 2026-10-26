# Round 13 acceptance and evidence

## Acceptance recorded before implementation

1. Download the exact starting main `ce897a34489f7006bb4270f975a0ea51016957b7` CI `layout-dumps` artifact. Record run, artifact ID/digest, complete dense/sparse/reference counts and panel state counts. No locally generated runtime proof.
2. Re-audit all remaining dense and sparse records against the read-only web source `/home/valentin/projects/hypertask/src`. Preserve full-precision measured values. For every surviving difference, record exact web file:line, content/cardinality or rendering cause, and why changing native geometry would violate the source. Inspect largest visible areas first.
3. Correct every source-proven native defect, largest-first, with focused hosted evidence. Do not change references, fixtures, tolerances, budgets or the comparison algorithm to conceal differences. Add no UI absent from the live web source. Scope stays inbox, board, task, AI sidebar and keyboard shortcuts.
4. Fetch/rebase `origin/main` before every push. Keep changes confined to this audit and any narrowly necessary native correction/regression. Preserve other helpers' work. No VPS builds, tests, lint, app/test-copy launches or laptop commands.
5. Deliver draft branch `look13` with exact-head required jobs green. Download its layout artifact and report complete before/after dense, sparse, reference and panel counts, every job result, full SHA, PR and CI URLs.

## Work plan

- Establish the hosted baseline before deciding whether product edits are justified.
- Inspect each source rule and native measurement, descending visible area.
- Fix only demonstrated native defects; otherwise retain an explicit per-record explanation.
- Self-review the diff and unchanged measurement contracts.
- Push a draft PR, wait once using the supplied CI receipt helper, and download exact-head evidence.

Prove-it-works chooses downloaded hosted renderer records over inferred parity. Sequence-verifiable-units keeps the baseline, audit/correction and exact-head proof distinct. Throughput checkpoint: classify the largest task/board areas before touching individual text boxes. Unavailable cursor-team-kit tools are replaced by serial source/diff self-review and existing hosted checks; no independent-review claim.

## Hosted baseline and measurement limits

Starting main is `ce897a34489f7006bb4270f975a0ea51016957b7`.
Its tree is `a346f1618c824db04f6f2c4b740f2505ab0a269a`.
Round 12 head `1153f32b13055ed20efe2b4ef3a88a9022ebe5a4` has that identical tree.
`git diff ce897a3 1153f32` produces no differences.
The already-downloaded round 12 artifact supplies the initial audit.
Run https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40272990929.
Artifact `layout-dumps`, ID `12629587813`, digest `sha256:2957abbb1611ce8561cad87004b1a7d79cc2d10ab5cfd9d17814296f5d1336cc`.
Retained download `/tmp/look12-final-layout`; SHA-bound receipt `receipt-look12/receipt.json`.
The starting-main artifact will independently confirm these counts.

Initial main receipt attempt encountered GitHub's API limit.
Run `40275646400` was still running, not failed.
`receipt-look13-before/failure.txt` preserves this incomplete receipt.
No incomplete run is claimed green.

Web source revision is `003a872014c35eeba1ab21047cdfdf70160e654e`.
Every `src/` citation resolves under `/home/valentin/projects/hypertask/`.
Web boxes are measurements, not new source constants.
Source citations explain the governing rendering rule.
Boxes below are full-precision `(x, y, width, height)`.
The table copies hosted JSON and complete CI reports.
No comparator, native renderer or test ran locally.

## Decision, largest area first

No remaining record demonstrates a native styling defect.
Largest rectangles are the content-sized board columns.
Thirteen dense sections require the source's 280px minimum.
The capture contains only three sections.
Next are task header, description and thread-dependent composer.
Different task content explains their different heights and positions.
Smaller text and presence records follow actual content.
Round 11 already corrected the mixed-tag row defects.
Those corrected records still differ through content and cardinality.
Their remaining counts must not be called unfixed defects.

No native code changes are justified by these records.
Hard-coding capture dimensions would damage populated accounts.
Removing rows or labels would hide valid data.
References, workloads, tolerances, budgets and gates remain untouched.
This is a record audit, not screenshot-equivalence proof.

## Complete remaining-difference table

All 54 records stay explicitly explained, not suppressed.
Numbers retain the round 11 inventory identities.
Dense has 13 inbox, 17 board, 14 task records.
Sparse has 4 inbox, 5 board, 1 task records.
Every row is a content or cardinality difference.
No unexplained rendering quirk is used as justification.

| # | Dataset | Element | Differing fields | Web box | Native box | Why native must not copy capture geometry | Exact web source |
| ---: | --- | --- | --- | --- | --- | --- | --- |
| 1 | Dense | `inbox.row.0.kind` | x, w | (686.390625, 134.390625, 653.609375, 19.5) | (1019.0, 134.0, 268.0, 19.0) | Preceding title differs. Remaining inline width follows that title. Fixed x would overlap valid task text. | `src/components/notifications/NotificationRow.tsx:70-78` |
| 2 | Dense | `inbox.row.0.title` | x, w | (395.390625, 134.390625, 283, 19.5) | (399.0, 134.0, 612.0, 19.0) | Different ticket and task title. Intrinsic text widths differ. Fixed width would clip valid task text. | `src/components/Common/TaskRowComponents/TaskTitle.tsx:185-208` |
| 3 | Dense | `inbox.row.0.author` | w | (118, 134.390625, 112, 19.5) | (118.0, 134.0, 184.0, 19.0) | Actual actor is Morgan Example, Alex Example. Reference actor is valentin+qa-owner. Name width must follow actual actors. | `src/components/notifications/NotificationRow.tsx:265-276` |
| 4 | Dense | `inbox.tab.4` | x, w | (528, 24, 69, 32) | (519.0, 24.0, 114.0, 32.0) | Actual split is Demo Board 1 6. Reference split is All 1. Width and preceding tabs follow actual labels. | `src/components/notifications/inboxSplit/SplitTitle.tsx:32-54` |
| 5 | Dense | `inbox.tab.3` | x, w | (395, 24, 124, 32) | (424.0, 24.0, 86.0, 32.0) | Actual split is Agents 113. Reference split is QA Team Board 1. Intrinsic label width must follow actual content. | `src/components/notifications/inboxSplit/SplitTitle.tsx:32-54` |
| 6 | Dense | `inbox.tab.1` | w | (223, 24, 72, 32) | (223.0, 24.0, 96.0, 32.0) | Actual split is @Mentions 9. Reference split is Agents 1. Fixed width would clip valid split text. | `src/components/notifications/inboxSplit/SplitTitle.tsx:32-54` |
| 7 | Dense | `inbox.tab.2` | x, w | (304, 24, 82, 32) | (328.0, 24.0, 87.0, 32.0) | Actual split is Updates 13. Reference split is MyBoard 1. Its x also follows preceding tab width. | `src/components/notifications/inboxSplit/SplitTitle.tsx:32-54` |
| 8 | Dense | `inbox.tab.5` | presence | Absent | (642.0, 24.0, 114.0, 32.0) | Six actual splits, five reference splits. Removing Demo Board 2 would hide valid data. | `src/components/notifications/inboxSplit/SplitTitle.tsx:32-54` |
| 9 | Dense | `inbox.row.3` | presence | Absent | (118.0, 234.0, 1316.0, 36.0) | Five actual notifications, one reference notification. Removing this row would hide valid data. | `src/components/notifications/inboxSplit/index.tsx:974-990,1025-1040`; `src/components/notifications/NotificationRow.tsx:49-59` |
| 10 | Dense | `inbox.row.2` | presence | Absent | (118.0, 198.0, 1316.0, 36.0) | Five actual notifications, one reference notification. Removing this row would hide valid data. | `src/components/notifications/inboxSplit/index.tsx:974-990,1025-1040`; `src/components/notifications/NotificationRow.tsx:49-59` |
| 11 | Dense | `inbox.row.1` | presence | Absent | (118.0, 162.0, 1316.0, 36.0) | Five actual notifications, one reference notification. Removing this row would hide valid data. | `src/components/notifications/inboxSplit/index.tsx:974-990,1025-1040`; `src/components/notifications/NotificationRow.tsx:49-59` |
| 12 | Dense | `inbox.row.4` | presence | Absent | (118.0, 270.0, 1316.0, 36.0) | Five actual notifications, one reference notification. Removing this row would hide valid data. | `src/components/notifications/inboxSplit/index.tsx:974-990,1025-1040`; `src/components/notifications/NotificationRow.tsx:49-59` |
| 13 | Dense | `inbox.row.0.taskid` | w | (347.390625, 136.390625, 44, 16.5) | (347.0, 137.0, 48.0, 16.0) | Actual ticket DB3-422, reference QABO-5. Ticket text determines width. Fixed width would clip valid identifiers. | `src/components/Common/TaskRowComponents/TaskTitle.tsx:193-208` |
| 14 | Dense | `board.column.2` | x, w | (1019.40625, 72, 458.8125, 865) | (662.0, 72.0, 280.0, 865.0) | Thirteen actual sections, three reference sections. Source minimum track is 280px. Widening tracks would violate source grid sizing. | `src/components/PageComponents/Kanban/KanbanHomepageComponents/Homepage.tsx:830-835,922-934` |
| 15 | Dense | `board.column.0` | w | (69.78125, 72, 458.8125, 865) | (70.0, 72.0, 280.0, 865.0) | Thirteen actual sections, three reference sections. Source minimum track is 280px. Widening tracks would violate source grid sizing. | `src/components/PageComponents/Kanban/KanbanHomepageComponents/Homepage.tsx:830-835,922-934` |
| 16 | Dense | `board.column.1` | x, w | (544.59375, 72, 458.8125, 865) | (366.0, 72.0, 280.0, 865.0) | Thirteen actual sections, three reference sections. Source minimum track is 280px. Widening tracks would violate source grid sizing. | `src/components/PageComponents/Kanban/KanbanHomepageComponents/Homepage.tsx:830-835,922-934` |
| 17 | Dense | `board.card.0` | w, h | (78.78125, 109, 440.8125, 58) | (79.0, 109.0, 262.0, 108.0) | Source minimum track narrows the card. Actual title wraps; notification, comments and labels exist. Shrinking height would clip valid content. Round 11 already fixed stacked tags. | `src/components/PageComponents/Kanban/KanbanTaskComponents/KanbanTaskCard.tsx:162-198,266-272`; `src/components/PageComponents/Kanban/KanbanTaskComponents/TaskTagsRow.tsx:113-114` |
| 18 | Dense | `board.card.1` | y, w, h | (78.78125, 183, 440.8125, 63.5) | (79.0, 233.0, 262.0, 102.0) | Different preceding card controls this y. Actual title and tags control this height. Forcing reference geometry would overlap valid cards. Round 11 already fixed stacked tags. | `src/components/PageComponents/Kanban/KanbanTaskComponents/KanbanTaskCard.tsx:162-198,266-272`; `src/components/PageComponents/Kanban/KanbanTaskComponents/TaskTagsRow.tsx:113-114` |
| 19 | Dense | `board.column.0.add` | x | (509.59375, 86, 10, 10) | (331.0, 86.0, 10.0, 10.0) | Add control follows actual column width. Source size remains 10px. Fixed reference x would leave its column. | `src/components/PageComponents/Kanban/KanbanHomepageComponents/Homepage.tsx:922-925`; `src/components/PageComponents/Kanban/KanbanSectionComponents/NewTaskButton.tsx:55-65` |
| 20 | Dense | `board.card.0.title` | y, w, h | (90.78125, 139.5, 211, 19.5) | (91.0, 145.0, 242.0, 38.0) | Actual title differs and wraps. Notification-bearing top row also raises title y. Forcing reference height would clip valid content. | `src/components/PageComponents/Kanban/KanbanTaskComponents/KanbanTaskCard.tsx:266-272`; `src/components/PageComponents/Kanban/KanbanTaskComponents/TaskTopRow.tsx:69-83,93-124` |
| 21 | Dense | `board.title` | w | (96.5, 29.5, 105, 21) | (97.0, 29.0, 83.0, 21.0) | Actual project Demo Board, reference QA Team Board. Intrinsic width follows the real project name. | `src/components/PageComponents/Kanban/HeaderComponents/TitleKanbanHeader.tsx:26-30` |
| 22 | Dense | `board.count` | x, w | (205.5, 32.5, 7, 16.5) | (184.0, 32.0, 14.0, 16.0) | Actual count 83, reference count 2. Position also follows the different project name. Fixed width would clip valid counts. | `src/components/PageComponents/Kanban/HeaderComponents/ViewTabsBar.tsx:192-196` |
| 23 | Dense | `board.card.0.id` | y, w | (90.78125, 117, 44, 16.5) | (91.0, 120.0, 64.0, 16.0) | Actual ticket DEMO-7382, reference QABO-7. Source centers ticket alongside notification and comment chips. Removing centering would break the shared row. | `src/components/PageComponents/Kanban/KanbanTaskComponents/TaskTopRow.tsx:69-83` |
| 24 | Dense | `board.card.0.badge` | presence | Absent | (241.0, 117.0, 65.0, 22.0) | Actual first task has a notification. Reference first task has none. Removing In inbox would hide real state. | `src/components/PageComponents/Kanban/KanbanTaskComponents/TaskTopRow.tsx:93-124` |
| 25 | Dense | `board.card.0.meta` | presence | Absent | (310.0, 117.0, 23.0, 22.0) | Actual first task has two comments. Reference first task has none. Removing count would hide real state. | `src/components/PageComponents/Kanban/KanbanTaskComponents/TaskTopRow.tsx:129-132` |
| 26 | Dense | `board.card.3` | presence | Absent | (79.0, 469.0, 262.0, 89.0) | Actual board contains additional tasks. Source renders section items. Removing this card would hide valid data. | `src/components/PageComponents/Kanban/KanbanSectionComponents/section.tsx:492-527` |
| 27 | Dense | `board.card.4` | presence | Absent | (79.0, 574.0, 262.0, 102.0) | Actual board contains additional tasks. Source renders section items. Removing this card would hide valid data. | `src/components/PageComponents/Kanban/KanbanSectionComponents/section.tsx:492-527` |
| 28 | Dense | `board.card.2` | presence | Absent | (79.0, 351.0, 262.0, 102.0) | Actual board contains additional tasks. Source renders section items. Removing this card would hide valid data. | `src/components/PageComponents/Kanban/KanbanSectionComponents/section.tsx:492-527` |
| 29 | Dense | `board.card.0.label` | presence | Absent | (142.0, 189.0, 102.0, 20.0) | Actual task has Layout coverage label. Reference task has no label. Source pill now measures 102x20, weight 500. Removing it would hide valid data. | `src/components/PageComponents/Kanban/KanbanTaskComponents/TaskTagsRow.tsx:113-114,186-199; src/components/Labels/LabelWrapper.tsx:13-35` |
| 30 | Dense | `board.column.3` | presence | Absent | (958.0, 72.0, 280.0, 865.0) | Actual board contains thirteen sections, reference three. Source renders actual sections. Removing this column would hide valid data. | `src/components/PageComponents/Kanban/KanbanHomepageComponents/Homepage.tsx:830-835,922-934` |
| 31 | Dense | `task.composer` | y | (270, 347.5, 680, 154) | (270.0, 18739.0, 680.0, 154.0) | Actual description and fifty comments precede composer. Reference has empty description and no comments. Source composer follows the thread. Pinning reference y would overlap thread content. | `src/components/PageComponents/TaskDetail/CommentAndDescription/index.tsx:123-154,208-224,273-280` |
| 32 | Dense | `task.description` | y, h | (290, 165.5, 644, 88) | (290.0, 181.0, 644.0, 359.0) | Actual rich description differs from empty placeholder. Its y also follows actual header height. Truncating height would hide valid task content. | `src/components/PageComponents/TaskDetail/CommentAndDescription/DescriptionContainer/InnerHtmlDescription.tsx:33-51` |
| 33 | Dense | `task.title` | h | (288, 36, 796, 37) | (288.0, 36.0, 796.0, 72.0) | Actual title wraps to two lines. Reference title occupies one line. Source autosizes the title. Forcing one line would hide valid text. | `src/components/PageComponents/TaskDetail/TopRow/TaskTitle.tsx:72-77,168-174,210` |
| 34 | Dense | `task.id` | y, w | (1089, 234.5, 52, 21) | (1090.0, 250.0, 76.0, 21.0) | Actual ticket DEMO-7041, reference QABO-5. Width follows ticket; y follows actual header. Fixed geometry would clip or misalign real metadata. | `src/components/PageComponents/TaskDetail/TaskInfoColumn/TaskInfo.tsx:288-307` |
| 35 | Dense | `task.header` | h | (206, 0, 1136, 108.5) | (206.0, 0.0, 1136.0, 124.0) | Actual two-line title has no summary. Reference one-line title has a summary. Source padding depends on summary presence. Fixed header height would clip valid title content. | `src/components/PageComponents/TaskDetail/TopRow/BaseTitleContainer.tsx:44-52; src/components/PageComponents/TaskDetail/TopRow/TaskDetailTitleContainer.tsx:38-66` |
| 36 | Dense | `task.meta.0` | y | (983, 135.5, 278, 24) | (983.0, 151.0, 278.0, 24.0) | Actual header adds 15.5px versus reference header. Metadata remains in the task body flow. Its width and height already agree. Manual offsets would break source body flow. | `src/components/PageComponents/TaskDetail/TopRow/BaseTitleContainer.tsx:44-52; src/components/PageComponents/TaskDetail/TopRow/TaskDetailTitleContainer.tsx:38-66` |
| 37 | Dense | `task.meta.1` | y | (983, 183.5, 278, 27) | (983.0, 199.0, 278.0, 27.0) | Actual header adds 15.5px versus reference header. Metadata remains in the task body flow. Its width and height already agree. Manual offsets would break source body flow. | `src/components/PageComponents/TaskDetail/TopRow/BaseTitleContainer.tsx:44-52; src/components/PageComponents/TaskDetail/TopRow/TaskDetailTitleContainer.tsx:38-66` |
| 38 | Dense | `task.meta.2` | y | (983, 234.5, 278, 27) | (983.0, 250.0, 278.0, 27.0) | Actual header adds 15.5px versus reference header. Metadata remains in the task body flow. Its width and height already agree. Manual offsets would break source body flow. | `src/components/PageComponents/TaskDetail/TopRow/BaseTitleContainer.tsx:44-52; src/components/PageComponents/TaskDetail/TopRow/TaskDetailTitleContainer.tsx:38-66` |
| 39 | Dense | `task.meta.3` | y | (983, 285.5, 278, 24) | (983.0, 301.0, 278.0, 24.0) | Actual header adds 15.5px versus reference header. Metadata remains in the task body flow. Its width and height already agree. Manual offsets would break source body flow. | `src/components/PageComponents/TaskDetail/TopRow/BaseTitleContainer.tsx:44-52; src/components/PageComponents/TaskDetail/TopRow/TaskDetailTitleContainer.tsx:38-66` |
| 40 | Dense | `task.meta.4` | y | (983, 333.5, 278, 24) | (983.0, 349.0, 278.0, 24.0) | Actual header adds 15.5px versus reference header. Metadata remains in the task body flow. Its width and height already agree. Manual offsets would break source body flow. | `src/components/PageComponents/TaskDetail/TopRow/BaseTitleContainer.tsx:44-52; src/components/PageComponents/TaskDetail/TopRow/TaskDetailTitleContainer.tsx:38-66` |
| 41 | Dense | `task.meta.5` | y | (983, 381.5, 278, 24) | (983.0, 397.0, 278.0, 24.0) | Actual header adds 15.5px versus reference header. Metadata remains in the task body flow. Its width and height already agree. Manual offsets would break source body flow. | `src/components/PageComponents/TaskDetail/TopRow/BaseTitleContainer.tsx:44-52; src/components/PageComponents/TaskDetail/TopRow/TaskDetailTitleContainer.tsx:38-66` |
| 42 | Dense | `task.comment.0` | presence | Absent | (270.0, 598.0, 680.0, 223.0) | Actual task has fifty comments, reference none. Source renders task comments. Removing this comment would hide valid data. | `src/components/PageComponents/TaskDetail/CommentAndDescription/index.tsx:208-224` |
| 43 | Dense | `task.comment.0.body` | presence | Absent | (291.0, 655.0, 638.0, 133.0) | Actual task has fifty comments, reference none. Source renders each comment body. Removing this body would hide valid content. | `src/components/PageComponents/TaskDetail/CommentAndDescription/index.tsx:208-224` |
| 44 | Dense | `task.comment.0.author` | presence | Absent | (291.0, 619.0, 115.0, 24.0) | Actual task has fifty comments, reference none. Source renders each comment author. Removing this author would hide attribution. | `src/components/PageComponents/TaskDetail/CommentAndDescription/index.tsx:208-224` |
| 45 | Sparse | `inbox.row.0.kind` | x, w | (686.390625, 134.390625, 653.609375, 19.5) | (543.0, 134.0, 797.0, 19.0) | Different preceding task title determines remaining inline width. Fixed reference x would leave an artificial gap. | `src/components/notifications/NotificationRow.tsx:70-78` |
| 46 | Sparse | `inbox.row.0.title` | x, w | (395.390625, 134.390625, 283, 19.5) | (392.0, 134.0, 143.0, 19.0) | Actual title Synthetic reminder task differs from reference. Actual ticket also differs. Fixed width would add artificial blank space. | `src/components/Common/TaskRowComponents/TaskTitle.tsx:185-208` |
| 47 | Sparse | `inbox.row.0.author` | w | (118, 134.390625, 112, 19.5) | (118.0, 134.0, 87.0, 19.0) | Actual actor Synthetic User, reference valentin+qa-owner. Name width must follow actual actors. | `src/components/notifications/NotificationRow.tsx:265-276` |
| 48 | Sparse | `inbox.row.0.taskid` | w | (347.390625, 136.390625, 44, 16.5) | (347.0, 137.0, 41.0, 16.0) | Actual ticket QA-103, reference QABO-5. Ticket text determines width. Fixed width would add artificial blank space. | `src/components/Common/TaskRowComponents/TaskTitle.tsx:193-208` |
| 49 | Sparse | `board.card.0.title` | w | (90.78125, 139.5, 211, 19.5) | (91.0, 139.0, 112.0, 19.0) | Actual title Synthetic card one differs from reference. Intrinsic width must follow the real task title. | `src/components/PageComponents/Kanban/KanbanTaskComponents/KanbanTaskCard.tsx:266-272` |
| 50 | Sparse | `board.card.1` | h | (78.78125, 183, 440.8125, 63.5) | (79.0, 182.0, 441.0, 57.0) | Actual second card has no notification. Reference second card has In inbox. Without that chip, top row is shorter. Adding a false badge would invent task state. | `src/components/PageComponents/Kanban/KanbanTaskComponents/TaskTopRow.tsx:93-124; src/components/PageComponents/Kanban/KanbanTaskComponents/TaskTagsRow.tsx:113-114` |
| 51 | Sparse | `board.count` | x | (205.5, 32.5, 7, 16.5) | (210.0, 32.0, 7.0, 16.0) | Both counts are 2. Actual project title is wider than reference. Count x follows the actual project name. Fixed x would overlap or detach the count. | `src/components/PageComponents/Kanban/HeaderComponents/TitleKanbanHeader.tsx:23-30`; `src/components/PageComponents/Kanban/HeaderComponents/ViewTabsBar.tsx:192-196` |
| 52 | Sparse | `board.title` | w | (96.5, 29.5, 105, 21) | (97.0, 29.0, 109.0, 21.0) | Actual project Synthetic Board, reference QA Team Board. Intrinsic width follows the real project name. | `src/components/PageComponents/Kanban/HeaderComponents/TitleKanbanHeader.tsx:26-30` |
| 53 | Sparse | `board.card.0.id` | w | (90.78125, 117, 44, 16.5) | (91.0, 117.0, 41.0, 16.0) | Actual ticket QA-101, reference QABO-7. Ticket text determines width. Fixed width would add artificial blank space. | `src/components/PageComponents/Kanban/KanbanTaskComponents/TaskTopRow.tsx:69-83` |
| 54 | Sparse | `task.id` | w | (1089, 234.5, 52, 21) | (1090.0, 234.0, 49.0, 21.0) | Actual ticket QA-103, reference QABO-5. Ticket text determines width. Fixed width would add artificial blank space. | `src/components/PageComponents/TaskDetail/TaskInfoColumn/TaskInfo.tsx:288-307` |

## Baseline counts and final proof contract

These are the downloaded complete CI report totals.
`parity` is the sparse dataset, not matched content.
No rows are filtered from these reports.

| Report | Inbox | Board | Task | Total |
| --- | ---: | ---: | ---: | ---: |
| `compare-complete-dense.txt` | 13 | 17 | 14 | 44 |
| `compare-complete-parity.txt` | 4 | 5 | 1 | 10 |
| `compare-complete-reference.txt` | 0 | 0 | 0 | 0 |

| Panel state | Measured records | Differences |
| --- | ---: | ---: |
| `ai-welcome` | 17 | 0 |
| `ai-messages` | 16 | 0 |
| `shortcuts` | 39 | 0 |
| Total | 72 | 0 |

Panel projection hash remains `7eb35baa37eb49601920706daf90fa149c119be2e0371b2b1645c3d1361baa98`.
Its source authorities remain in `docs/layout-round12.md`.
Frozen reports also retain unsampled rail keys.
Their separate totals remain 68 dense, 34 sparse, 24 reference.
Those reports are not replaced by the complete reports.

Before handoff, attach exact-head proof to the PR.
Include the starting-main confirmation and final artifact identities.
Include complete before/after screen and panel totals.
Include all eight applicable successful job results.
Retain the unchanged conditional historical-memory job as skipped.
Require the repository receipt to accept current-head artifacts.
A draft PR remains a draft; no merge authorized.

## Self-review and scope

Only this audit changes.
No native UI, references, fixtures or CI checks change.
No new regression is justified without a product correction.
Existing native layout, font, input and memory contracts remain.
The final hosted run must execute every required job.
No live SaaS authorization or full screenshot parity claimed.
