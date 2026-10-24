# HTPR-7381: native/web task data

Acceptance: creator, saved AI summary, server Draft markers, assignees, labels,
priority, due date, comment authors and counts are traced to the web's data
sources. Bearer-accessible discrepancies have synthetic regression coverage;
missing bearer fields are reported, never fabricated. Execute all Rust tests,
builds and the layout contract on GitHub, not the VPS.

## Implemented

- **Saved task AI summary:** consume the `ai_summary.content` / `updated_at`
  detail field shipped by hypertask PR #1996. A null or missing full-detail
  summary clears old saved content; partial writes and list deltas preserve it.
  No generation request or cookie endpoint is used. The field is server-flagged
  for user bearer tokens; disabled flags, agent tokens and management tokens do
  not expose it. The task hides an absent summary.
- **Creator:** `/api/mcp/tasks?project_id=…` omits `createdBy`, while
  `?task_id=…` returns it. Board sync hydrates missing creators in batches of
  100, also repairing existing list-only caches. Detail refresh still refetches
  the authoritative task. Later list deltas preserve detail-only fields. The description prefers the visible creator agent (including `createdBy.agent`),
  not that agent's human owner. An uncached/missing author is “Unknown author”, not a fictional “Hypertask”.
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

## Backlog 4 source contract (mem2)

Paths below are relative to `/home/valentin/projects/hypertask/src`:

| Data row | Web evidence | Native contract |
| --- | --- | --- |
| Real author | `components/PageComponents/TaskDetail/CommentAndDescription/index.tsx:100-108`; `lib/mcp/tasks/mappers.ts:46-60` | Agent before human; detail hydration repairs old list-only SQLite caches and retains identity in board cards/task details. |
| Board card identity | `components/PageComponents/Kanban/KanbanTaskComponents/KanbanTaskCard.tsx:162-198`; `TaskTopRow.tsx:134-175` in the same directory | Web card avatars are **assignees**, not authors. Do not invent an author label; retain the creator in the board data used to open a task. |
| AI summary | `components/PageComponents/TaskDetail/TopRow/TaskDetailTitleContainer.tsx:38-44`; `TaskSummary.tsx:17-54`; `utils/summaryTeaser.ts:1-17` | Show the stored teaser and full Markdown, never synthesize missing content. |
| Board Draft | `components/PageComponents/Kanban/KanbanTaskComponents/TaskTagsRow.tsx:112-115`; `utils/controllers/drafts/getUserDrafts.ts:5-6`; `../tailwind.config.ts:36` | Nonempty comment draft; untrimmed board rule; 11 px / 22 px / medium / `#C2CFA5`. |
| Task Draft | `hooks/General/useHasDrafts.ts:5-17,34-40` | Separate description/comment emptiness rules; clear markers on empty server snapshots; keep published content unchanged. |

The local web checkout predates PR #1996. Its merged source is independently
cited at `src/lib/mcp/operations/tasks/operation.ts:282-299,339-361` in commit
`2741d663d4b2358078aa4b20155e0dc70999c6a7` (hypertask PR #1996), which selects
the newest stored summary and maps the optional bearer-safe `ai_summary`.

`fixtures/task-fidelity.json` supplies actual MCP-shaped human/agent/absent
creators, present/null/missing saved summaries and server description/comment
drafts. The new core regression opens an existing list-only database, syncs,
closes/reopens, and checks board/task equality and durable values. The native
GPUI regression starts from that same sync contract and measures the rendered
author, full-summary overlay and web-styled Draft markers. Existing sync tests
also cover empty snapshots, null/missing summary removal and partial updates.
CI run links and exact final SHA are recorded in the PR evidence.
