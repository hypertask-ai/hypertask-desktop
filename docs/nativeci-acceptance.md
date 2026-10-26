# Native interaction CI acceptance

## Checks written before implementation

1. Migrate `inbox-archive-notification`, `board-rename-f2`, `task-description`, and `comment-send-stay` from `gates/behavior-cases.json`. Drive the release executable on hosted Windows, with a fresh synthetic fixture database per case. No owner credentials, laptop transport, or VPS execution.
2. Each case must observe its visible result and its exact durable SQLite mutation. Retain input actions, before/after screenshots, measured elements, database backups, revision, executable SHA-256, and owned-process/database cleanup outcomes.
3. Break the four production write effects deliberately on the hosted runner. The unchanged cases must each reject their missing visible effect, not startup or input errors. Restore source and the release executable, then require all four cases to pass again. Retain the patch and rejection evidence.
4. Keep all existing jobs, assertions, budgets, workloads, and artifacts. Require native interaction evidence in the exact-head receipt. All six required CI jobs must pass for the final SHA.
5. Add `docs/coverage.md`. Separate unit/GPUI, native OS, and live-server coverage for inbox, board, task, AI sidebar, and keyboard shortcuts. Unmigrated and authorization-blocked paths remain explicitly unverified.

## Work order

- Read the web contracts and existing input/evidence boundaries.
- Implement one bounded hosted Windows suite.
- Prove the unbroken suite on GitHub before adding rejection controls.
- Add source mutations, restore checks, and repeated green proof.
- Inspect exact-head jobs and retained artifacts before reporting.

## Throughput checkpoint

- Blocking first steps: acceptance checks and web citations precede code.
- Independent workstreams: other helpers own product behavior. This branch owns the new native gates, coverage ledger, and focused CI integration.
- Shared mutable state: one case owns one temporary database and process. Existing memory jobs retain their own state.
- Smallest safe decomposition: one worker avoids shared workflow edits. No delegation.

## Design and review

The case table is the data shape. Each case maps a native key sequence to a visible element and a durable write. Existing read-only behavior snapshots supply measured elements, not state setters. Hosted Win32 input exercises the shipped application. Self-review replaces unavailable Cursor-only review tools. Prove It Works requires actual retained output. Sequence Verifiable Units requires baseline green before rejection controls.

## Evidence

- Baseline `97d0f04c16d7b80c7ab7165a00246e7b8c3acd0e` passed all six required jobs. Hosted run: https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40238507075.
- Downloaded `windows-input` receipts showed four passed cases, exact pending writes, graceful exits, and removed temporary databases.
- Screenshot review caught pre-presentation capture. The follow-up waits two further frame boundaries and flushes the desktop compositor. Every final screenshot requires direct inspection.
- CI enforces deliberate source suppressions and restored proof. Record final SHA, run URL, per-case markers, and job conclusions on https://github.com/hypertask-ai/hypertask-desktop/pull/31. Require the current-head receipt before acceptance.

A first rejection run suppressed writes correctly, but its 30-second absence wait produced a corrupted board capture. The gate now observes completed native input plus two frames before checking synchronous effects. It keeps the full acknowledgement timeout and rejects empty captures. Negative SQLite queues must stay empty.

Premise under review: desktop BitBlt copying reliably observes the owned GPUI window. Two hosted runs produced blank/corrupted desktop copies despite acknowledged state and fresh frames, including before any input. Capture now uses the existing Windows memory gate's PrintWindow flags and retains simultaneous desktop copies plus both color counts. Native captures still must be nonblank and change after the action. Final artifacts must establish whether the methods disagree.
