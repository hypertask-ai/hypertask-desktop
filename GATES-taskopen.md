# Task open acceptance

Owned paths: task loading in `crates/ht-desktop/src/ui.rs`, its focused native tests, and the worker request completion boundary in `crates/ht-core/src/worker.rs`. No builds, tests, lint, app launches, or laptop access on this VPS. Proof runs only in public GitHub CI.

1. An uncached task open issues `/tasks?task_id=<id>` through the real worker and mocked transport; its native pane paints the web loading text until the response arrives, then paints the fetched title and persists it in SQLite.
2. A failed task request paints the existing offline message only after that specific request fails. Another task's failure must not contaminate a pending open.
3. A response for a previously opened task caches that task without replacing the current task pane. Cached task opens render immediately.
4. Audit inbox row, board card, search, command center, and startup/deep-link entry. Every implemented entry reaches the shared task loading boundary. Do not invent an unsupported deep-link protocol.
5. Cite the web's actual loading state and task endpoint by file:line. No new UI outside that source.
6. Fetch and rebase origin/main before pushing taskopen. Draft PR, exact HEAD CI run, all six required jobs and artifact receipt must succeed. Retain named regression output from GitHub CI. Existing memory budgets remain unchanged.

Hypotheses before implementation: task open never requests data; the worker drops successful task data; or the pane treats an asynchronous cache miss as a completed network failure. Read the shared open/sync/worker/cache boundaries, then verify the surviving mechanism with native regression tests in CI.

Principle-model-the-domain: use an explicit per-open pending request, not global sync health. Principle-prove-it-works: assert painted loading/title/error and persisted cache through the real worker. Principle-sequence-verifiable-units: acceptance contract, focused regression/fix, then exact-head CI receipt.

Unavailable host tools: Cursor control-ui and cursor-team-kit deslop. Use serial source/diff self-review and the existing native GPUI CI surface. No delegated reviewers.

## Source audit

- Root cause: `ui.rs` immediately rendered its offline fallback for `detail == None` after `switch` read SQLite. `request_sync` already queued a task fetch, and `Sync::task` already cached the result. There was no task-specific pending/failure signal; the first frame falsely represented an in-flight cache miss as offline. The worker also performed `/user/context` before the task GET, so an unrelated request could delay the missing task by its timeout.
- Web loading copy: `/home/valentin/projects/hypertask/src/app/detail/[...slug]/page.tsx:158` renders `Loading...` as task detail's Suspense fallback. Desktop uses exactly that text in its existing empty detail container, without new controls or guessed styling.
- Web endpoint: `/home/valentin/projects/hypertask/src/app/api/mcp/tasks/route.ts:1` delegates GET; `/home/valentin/projects/hypertask/src/lib/mcp/operations/tasks/operation.ts:210` reads `task_id`, and `:360` returns `{success:true,tasks:...}` after accessible-task resolution. Mock responses use that same envelope.
- Inbox click: `crates/ht-desktop/src/inbox.rs:425-428` reaches `open_task`.
- Board card: `crates/ht-desktop/src/ui.rs` card click reaches `open_task`.
- Search result: `crates/ht-desktop/src/shell_navigation.rs:190-191` reaches `open_task` (table rows do too).
- Command center: `crates/ht-desktop/src/interactions.rs:1096-1097` selects `ChoiceAction::Task` through `open_task`.
- Keyboard inbox/board open and task playlist navigation also reach `open_task`; unread review directly calls `reload` and the same `request_sync` boundary.
- Startup task destination: `Options::parse` accepts `--screen task --task ID`; `Desktop::new` reloads and requests through the same boundary. There is no registered desktop URL/deep-link handler in the current native source, so OS deep-link dispatch is NOT IMPLEMENTED, not falsely certified.
- Fix: an uncached open requests the task itself before context/comments/drafts. Its per-open completion receiver survives periodic cache reloads. Pending means loading; a completed failure means the existing offline fallback. Navigating away drops that receiver, not the worker's cache write. Existing periodic retry remains active.

## CI proof

Pending exact-head GitHub CI; no local execution performed.
