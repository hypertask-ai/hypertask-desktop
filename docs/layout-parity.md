# HTPR-7381 B: Graphite layout evidence

F011 is unchanged: POST `createGlobally` remains server-blocked under HTPR-7699.
No build, test, app, test copy, or owner-laptop command is run on the VPS.

## Acceptance and measurement

Acceptance was recorded before implementation in `internal/LOOK-B-GATES.md`.
CI runs the existing dense 1440x900/all-theme coverage plus Graphite 1920x1009
inbox, board, and task dumps. `layout-dumps` also contains `parity/`: a synthetic
three-column/two-card, one-notification, empty-description/no-comment fixture.
This preserves the dense fixture's badge, label, metadata and comment coverage,
while allowing comparison without mistaking different content cardinality for a
layout defect. Text content is never compared or copied from the live account.

Unmodified comparer:
`/home/valentin/projects/hypertask-desktop-wt-public/internal/gates/layout-compare.py`.
Live input: that worktree's `internal/live-graphite-1920` (QA board 7283/task 60253,
1920x1009, scale 1). All geometry, font and color fields remain compared.

First CI baseline (dense fixture): `91cfe30ccf63200ef4c65d99cbb802793f8406c5`,
[40086213038](https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40086213038),
all five applicable jobs successful. Downloaded `layout-dumps` into
`evidence/look-before`. Raw comparer output: inbox **39**, board **41**, task
**36**, total **116**. The supplied older `mismatch-baseline.txt` is not used as
the measured before count (it reports 184).

## Web source of changed values

Source repository `hypertask`, revision
`6f9901764d355aba7bfc76e4e62ff255e0b68b31` (source read only).

| Change | Web source |
| --- | --- |
| Collapsed rail controls inherit 16px/24px; chevron button is 16x20 and muted | `src/components/PageComponents/Kanban/HeaderComponents/AppShellRail.tsx:152-164,172-191` (no local 14px override on the buttons); live rail font measurement confirms inherited base |
| Rail inbox begins at y=82: 8px top + 20px chevron + 4px margin + 34px boards + 4px wrapper gap + 12px section margin | `src/components/PageComponents/Kanban/HeaderComponents/AppShellRail.tsx:152-159,172,191` |
| Inbox remains a constrained, centered 1440px surface; header inset 12+52px left, 12+20px right, 9px gap | `src/styles/globals.scss:52-62,72-94`; `src/app/inbox/Inbox.tsx:978-988`. Fix auto-width flex overflow, not the measured width by hardcoding a screen-specific value |
| Board plus inherits the base 16px/24px rather than column-title 13px/19.5px | `src/components/PageComponents/Kanban/KanbanSectionComponents/section.tsx:600-607` limits dense font to the title/count, not the plus |
| Board title content wraps inside the card rather than overflowing its width | `src/components/PageComponents/Kanban/KanbanTaskComponents/TaskDraggableContainer.tsx:94-96`; `src/components/PageComponents/Kanban/KanbanTaskComponents/KanbanTaskCard.tsx:181` |
| Quick-tip label capitalization; 8px key/label gap; 2px flex gap + 4px separator + two 4px margins (16px inter-tip spacing) | `src/components/Global/BottomSettings_QuickTips.tsx:375,390-418` |
| `footer.text` measures the first hint span, not the whole hints row | `src/components/Global/BottomSettings_QuickTips.tsx:413-415`; live dump selector `#gear ... > span:nth-child(2)` |

No extra rail controls are removed to game the comparer: the live collector only
samples icons 0-4, although the web rail also contains calendar, AI, search,
agents, invitation, announcements, feedback and settings. Their native-only dump
keys are evidence coverage differences, not invented UI.

## CI evidence

Further run IDs, raw before/after comparisons, and any content-dependent
residuals are recorded below after downloading the actual CI artifacts.
