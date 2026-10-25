# AI and rich comments

Open the AI sidebar, preserve drafts across navigation, and use native comment
and formatting controls without pretending unauthorized server features work.

## Sub-features

- `ai-presentation` opens, pins, resizes and closes the existing conversation.
- `ai-errors` preserves real refusals and unfinished-stream errors.
- `comments-drafts` retains the correct task/user draft through failed sends.
- `editor-format` edits supported HTML and rejects known lossy saves.

## How to get to it (user POV)

- Toggle AI from the rail or its documented keyboard shortcuts; Ctrl+Q focuses it.
- Open a task, focus a comment and use its ellipsis menu.
- Focus a rich editor, type slash or select text to use formatting controls.

## Driving it with GitHub CI

Preconditions: current-head workspace tests and synthetic fixture state. No
provider keys, copied cookies, owner-laptop state or live SaaS writes.

- Use the named tests in `docs/ai-parity.md`, `docs/compose-comments.md` and
  `docs/editor-formatting.md`; inspect GPUI real key/pointer assertions, queue
  restart/ownership tests and exact HTTP/SSE request fixtures together.
- Proof: the relevant named tests pass in the run log. `ai-live-request` contains
  Rust-generated request fixtures, not proof that a live account accepted them.
- Treat every route/auth blocker in those ledgers as a separate unmet outcome.
  A new feature must add CI native input evidence rather than using a setter.

## Gotchas

- The older AI receipt helper searches only branch aiside. Use the branch-neutral
  desktop receipt; inspect feature-specific tests in the chosen run log.
- HTTP mock success is not live authorization. New server flags may be per user.
- No local-only history, fake model answer or changed bearer audience substitutes
  for the web's durable authenticated contract.
- This initial map does not claim an external native AI click-through smoke.
