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
  Rust heap also includes
  GPUI text-layout objects; it does not isolate their private layout-cache allocations.
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
bodies; comment previews, actor photos/identity, split membership and all rendered metadata
are retained. The full `inbox()` API is unchanged. Each card moves into its
column instead of keeping a full-board JSON tree plus another cloned card set. Legacy
same-name section membership, card ordering and all remaining fields are preserved.
Task details, editing, search, sync and offline persistence still retain full descriptions.

The PR's full-read control compiles the identical source without this feature, retaining
lazy theme fonts and the existing bounded SQLite cache. CI compares the same database and
fixture hashes, requires lower working set, lower board/task live and peak Rust heap, and
requires identical layout JSON and screenshot pixels on every screen. This isolates the
change from unrelated main/prewire development. Shrinking fixtures or increasing a budget
is not an acceptable way to repair a failure. Negative controls exercise all screen limits
and each minimum fixture dimension in CI.

Measured SHA, CI URL, owner deltas and final budgets are recorded in `gates/mem3-GATES.md`.
