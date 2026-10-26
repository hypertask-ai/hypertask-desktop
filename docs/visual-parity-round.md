# Windows visual parity round

## Frozen baseline, before implementation

Main `8c6402b37c96ac6d843d424d856d510c3f7e7937`,
[CI 40301021475](https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40301021475).
Windows memory artifact 12640574552 (2,764,769 bytes); layout artifact
12641053839 (61,704 bytes). Both downloaded from this run. `memory.json`
Revision equals this SHA. All images are 1920x1009 at 96 DPI.
Frozen live PNG/JSON files are read-only, captured October 23.

The exact-layout reference-content artifact reports zero shared differences on
Linux (`compare-complete-reference.txt`). It is not Windows pixel proof.
The Windows screenshots use the dense synthetic account, not the small live
board. Never change real column widths to fit a different column count, hide
comments, or replace fixture content to improve comparison counts.

### Full measured element table

Same original thresholds: box maximum delta >1.5px; numeric style >0.5;
all comparable colors/styles retained. Each box is [x,y,width,height].
Before: Inbox 24, Board 24, Task 23 = **71** records (34 box, zero style,
37 presence). Of the presence records, 24 are collector omissions: eight rail
controls exist in both images but the live JSON samples only five rail icons.
The other 13 are synthetic content absent from the live account. No actual
missing rail control is inferred from a missing collector key.

| Screen | Element | Property | Web | Windows | Delta (x,y,w,h) |
| --- | --- | --- | --- | --- | --- |
| board | board.column.0 | box | [69.78125, 72, 458.8125, 865] | [70.0, 72.0, 280.0, 865.0] | {'x': 0.21875, 'y': 0.0, 'w': -178.8125, 'h': 0.0} |
| board | board.column.1 | box | [544.59375, 72, 458.8125, 865] | [366.0, 72.0, 280.0, 865.0] | {'x': -178.59375, 'y': 0.0, 'w': -178.8125, 'h': 0.0} |
| board | board.column.2 | box | [1019.40625, 72, 458.8125, 865] | [662.0, 72.0, 280.0, 865.0] | {'x': -357.40625, 'y': 0.0, 'w': -178.8125, 'h': 0.0} |
| board | board.column.3 | presence | None | [958.0, 72.0, 280.0, 865.0] |  |
| task | task.description | box | [290, 165.5, 644, 88] | [290.0, 181.0, 644.0, 338.0] | {'x': 0.0, 'y': 15.5, 'w': 0.0, 'h': 250.0} |
| task | task.comment.0 | presence | None | [270.0, 577.0, 680.0, 223.0] |  |
| task | task.header | box | [206, 0, 1136, 108.5] | [206.0, 0.0, 1136.0, 124.0] | {'x': 0.0, 'y': 0.0, 'w': 0.0, 'h': 15.5} |
| task | task.composer | box | [270, 347.5, 680, 154] | [270.0, 18781.0, 680.0, 154.0] | {'x': 0.0, 'y': 18433.5, 'w': 0.0, 'h': 0.0} |
| task | task.comment.0.body | presence | None | [291.0, 634.0, 638.0, 133.0] |  |
| task | task.title | box | [288, 36, 796, 37] | [288.0, 36.0, 797.0, 72.0] | {'x': 0.0, 'y': 0.0, 'w': 1.0, 'h': 35.0} |
| inbox | inbox.row.1 | presence | None | [118.0, 162.0, 1316.0, 36.0] |  |
| inbox | inbox.row.2 | presence | None | [118.0, 198.0, 1316.0, 36.0] |  |
| inbox | inbox.row.3 | presence | None | [118.0, 234.0, 1316.0, 36.0] |  |
| inbox | inbox.row.4 | presence | None | [118.0, 270.0, 1316.0, 36.0] |  |
| board | board.card.1 | box | [78.78125, 183, 440.8125, 63.5] | [79.0, 227.0, 262.0, 102.0] | {'x': 0.21875, 'y': 44.0, 'w': -178.8125, 'h': 38.5} |
| board | board.card.0 | box | [78.78125, 109, 440.8125, 58] | [79.0, 109.0, 262.0, 102.0] | {'x': 0.21875, 'y': 0.0, 'w': -178.8125, 'h': 44.0} |
| board | board.card.2 | presence | None | [79.0, 345.0, 262.0, 102.0] |  |
| board | board.card.4 | presence | None | [79.0, 568.0, 262.0, 102.0] |  |
| board | board.card.3 | presence | None | [79.0, 463.0, 262.0, 89.0] |  |
| inbox | inbox.row.0.kind | box | [686.390625, 134.390625, 653.609375, 19.5] | [998.0, 134.0, 290.0, 19.0] | {'x': 311.609375, 'y': -0.390625, 'w': -363.609375, 'h': -0.5} |
| inbox | inbox.row.0.title | box | [395.390625, 134.390625, 283, 19.5] | [397.0, 134.0, 593.0, 19.0] | {'x': 1.609375, 'y': -0.390625, 'w': 310.0, 'h': -0.5} |
| board | board.card.0.title | box | [90.78125, 139.5, 211, 19.5] | [91.0, 139.0, 242.0, 38.0] | {'x': 0.21875, 'y': -0.5, 'w': 31.0, 'h': 18.5} |
| task | task.meta.1 | box | [983, 183.5, 278, 27] | [983.0, 199.0, 278.0, 27.0] | {'x': 0.0, 'y': 15.5, 'w': 0.0, 'h': 0.0} |
| task | task.meta.2 | box | [983, 234.5, 278, 27] | [983.0, 250.0, 278.0, 27.0] | {'x': 0.0, 'y': 15.5, 'w': 0.0, 'h': 0.0} |
| task | task.meta.0 | box | [983, 135.5, 278, 24] | [983.0, 151.0, 278.0, 24.0] | {'x': 0.0, 'y': 15.5, 'w': 0.0, 'h': 0.0} |
| task | task.meta.3 | box | [983, 285.5, 278, 24] | [983.0, 301.0, 278.0, 24.0] | {'x': 0.0, 'y': 15.5, 'w': 0.0, 'h': 0.0} |
| task | task.meta.4 | box | [983, 333.5, 278, 24] | [983.0, 349.0, 278.0, 24.0] | {'x': 0.0, 'y': 15.5, 'w': 0.0, 'h': 0.0} |
| task | task.meta.5 | box | [983, 381.5, 278, 24] | [983.0, 397.0, 278.0, 24.0] | {'x': 0.0, 'y': 15.5, 'w': 0.0, 'h': 0.0} |
| inbox | inbox.tab.3 | box | [395, 24, 124, 32] | [421.0, 24.0, 83.0, 32.0] | {'x': 26.0, 'y': 0.0, 'w': -41.0, 'h': 0.0} |
| inbox | inbox.tab.4 | box | [528, 24, 69, 32] | [513.0, 24.0, 116.0, 32.0] | {'x': -15.0, 'y': 0.0, 'w': 47.0, 'h': 0.0} |
| inbox | inbox.tab.5 | presence | None | [638.0, 24.0, 116.0, 32.0] |  |
| inbox | inbox.row.0.author | box | [118, 134.390625, 112, 19.5] | [118.0, 134.0, 184.0, 19.0] | {'x': 0.0, 'y': -0.390625, 'w': 72.0, 'h': -0.5} |
| inbox | inbox.tab.1 | box | [223, 24, 72, 32] | [220.0, 24.0, 97.0, 32.0] | {'x': -3.0, 'y': 0.0, 'w': 25.0, 'h': 0.0} |
| inbox | inbox.tab.0 | box | [118, 24, 96, 32] | [118.0, 24.0, 93.0, 32.0] | {'x': 0.0, 'y': 0.0, 'w': -3.0, 'h': 0.0} |
| inbox | inbox.tab.active | box | [118, 24, 96, 32] | [118.0, 24.0, 93.0, 32.0] | {'x': 0.0, 'y': 0.0, 'w': -3.0, 'h': 0.0} |
| inbox | inbox.tab.2 | box | [304, 24, 82, 32] | [326.0, 24.0, 86.0, 32.0] | {'x': 22.0, 'y': 0.0, 'w': 4.0, 'h': 0.0} |
| task | task.comment.0.author | presence | None | [291.0, 598.0, 112.0, 24.0] |  |
| inbox | footer.text | box | [380.625, 980, 106, 21] | [376.0, 980.0, 109.0, 21.0] | {'x': -4.625, 'y': 0.0, 'w': 3.0, 'h': 0.0} |
| board | footer.text | box | [273.171875, 980, 106, 21] | [266.0, 980.0, 109.0, 21.0] | {'x': -7.171875, 'y': 0.0, 'w': 3.0, 'h': 0.0} |
| task | footer.text | box | [324.625, 980, 106, 21] | [322.0, 980.0, 109.0, 21.0] | {'x': -2.625, 'y': 0.0, 'w': 3.0, 'h': 0.0} |
| board | board.title | box | [96.5, 29.5, 105, 21] | [97.0, 29.0, 80.0, 21.0] | {'x': 0.5, 'y': -0.5, 'w': -25.0, 'h': 0.0} |
| task | task.id | box | [1089, 234.5, 52, 21] | [1090.0, 250.0, 79.0, 21.0] | {'x': 1.0, 'y': 15.5, 'w': 27.0, 'h': 0.0} |
| inbox | rail.icon.10 | presence | None | [5.0, 887.0, 36.0, 34.0] |  |
| inbox | rail.icon.11 | presence | None | [5.0, 925.0, 36.0, 34.0] |  |
| inbox | rail.icon.12 | presence | None | [5.0, 963.0, 36.0, 34.0] |  |
| inbox | rail.icon.5 | presence | None | [5.0, 196.0, 36.0, 34.0] |  |
| inbox | rail.icon.6 | presence | None | [5.0, 234.0, 36.0, 34.0] |  |
| inbox | rail.icon.7 | presence | None | [5.0, 272.0, 36.0, 34.0] |  |
| inbox | rail.icon.8 | presence | None | [5.0, 811.0, 36.0, 34.0] |  |
| inbox | rail.icon.9 | presence | None | [5.0, 849.0, 36.0, 34.0] |  |
| board | rail.icon.10 | presence | None | [5.0, 887.0, 36.0, 34.0] |  |
| board | rail.icon.11 | presence | None | [5.0, 925.0, 36.0, 34.0] |  |
| board | rail.icon.12 | presence | None | [5.0, 963.0, 36.0, 34.0] |  |
| board | rail.icon.5 | presence | None | [5.0, 196.0, 36.0, 34.0] |  |
| board | rail.icon.6 | presence | None | [5.0, 234.0, 36.0, 34.0] |  |
| board | rail.icon.7 | presence | None | [5.0, 272.0, 36.0, 34.0] |  |
| board | rail.icon.8 | presence | None | [5.0, 811.0, 36.0, 34.0] |  |
| board | rail.icon.9 | presence | None | [5.0, 849.0, 36.0, 34.0] |  |
| task | rail.icon.10 | presence | None | [5.0, 887.0, 36.0, 34.0] |  |
| task | rail.icon.11 | presence | None | [5.0, 925.0, 36.0, 34.0] |  |
| task | rail.icon.12 | presence | None | [5.0, 963.0, 36.0, 34.0] |  |
| task | rail.icon.5 | presence | None | [5.0, 196.0, 36.0, 34.0] |  |
| task | rail.icon.6 | presence | None | [5.0, 234.0, 36.0, 34.0] |  |
| task | rail.icon.7 | presence | None | [5.0, 272.0, 36.0, 34.0] |  |
| task | rail.icon.8 | presence | None | [5.0, 811.0, 36.0, 34.0] |  |
| task | rail.icon.9 | presence | None | [5.0, 849.0, 36.0, 34.0] |  |
| board | board.card.0.id | box | [90.78125, 117, 44, 16.5] | [91.0, 117.0, 62.0, 16.0] | {'x': 0.21875, 'y': 0.0, 'w': 18.0, 'h': -0.5} |
| board | board.card.0.label | presence | None | [142.0, 183.0, 49.0, 20.0] |  |
| inbox | inbox.row.0.taskid | box | [347.390625, 136.390625, 44, 16.5] | [347.0, 137.0, 46.0, 16.0] | {'x': -0.390625, 'y': 0.609375, 'w': 2.0, 'h': -0.5} |
| board | board.count | box | [205.5, 32.5, 7, 16.5] | [181.0, 32.0, 14.0, 16.0] | {'x': -24.5, 'y': -0.5, 'w': 7.0, 'h': -0.5} |
| board | board.column.0.add | box | [509.59375, 86, 10, 10] | [331.0, 86.0, 10.0, 10.0] | {'x': -178.59375, 'y': 0.0, 'w': 0.0, 'h': 0.0} |

### Screenshot-only inventory, outside shared JSON keys

Each row applies independently to Inbox, Board and Task. Counts below are
separate from the 71 measured records; they are not silently filtered from it.

| Element | Live pixels | Windows pixels | Classification / decision |
| --- | --- | --- | --- |
| Resize separator | Visible 1px white/15 track, centered 5x24 grip at y493 | No visible track or grip | Fix existing resize control, no new action |
| Starter buttons horizontal bounds | x1545..1880 (335px) | x1542..1880 (338px) | Missing in-flow 4px resize gutter; fix first |
| Composer horizontal bounds | x1513..1912 (399px) | x1510..1912 (402px) | Same missing gutter; fix first |
| Composer top/height | y908, 93px | y913, 88px | Existing native control inventory differs; do not invent fixed heights |
| Composer attachment control | 18px paperclip around x1797 | Absent | Missing action; retain as follow-up, no decorative dead button |
| Composer recording controls | 18px microphone plus dropdown around x1825 | Absent | Missing action; retain as follow-up |
| Composer context-add control | @ around x1867 | Absent | Missing action; retain as follow-up |
| Model label / scope / prompts | Live account's selected values | Synthetic account's values | Data/state, not layout constants |
| Glyph rasterization / equal-label advances | Footer label 106px | Footer label 109px | Windows DirectWrite versus captured Chromium; do not add label-specific compensation |

Nine screenshot-only rows per screen = **27** inventory records, including
three data/state records. Combined inventory before: **98** records. These
counts are element/property records, not a claim to enumerate individual
antialiasing pixels. Whole-image unequal pixels are Inbox 96,459, Board
663,693, Task 484,640 (1,937,280 pixels each). Content and rasterization are
included, so these are diagnostic differences, not isolated layout defects.

### Source authority and ordering

Web checkout revision `58363f91a86fde94a140bafa21eb106b95166162`.
Paths below are relative to `/home/valentin/projects/hypertask`.

| Value | Component | Token / cascade source |
| --- | --- | --- |
| In-flow 4px, nonshrinking desktop resize handle | `src/components/AI_CHAT/AiChatSidebarResizeHandle.tsx:69-80` | `src/styles/tailwind-entry.css:10-11` loads Tailwind spacing; `w-1` = 4px |
| 1px white/15 track; 5x24 grip, white/20 border, centered | `src/components/AI_CHAT/AiChatSidebarResizeHandle.tsx:75-90` | Same imported Tailwind spacing/opacity; `src/styles/tailwindThemes/graphite.css:32` background #1a1d21 |
| Outer 1px border #3a3f47; inner 1px #434343 | `src/components/AI_CHAT/AI_Chat_Sidebar.tsx:251-256` | `src/styles/tailwindThemes/graphite.css:188-190`; `tailwind.config.ts:86-87` |
| Welcome 40px horizontal inset, 8px gap, 12px text | `src/components/AI_CHAT/WelcomeScreen.tsx:398-407` | `tailwind.config.ts:171`; `src/styles/tailwind-entry.css:7-11` |
| Composer outer and inner 8px insets, 8px radius | `src/components/AI_CHAT/AI_Tiptap_Container.tsx:261,298-305` | `src/styles/tailwindThemes/graphite.css:31`; same spacing import |
| Attachment / recording / context-add actions | `src/components/AI_CHAT/AiChatComposerActionRow.tsx:146-152` | `AI_Tiptap_Container.tsx:748-801`; existing feature gaps, not layout fillers |
| Graphite font family and size scale | `src/lib/fonts/ibmPlexSans.ts:3-9` | `src/styles/tailwindThemes/graphite.css:169-172`; `tailwind.config.ts:167-178` |

Largest boxes in the measured table are content-dependent. The largest common
source-supported defect is the omitted full-height resize gutter: it shifts
all sidebar contents on all three screens. Correct that shared primitive
first; preserve task-open, first-run, author, summary and draft-marker code.
The frozen screenshot's observed 3px inset delta is not a source token. Use the
source's 4px gutter and retain any remaining one-pixel capture/cascade delta.

Existing source-panel projection omits the resize gutter. Extend it with the
actual source handle, preserving every existing box/style assertion, frozen
live reference, tolerance and negative-control requirement. This is an oracle
coverage correction, not permission to edit the live baseline or relax a gate.

## After

Pending exact-head GitHub screenshots and receipt. No parity claim yet.
