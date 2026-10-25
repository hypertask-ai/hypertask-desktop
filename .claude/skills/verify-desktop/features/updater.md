# Silent Windows updates

## Sub-features

Silent checks, trusted downloads, checksum verification, supervised swaps, rollback and release publication.

## How to get to it (user POV)

Open the Windows app normally. Updates stage silently and apply on the next start. No new UI appears.

## Driving it with GitHub CI

Preconditions: exact pushed SHA, draft PR, public GitHub runners, synthetic fixtures only. No laptop access.

`CI` runs `cargo test --locked --release -p ht-updater -- --nocapture` on Windows and Linux. Require `UPDATER_MANIFEST_VERIFIED`, `UPDATER_HASH_REFUSAL_VERIFIED`, `UPDATER_VERSION_VERIFIED`, `UPDATER_SWAP_ACCEPT_VERIFIED`, `UPDATER_ROLLBACK_VERIFIED`, `UPDATER_RUNNING_EXE_HANDOFF_AND_ROLLBACK_VERIFIED`, `UPDATER_STARTUP_TIMEOUT_ROLLBACK_VERIFIED`, and Windows `UPDATER_INSTALL_LOCK_VERIFIED`.

The subprocess test copies its own test binary into a temporary install directory. That copy renames itself while running, launches its staged replacement, and checks acceptance, nonzero-start rollback, and hung-start rollback. Workspace builds separately verify the real desktop startup and presented-frame hooks compile on both Windows architectures. This does not claim a real laptop installation or actual remote update happened.

`python gates/updater-release-tests.py` exercises the production packaging function. Require `UPDATER_RELEASE_SOURCE_VERIFIED` and `UPDATER_RELEASE_ASSETS_AND_SHA256_VERIFIED`. A failed, cancelled, forked, non-main or non-push source is rejected. Missing ARM64 prevents any package output.

After merge, `Release` accepts only successful main-push `CI`, downloads both artifacts from the triggering run, and publishes a complete draft containing two executables and `SHA256SUMS`. The release tag is `v<package-version>+<CI-run-number>.<12-character-SHA>`. Never substitute an artifact from another run.

On normal online Windows startup, the updater applies an already-verified stage before opening the database. It supervises the replacement until GPUI reports a presented frame. Startup failure restores and relaunches the backup. The background worker checks the public repository API at startup and every four hours. Offline fixture runs skip updates. Failures log fixed codes through the existing rotating sync log.

## Gotchas

An installed pre-updater binary cannot update itself. This change provides the updater-capable starting point; it never replaces an installed executable from the VPS. Subsequent releases arrive exclusively through the app's updater.

Proof: the exact-head full CI receipt is required from `../SKILL.md`, including both architectures and frozen memory budgets. Release publication itself starts only after merge; no draft-branch publication is claimed.
