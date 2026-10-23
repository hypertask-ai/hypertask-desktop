# Hypertask Windows renderer patch

Upstream: https://github.com/zed-industries/zed/tree/784e431c42c5c6ef2f10da9b20503a5e8f506df9/crates/gpui_windows

The crate is copied unchanged except for its standalone dependency manifest and `src/directx_renderer.rs`. Dependencies inherited from Zed's workspace are expanded with the same versions, features, and git revision. The Apache license is retained.

The renderer now creates its single-sample and 4x-MSAA path intermediates only on the first nonempty path batch. Both allocations must succeed before either is stored. Resize resets all four texture/view fields; device recovery replaces the resources. Actual path rendering retains the original formats, sample count, dimensions, clearing, blending, and resolve operations.

For Hypertask's path-free screens this avoids `physical_width * physical_height * 4 * (1 + 4)` bytes per window: 24.72 MiB at 1440x900 physical pixels, 55.62 MiB at 2160x1350. SVG icons are rasterised into the monochrome sprite atlas, not these intermediates. Normal text, selection/caret quads, rounded quads, underlines, and shadows also do not need these textures.

These are texture payload savings, not a measured Windows private-memory/working-set reduction. Validate on the owner's Intel laptop before claiming the sub-100-MB target. Paths still allocate the original full-size textures if used; this patch neither disables paths nor changes their antialiasing.

## DirectWrite font ownership (HTPR-7381)

`CreateInMemoryFontFileReference` copies the complete font when its owner is null.
The backend now supplies a COM owner holding the original immutable `Cow<'static,
[u8]>`. Borrowed embedded fonts stay in the executable mapping; owned fonts keep
their original allocation until DirectWrite releases its last reference. No fonts,
weights, glyphs, fallbacks, rasterization settings, or atlas sizes are removed or
changed.

The vendored backend is a workspace member so Windows CI executes its unit tests.
`custom_font_stream_retains_original_borrowed_and_owned_bytes` checks both variants:
the DirectWrite stream returns the original allocation and its bytes remain valid
after the text system is dropped. The window's `render_to_image` trait method is
gated on `test-support`, matching the dependency trait rather than this crate's
independent `cfg(test)`.

### Measured experiment

Owner's Intel Core Ultra 9 185H laptop, primary display 1920x1080, DPI 96, maximized
1920x1009 client, Graphite, offline copy of `parity.db`, board 7283/task 60253.
Three fresh processes per screen; medians after eight seconds. Decimal MB:

| Screen | Main working set | Font owner working set | Main private | Font owner private |
| --- | ---: | ---: | ---: | ---: |
| Inbox | 87.540 | 81.052 | 63.492 | 59.908 |
| Board | 87.482 | 81.093 | 63.513 | 59.851 |
| Task | 88.515 | 82.239 | 64.401 | 60.830 |

Before: `e5ce3d0d0ff75affe060dc4e3813d87c2ecadfdc`.
After: `f273648523fcb3bb9dabffc4995ac5da88fb602b`.
[After CI](https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40027373384)
passed all four platforms/jobs, including 20 Windows backend unit tests.
The older reported 116–160 MB was not reproduced on current main; it was already
below 100 MB before the font change. These numbers do not attribute that earlier
reduction to this patch.

The separate two-buffer experiment (`5d29f711871ea00f87586ba8c46b0c0deb28ffd8`)
measured 87.470 / 87.454 / 88.441 MB working set: only 0.03–0.07 MB different,
within the sample variation. It was dropped; the final swap chain still has three
buffers. No working-set trimming or allocator/cache changes are used.

`gates/ram-round.sh <sha> <out>` mirrors the operator's runner in
`hypertask-desktop-wt-public/internal/gates/ram-round.sh`. It waits for SHA-bound
GitHub CI, downloads the Windows artifact, uses `lh-put.sh` with hash verification,
and serializes with the laptop behavior runner. All app execution and DB copies
stay under `C:\Users\valen\AppData\Local\Temp\htram`. Only downloaded evidence is
processed on the VPS. Captures normalize hover without moving the physical cursor;
resize/minimize/maximize captures are taken after memory sampling, not before it.
