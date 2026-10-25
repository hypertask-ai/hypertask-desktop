# Updater acceptance

## Checks written before implementation

1. Public GitHub CI tests parse valid SHA-256 manifests and reject malformed, duplicate, missing, or unexpected assets.
2. Hash mismatches never create an executable stage. Staged files are reverified before swapping.
3. Equal and older releases skip. Same package version with a newer main-build sequence updates; the tag includes version, build sequence and short SHA.
4. Temporary-directory tests prove staging, executable rename, successful startup, failed startup rollback, and interrupted-swap recovery. Windows CI exercises the real running-executable rename and startup handshake.
5. Source checks reject release URLs outside this repository and unsafe redirects. Downloads use no token, bounded sizes and timeouts.
6. Existing rotating diagnostics record updates without response bodies or secrets. No new visible controls, prompts, or layout changes.
7. A separate release workflow accepts only successful main-push CI from this repository. It uses that exact run's Windows x64 and ARM64 artifacts, publishes both executables and their SHA-256 manifest, and never publishes a partial release.
8. The exact final branch SHA passes every required CI job and the repository's read-only CI receipt. Evidence consists only of public GitHub runner output. No VPS builds/tests/lint/app launches and no laptop commands.

## Plan and design

- Ground: `main.rs` opens the database before GPUI. `ui.rs` emits READY only after a presented frame. `ht-core/src/sync_log.rs` owns bounded rotating diagnostics. CI already builds both Windows architectures.
- Sketch: a small `ht-updater` crate owns release identities, trusted download boundaries, persisted stage metadata, file transitions and startup supervision. Desktop wires startup and first-frame health only. The downloader runs on a background thread every four hours.
- Alternatives: blind replacement cannot detect startup failure. A permanent external service adds installation and permission requirements. A short-lived renamed parent can supervise the new executable and restore its predecessor without either cost.
- State: downloaded -> verified stage -> old executable backup -> trial executable -> first presented frame -> accepted. An interrupted or unhealthy trial restores the backup. Failed release identity remains blocked until a newer release appears.
- Blocking first steps: read rules and write these gates before code.
- Independent workstreams: no delegation authorized; implementation and source review remain serial.
- Shared mutable state: one per-install file handle excludes concurrent updater writers; the same guard covers checks and swaps. Existing diagnostics serialize rotation across threads.
- Smallest safe decomposition: protocol/file logic in one crate; Windows process handoff isolated from portable tests; CI publication separate from build CI.
- Verify each unit through GitHub before advancing. Unavailable Cursor review tools are replaced by explicitly labeled self-review and existing CI.

## Web source boundary

No UI values are added. Preserve the web surfaces at `/home/valentin/projects/hypertask/src/components/PageComponents/Kanban/HeaderComponents/AppShellRail.tsx:55` (board/global rail) and `/home/valentin/projects/hypertask/src/components/AI_CHAT/AI_Chat_Layout.tsx:55` (AI panels). Timing, hashes and release identities are updater protocol choices, not copied UI values.

## Evidence

Pending exact-head GitHub CI. This branch does not merge or touch an installed laptop executable. The first updater-capable binary must already be installed before later releases can arrive through this mechanism; existing binaries without an updater cannot bootstrap themselves.
