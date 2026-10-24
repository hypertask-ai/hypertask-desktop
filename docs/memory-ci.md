# Windows memory gate (HTPR-7381)

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
enforces the inbox budget, and proves its rejection path using an over-limit
receipt. Historical comparison/visual-proof jobs run only for the `ram2` PR.

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

The frozen inbox limit is `floor(62,394,368 * 105 / 100) = 65,514,086` bytes.
The normal Windows job fails above this value; the threshold does not follow the
candidate. The same oracle must reject a receipt at 65,514,087 bytes and print
`MEMORY_OVER_BUDGET_CONTROL_REJECTED`. Full raw samples remain in that CI run's
`windows-memory` artifact. Compared with PR #11 in the same run, private memory
fell by 1.044 / 2.118 / 4.866 MB; working-set improvements were much smaller
(0.115 / 0.180 / 0.242 MB). Do not present those small working-set differences as
a demonstrated fix for the un-reproduced 119 MB laptop observation.
