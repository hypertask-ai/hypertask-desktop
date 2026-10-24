# HTPR-7381: sticky sync banner

## Installed-app evidence (read-only, 2026-10-24)

The owner's running process was PID 9104, `Hypertask Desktop/ht-desktop.exe`,
started at 23:20:20. The current Rust application's state directory,
`%APPDATA%\Hypertask`, contained `local.db`, `local.db-shm` and `local.db-wal`,
but no sync log. An older, unrelated `%LOCALAPPDATA%\Hypertask\log.txt` was
last modified in August; it is not evidence about this running Rust app.

Read-only SQLite access (`mode=ro&immutable=1`, with the WAL size verified as
zero) returned:

```text
mutation_queue []
refetch_queue_count 0
```

Unauthenticated `Test-NetConnection app.hypertask.ai -Port 443` returned `True`;
`Invoke-WebRequest https://app.hypertask.ai` returned `200`.
These prove public HTTPS reachability, not bearer validity or realtime health.
No account data, tokens, cookies or raw server bodies were read into output.
No application was launched, restarted or interacted with.

**The original transport error cannot be recovered.** At installed commit
`6e639bbd`, `crates/ht-desktop/src/ui.rs:291-297` consumed the error receiver,
threw away the error string, and latched the generic message into `self.error`.
`crates/ht-core/src/worker.rs:157-159` only sent errors to that receiver; it
persisted no diagnostics. Thus an isolated historical failure looks identical
to an ongoing outage. The evidence does not establish a 401, 429, server outage,
websocket failure, or regression from PRs #19/#20/#21. In particular, it cannot
attribute an HTTP error to working-set trimming.

## Approved behavior and web references

The web has no corresponding global offline banner to copy. Source in
`/home/valentin/projects/hypertask/src` shows:

- `hooks/Inbox/useGetNotifications.ts:456-469`: inbox prefetch retries (`retry: 3`).
- `hooks/realtime/useInboxRealtime.ts:40-48`: reconciliation refetches inbox
  and active agent-inbox queries.
- `hooks/realtime/useInboxRealtime.ts:65-72`: subscription catch-up and refetch
  on an established connection's reconnect.

Per the owner's approved default, keep the existing desktop text, **“Sync
unavailable — edits are saved locally”**, without adding new UI. Failed worker
operations retry independently with the existing 1/2/4/8/30/60-second backoff.
One or two failed attempts remain silent; three failed attempts for the same
outstanding operation show the banner. Successful unrelated reads, local
realtime events, idle queue ticks, and reconnect notifications are not recovery
proof. The failed operation must complete successfully (or a mutation queue
must finish processing its outstanding writes). A reconnect wakes and refetches
tracked views and retries outstanding work immediately; it does not itself
clear failures. Independent UI validation errors remain independent.

## Diagnostics

The worker now writes `local.sync.log` beside `local.db` (normally
`%APPDATA%\Hypertask\local.sync.log` on Windows). It rotates at 128 KiB,
retaining just `local.sync.log.1` as its predecessor (256 KiB total bound).
Entries contain UTC time, a fixed operation name, failure/recovery/wake events,
attempt counts and allowlisted codes: HTTP status, offline, timeout,
invalid-response, database or invalid-data. `wake` also covers user edits;
it must not be interpreted as proof of a websocket reconnect.

Arbitrary errors, payloads, SQL, identifiers, URLs, credentials and server bodies
are never formatted into this log. Rotation/write errors are nonfatal; Unix
creation mode is 0600. Read the code and operation from the next failure before
assigning a network/authentication/server cause.

## CI-only verification

`crates/ht-core/tests/worker_recovery.rs` runs a real localhost HTTP mock server
through `Http` and `Worker`: transient 503 retry/recovery, persistent 503 banner
and verified clearing, unrelated-success suppression, reconnect refetch,
mutation retry/recovery, and safe 401/429 diagnostics. Worker status supplies
the exact banner text consumed by the UI, rather than a separate test copy.
`sync_log` unit tests exercise size limits, rotation, secret omission and
nonfatal logging failure. Existing websocket coverage verifies that a real
subscriber connection wakes the worker and dispatches events.

All builds and tests run on GitHub Actions, never on the VPS or the owner's
laptop. This change was not installed on the laptop as part of this task.
