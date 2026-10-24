# Board and task editing

Open cards, navigate tasks, edit titles and comments, and move tasks using the
native controls. Accepted edits use the durable write queue.

## Sub-features

- `board-navigation` opens cards and returns to the originating selection.
- `task-title` saves or cancels title editing without changing another task.
- `task-status` chooses a column and preserves authoritative server identity.
- `task-comment` queues a comment without losing a newer draft.

## How to get to it (user POV)

- Choose Board on the rail, then a card or Enter to open Task detail.
- Click a task title or Enter on detail, type, then Enter to save or Escape to cancel.
- Use the Status property/M picker; focus the comment composer with Ctrl+M.

## Driving it with GitHub CI

Preconditions: workspace release tests on all three native platforms and Linux
layout dumps for inbox/board/task. Follow ../SKILL.md for the run doctor.

- Read `docs/p2-parity.md` for per-row GPUI input tests, including
  `escape_returns_to_same_searched_board_column_and_focused_card` and
  `inline_status_arrow_does_not_open_picker_and_project_arrow_never_moves_task`.
- Read `docs/compose-comments.md` for durable send, rejection, ownership and
  draft races. Inspect the named GPUI tests and core HTTP/queue tests in the CI
  log, not an unrelated test with a similar name.
- Proof: named input tests pass for the current SHA; `layout-dumps` contains
  populated board/task data for each of four themes. A new changed interaction
  needs its own real input/result assertion in CI before claiming it works.

## Gotchas

- Full-row blockers in p2-parity.md remain blocked despite implemented subsets.
- Fixture board/task IDs are 15/58006, not the live account's 7283/60253.
- Unit/GPUI tests are not an end-to-end external X11 click-through for this map.
- No server search, ranking, team/flag data or creation defaults may be guessed.
