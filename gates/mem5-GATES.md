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
