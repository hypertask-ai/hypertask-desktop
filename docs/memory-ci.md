# Windows memory gate (HTPR-7381)

## mem5: allocation-only policy

The mem5 candidate removes the previous startup working-set eviction entirely.
Both its normal build and same-source retained-inbox-metadata control run without
trimming. Production projects inbox display metadata before Rust deserialization, then
drops actor/identity JSON already represented by typed row fields. The full
notification remains in SQLite and `inbox()`. Rows stream instead of retaining
an intermediate complete JSON vector. Ordinary releases also stop compiling
unused GPUI performance history (a fixed 4 MiB journal plus histograms); the
`input-benchmark` feature retains it for the frame-time gate. READY waits for
successive native frame callbacks instead of retaining diagnostic histograms.
Structured instance buffers use GPU-default storage with bounded uploads, not
CPU-writable discard maps that permit driver staging/renaming. Constant buffers
remain dynamic. Actual D3D buffer descriptions report the writable capacity.
AI welcome choices now use one random sidebar seed instead of changing on every
render (which also invalidated the view). Both CI builds use recorded fixture
seed 7381 for comparable content, without masking pixels or shrinking the pool.
The Windows segment heap was already
enabled upstream; that hypothesis was rejected. The harness reads the actual PE
manifest resource into `memory.json`; `mem5-proof.py` checks identical heap policy,
lower working-set/private-commit medians at both capture points, identical
visible data and GPU/font/store ownership, and exact RGB/layout on demo and heavy workloads.
Existing frozen budgets, fixtures, display geometry and warm redraws stay fixed.
This is a hypothesis until the mem5 CI receipt proves it; no new laptop
measurement or sustained laptop working-set guarantee is claimed.

The historical measurements below predate mem5. See [memory-laptop.md](memory-laptop.md)
for the read-only laptop attribution and historical mem4 eviction policy.
New receipts include warm counters and private/shared resident axes.

Measurements run only on GitHub-hosted `windows-2025`, never on the VPS or the
owner's laptop. The ordinary Windows build job starts the release executable
against a fresh copy of the imported `fixtures/demo` database for each sample:

```
--offline --theme graphite --screen inbox|board|task --board 15 --task 58006
```

Task 58006 includes 50 fixture comments. The harness requests a 1920x1080 display,
maximizes the window, waits eight seconds, and reads `WorkingSet64` and
`PrivateMemorySize64` from the live process. Each screen gets three fresh
processes; the gate uses their median. It does not trim working sets, force GC,
minimize the app, disable rendering, or simulate memory values. Display request
failure is allowed, but actual display/client dimensions, DPI and maximized state
are recorded. The measured hosted runners used 1920x1080, 1920x1009 client, DPI 96.

`windows-memory` contains `memory.json` (raw samples, medians, source/build
revision, executable/database SHA-256 and runner image), screen PNGs, layout
JSON, fixture/run databases, and resident memory maps. The maps are collected
**after** the counters, using `VirtualQueryEx`/`QueryWorkingSetEx`, without
reading or faulting in the target's pages. Console and job summaries use decimal
MB (1,000,000 bytes), not MiB.

The frozen budget is in `gates/windows-memory-budget.json`. Updating it requires
an intentional review of new CI measurements; it is not recalculated from the
candidate under test. `windows-memory-check.py` validates the samples/medians,
enforces the frozen per-screen budgets and display geometry, and proves each
screen's rejection path using an over-limit receipt. Historical comparison/visual-proof jobs run only for the `ram2` PR.

## Historical measurements

[CI 40057951407](https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40057951407)
built both historical revisions and ran the same harness and fixture inputs.
All six jobs passed. Values are median working set / private MB:

| Revision | Inbox | Board | Task (50 comments) |
| --- | ---: | ---: | ---: |
| `3d002231efdbc5ea4b823e5a1af430e912a168d9` | 60.445 / 34.824 | 69.853 / 45.646 | 85.185 / 64.451 |
| PR #11, `5016a9ebe48b03dbac2f2312c170878166586c59` | 62.542 / 37.155 | 69.960 / 45.609 | 80.691 / 60.236 |

The reported laptop 105–106 -> 119 MB increase **does not reproduce** in this
hosted-runner/fixture workload. CI reproduces only +2.097 MB inbox working set
(+2.331 MB private); board is approximately unchanged and task decreases.
The resident maps show inbox private resident pages increasing from 21.69 to
23.41 MB; executable-resident pages increase from 4.38 to 4.66 MB. These are
third-run snapshots, not medians or a heap allocation trace.

PR #11 consumes authoritative server split membership. The saved layout proves
that the Important inbox renders five fixture rows instead of two. This is a
real behavior change in PR #11, not something the memory fix should undo. The
imports use identical fixture JSON; their database hashes differ because the
new importer also caches those authoritative splits. The emoji catalog already
uses `OnceLock` behind picker-only calls; it is not eagerly parsed at startup.

## Allocation changes

- Windows monochrome/image atlases start at 256x256 rather than 1024x1024.
  Subsequent atlases grow to 512 and then 1024, and large individual images still
  allocate their required dimensions. ClearType atlases retain their original
  size/packing: shrinking them failed the strict CI comparison by one blue-channel
  level in one task-screen pixel. Formats, shaders, glyphs and filtering are
  unchanged. A sparse image atlas previously reserved 4 MiB; its initial atlas now
  reserves 256 KiB. CI covers growth, large images and tile-space reuse.
- Measured editor layout borrows the live entity's document instead of keeping
  a deep-cloned document tree in every measurement closure. This removes a
  duplicate for every read-only comment introduced by PR #11 as well as editors;
  layout, selection and editing continue to use the same document implementation.

The PR's visual-proof job requires byte-identical RGB screenshots and identical
layout JSON against PR #11 for all three screens, in addition to the release
memory target. Backend/editor unit tests and Linux's multi-theme layout contract
remain in the normal CI matrix. Hosted-runner numbers cannot establish the
owner's hardware working set; no laptop measurement is claimed here.

## Frozen fixed baseline and proof

[CI 40059480037](https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40059480037),
source `f34cbdc21b90ce6561843d518f0fd1080391fbb8`, passed all seven jobs,
including `small_atlases_grow_and_large_images_keep_their_size` and exact
`MEMORY_VISUAL_IDENTICAL inbox`, `board`, and `task` checks. The database SHA-256
also matches PR #11's seed. Working set / private decimal MB:

| Screen | Fixed working set | Fixed private |
| --- | ---: | ---: |
| Inbox | 62.394368 | 36.163584 |
| Board | 69.804032 | 44.556288 |
| Task | 80.318464 | 54.538240 |

The original frozen inbox limit was `floor(62,394,368 * 105 / 100) = 65,514,086` bytes.
The normal Windows job fails above this value; the threshold does not follow the
candidate. The same oracle must reject a receipt at 65,514,087 bytes and print
`MEMORY_OVER_BUDGET_CONTROL_REJECTED`. Full raw samples remain in that CI run's
`windows-memory` artifact. Compared with PR #11 in the same run, private memory
fell by 1.044 / 2.118 / 4.866 MB; working-set improvements were much smaller
(0.115 / 0.180 / 0.242 MB). Do not present those small working-set differences as
a demonstrated fix for the un-reproduced 119 MB laptop observation.

## mem2 owner receipt and rejected allocator experiment

[CI 40110976285](https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40110976285)
compared the system allocator (`a5505cc`) with mimalloc (`d821784`), with identical
pixels and layout on all three screens. The experiment **regressed** and was
removed, including its dependency/lock entries:

| Screen | System WS / private MB | mimalloc WS / private MB |
| --- | ---: | ---: |
| Inbox | 62.787584 / 36.179968 | 65.245184 / 76.550144 |
| Board | 73.314304 / 47.456256 | 78.753792 / 94.556160 |
| Task | 80.814080 / 55.414784 | 88.510464 / 112.275456 |

Third-run system-allocator maps attribute 23.564 / 33.583 / 40.440 MB resident
pages to private regions; mapped regions use 8.831 / 8.954 / 9.081 MB and image
regions 30.478 / 30.589 / 30.962 MB. These snapshots are not the median samples
and are not a heap allocation trace. Private pages remain the largest adjustable
category; the measured mimalloc result is evidence against adopting it here.

The new backend receipt measures actual allocated resource dimensions/capacities:
all screens use a 4,194,304-byte ClearType font atlas; monochrome atlases use
65,536 / 65,536 / 327,680 bytes; image atlases 0 / 262,144 / 262,144 bytes;
instance buffers 302,592 / 341,568 / 532,416 bytes; path textures use **zero**.
Image caches/buffers therefore do not explain the 114 MB report in this workload.
These GPU payloads are not resident pages and must not be added to working set.
Additional receipts report nominal swap-chain payload, SQLite's actual connection
cache/schema/statement bytes, and registered embedded font bytes/face counts.
Unattributed private memory is not falsely labelled as allocator or image cache.

Font registration now defers Inter/Newsreader until Dia is used.
Plex and the ticket's monospace face remain available immediately; selecting Dia
loads its original, unmodified faces before rendering, once per application.
All bytes remain embedded, and no font is subsetted or rasterization changed.
The CI eager-fonts/triple-buffer build uses the same source/fixture with only
the optimization features changed; this separates memory effects from intentional fidelity fixes (such as
showing the creator agent rather than its owner). Both raw receipts, exact RGB
screenshots and layout JSON are compared in GitHub, not on the VPS or laptop.

The owner receipt also exposed the largest nominal GPU allocation: the three
1920x1009 BGRA swap-chain buffers reserve 23,247,360 payload bytes. The optimized
FLIP_SEQUENTIAL chain uses two buffers (15,498,240 bytes), including after resize.
`gpui_windows/triple-buffer` exists only to build the identical pre-optimization
comparison in CI; production defaults to two. Formats, alpha mode, shaders,
present interval and pixel content are unchanged. The fixed font atlas is left
alone because earlier repacking experiments changed a filtered pixel channel.

## Lower all-screen frozen budgets

[CI 40113971853](https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40113971853),
source `9871cc2f9f1530a7cbcdbdef59e2415425538445`, passed all six jobs, including
three exact `MEMORY_VISUAL_IDENTICAL` checks. Same-source eager/triple-buffer
versus optimized working sets were 62.676992 -> 62.476288 MB (inbox),
73.478144 -> 73.166848 MB (board), and 80.392192 -> 80.048128 MB (task).
These small working-set changes must not be confused with the much larger
nominal GPU payload savings or a fix proven on the owner's laptop.

The new frozen limits use this measured optimized revision plus **2%**, not the
previous 5%: 63,725,813 / 74,630,184 / 81,649,090 bytes for inbox/board/task.
The inbox limit is strictly lower than the old 65,514,086-byte cap; board/task
now have much tighter gates than the prior 100 MB target. The exact frozen
Full-HD client dimensions and DPI are enforced. Each screen's own over-limit
negative control must fail; the mem2 comparison must also show a real working-set
reduction on every screen, not merely unchanged pixels under the old target.

The receipt measures SQLite page-cache ownership at 914,616 / 1,093,048 /
1,358,520 bytes before the final cache bound. The desktop now requests a 256 KiB
connection page-cache budget (`PRAGMA cache_size=-256`), not deletion or truncation
of durable synced rows. A disk-backed CI regression exceeds that cache with 128
large tasks, then reopens the database and reads every complete description.
The same-source A/B build retains the original SQLite cache as well as eager
fonts/three buffers, so the final receipt measures their combined effect. No
working-set trimming, forced heap collection or measurement-only runtime tuning
is used. The final SHA-bound six-job result and raw receipt are linked in PR #19.

The initial 512 KiB request exceeded the regression's actual 600,000-byte cache
allocation ceiling on all three CI platforms. SQLite's setting is a soft page
target, not an allocation-byte ceiling. The request was tightened to 256 KiB;
the original actual-memory assertion remains unchanged, and both the setting
and actual cache bytes are now recorded rather than assuming they are equal.

## Bounded-cache receipt

[CI 40116151008](https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40116151008),
source `7661604d9778ad6cd3b092309e7bcaa365ebb17b`, passed all six jobs. Same-source
before -> after median working sets were:

| Screen | Before MB | After MB |
| --- | ---: | ---: |
| Inbox | 62.779392 | 61.845504 |
| Board | 73.478144 | 72.122368 |
| Task | 80.539648 | 79.085568 |

All three exact RGB/layout comparisons and all three over-budget rejection
controls passed. SQLite's measured cache is now **266,168 bytes on every
screen**, versus 914,616 / 1,093,048 / 1,358,520 bytes in the original cache.
The setting receipt independently reports `-256`. The 128-task persistence and
actual-cache-ceiling regression passed on Ubuntu, Windows and macOS, as did the
synced-author/summary/Draft rendering regression. This is a hosted CI result,
not a measurement of the owner's reported 114 MB laptop workload.


## mem6: compact text styles and layout storage

[CI 40291404083](https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40291404083)
verified source `92bbea228a5af9d15c26bcea7311a77c9ea712d4`.
All nine jobs passed. Both workloads retained identical cold/warm RGB pixels.
Layout JSON and counts matched exactly.

Same-source control → production, median of three launches.
Working sets use decimal MB. Private commit uses bytes.

| Workload/screen | 8 s WS MB | Warm WS MB | 8 s private committed bytes | Warm private committed bytes |
| --- | ---: | ---: | ---: | ---: |
| demo inbox | 56.532992 → 55.394304 | 61.718528 → 60.497920 | 30,572,544 → 28,631,040 | 30,629,888 → 29,470,720 |
| demo board | 61.771776 → 60.649472 | 67.678208 → 66.158592 | 37,527,552 → 36,061,184 | 39,096,320 → 36,663,296 |
| demo task | 73.359360 → 69.423104 | 79.994880 → 75.923456 | 48,058,368 → 43,679,744 | 49,176,576 → 45,600,768 |
| heavy inbox | 69.222400 → 67.289088 | 67.469312 → 65.327104 | 36,716,544 → 34,820,096 | 36,802,560 → 33,845,248 |
| heavy board | 87.638016 → 83.464192 | 90.566656 → 85.262336 | 56,557,568 → 52,428,800 | 59,031,552 → 54,542,336 |
| heavy task | 90.017792 → 83.623936 | 90.296320 → 82.604032 | 58,740,736 → 52,588,544 | 61,378,560 → 54,214,656 |

The largest changed owner was retained text storage.
`ShapedLine` shrank from 2,984 to 136 bytes.
It now embeds one style, not 32.
Wrapped text also starts with one style.
Both containers grow for arbitrary style counts.
The regression paints 64 alternating underline styles.
It checks every painted range and color.
Fonts, shaping, glyphs, packing and painting stay unchanged.

Filtered notifications now release oversized backing maps.
Keys and values move without cloning or omission.
Heavy retained Rust heap falls by ~1.536 MB.
That saving occurs on every heavy screen.
Total heavy live requested Rust heap falls
12.268 → 10.397 / 26.560 → 21.988 /
32.554 → 23.798 MB (inbox/board/task).
These requested allocations are not resident-page measurements.

The arena also releases unused historical frame chunks.
Active nested frames still defer clearing.
Demo board capacity falls 2,097,152 → 1,048,576 bytes.
Heavy capacity remains unchanged at 1/3/3 MiB.
No heavy saving is attributed to this change.

GPUI source is pinned and vendored with licensing.
`vendor/gpui/VENDORED.md` identifies the narrow upstream changes.
The `legacy-layout-storage` control restores both original capacities.
It also retains the original inbox backing maps.
Atlas, font, SQLite and stored-data receipts remain unchanged.
Budgets, workloads, geometry and timing remain frozen.
No eviction, working-set limits or forced collection.

Heavy screens remain above 80 MB.
The laptop target remains unverified.
No owner-laptop access or measurement occurred.
See [mem6 acceptance](../gates/mem6-GATES.md) for rejected attempts.
