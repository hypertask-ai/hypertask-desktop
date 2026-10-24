# HTPR-7381: installed laptop versus hosted Windows

## Read-only laptop observation

At 2026-10-24 20:21 UTC, the existing installed `ht-desktop` PID 23744 was
queried through `lh shell`. No app was started, stopped, installed, focused,
clicked, typed into, minimized or trimmed. The inspector uses only process
queries, `VirtualQueryEx`, `QueryWorkingSetEx`, mapped-file names, module lists,
window geometry/DPI queries and built-in GPU performance counters. Its process
handle has QUERY_INFORMATION | VM_READ rights, not mutation rights; it never
reads target page contents. The inspector's own Add-Type compilation is not an
app build or test copy.

The initial Get-Process observation was 114,769,920 working-set bytes
(**109.45 MiB**, **114.77 decimal MB**), with 92,413,952 private committed bytes.
This distinction alone explains part of the apparent 109 versus CI decimal MB
gap. The map captured 114,724,864 resident bytes:

| Region backing | Resident MB | Private resident MB | Shareable resident MB |
| --- | ---: | ---: | ---: |
| Private | 67.051520 | 67.043328 | 0.008192 |
| Image (EXE/DLL) | 46.055424 | 4.157440 | 41.897984 |
| Mapped | 1.617920 | 0 | 1.617920 |
| Total | 114.724864 | 71.200768 | 43.524096 |

Shareable pages had a share count greater than one at this snapshot. Image
copy-on-write pages are counted as private resident, not falsely labelled shared.
These axes overlap: **do not add image MB to private/shared MB**. Module virtual
sizes and committed private bytes are also not resident-page measurements.

The largest resident image files were Intel `igd10umt64xe.dll` (6.586368 MB),
Intel `igc64.dll` (6.025216 MB), and the executable (5.611520 MB). Their code is
required by the active driver/rendering path, not an optional DLL we can safely
unload. The graphics counter instance for this PID reported zero dedicated and
37.220352 MB shared GPU memory, with the same total committed. GPU accounting
is **not additive to process working set**; it neither proves 37 MB exclusive
CPU residency nor isolates driver heap versus surfaces.

Client geometry was maximized 1920x1009 at DPI 96, exactly the CI geometry.
There is no high-DPI multiplier in this observed window. A single BGRA surface
at this size has 7.749120 MB payload; CI's two-buffer swapchain is 15.498240 MB.
External read-only built-in queries cannot enumerate this installed process's
D3D resource objects or label individual driver-private pages as atlas,
swapchain or per-monitor compositor surfaces. Those figures are therefore
**CI resource receipts / nominal arithmetic, not laptop resource measurements**.
The installed process's renderer receipt was not enabled and is not injected.

The map is not atomic: the post-map process counter had fallen to 106.840064 MB
working set / 83.890176 MB private commit. A later read was 108.032000 / 85.082112
MB. A repeat map failed because PID 23744 no longer existed; we did not restart
it. Do not describe this as a controlled fixed-state laptop benchmark or claim
an after-install result.

## Comparison with PR #20

CI run [40126115158](https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40126115158),
source `05bc7fc5f9262de1b63563187d086ef47be4db3d`, has third-run maps:

| CI heavy screen | Private-region resident MB | Image resident MB | Mapped resident MB |
| --- | ---: | ---: | ---: |
| Inbox | 28.708864 | 30.330880 | 16.551936 |
| Board | 48.840704 | 30.765056 | 16.699392 |
| Task | 50.536448 | 30.756864 | 16.809984 |

Its median working sets are 75.624448 / 96.362496 / 98.131968 MB.
The laptop has roughly 15.3 MB more image residency than CI, dominated in part
by its Intel driver, substantially less mapped residency, and more private
resident memory. Workloads, installed revision and time since startup are not
identical, so this is attribution, not a causal A/B. The heavy receipt already
shows font atlas 4.194304 MB, embedded registered fonts 1.043 MB and no path
textures. Atlas repacking previously changed a pixel; it is not repeated here.
Two buffers are already the flip-model minimum; reducing to one is invalid.
Disabling DirectComposition can add HWND redirection surfaces, so it is not a
justified blind replacement either.

## Production change and CI contract

Once, five seconds after opening the foreground window, Windows is asked to
evict the process's working set (`EmptyWorkingSet`). There is no periodic trim,
no change to the process's working-set limits, no minimize trick and no
receipt-dependent activation. All allocations, fonts, GPU resources and pixels
remain intact; needed pages fault back in. This is **residency management, not
an allocation reduction**, and can incur soft/hard faults on subsequent use.
It is not a promise that 109 MiB will stay low after prolonged activity.

The `untrimmed-startup` feature exists only for CI's otherwise identical control.
Both builds keep compact task reads, visible-column culling, lazy theme fonts
and the bounded SQLite cache. Demo and heavy data, hashes and limits stay fixed.
CI compares exact RGB/layout between builds, unchanged renderer/font payloads,
a successful one-shot receipt for every sample, and median working sets at 8 s
and after a real restore/maximize cycle plus 22 further seconds. Warm residency
must remain at least 10% below control on every screen; private commit must not
regress by more than 5%. Each restore and maximize must update the actual renderer receipt, and final
geometry must match. Warm screenshots are captured after settling, and must
match between builds after the identical resize interaction. They are not
compared with the pre-resize state: resizing legitimately changes chat scroll
anchors (for example the existing jump-to-bottom indicator). Warm resident
maps are also captured, after the independent warm counters. This does not prove interactive navigation latency.
The initial external force-render message was rejected by GPUI's validation
token; its apparently successful CI run is not accepted as warm-render proof.

The shared read-only inspector also captures private/shareable/shared resident
axes, image/mapped/private regions and GPU counters on CI. A missing GPU counter
is recorded as unavailable, never invented. All app builds/tests and pixel
checks run on GitHub, not this VPS or the laptop.

Raw non-sensitive laptop measurements are retained in `memory-laptop-before.json`.
## Accepted CI proof

[CI 40133081908](https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40133081908),
source `cb9c4939c064ab6972e7bc39bd352d0de02a7e6b`, passed all six jobs:
Ubuntu 24.04, Windows 2025, macOS 15, Windows ARM64, untrimmed-startup control,
and Windows memory receipt/budget. Both demo and heavy emitted three exact
`MEMORY_VISUAL_IDENTICAL` and three `MEM4_WARM_VISUAL_IDENTICAL` results, plus
all three unchanged-warm-residency rejection controls. All fixtures, stored
bodies, atlas formats/dimensions, font bytes and swapchain payloads are unchanged.

Heavy medians, decimal MB (before is the same-source untrimmed control):

| Screen | 8 s WS before → after | Warm WS before → after | Warm private commit before → after |
| --- | ---: | ---: | ---: |
| Inbox | 75.632640 → 2.125824 | 73.789440 → 36.622336 | 43.446272 → 44.212224 |
| Board | 96.649216 → 2.347008 | 110.309376 → 67.637248 | 67.284992 → 67.444736 |
| Task | 98.021376 → 2.400256 | 96.702464 → 56.127488 | 66.285568 → 67.346432 |

**The 2 MB figures are idle residency after eviction, not the app's memory
allocation or its active-use cost.** The real-redraw warm figures are the useful
headline. Private commit is not reduced; it slightly increases in these samples.
Demo warm WS falls 66.953216 → 30.621696 / 72.622080 → 35.082240 /
85.782528 → 46.514176 MB. Warm is at least 30 s after launch (8 s, the
restore/maximize/receipt-update cycle, 22 s); independent counter samples precede
PrintWindow capture and the second resident scan.

Third-run heavy *warm* maps, not medians, decimal MB:

| Screen | Private-region resident before → after | Image resident before → after | Mapped resident before → after |
| --- | ---: | ---: | ---: |
| Inbox | 29.433856 → 16.916480 | 30.343168 → 6.606848 | 13.979648 → 13.058048 |
| Board | 52.453376 → 34.414592 | 30.773248 → 7.008256 | 27.049984 → 26.013696 |
| Task | 51.691520 → 35.991552 | 30.752768 → 6.987776 | 14.221312 → 13.156352 |

This demonstrates reduced resident cold image/private pages, not released GPU
textures or reduced data ownership. CI GPU TotalCommitted warm counters were
30.523392 → 30.523392 / 31.715328 → 31.715328 / 31.543296 → 26.357760 MB;
cache-dependent counters are not a claimed resource-allocation saving. The
renderer receipts still report the identical two-buffer 15.498240 MB swapchain,
4.194304 MB ClearType atlas and original mono/image/instance capacities.

Frozen demo/heavy budgets now apply independently to **both** original 8 s
medians and real-redraw warm medians on every normal Windows CI run. Every
screen has an additional warm-over-budget negative control. No limit is raised;
trimming cannot turn the gate into a check of only the 2 MB idle snapshot.

The live-account poller can touch pages again each second, and network/sync and
later navigation can fault other pages back in. This offline CI proof therefore
does not guarantee a sustained installed-laptop number or navigation latency.
The final PR head is reverified by GitHub after this evidence/budget-only commit;
its current checks and final run URL are recorded in PR #21, not guessed here.
