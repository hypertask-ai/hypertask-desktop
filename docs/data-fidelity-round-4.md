# Data fidelity round 4

Baseline `2ceb325` from `origin/main`. Sources below are relative to
`/home/valentin/projects/hypertask/src`, never screenshots or invented examples.
Native paths are repository-relative. Status concerns field/text fidelity,
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
`missing` means absent UI/data, not a made-up replacement. After status remains
pending until the exact-head hosted checks finish.

| ID | Visible value | Web source field and file:line | Native source before | Before | After |
| --- | --- | --- | --- | --- | --- |
| I01 | Inbox task title | `task.title`; `components/Common/TaskRowComponents/TaskTitle.tsx:201-209` | `db.rs:read_inbox`, Task.title / Notification.task.title | exact | pending |
| I02 | Inbox ticket | `task.ticketNumber ?? ""`; same file `:193-199` | Task.ticketNumber / Notification.task.ticketNumber | exact | pending |
| I03 | Inbox actor/group | `recentActors` when >=2, otherwise `fromAgent.displayName ?? fromUser.displayName ?? ""`; `components/notifications/NotificationRow.tsx:268-276` | `db.rs:read_inbox` source | exact | pending |
| I04 | Inbox unread count | `unreadCount > 0`, never invitations; same file `:319-323` | Notification.unreadCount, incorrectly includes invitations | wrong | pending |
| I05 | Inbox kind/content | `type`, task.status/section/dueDate, comment.text/commentId, reaction.emoji, message, staleNudgeDays; same file `:211-256,387-432`; `lib/nativeAgent/agentMessageEnvelope.ts:12-16` | `inbox.rs:inbox_row`, any comment or split enum name | wrong | pending |
| I06 | Inbox timestamp | notification.createdAt; same file `:109-127`; `utils/generateTime.ts:1-30` | InboxItem.created_at, inbox_time | exact | pending |
| I07 | Inbox due date | task.dueDate; `components/Common/TaskRowComponents/TaskTitle.tsx:34-53,173-181`; `components/Labels/DueDateLabel.tsx:24-33` | Notification.task.dueDate, short_date omits year and uses wrong format | wrong | pending |
| I08 | Inbox priority | task.priority.priority_index -> constants; `components/Modals/TaskPriority/PriorityLabelComponent.tsx:27-31,65`; `lib/constants/constants.ts:86-113` | task.priority.Priority_Value | wrong | pending |
| I09 | Inbox size | task.estimate.estimate_index -> constants; `components/Modals/TaskEstimate/EstimateLabelComponent.tsx:24-28,52`; `lib/constants/constants.ts:115-153` | task.estimate.estimate_value | wrong | pending |
| I10 | Inbox tags/overflow | task.taskLabels[].label.value, first three nonempty, +remaining; `components/Common/TaskRowComponents/TaskTitle.tsx:31-33,71-88` | inbox_metadata taskLabels | exact | pending |
| I11 | Inbox unread indicator | notification.seen; `components/notifications/NotificationRow.tsx:365-367` | InboxItem.seen | exact | pending |
| I12 | Inbox saved/blocked/reminder indicator | task.savedContent, waitingOnUserId, userId, returnedFromReminders, type; same file `:332-367` | compact rows drop these fields; unread-only blue dot | wrong | pending |
| I13 | Inbox author, created date, assignees, summary, draft, subtask count | No corresponding row UI; same file `:36-103` | Intentionally absent; actor/time above are notification values | exact | pending |
| B01 | Board card title | task.title; `components/PageComponents/Kanban/KanbanTaskComponents/KanbanTaskCard.tsx:179-181` | Task.title, board_cards | exact | pending |
| B02 | Board ticket | task.ticketNumber ?? ""; same file `:169`; `TaskTopRow.tsx:83` same directory | Task.ticketNumber | exact | pending |
| B03 | Board comment count | task.totalComments ?? task._count.comments; KanbanTaskCard `:167`; `CommentAndAssignees.tsx:14-30` same directory | Task.commentCount / totalComments | exact | pending |
| B04 | Board assignee identity | human/agent assignments, not numeric count; KanbanTaskCard `:170-171`; TaskTopRow `:134-175`; `lib/assignees.ts:40-67` | assigneeCount shown as numeric user count | placeholder | pending |
| B05 | Board tags/overflow | taskLabels label.value; TaskTagsRow `:186-209` same directory | Task.labels mapped by `lib/mcp/tasks/mappers.ts:69-75,288`; all labels displayed | wrong | pending |
| B06 | Board priority | priority.priority_index -> constants; TaskTagsRow `:159-166`; PriorityLabelComponent `:27-31`; constants `:86-113` | Task.priority string/object label | wrong | pending |
| B07 | Board size | estimate.estimate_index -> constants; TaskTagsRow `:172-178`; EstimateLabelComponent `:24-28` | Task.estimate exists; badge absent | missing | pending |
| B08 | Board due date | dueDate; TaskTagsRow `:146-154`; DueDateLabel `:24-33` | Task.dueDate, short_date | wrong | pending |
| B09 | Board Draft | user comment draft text, untrimmed empty rule; `utils/controllers/drafts/getUserDrafts.ts:4-7`; TaskTagsRow `:140-143` | Task.hasBoardDraft from `/drafts?task_id=` | exact | pending |
| B10 | Board In inbox | task._count.notifications; KanbanTaskCard `:172`; TaskTopRow `:54-58,93-114` | active Notification membership for task | exact | pending |
| B11 | Board subtask count | task.subTasks.length, only currentSetting != None; KanbanTaskCard `:173-174`; `SubtaskCount.tsx:14-22` | `sub_tasks` in task detail but compact cards drop it; currentSetting unavailable | missing | pending |
| B12 | Board author, created date, saved AI summary | Not displayed; KanbanTaskCard `:162-198`, TaskTopRow `:134-175` | Retained detail data; intentionally no author/date/summary label | exact | pending |
| B13 | Board saved/unread dot | notifications[0].seen / savedContent or _count.savedContent; TaskTopRow `:52-54,74-79` | Native has no card dot | missing | pending |
| B14 | Board avatar photo/agent artwork | UserAvatar photoURL or generated agent ID; TaskTopRow `:151-175`; `components/Common/UserAvatar.tsx:39-69` | MCP assignee has id/email/displayName/agent, no human photoURL | missing | pending |
| T01 | Task title | currentTask.title; `components/PageComponents/TaskDetail/TopRow/TaskDetailTitleContainer.tsx:29-44` | Task.title | exact | pending |
| T02 | Task description | task.description_ content; `lib/mcp/tasks/mappers.ts:21-25,243` | Task.description / RichText | exact | pending |
| T03 | Description author | task.agent.displayName ?? task.user.displayName ?? ""; `components/PageComponents/TaskDetail/CommentAndDescription/index.tsx:109-118`; mapper `:47-61,331-336` | Task.agent / createdBy; invented Unknown author/email fallback | placeholder | pending |
| T04 | Description avatar initials | Author name -> first/last grapheme; `lib/avatar.ts:26-36` | first two characters, not first/last words | wrong | pending |
| T05 | Task created date | currentTask.createdAt -> RelativeTime; `components/PageComponents/TaskDetail/CommentAndDescription/DescriptionContainer/TopRow/DescriptionTopRight.tsx:15,50`; `utils/generateTime.ts:1-30` | Task.createdAt -> short_date, omits time/year | wrong | pending |
| T06 | Task human assignees | humanAssignees displayName; `lib/assignees.ts:16-20,40-67`; `components/PageComponents/TaskDetail/AssigneesContainer.tsx:88-110` | Task.assignees without agent; email fallback; invented No assignees instead of The Assignees | wrong | pending |
| T07 | Task agents | agentAssignees displayName; same sources; No agents is web empty state | Task.assignees[].agent | exact | pending |
| T08 | Task ID | ticketNumber, no database-ID fallback; `components/PageComponents/TaskDetail/TaskInfoColumn/TaskInfo.tsx:335-355` | ticketNumber or numeric Task.id | placeholder | pending |
| T09 | Task status | currentTask.section; TaskInfo `:374-405`; mapper `:240-241` | Task.section, empty when only sectionId supplied despite available Section | wrong | pending |
| T10 | Task project | currentTask.project.title; TaskInfo `:483-520`; mapper `:244-245` | Task.boardTitle | exact | pending |
| T11 | Task due date | currentTask.dueDate, otherwise -; TaskInfo `:439-452`; DueDateLabel `:24-33` | Task.dueDate -> short_date | wrong | pending |
| T12 | Task priority | priority_.priority_index -> constants, absent No Priority; TaskInfo `:594-603`; PriorityLabelComponent `:27-31`; constants `:86-113` | Task.priority raw label | wrong | pending |
| T13 | Task size | estimate_.estimate_index -> estimate_full_value, absent -; TaskInfo `:630-638`; constants `:115-153` | estimate_value abbreviated instead of full value | wrong | pending |
| T14 | Task tags | taskLabels label.value; TaskInfo `:655-679`; mapper `:69-75,288` | Task.labels names, No tags web empty state | exact | pending |
| T15 | Task saved AI summary | Task_Summary[0].content; `components/PageComponents/TaskDetail/TopRow/TaskDetailTitleContainer.tsx:38-44`; `TaskSummary.tsx:30-54`; `utils/summaryTeaser.ts:1-17`; `lib/mcp/operations/tasks/operation.ts:283-300,340-350` | bearer ai_summary.content -> saved teaser/full Markdown | exact | pending |
| T16 | Description Draft marker | separate description draft; `hooks/General/useHasDrafts.ts:5-17,34-40`; `components/PageComponents/TaskDetail/CommentAndDescription/DescriptionContainer/index.tsx:141-151` | hasDescriptionDraft green border, no invented text | exact | pending |
| T17 | Comment draft marker | no Draft text; `components/PageComponents/TaskDetail/CommentAndDescription/CommentContainer/NewCommentComponent.tsx:448-482` | hasCommentDraft retained, no invented text | exact | pending |
| T18 | Comment body | comment.text; `components/PageComponents/TaskDetail/CommentAndDescription/CommentContainer/InnerHTMLComment.tsx`; MCP comments operation `:232-234` | Comment.text -> RichText | exact | pending |
| T19 | Comment author | agent.displayName ?? agentDisplayName ?? creator.displayName ?? ""; `components/PageComponents/TaskDetail/CommentAndDescription/CommentContainer/CommentCreatedBy.tsx:22-26`; `lib/mcp/operations/comments/operation.ts:216-251`; durable attribution below | Comment agent_display_name before agent; invented Unknown author/You Pending/email | placeholder | pending |
| T20 | Comment time | comment.createdAt -> RelativeTime; `components/PageComponents/TaskDetail/CommentAndDescription/CommentContainer/CommentOptions.tsx:145-163`; `utils/generateTime.ts:1-30` | Comment.createdAt -> short_date | wrong | pending |
| T21 | Comment summary | comment.summary; `components/PageComponents/TaskDetail/CommentAndDescription/CommentContainer/InnerHTMLComment.tsx`; `/api/comments/getByTask` | Not in bearer CommentItem, `lib/mcp/operations/comments/operation.ts:60-85` | missing | pending |
| T22 | Task subtask titles/count | currentTask.subTasks; `components/PageComponents/TaskDetail/CommentAndDescription/DescriptionContainer/DescriptionSubTasks/DescriptionSubTasks.tsx:35-45` | Task.sub_tasks available, no native list/count | missing | pending |

## Ownership and implementation shape

One Codex session, serial self-review. Own this ledger, `db.rs`, display-only
helpers/renderers in `ui.rs`, `inbox.rs`, `comment_actions.rs`, and native/core
regressions. No new endpoint, worker state, compatibility layer or generation.
Reuse JSON snapshots; project compact metadata only when rendering needs it.
Keep missing UI rows separate from wrong values. Do not silently expand scope
into full board settings, subtask interactions or avatar artwork support.

Sequence-verifiable-units separates this acceptance commit, regressions,
implementation, and hosted evidence. Prove-it-works requires actual named CI
output and the strict final-SHA receipt, not a local test or compile assertion.
Unavailable cursor-team-kit steps use serial source/diff review per host rules.

## Blockers and evidence

Pending source reconciliation, fixes, and hosted execution. No local checks run.
