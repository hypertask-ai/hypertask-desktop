# HTPR-7381 repeated inbox sync failure

## Acceptance and diagnosis

`GATES-sync2.md` records acceptance checks written before implementation.
The screenshot https://screencast2.com/plC4o.png shows the global sync banner over a cached inbox.

Read-only laptop observations on 2026-10-25 identified the running installed `ht-desktop` process and `%APPDATA%\Hypertask\local.sync.log`.
The log contains:

```text
2026-10-25T09:58:14.169697500+00:00 event=failed operation=inbox code=invalid-data attempts=330
```

The log records a parse/validation failure, not an HTTP rejection, transport error or SQLite error.
The sole HTTP request in `Sync::inbox` is GET `/api/mcp/inbox/list`.
Read-only SQLite queries found 939 cached notifications, all integer IDs, no synthetic rows, no queued edits, and no invalid cached task timestamps.
No laptop process, file or UI was changed. No credentials were read from the laptop.

A separate read-only request from the VPS used the existing CLI authentication in memory, with no token output.
`/mcp/user/context` identified the same user ID, 6, as the installed database.
Both public API hosts returned the following secret-free response metadata for `/api/mcp/inbox/list`:

```text
HTTP 200 success=True
notification_count=505 id_types=['int', 'str'] synthetic_count=13
tab_count=16 data_count=16
noninteger_id_kinds=['synthetic']
bad_split_id_count=23
```

The server creates `waitingOnSynthetic: true` rows with `id: "-<taskId>"` in `src/utils/controllers/notifications/getAll.ts`.
Desktop split parsing calls `as_i64()` on those IDs and fails with `inbox split index is invalid` before committing any inbox snapshot.
Even without split metadata, `Sync::inbox` rejects those IDs through `integer(item, "id")`.
Retries cannot repair this valid response shape. The HTTP token is accepted; this failure is not in the websocket or SQLite.
The installed log does not itself contain HTTP status or the finer parser error; the status and precise mismatch are established by the separate live response and source trace.

## Design and verification plan

Keep integer notification IDs inside SQLite and existing UI/queue interfaces.
Normalize only the server's explicit synthetic negative string IDs at the inbox boundary, preserving rows, split order and membership.
Do not globally loosen integer parsing or drop synthetic rows.
Synthetic task rows are not persisted server notification IDs, so notification writes must reject them before queue insertion.

The regression exercises `Sync::inbox`, durable SQLite rows and compact/expanded split membership using synthetic data only.
Run its pre-fix rejection and fixed workspace tests on GitHub, never on the VPS.
Cursor control/deslop tools and independent reviewers are unavailable in this session; use serial self-review and the existing native CI instead.
The installed application will not be restarted or replaced, so live post-install recovery remains unverified.
