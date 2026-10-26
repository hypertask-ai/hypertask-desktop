# Data fidelity round 4

Baseline `2ceb325` from `origin/main`. Sources below are relative to
`/home/valentin/projects/hypertask/src`, never screenshots or invented examples.
Native paths are repository-relative under `crates/ht-core/src` or
`crates/ht-desktop/src`, as appropriate. Status concerns field/text fidelity,
not complete visual or interaction parity. Fixture people remain synthetic.

## Acceptance checks, before implementation

1. Inventory every displayed data value across inbox rows, board cards and task
   detail. Record the web field, its native read path, and the baseline status.
2. Fix wrong text/field selection and invented placeholders using existing
   SQLite snapshots and bearer routes. Keep absent values absent. Preserve
   web-authored empty-state labels only where the web displays them.
3. Native GPUI regressions must render MCP-shaped task, comment, notification
   and draft responses through Sync/Db, including absent identities, explicit
   empty names, conflicting names, different years, size indices and label caps.
4. Keep saved AI summaries and draft removal coverage. No AI sidebar, keyboard,
   command center, workflow, auth, laptop or local-build changes.
5. Public GitHub CI must execute named native tests, workspace tests, Linux,
   Windows, macOS, ARM64 and both frozen Windows memory scenarios at the final
   SHA. Require the repository's strict receipt and retained proof artifacts.

## Field inventory

`exact` includes intentionally absent values where web has no such UI.
`missing` means absent UI/data, not a made-up replacement. After status describes
implemented behavior. Runtime evidence belongs in the draft PR's exact-head
receipt, not an inference from this source audit.

54 inventory rows. Exact rows increase from 21 to 50. The baseline contains
21 wrong rows, five placeholders and seven missing rows. After implementation,
all wrong/placeholder rows are exact; four missing rows stay explicit below.
The three newly populated rows are board size, board saved/unread indicator,
and task subtask links. These are row counts, not claims of full app parity.

| ID | Visible value | Web source field and file:line | Native source before | Before | After |
| --- | --- | --- | --- | --- | --- |
| I01 | Inbox task title | `task.title`; `components/Common/TaskRowComponents/TaskTitle.tsx:201-209` | `db.rs:read_inbox`, Task.title / Notification.task.title; now suppressed on invitations/non-task events | wrong | exact |
| I02 | Inbox ticket | `task.ticketNumber ?? ""`; same file `:193-199` | Task.ticketNumber / Notification.task.ticketNumber; now suppressed on invitations/non-task events | wrong | exact |
| I03 | Inbox actor/group | `recentActors` when >=2, otherwise `fromAgent.displayName ?? fromUser.displayName ?? ""`; `components/notifications/NotificationRow.tsx:268-276` | `db.rs:read_inbox` source; full snapshots now clear obsolete grouped/agent actors | wrong | exact |
| I04 | Inbox unread count | `unreadCount > 0`, never invitations; same file `:319-323` | Notification.unreadCount, incorrectly includes invitations | wrong | exact |
| I05 | Inbox kind/content | `type`, task.status/section/dueDate, comment.text/commentId, reaction.emoji, message, staleNudgeDays; same file `:211-256,387-432`; `lib/nativeAgent/agentMessageEnvelope.ts:12-16` | `inbox.rs:inbox_row`, any comment or split enum name | wrong | exact |
| I06 | Inbox timestamp | notification.createdAt; same file `:109-127`; `utils/generateTime.ts:1-30` | InboxItem.created_at, inbox_time | exact | exact |
| I07 | Inbox due date | task.dueDate; `components/Common/TaskRowComponents/TaskTitle.tsx:34-53,173-181`; `components/Labels/DueDateLabel.tsx:24-33` | Notification.task.dueDate, short_date omits year and uses wrong format | wrong | exact |
| I08 | Inbox priority | task.priority.priority_index -> constants; `components/Modals/TaskPriority/PriorityLabelComponent.tsx:27-31,65`; `lib/constants/constants.ts:86-113` | task.priority.Priority_Value | wrong | exact |
| I09 | Inbox size | task.estimate.estimate_index -> constants; `components/Modals/TaskEstimate/EstimateLabelComponent.tsx:24-28,52`; `lib/constants/constants.ts:115-153` | task.estimate.estimate_value | wrong | exact |
| I10 | Inbox tags/overflow | task.taskLabels[].label.value, first three nonempty, +remaining; `components/Common/TaskRowComponents/TaskTitle.tsx:31-33,71-88` | inbox_metadata taskLabels | exact | exact |
| I11 | Inbox unread indicator | notification.seen; `components/notifications/NotificationRow.tsx:365-367` | InboxItem.seen | exact | exact |
| I12 | Inbox saved/blocked/reminder indicator | task.savedContent, waitingOnUserId, userId, returnedFromReminders, type; same file `:332-367` | compact rows drop these fields; unread-only blue dot | wrong | exact |
| I13 | Inbox author, created date, assignees, summary, draft, subtask count | No corresponding row UI; same file `:36-103` | Intentionally absent; actor/time above are notification values | exact | exact |
| B01 | Board card title | task.title; `components/PageComponents/Kanban/KanbanTaskComponents/KanbanTaskCard.tsx:179-181` | Task.title, board_cards | exact | exact |
| B02 | Board ticket | task.ticketNumber ?? ""; same file `:169`; `TaskTopRow.tsx:83` same directory | Task.ticketNumber | exact | exact |
| B03 | Board comment count | task.totalComments ?? task._count.comments; KanbanTaskCard `:167`; `CommentAndAssignees.tsx:14-30` same directory | Task.commentCount / totalComments | exact | exact |
| B04 | Board assignee identity | human/agent assignments, not numeric count; KanbanTaskCard `:170-171`; TaskTopRow `:134-175`; `lib/assignees.ts:40-67` | assigneeCount shown as numeric user count | placeholder | exact |
| B05 | Board tags/overflow | taskLabels label.value; TaskTagsRow `:186-209` same directory | Task.labels mapped by `lib/mcp/tasks/mappers.ts:69-75,288`; all labels displayed | wrong | exact |
| B06 | Board priority | priority.priority_index -> constants; TaskTagsRow `:159-166`; PriorityLabelComponent `:27-31`; constants `:86-113` | Task.priority string/object label | wrong | exact |
| B07 | Board size | estimate.estimate_index -> constants; TaskTagsRow `:172-178`; EstimateLabelComponent `:24-28` | Task.estimate exists; badge absent | missing | exact |
| B08 | Board due date | dueDate; TaskTagsRow `:146-154`; DueDateLabel `:24-33` | Task.dueDate, short_date | wrong | exact |
| B09 | Board Draft | user comment draft text, untrimmed empty rule; `utils/controllers/drafts/getUserDrafts.ts:4-7`; TaskTagsRow `:140-143` | Task.hasBoardDraft from `/drafts?task_id=` | exact | exact |
| B10 | Board In inbox | task._count.notifications; KanbanTaskCard `:172`; TaskTopRow `:54-58,93-114` | active Notification membership for task | exact | exact |
| B11 | Board subtask count | task.subTasks.length, only currentSetting != None; KanbanTaskCard `:173-174`; `SubtaskCount.tsx:14-22` | `sub_tasks` in task detail but compact cards drop it; currentSetting unavailable | missing | missing |
| B12 | Board author, created date, saved AI summary | Not displayed; KanbanTaskCard `:162-198`, TaskTopRow `:134-175` | Retained detail data; intentionally no author/date/summary label | exact | exact |
| B13 | Board saved/unread dot | notifications[0].seen / savedContent or _count.savedContent; TaskTopRow `:52-54,74-79` | Native has no card dot | missing | exact |
| B14 | Board avatar photo/agent artwork | UserAvatar photoURL or generated agent ID; TaskTopRow `:151-175`; `components/Common/UserAvatar.tsx:39-69` | MCP assignee has id/email/displayName/agent, no human photoURL | missing | missing |
| T01 | Task title | currentTask.title; `components/PageComponents/TaskDetail/TopRow/TaskDetailTitleContainer.tsx:29-44` | Task.title | exact | exact |
| T02 | Task description | task.description_ content; `lib/mcp/tasks/mappers.ts:21-25,243` | Task.description / RichText | exact | exact |
| T03 | Description author | task.agent.displayName ?? task.user.displayName ?? ""; `components/PageComponents/TaskDetail/CommentAndDescription/index.tsx:109-118`; mapper `:47-61,331-336` | Task.agent / createdBy; invented Unknown author/email fallback | placeholder | exact |
| T04 | Description avatar initials | Author name -> first/last grapheme; `lib/avatar.ts:26-36` | first two characters, not first/last words | wrong | exact |
| T05 | Task created date | currentTask.createdAt -> RelativeTime; `components/PageComponents/TaskDetail/CommentAndDescription/DescriptionContainer/TopRow/DescriptionTopRight.tsx:15,50`; `utils/generateTime.ts:1-30` | Task.createdAt -> short_date, omits time/year | wrong | exact |
| T06 | Task human assignees | humanAssignees displayName; `lib/assignees.ts:16-20,40-67`; `components/PageComponents/TaskDetail/AssigneesContainer.tsx:88-110` | Task.assignees without agent; email fallback; invented No assignees instead of The Assignees | wrong | exact |
| T07 | Task agents | agentAssignees displayName; same sources; No agents is web empty state | Task.assignees[].agent; blank names incorrectly used empty-group text | wrong | exact |
| T08 | Task ID | ticketNumber, no database-ID fallback; `components/PageComponents/TaskDetail/TaskInfoColumn/TaskInfo.tsx:288-308` | ticketNumber or numeric Task.id | placeholder | exact |
| T09 | Task status | currentTask.section; TaskInfo `:335-362`; mapper `:240-241` | Task.section, exactly the web field, no section-ID fallback | exact | exact |
| T10 | Task project | currentTask.project.title; TaskInfo `:525-549`; mapper `:244-245` | Task.boardTitle | exact | exact |
| T11 | Task due date | currentTask.dueDate, otherwise -; TaskInfo `:439-452`; DueDateLabel `:24-33` | Task.dueDate -> short_date | wrong | exact |
| T12 | Task priority | priority_.priority_index -> constants, absent No Priority; TaskInfo `:594-603`; PriorityLabelComponent `:27-31`; constants `:86-113` | Task.priority raw label | wrong | exact |
| T13 | Task size | estimate_.estimate_index -> estimate_full_value, absent -; TaskInfo `:630-638`; constants `:115-153` | estimate_value abbreviated instead of full value | wrong | exact |
| T14 | Task tags | taskLabels label.value; TaskInfo `:655-679`; mapper `:69-75,288` | Task.labels names; blank label incorrectly used No tags instead of blank | wrong | exact |
| T15 | Task saved AI summary | Task_Summary[0].content; `components/PageComponents/TaskDetail/TopRow/TaskDetailTitleContainer.tsx:38-44`; `TaskSummary.tsx:30-54`; `utils/summaryTeaser.ts:1-17`; `lib/mcp/operations/tasks/operation.ts:283-300,340-350` | bearer ai_summary.content -> saved teaser/full Markdown | exact | exact |
| T16 | Description Draft marker | separate description draft; `hooks/General/useHasDrafts.ts:5-17,34-40`; `components/PageComponents/TaskDetail/CommentAndDescription/DescriptionContainer/index.tsx:141-151` | hasDescriptionDraft green border, no invented text | exact | exact |
| T17 | Comment draft marker | no Draft text; `components/PageComponents/TaskDetail/CommentAndDescription/CommentContainer/NewCommentComponent.tsx:448-482` | hasCommentDraft retained, no invented text | exact | exact |
| T18 | Comment body | comment.text; `components/PageComponents/TaskDetail/CommentAndDescription/CommentContainer/InnerHTMLComment.tsx:44-51`; MCP comments operation `:232-234` | Comment.text -> RichText | exact | exact |
| T19 | Comment author | agent.displayName ?? agentDisplayName ?? creator.displayName ?? ""; `components/PageComponents/TaskDetail/CommentAndDescription/CommentContainer/CommentCreatedBy.tsx:22-26`; `lib/mcp/operations/comments/operation.ts:216-251`; durable attribution below | Comment agent_display_name before agent; invented Unknown author/You Pending/email | placeholder | exact |
| T20 | Comment time | comment.createdAt -> RelativeTime; `components/PageComponents/TaskDetail/CommentAndDescription/CommentContainer/CommentOptions.tsx:145-163`; `utils/generateTime.ts:1-30` | Comment.createdAt -> short_date | wrong | exact |
| T21 | Comment summary | comment.summary; `components/PageComponents/TaskDetail/CommentAndDescription/CommentContainer/InnerHTMLComment.tsx:51,117`; `/api/comments/getByTask` | Not in bearer CommentItem, `lib/mcp/operations/comments/operation.ts:60-85` | missing | missing |
| T22 | Task subtask titles/count | currentTask.subTasks; `components/PageComponents/TaskDetail/CommentAndDescription/DescriptionContainer/DescriptionSubTasks/DescriptionSubTasks.tsx:35-45` | Task.sub_tasks available; now actual linked child titles/tickets, never an invented numeric count | missing | exact |

| T23 | Comment reaction emoji/count | reactions grouped by emoji and actor; `components/PageComponents/TaskDetail/CommentAndDescription/CommentContainer/CommentReactions.tsx:159-174`; `lib/mcp/comments/reactionResponse.ts:60-93` | Comment.reactions[].emoji, rendered group length | exact | exact |
| T24 | Reaction actor tooltip | users[].displayName/current user; `components/PageComponents/TaskDetail/CommentAndDescription/CommentContainer/reactorsLabel.ts:1-43`; `EmojiComponent.tsx:41-43` same directory | agent/user displayName or invented numeric userId | placeholder | exact |
| T25 | Timer Start/Stop state | runningEntry and enabled; `components/PageComponents/TaskDetail/TaskInfoColumn/TaskTime.tsx:166-174` | server receipt timer-state timer_running/timer_enabled; keyboard-owned code unchanged | exact | exact |
| T26 | Empty description prompt | web's explicit empty-state copy; `components/PageComponents/TaskDetail/CommentAndDescription/DescriptionContainer/InnerHtmlDescription.tsx:33-34` | Same Add Description and keyboard hint, never substitute task body | exact | exact |
| T27 | Description photo | task.agent.photoURL ?? task.user.photoURL; `components/PageComponents/TaskDetail/CommentAndDescription/index.tsx:110-111`; `components/Common/UserAvatar.tsx:39-69` | Only name/initials available; bearer mapper omits photoURL | missing | missing |

## Ownership and implementation shape

One Codex session, serial self-review. Own this ledger, `db.rs`, `sync.rs`, display-only
helpers/renderers in `ui.rs`, `inbox.rs`, `comment_actions.rs`, and native/core
regressions. No new endpoint, worker state, compatibility layer or generation.
Reuse JSON snapshots; project compact metadata only when rendering needs it.
Keep missing UI rows separate from wrong values. Do not silently expand scope
into full board settings or avatar artwork support. Existing task navigation
opens the real linked child; no new subtask editing controls.

Sequence-verifiable-units separates this acceptance commit, regressions,
implementation, and hosted evidence. Prove-it-works requires actual named CI
output and the strict final-SHA receipt, not a local test or compile assertion.
Unavailable cursor-team-kit steps use serial source/diff review per host rules.

## Blockers and evidence

### Remaining data boundaries

- **B11, board subtask count visibility.** Detail `/api/mcp/tasks?task_id=`
  supplies `sub_tasks` (`lib/mcp/tasks/mappers.ts:218-232,255`), not the
  active board view's `board_subtask_setting`. Web hides the count for None
  (`components/PageComponents/Kanban/KanbanTaskComponents/SubtaskCount.tsx:14-22`).
  Native board loading fetches projects/sections/tasks, not an active View.
  `/api/mcp/projects` omits that setting
  (`lib/mcp/operations/projects/operation.ts:163-204`). The bearer route
  **already exists** at `/api/mcp/view?projectId=ID&include_settings=true`
  (`lib/mcp/operations/view/operation.ts:94-105,112-126`) and
  `/api/mcp/view/{viewId}` (`lib/mcp/operations/view/[viewId]/operation.ts:57-71`).
  Missing native field is the selected View ID/`board_subtask_setting`, not a
  nonexistent route. Choosing a guessed View would show counts the web hides.
- **B14/T27, avatar photos/artwork.** Task human mappings omit `photoURL`
  (`lib/mcp/tasks/mappers.ts:28-44,47-61`); agent summaries also omit photos
  (`lib/mcp/agents.ts:8-11,38-44`). Web selects photos or generated agent
  artwork (`components/Common/UserAvatar.tsx:39-69`). Native now shows real
  human initials and real agent name chips, not a numeric people placeholder.
  Exact avatar photo fidelity needs those omitted response fields. Generated
  agent artwork remains a presentation gap, not a missing server field.
- **T21, comment summaries.** `summary` is not in bearer `CommentItem`
  (`lib/mcp/operations/comments/operation.ts:60-85,226-251`). Web's
  `/api/comments/getByTask` requires `getSessionUser`
  (`pages/api/comments/getByTask.ts:1,21-27`). Missing bearer field is
  `comments[].summary`. No cookies or generation requests were introduced.
- **T22, child completion icons.** Child titles and tickets are available and
  now shown. The mapper's `sub_tasks` omits child `status`
  (`lib/mcp/tasks/mappers.ts:218-232`), while web's check icon uses it
  (`components/PageComponents/TaskDetail/CommentAndDescription/DescriptionContainer/DescriptionSubTasks/DescriptionSubTasks.tsx:64-71`).
  No invented child completion state or task-page numeric count.

### Verification ledger

- Acceptance first, commit `2b2746b`; native regressions next, `fd52fdc`.
- Run https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40396170922
  rejected a closing delimiter in the description timestamp chain. Simplified
  that chain; no thresholds changed. No new fidelity runtime test ran.
- Native regressions exercise `data4_task_fields_use_web_names_indices_and_relative_dates`,
  `data4_board_shows_real_assignees_size_and_three_tags`,
  `data4_inbox_renders_web_kinds_actors_indicators_and_metadata` and
  `data4_web_constants_and_grapheme_initials_reject_raw_label_fallbacks`.
  Reaction label cases use `data4_reaction_names_use_web_labels_not_database_ids`;
  the native task regression also measures the actual grouped reaction count.
  Core regression `data4_inbox_full_snapshot_clears_old_actor_and_event_context`
  covers snapshot removal. Existing saved-summary/draft tests stay intact.
- Run https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40398285917
  rejected a tooltip call after `layout_key` erased the element type. Moved
  capture to the chain end, keeping the real tooltip and click handler.
- Run https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40399206180
  passed all six new regressions and ARM64, but rejected three older native
  contracts. Restored task-backed inbox titles without using message text as a
  task title. Replaced the old count-only assignee fixture with actual people,
  and used long real tag values rather than impossible long priority names.
  All original geometry/overflow assertions remain unchanged.
- Exact-head CI jobs, named-test output and the strict receipt are recorded at
  https://github.com/hypertask-ai/hypertask-desktop/pull/57 before completion.
  No VPS builds, tests, lints, app launches, test copies, laptop commands or
  workflow changes.

### Native read and display paths

- `crates/ht-core/src/sync.rs:150-173` stores detail reads; `:259-329` applies
  authoritative inbox snapshots without stale optional event/actor data.
- `crates/ht-core/src/db.rs:342-345,380-439` projects visible board/inbox
  metadata while retaining complete task/notification snapshots on disk.
- `crates/ht-desktop/src/ui.rs:1529-1624,2089-2186,2220-2228` chooses the real
  property fields, dates, initials, constants and author names.
- `crates/ht-desktop/src/inbox.rs:93-146` chooses the web's event copy and
  notification indicators, never enum-name fallback text.
- `crates/ht-desktop/src/comment_actions.rs:65-80,503-740` groups real reaction
  identities/counts and renders comment author/time from their source fields.


