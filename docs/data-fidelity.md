# HTPR-7381: native/web task data

Acceptance: creator, saved AI summary, server Draft markers, assignees, labels,
priority, due date, comment authors and counts are traced to the web's data
sources. Bearer-accessible discrepancies have synthetic regression coverage;
missing bearer fields are reported, never fabricated. Execute all Rust tests,
builds and the layout contract on GitHub, not the VPS.

## Implemented

- **Creator:** `/api/mcp/tasks?project_id=…` omits `createdBy`, while
  `?task_id=…` returns it. Board sync hydrates missing creators in batches of
  100, also repairing existing list-only caches. Detail refresh still refetches
  the authoritative task. Later list deltas preserve detail-only fields. An
  uncached/missing author is “Unknown author”, not a fictional “Hypertask”.
- **Task-page drafts:** `/api/mcp/drafts?task_id=…` is scoped by the authenticated
  user and task access. Description/comment markers reproduce the separate
  `useHasDrafts` empty-content rules, including the two empty HTML documents.
  An empty response clears old markers. Pending local mutations are not server
  drafts; markers do not publish or overwrite draft content.
- **Board Draft badge:** web `useGetUserDrafts`/`getUserDrafts` selects comment
  drafts, with a different (untrimmed) empty-content rule. Preserve that rule
  separately from task-page markers. There is no bearer bulk-user-drafts route;
  the worker progressively refreshes tracked board cards round-robin, at most
  one request per second. Badges appear as those snapshots arrive; a large
  board takes multiple minutes to sweep, rather than flooding the default
  120-request/minute user-token quota. Opening a task refreshes its markers
  immediately; active tasks refresh with the normal minute pull.
- **Priority:** MCP list rows use a string; detail uses `Priority_Value` in an
  object. Both render correctly, including during a later list delta. Board
  cards now show available priority and due date.
- **Removed fields:** full list/detail snapshots clear omitted priority and due
  date; detail also clears omitted creator, estimate and creator-agent. Partial
  write/realtime updates still merge, so a title edit does not clear a due date.
- **Assignees/tags/counts:** retain MCP's visible human/agent assignments and
  labels, with list `commentCount`/detail `totalComments` aliases. Detail separates
  human and agent assignments (rather than listing agents twice), uses real
  names/email and an honest empty-assignees placeholder. Existing array/count
  normalization is covered across list, detail and board reads.
- **Comment authors:** honor durable `agent_display_name` (including “Private
  agent”) before agent/user rows. Null or nameless agent rows cannot hide a real
  person; deleted-agent attribution is not incorrectly replaced by its owner.
- **Inbox:** timestamped, non-older nested server task titles/status/tickets
  update cached tasks before pending edits are reapplied, preventing the SQLite
  join from showing an obsolete title without regressing newer task refetches.
  Missing timestamps cannot overwrite authoritative task data. Rows now render available due date, priority, size and
  task-label metadata from the inbox API, including the web's three-label cap.
  Sender names follow the web's recent-actor grouping and null-agent fallback;
  unread counts and available comment previews are no longer dropped.

## Bearer blockers (web repository source audit)

- **Saved task AI summary:** web
  `components/PageComponents/TaskDetail/TopRow/TaskDetailTitleContainer.tsx`
  renders `Task_Summary[0].content`; the relation is loaded by
  `utils/controllers/taskDetail/load.ts` through **GET `/api/tasks/getTask`**.
  That route delegates to `taskWriteHandler`/`taskWriteRoute` in
  `lib/api/task-writes/route.ts`, authenticated by **`getSessionUser`** in
  `lib/auth/getSessionUser.ts`. It reads cookies, not the CLI MCP bearer token.
  `lib/mcp/tasks/mappers.ts` neither includes nor maps `Task_Summary`;
  `/api/mcp/tasks/context`'s `summary=true` only limits comment context, not a
  saved AI summary. `/api/v1/*` rewrites to `/api/mcp/*` (`next.config.js`), so
  it does not add another summary source. The native saved-summary renderer
  remains usable when content is present, but absent content now hides the line
  instead of inventing “Not enough content for summary”. Full parity needs a
  bearer-safe saved-summary field on the server.
- **Comment AI summaries:** the web's comment body can render `comment.summary`
  from **GET `/api/comments/getByTask`**, authenticated by **`getSessionUser`**.
  MCP `operations/comments/operation.ts` does not map that field. Native cannot
  obtain these summaries with its bearer token either.

Bearer operations above authenticate via `validateMcpRouteAuth` (alias
`validateMcpAuth` in operation files), delegating to MCP bearer auth. No session
cookie endpoint or summary-generation endpoint was added to the desktop client.

## Regression evidence

`fixtures/data-fidelity.json` is entirely synthetic; its separate `webSummary`
entry is presentation input, deliberately **not** part of a bearer task fixture.
`ht-core/tests/sync.rs` covers creator hydration/batching/cache reuse, snapshot
clears versus partial merges, draft semantics/removal/wrong-task rejection,
progressive board refresh, count/assignment/label preservation, and inbox title
refresh with pending edits. `ht-desktop/src/ui.rs` unit tests cover the two
priority shapes, creator and durable comment attribution, summary presence/absence
and inbox metadata. CI also executes the existing workspace regression tests,
three desktop builds and Linux layout-dump contract, plus Windows ARM64 build.
