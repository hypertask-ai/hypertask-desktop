# Desktop patch

Vendored unchanged from zed-industries/zed
`784e431c42c5c6ef2f10da9b20503a5e8f506df9`, except normalized workspace dependency
metadata, relocated upstream test-font includes (with their licenses), and the text-layout hinting flag in `src/cosmic_text_system.rs`.

The upstream backend rasterizes hinted glyphs but lays them out with
`cosmic_text::Hinting::Disabled`. Browser 100%-scale Graphite uses hinted advances.
The CI pinned-web-font probe measures the actual native footer/tab widths against
Chromium, preventing a return to unhinted advances or arbitrary UI compensation.
The inherited Apache-2.0 license is retained in LICENSE-APACHE.
