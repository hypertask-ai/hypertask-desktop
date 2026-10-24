# Desktop patch

Vendored from zed-industries/zed `784e431c42c5c6ef2f10da9b20503a5e8f506df9`,
with normalized dependency metadata and relocated licensed upstream test fonts.

Linux layout retains HarfRust shaping but applies FreeType's normal hinted
advances to the selected faces, including fallback and variable-font weights.
The source web Plex asset is unhinted, so FreeType auto-hints it; merely rounding
unhinted design advances does not reproduce the browser. No label-specific
widths or offsets are used. Kerning, clusters and visual bidi order are retained.

`gates/layout-fonts.mjs` compares native layout with both the pinned actual web
asset and full-coverage faces in CI Chromium, at the original strict tolerance.
See `docs/layout-parity.md` for web-source citations and CI evidence.
The inherited Apache-2.0 license is retained in LICENSE-APACHE.
