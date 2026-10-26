# HTPR-7381: native/web task data

Acceptance: creator, saved AI summary, server Draft markers, assignees, labels,
priority, due date, comment authors and counts are traced to the web's data
sources. Bearer-accessible discrepancies have synthetic regression coverage;
missing bearer fields are reported, never fabricated. Execute all Rust tests,
builds and the layout contract on GitHub, not the VPS.

## Round 2 acceptance and source audit (data3)

Sources are relative to `/home/valentin/projects/hypertask/src`.
After-status describes the acceptance target until exact-head CI verifies it.

| Field | Web source file:line | Native status before | Native status after |
| --- | --- | --- | --- |
| Author (`createdBy`, optional `agent`) | `lib/mcp/tasks/mappers.ts:47-61,331-336`; `components/PageComponents/TaskDetail/CommentAndDescription/index.tsx:109-118` | Existing hydration and task author rendering; old creator-only caches skip summary hydration. | Upgrade those caches once; preserve real human/agent authors through sync, reopen and native task navigation. |
| Board identity | `components/PageComponents/Kanban/KanbanTaskComponents/KanbanTaskCard.tsx:162-198`; `TaskTopRow.tsx:134-175` in that directory | Cards retain creators; visible avatars are assignees. | Same web placement; no invented card author or summary label. |
| Saved AI summary | `lib/mcp/operations/tasks/operation.ts:283-300,340-350`; `components/PageComponents/TaskDetail/TopRow/TaskDetailTitleContainer.tsx:38-44`; `TaskSummary.tsx:17-54` in that directory | Stored summary supported; existing creator-only caches miss it; an empty teaser hides nonempty saved content. | Hydrate missing summary fields; show/expand nonempty saved content even when its teaser is empty; clear absent/null summaries. |
| Board Draft | `utils/controllers/drafts/getUserDrafts.ts:4-7`; `components/PageComponents/Kanban/KanbanTaskComponents/TaskTagsRow.tsx:140-143` | Synced comment-draft badge supported. | Native regression proves appearance and removal from synced snapshots, including the untrimmed board rule. |
| Task Draft | `hooks/General/useHasDrafts.ts:5-17,34-40` | Separate description/comment flags supported. | Native regression proves both markers clear from empty snapshots; description-only drafts never mark cards. |

Acceptance checks, written before implementation:

1. **Cache upgrade.** Seed creator-only cached tasks, sync MCP list/detail and
   drafts, close/reopen SQLite, and assert board/task creator, saved summary and
   draft flags match the synthetic server snapshots. A second board sync must
   not rehydrate known null summaries.
2. **Native positive rendering.** From synced data, measure the real human and
   agent author, saved-summary toggle/full body, and task/board Draft markers.
   No mutation or generation request may be made.
3. **Native removal and placement.** Sync empty drafts and null/missing summaries,
   redraw, and assert markers/summary controls disappear without altering the
   published description or author. Description-only and whitespace comment
   drafts must follow the web's distinct card/task rules.
4. **Nonempty summary.** Sync a nonempty Markdown summary with an empty teaser;
   the summary toggle and I-key expansion must remain available, as on web.
5. **Public CI only.** Require every exact-head CI job and the repository receipt,
   retaining native/layout/memory artifacts. No VPS tests/builds or laptop access.

Self-review stays serial; this change is one coupled sync/render contract.
Owned paths are this ledger, `crates/ht-core/src/sync.rs`, its existing sync
regressions, and native fidelity/render/shortcut files. Existing creator,
summary and draft implementations are retained, not replaced. The red-before run was
not executed: this host forbids tests, and full hosted CI is the verification
surface. The new regressions were committed before the production fix.

## Implemented

- **Saved task AI summary:** consume the `ai_summary.content` / `updated_at`
  detail field shipped by hypertask PR #1996. A null or missing full-detail
  summary clears old saved content; partial writes and list deltas preserve it.
  Presence follows stored content, not the shortened teaser; even a nonempty
  heading with an empty teaser remains expandable. No generation request or
  cookie endpoint is used. The field is server-flagged
  for user bearer tokens; disabled flags, agent tokens and management tokens do
  not expose it. The task hides an absent summary.
- **Creator:** `/api/mcp/tasks?project_id=…` omits `createdBy`, while
  `?task_id=…` returns it. Board sync hydrates missing creators in batches of
  100, also repairing existing list-only and creator-only caches. Detail refresh still refetches
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

The current local web checkout includes the bearer summary implementation at
`lib/mcp/operations/tasks/operation.ts:283-300,340-350`, which selects the newest
stored summary and maps the optional bearer-safe `ai_summary`. The user-token
feature gate remains authoritative; disabled or absent data is never fabricated.

`fixtures/task-fidelity.json` supplies actual MCP-shaped human/agent/absent
creators, present/null/missing saved summaries and server description/comment
drafts. The new core regression opens an existing list-only database, syncs,
closes/reopens, and checks board/task equality and durable values. The native
GPUI regression starts from that same sync contract and measures the rendered
author, full-summary overlay and web-styled Draft markers. Existing sync tests
also cover empty snapshots, null/missing summary removal and partial updates.
CI run links and exact final SHA are recorded in the PR evidence.

## Fidelity acceptance (backlog item 4 recheck)

Written before changing implementation. Git baseline: `origin/main` at
`8c6402b`; prior data3/mem2 work already stores creators, saved summaries and
separate board/description/comment draft flags. Recheck rather than duplicate.

1. Trace each requested field from the current web component and API mapper to
   SQLite and native rendering. Record already-correct rows, including the web
   card's assignee-only identity (no invented creator UI).
2. Match author text and placement, saved-summary teaser/expanded placement,
   and both task Draft markers against current web styles. Close only proven
   gaps, preserving existing response fields and hydration paths.
3. Every corrected behavior gets a native GPUI regression driven by mocked
   MCP task/draft responses; assert visible text, geometry and style tokens.
   Keep existing positive/removal and durable-reopen regressions.
4. Leave `task_open_tests.rs` and worker `LoadTask` unchanged. No VPS builds,
   tests, lint or test copies; no laptop commands.
5. Rebase before push; require exact-head public GitHub CI jobs and the full
   repository receipt. Retain named-test output, final SHA and CI/PR URLs.

Current-source gaps found: task Draft **text** was invented. The web description
uses a four-pixel green left border (`DescriptionContainer/index.tsx:141-151`);
its top row has only author/actions (`TopRow/DescriptionTopRow.tsx:22-41`). The
comment composer has no Draft text (`CommentContainer/NewCommentComponent.tsx:448-482`).
Keep the existing server flags, replace description text with that border,
and remove the invented composer label. Board Draft already matches.

Saved-summary transport already matches. Presentation gaps: teaser removed
unpaired Markdown markers, lacked medium weight/single-line 65% cap, and the
expanded panel used inset card padding instead of the full title-container
width. Sources: `utils/summaryTeaser.ts:1-17`, `TopRow/TaskSummary.tsx:30-54`,
`styles/globals.scss:68-70` (all relative to the web `src`). Native regressions
are committed first; no red-before execution on this forbidden test host.
