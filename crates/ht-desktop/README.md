# Hypertask native desktop

GPUI only: no browser, webview, or remote reads in the rendering path. Inbox and board lists are virtualized. The four visual themes and IBM Plex Sans, Inter, and Newsreader fonts are embedded.

```sh
source ~/.cargo/env
nice cargo build --release -j 8 -p ht-desktop -p ht-core
./target/release/ht-core-cli --db /tmp/hypertask.db import-fixtures fixtures/demo
./target/release/ht-desktop --db /tmp/hypertask.db --offline --screen board --board 15 --theme porcelain
```

`--db` defaults to the OS data directory's `Hypertask/local.db`. `--screen` accepts `inbox`, `board`, and `task`; `--theme` accepts `porcelain`, `graphite`, `amoled`, and `dia`. `--task 58006` selects the fixture with comments. The window is 1440×900 at (0,0).

Without `--offline`, a nonempty `HT_TOKEN` starts the ht-core background worker. Rendering and optimistic edits still use SQLite exclusively. Tokens are never logged. No live writes are part of the verification suite.

## Input

- Rail buttons switch Inbox / Board. Click a notification or card, or press Enter, to open its task. Esc returns.
- J/K navigate notifications, cards, or comments; Tab / Shift+Tab cycle inbox splits or board columns.
- E archives the selected inbox notification.
- Shift+L / Shift+H move the selected card to the next / previous column.
- Click a task title, or press Enter on its detail, to edit; Enter saves and Esc cancels.
- Ctrl+M focuses the plain-text comment composer. Ctrl+Enter or Send queues the comment. Ctrl+D scrolls to the description.
- M or the Status property opens the task's column picker.

The copied footer also shows the web app's broader shortcut vocabulary; assignment, priority, task size, tags, command palette, task creation, sharing, reminders, task archiving, and task completion are outside this pilot's four-write contract and are not implemented.

## Presentation and verification

`--hold-after-ready` emits and flushes `READY` only after GPUI records a presented data frame. `--exit-after-ready` then exits. `--bench-input N` dispatches actual `PlatformInput::KeyDown` events through GPUI and records one presentation-latency histogram sample per input; it does not time handler calls or layout alone.

```sh
node crates/ht-desktop/verify.mjs edits
node crates/ht-desktop/verify.mjs shots
node crates/ht-desktop/verify.mjs performance
```

Each command uses its own automatically allocated Xvfb display and a temporary fixture database. `shots` uses the same 300ms capture-settle interval as the root pixel gate and asserts nonblank captures and exact window geometry, saves twelve PNGs, and reports masked odiff percentages. `edits` sends real X11 keyboard events and checks SQLite rows and all four queue kinds, including cancelled title edits. `performance` imports 2,000 synthetic tasks using the same core CLI as the root gates, runs 300 navigation inputs, measures peak RSS with `/usr/bin/time`, runs 15 hyperfine samples, saves the results, and fails when a budget is exceeded.

The verification screenshots show SQLite fixture content from `fixtures/demo` (board 15, tasks 58006 / 64897) rather than hardcoding a reference task.

## Measured handoff

The release passes its unit tests, formatting, all four native queued-edit checks, exact-window/nonblank capture checks, and all twelve masked pixel comparisons (1.04–3.54%, against a 5% limit).

Performance is **not accepted** on this shared Xvfb/lavapipe host. On 2,092 tasks, 300 genuine inputs, and 15 hyperfine samples:

| Measurement | Observed | Required |
|---|---:|---:|
| Cold-start median | 1282.3 ms | <300 ms |
| Input-to-frame p95 | 299.893 ms | <16 ms |
| Peak RSS | 199.22 MB | <100 MB |

Even an empty-inbox control with no tasks took 1058.8 ms and peaked at 147.68 MB. This points to a substantial renderer/platform baseline; no budget has been relaxed or substituted with a handler-only timer.
