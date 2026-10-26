# GPUI source provenance

Source: https://github.com/zed-industries/zed/tree/784e431c42c5c6ef2f10da9b20503a5e8f506df9/crates/gpui
License: LICENSE-APACHE, unchanged.

The source/build/resources are copied unchanged except these storage changes:

- `src/text_system/line.rs`: one inline DecorationRun rather than 32.
- `src/text_system.rs`: use that same inline capacity; wrapped lines initially
  reserve one run rather than 32. Both containers still grow for arbitrary styles.

`legacy-layout-storage` selects the original capacities for same-source CI A/B.
`src/arena.rs` also releases chunks unused by the last completed frame after
all arena values are dropped, rather than retaining a historical resize peak.
Active nested scopes still defer clear. `src/app.rs` exposes requested element
arena capacity for the existing owner receipt. `src/gpui.rs` re-exports Arena
only under the existing test-support feature for the lifetime regression. No working-set calls are made.

No shaping, font selection, glyph, decoration, raster, packing or painting logic changes.
The standalone Cargo manifest resolves inherited dependencies to the same upstream
revision/registry requirements, and disables upstream example/integration/doctest
entrypoints not vendored here. Existing application/backend tests remain enabled.
The crate is vendored because private GPUI line storage cannot be changed safely
through its public API. Do not replace it with an unsafe layout cast or allocator trick.
