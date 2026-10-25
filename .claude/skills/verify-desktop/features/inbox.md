# Inbox archive

Select an inbox notification and archive it with E. The notification leaves the
active inbox and its offline write remains durable until sync acknowledges it.

## Sub-features

- `inbox-ready` displays fixture notifications after a presented data frame.
- `inbox-archive` archives exactly the selected notification.
- `inbox-persist` retains the archive and pending write after app shutdown.

## How to get to it (user POV)

- Open Inbox from the left rail; select a notification and press E.
- The fixture smoke starts directly in Inbox with the first row selected.

## Driving it with GitHub CI

Preconditions: the exact-head CI release binaries, fixtures/demo, isolated Xvfb,
and a new temporary database. No HT_TOKEN and no production state.

- Run the CI `Native inbox keyboard archive` step. It imports the fixtures,
  launches the real app, waits for READY, captures the window and sends E.
- Proof: `DESKTOP_INPUT_ARCHIVE_VERIFIED` in the Linux log and `desktop-input`
  artifact. Inspect `before.png`, `after.png`, `app.log`, `receipt.json` and
  `result.db`. The database must contain exactly one new Archive notification
  with archivedAt and one pending inbox_archive mutation for that same ID.
- The helper terminates its own process and reopens SQLite before retaining
  the DB backup. Evidence must survive temporary-directory cleanup.

## Gotchas

- QUEUED is local optimistic success, not server acceptance.
- E inside an editor types text; the smoke starts in the inbox list context.
- A screenshot alone cannot prove a durable write. Require the database too.
- This keyboard recipe does not certify every inbox mouse/undo/reminder path.
