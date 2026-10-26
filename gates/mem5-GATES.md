# mem5 acceptance, written before implementation

- Host: public GitHub CI only. No VPS builds/tests/launches; no laptop access.
- No working-set eviction, forced collection, minimized measurements, changed fixture,
  changed geometry, raised budget, or receipt-only runtime optimization.
- Inspect the existing `windows-memory` resident maps and owner receipts first.
  Maps identify backing/residency, not individual Rust allocation owners.
- Compare otherwise identical release builds with the candidate optimization
  disabled/enabled. Neither side may trim its working set.
- Each screen: three fresh maximized 1920x1009, DPI 96 processes; independent
  Windows working-set/private-commit counters at 8 seconds and after warm redraw.
- Require lower median private committed bytes and working set on inbox, board,
  and task, under all existing frozen demo/heavy budgets and 100 decimal MB.
- Require identical RGB screenshots and identical layout JSON on each screen;
  retain all existing layout/font contracts and counts.
- Retain all fixture rows, descriptions, comments and AI history. Check the
  allocation owner changed through actual capacity/requested-byte receipts.
- Final exact pushed SHA: every required CI job green, nonexpired artifacts,
  SHA-bound receipt from the supplied wait/receipt script. Report all jobs,
  per-screen before/after medians, PR and CI URLs. No laptop result claimed.

## Initial evidence

GitHub run 40259206578, source ff52d313c1488a1e70816308c3cc48ed7ccd5bfc:
`windows-memory` and `windows-memory-heavy` downloaded read-only to `/tmp/mem5-before-*`.
Those production receipts include startup eviction; their ~2 MB cold working sets
are not allocation baselines and will not be used as optimization evidence.
The mem5 comparison must measure both builds without eviction.

## Work order

1. Identify the largest controlled retained allocation; record the hypothesis.
2. One focused allocation change plus CI-only same-source comparison.
3. Inspect real CI counters, owner data and exact visual/layout comparison.
4. Fix failed checks without relaxing them; retain final exact-head evidence.

Review is serial self-review; no independent subagent claimed.

## Allocation hypothesis and design

The latest maps' private-region committed totals are 33.153/39.051/49.504 MB
(demo inbox/board/task), and 40.497/61.186/63.631 MB (heavy). Maps have no
allocation stack/address owner labels, so these are not falsely called caches.
The corresponding live requested Rust heap is much smaller than total commit;
heavy board/task retain 34.611/40.218 MB. SQLite is already 266,184 bytes,
registered fonts ~1 MB, and actual atlas/buffer payloads are unchanged from mem2.
Images/avatars have no hidden decoder to disable. Repacking ClearType was rejected
previously for changing pixels. AI histories hold ~1.4 MB, below retained UI/data.

Candidate A: change JSON retained shapes further. Rejected for this first unit:
this crosses UI/data contracts and would need a field-by-field proof.
Candidate B: request Windows' segment heap in the executable manifest. Selected:
the process-owned small-allocation heap policy changes, but Rust's System
allocator, requested layouts, fonts, textures and rendering algorithms do not.
Hypothesis, not yet measured: lower native heap overhead/fragmentation reduces
commit and residency. A previous mimalloc experiment regressed and is not reused.

Shape: build.rs embeds one process manifest only for MSVC Windows; a CI-only
`legacy-windows-heap` feature omits that manifest. Both builds delete the previous
startup eviction. No runtime receipt/env toggle changes heap policy. CI measures
both same-source releases, verifies actual PE manifest selection, exact pixels,
layout, retained workload, and private-commit/working-set improvements. The
sequence/prove-it principles require real CI evidence before accepting this unit;
the laziness principle chooses the manifest over a new allocator implementation.


## Hypothesis rejected; revised unit

CI 40264147263 rejected the segment-heap manifest at the Windows linker:
`CVTRES : fatal error CVT1100: duplicate resource. type:MANIFEST, name:1`.
The upstream GPUI manifest already requests SegmentHeap, with DPI/Common-Controls
settings. The added manifest/build script are removed, not patched around.
No allocation improvement was established for that rejected attempt.

The selected owner is now retained notification JSON: measured heavy inbox reads
retain ~7.7 MB requested Rust bytes on every screen, versus only ~2.2 MB of
board-card reads. The display reads only seven raw metadata fields after identity,
title, ticket, source, seen and timestamp have been extracted to typed fields.
A compact SQL projection omits unused top-level payloads before deserialization;
actor/source/identity JSON is released after extracting those typed fields.
The full `inbox()` and persisted bodies remain unchanged. Rows stream, avoiding
the intermediate full Vec<Value>. Existing task metadata projection is unchanged.

Same-source CI control disables only `compact-inbox-metadata`; it retains all
other production optimizations and also has no eviction. Both builds use the
same existing segment heap. Compare exact layouts and cold/warm RGB, typed data
counts, GPU/fonts/SQLite ownership, lower retained payload and inbox read allocation,
and lower Windows working-set/private-commit medians at both capture points.
The regression explicitly preserves bool flags, actor precedence/multi-actor names,
uncached titles, invitation fallback, splits, metadata and full stored photos.


## Second receipt: real reduction, insufficient warm budget

Run 40265726201 passed Linux/macOS/ARM64/workspace and the retained-metadata
control. Demo cold WS before -> after: 61,657,088 -> 60,801,024 inbox;
67,461,120 -> 66,371,584 board; 78,848,000 -> 77,926,400 task.
Inbox net read ownership fell 2,260,424 -> 1,429,863 requested bytes on every
screen. Warm WS remained 66,269,184 / 72,339,456 / 85,196,800 bytes; the frozen
inbox warm limit correctly failed. No budget is changed.

The next controlled owner is unconditional `gpui/profiler`. Its foreground
journal allocates a fixed 4 MiB ring even without a diagnostic consumer;
WindowProfiler also retains six histograms plus per-refresh clones. The maps'
~4.55 MB common private allocation is consistent with these owners, not a
stack-traced attribution. Ordinary release builds never consume them, except
READY previously queried frame-duration histograms.

Production now omits the profiler feature. `input-benchmark` opts into the same
original profiler for the frame-time gate and a new CI command smoke. READY uses
two successive native frame callbacks on both compared builds: GPUI drains a
callback batch before draw/present, so the second batch follows presentation.
The existing native Windows/Linux input and negative controls must still pass.
No UI, draw path, bitmap, atlas, GPU format, or memory-measurement timing changes.
The control retains both original inbox metadata and the profiler. Both sides
now check out the exact head SHA, avoiding a head versus PR-merge BuildRevision
mismatch in the stronger comparison contract. Require a >4 MB requested-Rust-heap
reduction as well as lower commit/WS on every screen. No benchmark result is
claimed from an uninstrumented executable; --bench-input explicitly rejects it.


## Third receipt: demo accepted; heavy warm staging still over budget

Run 40268307390 passed demo cold/warm budgets and the opt-in input benchmark smoke.
Demo cold WS before -> after: 61.719 -> 56.017 / 67.342 -> 61.575 /
78.889 -> 73.093 decimal MB; warm: 67.187 -> 61.186 / 73.265 -> 67.486 /
86.405 -> 80.327 MB. Requested Rust heap fell by ~5.8 MB per demo screen.
Heavy cold WS after was 67.744/87.474/89.735 MB; warm 65.991/101.028/106.328 MB.
Heavy private commit still fell on every screen, but warm board/task correctly
failed unchanged limits. The heavy maps distinguish mapped driver staging from
private Rust data; working set cannot be equated with nominal resource payloads.

Next unit: structured instance buffers use D3D11_USAGE_DEFAULT and bounded
UpdateSubresource uploads rather than CPU-writable DYNAMIC/WRITE_DISCARD buffers.
The latter permit driver-owned renamed/staging allocations across redraw/resize.
Constant buffers stay dynamic. Instance capacities, stride, shaders, formats and
all initialized bytes are identical. Empty updates do no work; partial updates
use a byte-sized D3D11_BOX so unused capacity is never read from the Rust slice.
The CI-only mapped-instance-buffers control retains the old policy. Actual
ID3D11Buffer descriptions prove CPU-writable instance capacity goes to zero,
while all other GPU capacities and font/SQLite/retained-visible-data checks stay.
The end-to-end CI gates, not assumed driver behavior, decide whether this helps.
