# ht-core

UI-free, blocking sync primitives plus a dedicated `ht-sync` worker. No Tokio or UI dependencies.

```sh
source ~/.cargo/env
nice cargo run -j 8 -p ht-schema-gen -- /path/to/schema.prisma
node gates/schema-fresh.mjs
node gates/core.mjs
nice cargo test -j 8 -p ht-core --release --all-features

HT_DB=hypertask.sqlite3 HT_TOKEN=... nice cargo run -j 8 -p ht-core --bin ht-core-cli -- sync --board 15
HT_DB=hypertask.sqlite3 nice cargo run -j 8 -p ht-core --bin ht-core-cli -- write task_title 58006 'New title'
HT_DB=hypertask.sqlite3 nice cargo run -j 8 -p ht-core --bin ht-core-cli -- write task_section 58006 4309
HT_DB=hypertask.sqlite3 nice cargo run -j 8 -p ht-core --bin ht-core-cli -- write task_comment 58006 'A comment'
HT_DB=hypertask.sqlite3 nice cargo run -j 8 -p ht-core --bin ht-core-cli -- write inbox_archive 322068
HT_DB=hypertask.sqlite3 nice cargo run -j 8 -p ht-core --bin ht-core-cli -- dump
```

The CLI also accepts `--db path`. Without `HT_TOKEN`, writes remain queued offline. With it, `write` attempts one drain on a background thread. `sync` drains ready mutations, pulls projects, board and inbox, and services targeted refetches. A transient failure leaves its durable retry deadline intact.

## UI boundary

Open `Db` on the UI side; read `inbox`, `board` and `task_detail` only from SQLite. `enqueue(Write, unix_seconds)` atomically changes the local projection and appends a mutation. `mutations()` exposes pending/inflight/failed rows and their errors. Start `Worker::start(path, Http::new(token)?)` with the same file, then use its cloneable `handle`:

- `sync_board(id)` registers a foreground board and pulls its sections and delta.
- `sync_task(id)` registers the most recent task detail and loads all comment pages.
- `sync_inbox()` registers and pulls the inbox.
- `wake()` drains after a local write and refreshes registered views on foreground/connectivity restoration.
- `event(frame)` applies a Pusher frame and runs targeted refetches.

The worker drains retries every second and refreshes registered views every 60 seconds until dropped. Drop stops and joins it; HTTP requests have a 30-second timeout. Receive background failures through `errors()`.

## Contracts

- Generated model names and scalar types come from Prisma: `Task`, `Project`, `Section`, `Comment`, `Notification`, `Assignees`, `Label`, `TaskLabel`. DDL permits sparse API projections; relation fields are not fabricated. MCP assignees are user summaries, not `Assignees` join rows, so their original nested data remains in task `raw_json`. Labels are also stored as `Label` rows.
- Section `ranking` is a lexicographic string. Columns sort by ranking; Normal cards sort by `updatedAt` descending, then id ascending. There is no local invented task rank.
- Task details use the observed `tasks` array (the singular `task` shape is also accepted). `totalComments` and list `commentCount` are normalized without discarding nested detail fields. Notification JSON is preserved, including source users/agents, nested task, unread state, and timestamps.
- Delta state is per `(board,status)`. A completed empty initial Normal pull is recorded, so later Archive/Deleted pulls are not skipped forever. Pages use limit 100 and URL-encoded cursors with a one-second overlap; only observed timestamps advance cursors. An incomplete status scan applies neither its batch nor its cursor.
- Last write wins; the server is authoritative. Returned server rows replace resolved optimistic values. Still-pending later mutations are replayed over the server projection. Targeted refetches reconcile failed writes, cached idempotent responses, and archive side effects.
- Drain is globally serial in insertion order. A backoff on the head blocks later mutations, including on the same entity. Failed rows remain visible but no longer retry. A persisted 120-second inflight lease excludes concurrent drainers and recovers after a crash; the client UUID survives every retry.
- Every write sends both `X-Correlation-ID` and `Idempotency-Key`. The current server implements task-update/comment idempotency; archive is naturally idempotent. Comment creation sends the server-supported `kind: "qa"` so an agent bearer token cannot silently route the user's comment into an agent activity log. Temporary negative comment ids are replaced atomically on acknowledgement; failed comment text remains in its failed queue row.

## Optional websocket subscriber

Enable `realtime-ws` and call `realtime::ws::Subscriber::spawn(url, private_channels, worker.handle(), authorize)`. Supply the public Pusher websocket URL including `/app/<key>?protocol=7`, and a callback that obtains the private-channel signature for `(socket_id, channel_name)`. The subscriber authenticates each subscription, answers Pusher ping and websocket ping, dispatches events, and reconnects with bounded backoff. Drop stops and joins it.

The server's `/api/pusher/auth` uses browser-session cookies, **not MCP bearer authentication**. `SessionAuth { endpoint, cookie }.authorize(...)` is available for callers with a session; alternatively supply an external session authorizer. A bearer-only caller uses delta polling. No cookie, token, or channel signature is logged or persisted by the core. Realtime emits invalidations for board/task/inbox and often comments; these queue targeted HTTP pulls. A full comment payload can be applied immediately, then reconciled by a comment snapshot.

Verification uses committed synthetic read fixtures (`fixtures/demo`) and local simulated servers only. It does not touch any live board.
