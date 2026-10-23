# Hypertask Windows renderer patch

Upstream: https://github.com/zed-industries/zed/tree/784e431c42c5c6ef2f10da9b20503a5e8f506df9/crates/gpui_windows

The crate is copied unchanged except for its standalone dependency manifest and `src/directx_renderer.rs`. Dependencies inherited from Zed's workspace are expanded with the same versions, features, and git revision. The Apache license is retained.

The renderer now creates its single-sample and 4x-MSAA path intermediates only on the first nonempty path batch. Both allocations must succeed before either is stored. Resize resets all four texture/view fields; device recovery replaces the resources. Actual path rendering retains the original formats, sample count, dimensions, clearing, blending, and resolve operations.

For Hypertask's path-free screens this avoids `physical_width * physical_height * 4 * (1 + 4)` bytes per window: 24.72 MiB at 1440x900 physical pixels, 55.62 MiB at 2160x1350. SVG icons are rasterised into the monochrome sprite atlas, not these intermediates. Normal text, selection/caret quads, rounded quads, underlines, and shadows also do not need these textures.

These are texture payload savings, not a measured Windows private-memory/working-set reduction. Validate on the owner's Intel laptop before claiming the sub-100-MB target. Paths still allocate the original full-size textures if used; this patch neither disables paths nor changes their antialiasing.
