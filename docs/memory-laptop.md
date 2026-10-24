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
and after a forced GPUI scene rebuild/render plus 22 further seconds. Warm residency
must remain at least 10% below control on every screen; private commit must not
regress by more than 5%. This does not prove interactive navigation latency.

The shared read-only inspector also captures private/shareable/shared resident
axes, image/mapped/private regions and GPU counters on CI. A missing GPU counter
is recorded as unavailable, never invented. All app builds/tests and pixel
checks run on GitHub, not this VPS or the laptop.

Raw non-sensitive laptop measurements are retained in `memory-laptop-before.json`.
Final SHA-bound CI before/after figures will be appended after successful proof.
