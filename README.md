# Hypertask Desktop

A native Rust client for [Hypertask](https://app.hypertask.ai), built with [GPUI](https://github.com/zed-industries/zed/tree/main/crates/gpui). It has no browser engine. Every read comes from a local SQLite database, edits apply instantly, and a background worker syncs them with the Hypertask API.

**Status: early pilot.** It covers Inbox, Board and Task detail. Expect rough edges and breaking changes.

## Layout

- `crates/ht-core`: UI-free sync engine (SQLite store, offline write queue, HTTP client, realtime events) and the `ht-core-cli` tool.
- `crates/ht-desktop`: the GPUI app. See `crates/ht-desktop/README.md`.
- `crates/ht-schema-gen`: generates the Rust data model from the server schema.
- `fixtures/demo`: synthetic API responses (fake people, fake text) used by the tests and for offline demos.
- `vendor/gpui_windows`: a patched copy of Zed's Windows renderer, under its original Apache-2.0 license.

## Build

You need a recent stable Rust toolchain. On Linux, install the GPUI system packages first (see `.github/workflows/ci.yml` for the exact list).

```sh
cargo test --workspace --release
cargo build --release -p ht-desktop -p ht-core
```

## Try it offline

```sh
./target/release/ht-core-cli --db /tmp/hypertask.db import-fixtures fixtures/demo
./target/release/ht-desktop --db /tmp/hypertask.db --offline --screen board --board 15
```

With a Hypertask API token in `HT_TOKEN`, the app syncs your own boards instead.
