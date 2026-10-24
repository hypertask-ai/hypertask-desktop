# Heavy-account Windows memory contract

All measurements/builds/tests run on GitHub Windows 2025, never the VPS or the owner's laptop.

`gates/heavy-fixture.py` generates non-private, deterministic data: 640 inbox notifications,
320 cards with ~28 KB HTML descriptions and three image references each, 160 distinct
1024-square avatar PNGs, three 2560×1440 description PNGs displayed at 640×360, 48 comments,
and 40 AI histories containing 960 messages. A loopback asset server supplies real PNGs.
The AI receipt hook requires both `GPUI_MEMORY_RECEIPT` and `HT_MEMORY_AI_HISTORY`; it opens
one transcript through the ordinary background history-selection path, with no credentials.
No production UI is added.

Each screen gets three fresh processes, a 1920×1080 display, maximized 1920×1009 client at
96 DPI, and eight seconds to settle. Median working set is measured independently through
Windows, not inferred from payload counts. The original demo receipt and its frozen limits
remain mandatory. The additional heavy limits are strictly below 100,000,000 bytes.

## Attribution

- `*-map.json`: VirtualQueryEx regions and QueryWorkingSetEx resident pages, grouped by
  private/image/mapped. DLL pages can be shared; these are not all exclusive app RAM.
- `*-owners.json`: actual GPU texture/buffer payload capacities, including both swap-chain
  buffers, image atlas and ClearType atlas. Payload is not resident working set and must not
  be added to it as if independent.
- `*-owners.app.json`: exact live/peak *requested Rust allocation* counters, frontend data
  counts, serialized payload lower bounds counted without allocating a second serialized
  copy, retained attachment counts, RichText payloads and AI history/open-transcript content.
  `DataReads` records the allocator's net live requested-byte delta across the inbox/board
  reads (not peak/transient reads; concurrent background allocation may contribute).
  Rust heap also includes GPUI text-layout objects; it does not isolate their private layout-cache allocations.
- `*-owners.store.json`: SQLite cache/schema/statement memory, persisted raw JSON and
  description sizes, and whether the compact-read optimization was compiled in. Persisted
  byte counts are disk payloads, not claims about resident memory.
- `*-owners.fonts.json`: registered face count and font bytes.

This client currently uses initials, not decoded avatar photos, and its description reader
is text-only. Description sources are 16× the declared displayed pixel area
(2560×1440 versus 640×360); avatar sources are 1024², but attributing that hypothetical decoded size to app RAM
would be false. The receipt explicitly flags these absent decoders; the image atlas still
accounts for images the application actually renders (such as the logo). No image quality
or visible content is reduced to meet the budget.

## Optimization and control

The `compact-task-reads` feature (enabled by the desktop by default) reads only current
stored title/ticket columns for inbox items and streams board cards with the non-displayed
HTML description and hidden attachment/PR/custom-field/follower/subtask bodies removed
*in SQLite before Rust deserialization*. Inbox rows similarly omit only hidden nested task
bodies, retaining the task identity and displayed due/priority/estimate/label metadata;
comment previews, actor photos/identity and split membership are retained. The full `inbox()` API is unchanged. Each card moves into its
column instead of keeping a full-board JSON tree plus another cloned card set. Legacy
same-name section membership, card ordering and all remaining fields are preserved.
Task details, editing, search, sync and offline persistence still retain full descriptions.

The `visible-board-columns` feature retains every column header/box and its scroll state,
but only builds card lists intersecting the horizontal viewport (plus the selected target
before keyboard autoscroll). Wheel events invalidate the view so newly exposed cards are
materialized immediately. The wide-board regression covers keyboard autoscroll and an
independent offset-only return without dropping any stored cards.

The PR's full-read/all-columns control compiles identical source without these two features,
retaining lazy theme fonts and the existing bounded SQLite cache. CI compares the same database and
fixture hashes, requires lower working set, lower board/task live and peak Rust heap, and
requires identical layout JSON and screenshot pixels on every screen. This isolates the
change from unrelated main/prewire development. Shrinking fixtures or increasing a budget
is not an acceptable way to repair a failure. Negative controls exercise all screen limits
and each minimum fixture dimension in CI.

Measured SHA, CI URL, owner deltas and final budgets are recorded in `gates/mem3-GATES.md`.

## Measured heavy baseline and frozen limits

[GitHub CI 40126115158](https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40126115158)
(source `05bc7fc5f9262de1b63563187d086ef47be4db3d`) passed all six jobs. Same-source
full-read/all-columns control versus optimized median working set, decimal MB:

| Screen | Before | After | Frozen limit |
| --- | ---: | ---: | ---: |
| Inbox | 101.671 | 75.624 | 77.137 |
| Board | 154.558 | 96.362 | 98.289 |
| Task | 145.465 | 98.132 | 99.000 |

The exact byte values and fixture hash are pinned in `windows-memory-heavy-budget.json`.
Limits are the smaller of the existing 99 MB cap and this measured baseline plus 2%;
no old demo limit is raised. The baseline manifest independently matches the database:
640 inbox rows, 320 cards, 9,168,480 description bytes, 960 image references, 160 distinct
avatar URLs, 48 comments, and 40 AI histories/960 messages (one 24-message transcript open).
All nine samples are 1920×1080 maximized, with 1920×1009 client pixels at 96 DPI.

The largest proven adjustable owners were the deserialized sync-store copies. Net live
requested allocations across inbox reads fell from about 31.4 to 7.6 MB; board reads fell
from 21.4 to 2.2 MB. Retained hidden attachment entries fell from 19,200 to zero in inbox
rows and 9,600 to zero in board cards, without deleting the stored bodies. Board-card
retained description bytes fell from 9,168,480 to zero. Total live requested Rust heap
fell from 41.1/82.6/76.0 MB to 17.4/29.8/33.1 MB (inbox/board/task); board peak fell
from 121.1 to 31.5 MB. Horizontal culling avoids constructing/shaping card lists behind
the clip on this 13-column board; it does not claim every saved byte was a text-cache byte.

GPU payload stayed unchanged: both swap-chain buffers 15.50 MB, ClearType atlas 4.19 MB,
image atlases 0/0.262/0 MB, monochrome atlases 0.066/0.066/0.328 MB, and instance buffers
0.303/1.081/0.991 MB. SQLite cache stayed about 0.266 MB, registered font data 1.043 MB,
and AI history content 1.401 MB. These are not decoded-image caches or additive resident
working-set categories. Description/avatar photo decoding is absent in this client;
no claim of a decoded-image saving or a laptop measurement is made.

The comparison emitted `MEMORY_VISUAL_IDENTICAL` for every screen and passed all budget
and undersized-fixture negative controls. Linux, Windows and macOS logs each prove all
four compact-data regressions plus the wide-board scrolling regression passed.
