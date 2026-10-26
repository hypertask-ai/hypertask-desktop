# Command center round 2 audit

Web root: `/home/valentin/projects/hypertask/src`.
Every web citation below is relative to that root.
Historical inventory: `docs/cmdk-parity.md`.
That table records pre-fix support, not current capabilities.
No creation or quick-entry changes. HTPR-7699 remains.
No authorization changes, cookies, laptop access, or VPS execution.

## Shape and decision

Existing `Command` rows dispatch existing native actions.
Existing board and task queues retain authenticated delivery.
No parallel command-specific transport or replacement modal framework.
Alternative rejected: bulk archiving locally cached Inbox rows.
It loses server rules, preview versions, and confirmation.
Alternative rejected: treating every MCP route as equivalent.
The exact read-model fields matter too.

## Newly available commands

| ID | Command / web behavior | Handler and route evidence | Native reuse / limits |
| --- | --- | --- | --- |
| U1 | `blockedByPerson`, **Blocked by person…**, Task, Shift+B | `components/Modals/commands/HTC/taskCommands.ts:51-56`; panel `components/commandModalPanels2.tsx:273-289`; picker `components/Modals/BlockedByPerson/BlockedByPerson.tsx:53-69,81-98`; POST `/api/tasks/waiting-on`. Bearer export `app/api/mcp/tasks/waiting-on/route.ts:1-3`; auth and flag `lib/mcp/operations/tasks/waiting-on/operation.ts:22-38,46-62`. | Existing person-only picker and `BoardWrite::WaitingOn`. Nobody clears. Server retains flag enforcement. Cached people only; no invented agents. |
| U2 | `starTask`, **Star task / Unstar task**, Task, Alt+S | Registry `components/Modals/commands/HTC/taskCommands.ts:418-423`; dispatcher `components/commandDispatcher.ts:790-792`; web state `hooks/Task Detail/useTaskDetailGlobalStates.ts:610-624`; `/api/savedContent/toggleSaved`, `pages/api/savedContent/toggleSaved.ts:17-20,67-83`. Bearer `/api/mcp/tasks/card-actions`, `app/api/mcp/tasks/card-actions/route.ts:1-4`; `lib/mcp/tasks/cardActionHandler.ts:40-91,126-175`. | Existing `BoardWrite::StarTask`, explicit desired state. Labels require known `savedContent`, not guessed false. Real saved IDs reconciled. |
| U3 | `toggleTimeTracking`, **Start or stop timer**, Time, W | Registry `components/Modals/commands/HTC/navigationCommands.ts:187-201,240`; dispatcher `components/commandDispatcher.ts:946-948`; handler `hooks/MultiPages/HTC/useHTCTaskAndComments.ts:276-295`; GET `/api/time/task`, POST `/api/time/start` or `/api/time/stop`. Bearer exports `app/api/mcp/time/status/route.ts:1-3`, `app/api/mcp/time/start/route.ts:1-3`, `app/api/mcp/time/stop/route.ts:1-3`. | Existing authoritative status-then-effect operation. Requires known enabled project or timer summary. Unknown omitted. Never automatically replay unknown effects. Project-list omission remains `lib/mcp/operations/projects/operation.ts:22-40`; native keyboard W can hydrate status. |
| U4 | `declineTask`, **Decline task**, Task | Registry `components/Modals/commands/HTC/taskCommands.ts:323-326,395-402`; dispatcher `components/commandDispatcher.ts:376-378` calls the archive handler. Handler `hooks/MultiPages/HTC/useHTCTaskAndComments.ts:238-252`; status write `/api/tasks/(un)archive`, `lib/constants/APIRouteConstants.ts:8`. Bearer status field `lib/mcp/tasks/fields/types.ts:48-64`. | Existing task Archive operation, never notification archive. Omitted for archived tasks. |
| U5 | `showInInbox`, **Move task to inbox**, Task | Registry `components/Modals/commands/HTC/taskCommands.ts:511-515`; dispatcher `components/commandDispatcher.ts:769-771`; handler `hooks/MultiPages/HTC/useHTCTaskAndComments.ts:198-220`; POST `/api/notifications/moveTaskToInbox`, session wrapper `lib/api/notification-writes/move-task-to-inbox.ts:32-40`. Equivalent bearer card action `lib/mcp/tasks/cardActionHandler.ts:92-109,126-175`; same state utility `lib/taskCardActions/inboxState.ts`. | Existing board queue and card-action transport, `action=inbox, active=true`. Account/project checks preserved. Refresh supplies real notification identity; no fabricated local notification. Repeated action remains desired-state, not toggle. |
| U6 | `acceptTask`, **Accept task**, Task | Registry `components/Modals/commands/HTC/taskCommands.ts:317-320,395-402`; dispatcher `components/commandDispatcher.ts:373-375`; handler `components/boardCommandActions.ts:256-308`; selection `utils/triageDestination.ts:6-38`, intake names `lib/mcp/boards/columnRole.ts:13-38`. Web PUT `/api/tasks/moveTask`. Bearer `/api/mcp/tasks/move`, `lib/mcp/operations/tasks/move/operation.ts:228-242,273-284`; same-board branch `utils/controllers/tasks/moveToDifferentBoard.ts:470-492`. | Existing `BoardWrite::MoveTask`. Ranked complete cached sections, including hidden destinations. Skip later intake columns only from intake. No later destination reports the web error. Archived tasks omitted. |
| U7 | `removeParenttask`, **Remove as sub-task**, Task | Registry `components/Modals/commands/HTC/taskCommands.ts:503-509`; dispatcher `components/commandDispatcher.ts:922-924`; handler `hooks/MultiPages/HTC/useHTCTaskAndComments.ts:260-268`; `/api/tasks/removeParent`, `lib/constants/APIRouteConstants.ts:31`. Bearer nullable `parent_task_id`, `lib/mcp/tasks/fields/types.ts:70`; applied via `/api/tasks/single`, `lib/mcp/tasks/fields/text.ts:15-35`. Read alias `parent_id`, `lib/mcp/tasks/mappers.ts:246-254`. | Existing task patch queue, explicit null parent. Normalize bearer parent identity. Row disappears after unlink. No guessed parent. |

| U8 | `branchInNewChat`, **Branch in new chat**, Comment; **Ask AI about this**, AI | Registry `components/Modals/commands/HTC/taskCommands.ts:262-267`, `components/Modals/commands/HTC/AllCommands.ts:460-465`; dispatcher `components/commandDispatcher.ts:710-712`; actual handler `app/detail/[...slug]/useTaskDetailCommandActions.tsx:97-120` creates a fresh session then sets an attributed quote. | Existing native new-chat plus comment-quote operations. Both labels require focused comment/creator. No prompt automatically sent. |
| U9 | `summarizeTicket`, **Summarize ticket**, Task | Registry `components/Modals/commands/HTC/taskCommands.ts:159-164,297-303,556-561`; dispatcher `components/commandDispatcher.ts:701-703`; exact prompt and busy guard `hooks/MultiPages/AIChat/useCommentToAiChat.ts:117-129`. | Existing native chat sender, same current-task context. Not cached-summary expansion or task creation. |
| U10 | `summarizeComment`, **Summarize comment**, Comment | Registry `components/Modals/commands/HTC/taskCommands.ts:195-200`; dispatcher `components/commandDispatcher.ts:716-718`; quote/prompt and busy guard `hooks/MultiPages/AIChat/useCommentToAiChat.ts:93-114`. | Existing native quote and chat sender. Current chat retained; old draft replaced by attributed quote plus exact prompt. Busy/history-loading guard prevents accidental queueing. |

Both summaries use POST `/api/ai/chat/stream`.
Native request tests capture the authenticated request.
A synthetic HTTP refusal proves transport, not server acceptance.

Task order follows `taskCommands.ts:395-549`.
Time follows Task, before Navigate, `AllCommands.ts:1364-1372`.
Board-name insertion stays unchanged, `commands.tsx:308-315`.
Labels and context originate from the registry, not shortcuts.

## Exact remaining contracts

“Blocked” means missing an equivalent authorized contract.
“Native gap” means no such server claim.
“Excluded” means outside the requested surfaces.
These distinctions deliberately prevent false authorization blockers.

| ID | Exact missing route or field / source evidence | Bearer comparison and disposition |
| --- | --- | --- |
| B1 | POST `/api/notifications/inbox-zero/preview` and `/execute`, with rules and `previewVersion`. Presets `components/useCommandsState.ts:112-120`; sheet handoff `components/commandDispatcher.ts:522-525`, `components/commandModalPanels2.tsx:581-586`; request `components/notifications/InboxZeroSheet.tsx:97-112`, `hooks/Inbox/useInboxZeroActions.ts:53-78`. Session guards `app/api/notifications/inbox-zero/preview/route.ts:15-18`, `app/api/notifications/inbox-zero/execute/route.ts:62-65`. | `/api/mcp/inbox/archive` accepts selected IDs, not rules/version; `lib/mcp/operations/inbox/archive/operation.ts:53-75,239-275`. Missing authoritative preview/confirmation, not an archive primitive. All three remain blocked. |
| B2 | GET `/api/notifications/markAsUnseen`, explicit false or task toggle. `components/commandDispatcher.ts:517-519`; `components/generalCommandActions.ts:42-58`; `lib/api/notification-writes/mark-unseen.ts:13-16,35-42,58-75`. | MCP `/inbox/seen` only sets true, `app/api/mcp/inbox/seen/route.ts:79-85`. Missing false/toggle mutation. |
| B3 | POST `/api/queues/inboxReminder`, task identity plus `remindTask` and reminder option. `components/commandDispatcher.ts:884-886`; `hooks/MultiPages/HTC/useHTCTaskAndComments.ts:271-274`; `app/detail/[...slug]/useTaskDetailCommandActions.tsx:56`; `components/Modals/RemindMe/RemindMeComponent.tsx:86-111`; `hooks/Inbox/useGlobalFocusHandler.tsx:87-99`; session `pages/api/queues/inboxReminder.ts:27-31`. | MCP `/inbox/archive` needs existing active notification IDs and supports a duration reminder, `lib/mcp/operations/inbox/archive/operation.ts:53-75,96-127,181-207`. No arbitrary-task `remindTask`/reminder-option contract. Cached Inbox reminder is not Set reminder parity. |
| B4 | POST `/api/tasks/searchAll`, recent mode and all-project search, then `/api/tasks/addRelations`. Mode types `components/useCommandsState.ts:121-141`; panel `components/commandModalPanels2.tsx:105-121`; picker `components/Modals/TaskRelationPicker/TaskRelationPicker.tsx:26-61`; search `components/Modals/commands/searchTasks.tsx:35-76`; routes `lib/api/task-writes/search-all.ts:9-23,43-57`, `hooks/Task Detail/useTaskRelations.ts:24-39`, `pages/api/tasks/addRelations.ts:1-5`; session guard `lib/api/task-writes/route.ts:31-38`. | MCP relation writes exist, `lib/mcp/operations/tasks/relations/operation.ts:47-65,80-132`, but missing the authorized recent-task catalog. MCP search fields `lib/mcp/operations/tasks/search/operation.ts:30-64` do not include recent-work semantics. Duplicate additionally missing from allowed relation enum, `lib/mcp/tasks/relationType.ts:1-17`. Person-blocked is not task dependency. |
| B5 | POST `/api/tasks/setStartDate`, `{taskId,startDate}`, `pages/api/tasks/setStartDate.ts:1-5`, `lib/api/task-writes/start-date.ts:10-28`; POST `/api/tasks/setRecurrence`, `pages/api/tasks/setRecurrence.ts:1-5`, `lib/api/task-writes/recurrence.ts:19-32`; panel `components/commandModalPanels2.tsx:160-190`; handler `components/boardCommandActions.ts:26-35`. | Session-only wrapper `lib/api/task-writes/route.ts:31-38`. MCP update lacks `startDate` and `recurrence`, `lib/mcp/tasks/fields/types.ts:50-75`. Due date is not either field. |
| B6 | GET/POST `/api/task-templates`, `components/useCommandsState.ts:327`, `components/boardCommandActions.ts:39-57`; session guards `app/api/task-templates/route.ts:20-24,72-76`. | Missing authorized template catalog/save. Creation from template also HTPR-7699. No invented template UI. |
| B7 | POST `/api/follower/createFollower`, `/api/follower/unFollowTask`, plus follower-row ID lookup. Handler `hooks/MultiPages/HTC/useHTCTaskAndComments.ts`; dispatcher `components/commandDispatcher.ts:358-363`; guards `pages/api/follower/createFollower.ts:9-25`, `pages/api/follower/unFollowTask.ts:10-29`, `pages/api/follower/getFollower.ts:10-18,30-40`. | MCP task followers are user summaries, `lib/mcp/tasks/mappers.ts:282-286`, not mutation row IDs. Update fields omit followers, `lib/mcp/tasks/fields/types.ts:50-75`. |
| B8 | GET/POST/PUT `/api/share/createShareLink`; session guard `pages/api/share/createShareLink.ts:12-21`; web handler `hooks/MultiPages/HTC/useHTCTaskAndComments.ts:130-141`, dispatcher `components/commandDispatcher.ts:775-780`. | Missing authorized share ID/type/live-expiry state. Private copy cannot replace public sharing. |
| B9 | POST `/api/tasks/create`, GET/POST `/api/tasks/createGlobally`; exact form/default/rank contract remains HTPR-7699. Dispatcher `components/commandDispatcher.ts:640-648,668-670,695-697,757-759`; web duplicate read `hooks/MultiPages/HTC/useHTCTaskAndComments.ts:121-127`; task linker `components/Modals/SubtaskLinkingModal/SubtaskLinking.tsx:46-57,95-109,122-135`. | Existing bearer append/AI writer cannot promise full creation or duplicate source/defaults. Exact web create auth `lib/api/task-writes/create.ts:17-50`; defaults/save `lib/api/task-writes/create-global.ts:48-61,297-304,394`. Explicitly skipped. |
| B10 | Personal view routes `/api/projects/views/unsaved-view`, `/update-view`, `/switch-view`, `/create-view`, `/delete-rename-view`. Constants `lib/constants/APIRouteConstants.ts:33-38`; writes `hooks/Homepage/Views/useKanbanViews.ts:195-215,257-267,279-356,387-429,436-458,490-527`; hide column `components/generalCommandActions.ts:229-255`; subtask settings `hooks/SubTask/useSubTask.ts:13-35`. | Missing personal unsaved view `baseViewId`, active layout/filter/columns/subtask-setting read/write. Named MCP view CRUD/apply is not that user preference. Compare `app/api/mcp/view/[viewId]/apply/route.ts:1-3`. |
| B11 | Staleness, auto-archive, column auto-assign, status report routes. Exact calls `components/boardCommandActions.ts:72-75,96-102,127-133,182-187,217-223`; GET/personal sorting uses B10. Session guards `app/api/projects/staleness/route.ts:6-10`, `app/api/projects/auto-archive/route.ts:8-12`. | Missing equivalent project `staleness`/`autoArchive`/auto-assignment configuration and report fields. Task move/assignee primitives do not represent these policies. |
| B12 | Manage columns combines personal view visibility (B10) with edits. Catalog POST `/api/section/getProjectSections`, `utils/api/global/apiHelpers/getAllManageColumnsAPI.ts:9`; tags POST/DELETE `/api/labels/updateLabel`, `utils/api/global/apiHelpers/updateLabelsAPI.ts:3-20`. | Existing native add/rename/delete column rows remain. Full manager visibility uses B10. MCP tag route exports GET/POST only, `app/api/mcp/projects/[projectId]/labels/route.ts:1-4`; no equivalent tag update/delete. |
| B13 | Board join token/reset and destructive delete contract. Registry `components/Modals/commands/HTC/AllCommands.ts:378-388,415-419`; dispatcher panel `components/commandDispatcher.ts:290-296`; token UI modes pass through modal panels. | Missing board invitation `inviteLink`/`key` read/reset contract; reset POST `/api/invite/generatePublicInvite`, `components/Modals/commands/inviteMember.tsx:335-356` and board delete operation; MCP board PATCH exports only PATCH, `app/api/mcp/projects/[projectId]/route.ts:1-3`; archive is not delete. |
| B14 | GET `/api/users/preferences` `settings.snippets`, `lib/snippets.ts:58-69,128`; snippet writes `hooks/General/useSnippetManager.ts:22-35`; dispatcher `components/commandDispatcher.ts:651-666`. | Missing authorized snippets catalog; session guards `app/api/users/preferences/route.ts:65-85,125-142`. MCP profile only displayName/photoURL, `lib/mcp/operations/user/profile/operation.ts:76-84`. |
| B15 | Web Time modal GET/POST/PATCH/DELETE `/api/time/entries`, `components/Modals/TimeLog/TimeLogModal.tsx:105-129,143-165,173-195`; GET `{canManage,entries}` `app/api/time/entries/route.ts:40-58`; session `app/api/time/_lib.ts:11-20`. Board enabled write `/api/projects/time-tracking`, `components/generalCommandActions.ts:125-140`, `app/api/projects/time-tracking/route.ts:11-16`. | MCP log/update/delete/report exist, but full all-member `canManage` modal contract missing. MCP report is entries only, `lib/mcp/operations/time/report/operation.ts:68-84`. Timer toggle U3 does not imply time-log/report UI. Report destinations outside this task remain X. |
| B16 | Local persisted show-history boolean; full feed combines comment activities and agent runs. `components/commandDispatcher.ts:390-393`; `hooks/Task Detail/useTaskDetailGlobalStates.ts:207-223`; feed `lib/agentRuns/taskActivityFeed.ts:30-64`; session comments `/api/comments/getByTask`, `pages/api/comments/getByTask.ts:21-27`. | No route needed for boolean. Missing complete feed fields including approvals/retry/lease/step state, not a fake auth requirement on the toggle. MCP `include_activity` and agent-activity are partial projections; see source-backed R11 in `docs/keyboard-parity.md`. Omitted until full native feed exists. |
| B17 | Web version read `{current.contentText,hasMore,versions[].contentText,createdAt,actor}`, `components/Modals/TaskDescriptionHistory/TaskDescriptionHistoryModal.tsx:19-30,60-69,89-99`; route constants `lib/constants/APIRouteConstants.ts:21-24`. | Bearer version read/restore exists, but read returns markdown `content`, IDs, note/date only, `lib/mcp/operations/tasks/description-versions/operation.ts:53-76`. Missing current content, actor display names, `hasMore`, and exact plain-text preview field. Restore route exists, `lib/mcp/operations/tasks/description-restore/operation.ts:15-20,27-35`; do not call restore an auth blocker. |
| B18 | Suggest reply POST `/api/ai/suggest-reply`, `components/commandDispatcher.ts:925-932`, `components/RTE/useTaskDetailEditorEvents.tsx:166-194`, session `app/api/ai/suggest-reply/route.ts:160-170`. Speech POST `/api/ai/audio-transcript`, `app/api/ai/audio-transcript/route.ts:102-104`. | Missing authorized generation/audio contract; microphone capture absent too. MCP improve transforms supplied text, not task reply generation. |
| B19 | Flag-gated selection rows `components/Modals/commands/HTC/navigationCommands.ts:301-322`; dispatcher `components/commandDispatcher.ts:55-65`. Smart split conditional registry `components/Modals/commands/HTC/AllCommands.ts:306-319`. | Missing authoritative account flag in native palette context. Selection capability alone does not enable flags. No server mutation needed for selection. Smart split also B10. |
| B20 | Whole heterogeneous undo history; `components/commandDispatcher.ts:267-273`; `hooks/General/useUndo.tsx:194-220`; `/api/notifications/inbox-zero/undo` and saved toggle, `utils/undoActions/helperFuncs.ts:21-68`. | Missing whole action-history inverse contract. MCP inbox unarchive is only one inverse. Native text undo remains intact. |
| N1 | Native editor/flag gap, not blanket API refusal. `components/commandDispatcher.ts:698-700`; callback `app/detail/[...slug]/useTaskDetailCommentActions.tsx:172-174`; exact dispatcher `hooks/Task Detail/useTaskDetailGlobalStates.ts:448-476` needs focused editor identity and `editWithAiEnabled`/`commentInlineAi` choices. | Native Compose is a creation draft, not this focused-description/comment writer. Missing authoritative editor/flag context, not an unavailable generic AI route. No substitute improvement action advertised. |
| N2 | Native gap. Web links feed GET `/api/urls/fetchUrls`, `components/Modals/LinksModal/index.tsx:83`, session `pages/api/urls/fetchUrls.ts:10-24`; actual view-subtasks callback `hooks/MultiPages/HTC/useHTCTaskAndComments.ts:223-225`. Remove-child picker needs complete child objects, `components/Modals/SubtaskLinkingModal/RemoveSubtask.tsx:15-18,28-49`. | Parent unlink U7 available. Child detach primitive also available, but complete native child-picker/read-model absent. MCP `sub_tasks` projection `lib/mcp/tasks/mappers.ts:218-232,255` is not a hydrated native child task cache. No missing write authorization claimed. |
| N3 | DELETE `/api/ai-chat/delete-session?delete=ALL`, `components/generalCommandActions.ts:109-119`, dispatcher `components/commandDispatcher.ts:1031-1034`. Session/compat authentication `app/api/ai-chat/delete-session/route.ts:29-46`. | Missing equivalent atomic ALL scope. MCP item delete requires a UUID, `lib/mcp/ai/sessionHandler.ts:191-200`. Enumerating cached sessions cannot guarantee ALL. Individual bearer session deletion is available, not blocked. |
| X | Explicit scope exclusion; declaration and dispatcher cited per row below. Destinations include account/team/billing, calendar/table, timer reports, agents/Agent Chat, archives/trash/drafts/favorites, developer setup, onboarding, and mobile-only browser actions. | No missing API inferred. These are not blocked in-scope commands. No new out-of-scope UI. |

## Every historical unresolved row

Repeated declarations retain separate rows.
B/N codes resolve to exact contracts above.
X rows retain their handler citations too.

| Command | Current declaration | Web handler / mode handoff | Result |
| --- | --- | --- | --- |
| `reloadApp` | `components/Modals/commands/HTC/AllCommands.ts:14` | `components/commandDispatcher.ts:274` | X |
| `clearInboxToZero` | `components/Modals/commands/HTC/AllCommands.ts:64` | `components/commandDispatcher.ts:522` | B1 |
| `archiveAllReadNotifications` | `components/Modals/commands/HTC/AllCommands.ts:70` | `components/commandDispatcher.ts:523` | B1 |
| `archiveReactionNotifications` | `components/Modals/commands/HTC/AllCommands.ts:76` | `components/commandDispatcher.ts:524` | B1 |
| `createTeam` | `components/Modals/commands/HTC/AllCommands.ts:88` | `components/commandModalPanels2.tsx:296` | X |
| `teamSettings` | `components/Modals/commands/HTC/AllCommands.ts:94` | `components/commandDispatcher.ts:498` | X |
| `ManageTeamMembers` | `components/Modals/commands/HTC/AllCommands.ts:101` | `components/commandDispatcher.ts:495` | X |
| `billing` | `components/Modals/commands/HTC/AllCommands.ts:108` | `components/commandDispatcher.ts:507` | X |
| `manageSubscriptions` | `components/Modals/commands/HTC/AllCommands.ts:115` | `components/commandDispatcher.ts:504` | X |
| `manageTeamAIAPIKeys` | `components/Modals/commands/HTC/AllCommands.ts:122` | `components/commandDispatcher.ts:510` | X |
| `toggleBoardZoom` | `components/Modals/commands/HTC/AllCommands.ts:137` | `components/commandDispatcher.ts:277` | X |
| `createTask` | `components/Modals/commands/HTC/AllCommands.ts:147` | `components/commandDispatcher.ts:640` | B9 |
| `newTaskFromTemplate` | `components/Modals/commands/HTC/AllCommands.ts:161` | `components/commandModalPanels2.tsx:191` | B6 |
| `generateStatusUpdate` | `components/Modals/commands/HTC/AllCommands.ts:168` | `components/commandDispatcher.ts:637` | B11 |
| `sortBoard` | `components/Modals/commands/HTC/AllCommands.ts:181` | `components/commandModalPanels2.tsx:418` | B10 |
| `toggleFilterValueMatch` | `components/Modals/commands/HTC/AllCommands.ts:190` | `components/commandDispatcher.ts:330` | B10 |
| `toggleStaleness` | `components/Modals/commands/HTC/AllCommands.ts:200` | `components/commandDispatcher.ts:949` | B11 |
| `toggleStalenessView` | `components/Modals/commands/HTC/AllCommands.ts:208` | `components/commandDispatcher.ts:952` | B11 |
| `toggleAutoArchive` | `components/Modals/commands/HTC/AllCommands.ts:216` | `components/commandDispatcher.ts:955` | B11 |
| `boardVelocityReport` | `components/Modals/commands/HTC/AllCommands.ts:224` | `components/commandDispatcher.ts:581` | B11 |
| `sortByTimeInColumn` | `components/Modals/commands/HTC/AllCommands.ts:231` | `components/commandDispatcher.ts:958` | B11 |
| `sortByLastComment` | `components/Modals/commands/HTC/AllCommands.ts:237` | `components/commandDispatcher.ts:961` | B11 |
| `autoAssignColumn` | `components/Modals/commands/HTC/AllCommands.ts:247` | `components/commandDispatcher.ts:288, components/commandModalPanels2.tsx:289` | B11 |
| `manageColumns` | `components/Modals/commands/HTC/AllCommands.ts:255` | `components/commandModalPanels2.tsx:350` | B12 |
| `hideColumn` | `components/Modals/commands/HTC/AllCommands.ts:267` | `components/commandDispatcher.ts:409` | B10 |
| `goToViews` | `components/Modals/commands/HTC/AllCommands.ts:297` | `components/commandModalPanels2.tsx:494` | B10 |
| `manageViews` | `components/Modals/commands/HTC/AllCommands.ts:304` | `components/commandModalPanels2.tsx:485` | B10 |
| `createSmartSplit` | `components/Modals/commands/HTC/AllCommands.ts:312` | `components/commandModalPanels2.tsx:488` | B19 |
| `toggleBoardLayout` | `components/Modals/commands/HTC/AllCommands.ts:321` | `components/commandDispatcher.ts:436` | X |
| `configureTableColumns` | `components/Modals/commands/HTC/AllCommands.ts:328` | `components/commandDispatcher.ts:440, components/commandModalPanels2.tsx:539` | X |
| `manageLabels` | `components/Modals/commands/HTC/AllCommands.ts:334` | `components/commandModalPanels2.tsx:421` | B12 |
| `createCustomField` | `components/Modals/commands/HTC/AllCommands.ts:340` | `components/commandDispatcher.ts:1035, components/commandModalPanels2.tsx:571` | X |
| `manageCustomFields` | `components/Modals/commands/HTC/AllCommands.ts:346` | `components/commandDispatcher.ts:446, components/commandModalPanels2.tsx:543` | X |
| `toggleEmptyColumns` | `components/Modals/commands/HTC/AllCommands.ts:353` | `components/commandDispatcher.ts:398` | B10 |
| `toggleArchivedOnBoard` | `components/Modals/commands/HTC/AllCommands.ts:363` | `components/commandDispatcher.ts:413` | B10 |
| `copyBoardLink` | `components/Modals/commands/HTC/AllCommands.ts:378` | `components/commandModalPanels2.tsx:360` | B13 |
| `resetBoardLink` | `components/Modals/commands/HTC/AllCommands.ts:384` | `components/commandModalPanels2.tsx:360` | B13 |
| `manageTeams` | `components/Modals/commands/HTC/AllCommands.ts:390` | `components/commandDispatcher.ts:310` | X |
| `createBoard` | `components/Modals/commands/HTC/AllCommands.ts:396` | `components/commandDispatcher.ts:323, components/commandModalPanels2.tsx:300` | X |
| `boardCreationAssistant` | `components/Modals/commands/HTC/AllCommands.ts:402` | `components/commandDispatcher.ts:324, components/commandModalPanels2.tsx:306` | X |
| `deleteBoard` | `components/Modals/commands/HTC/AllCommands.ts:415` | `components/commandDispatcher.ts:291, components/commandModalPanels2.tsx:138` | B13 |
| `toggleAppShellRail` | `components/Modals/commands/HTC/AllCommands.ts:428` | `components/commandDispatcher.ts:491` | X |
| `branchInNewChat` | `components/Modals/commands/HTC/AllCommands.ts:460` | `components/commandDispatcher.ts:710` | U8 |
| `createAgent` | `components/Modals/commands/HTC/AllCommands.ts:472` | `components/commandModalPanels2.tsx:568` | X |
| `manageAgents` | `components/Modals/commands/HTC/AllCommands.ts:478` | `components/commandDispatcher.ts:824` | X |
| `disabledAgents` | `components/Modals/commands/HTC/AllCommands.ts:485` | `components/commandDispatcher.ts:869` | X |
| `deleteAllChats` | `components/Modals/commands/HTC/AllCommands.ts:491` | `components/commandDispatcher.ts:1031` | N3 |
| `systemTheme` | `components/Modals/commands/HTC/AllCommands.ts:503` | `components/commandDispatcher.ts:989` | X |
| `profilePicture` | `components/Modals/commands/HTC/AllCommands.ts:533` | `components/commandDispatcher.ts:298, components/commandDispatcher.ts:515` | X |
| `switchAccount` | `components/Modals/commands/HTC/AllCommands.ts:540` | `components/commandModalPanels2.tsx:523` | X |
| `manageFavorites` | `components/Modals/commands/HTC/AllCommands.ts:546` | `components/commandModalPanels2.tsx:309` | X |
| `calendarSettings` | `components/Modals/commands/HTC/AllCommands.ts:552` | `components/commandDispatcher.ts:964` | X |
| `toggleCalendarWeekends` | `components/Modals/commands/HTC/AllCommands.ts:559` | `components/commandDispatcher.ts:968` | X |
| `calendarWeekStartsMonday` | `components/Modals/commands/HTC/AllCommands.ts:565` | `components/commandDispatcher.ts:975` | X |
| `calendarWeekStartsSunday` | `components/Modals/commands/HTC/AllCommands.ts:571` | `components/commandDispatcher.ts:982` | X |
| `subscribeGoogleCalendar` | `components/Modals/commands/HTC/AllCommands.ts:577` | `components/commandDispatcher.ts:763` | X |
| `useSnippet` | `components/Modals/commands/HTC/AllCommands.ts:591` | `components/commandDispatcher.ts:651` | B14 |
| `createSnippet` | `components/Modals/commands/HTC/AllCommands.ts:598` | `components/commandDispatcher.ts:657` | B14 |
| `createSnippetFromDraft` | `components/Modals/commands/HTC/AllCommands.ts:604` | `components/commandDispatcher.ts:658` | B14 |
| `onboardingTour` | `components/Modals/commands/HTC/AllCommands.ts:1243` | `components/commandDispatcher.ts:722` | X |
| `latestUpdates` | `components/Modals/commands/HTC/AllCommands.ts:1255` | `components/commandDispatcher.ts:386` | X |
| `goToWelcome` | `components/Modals/commands/HTC/AllCommands.ts:1261` | `components/commandDispatcher.ts:611` | X |
| `startKanbanTutorial` | `components/Modals/commands/HTC/AllCommands.ts:1267` | `components/commandDispatcher.ts:993` | X |
| `startTaskWriterTutorial` | `components/Modals/commands/HTC/AllCommands.ts:1273` | `components/commandDispatcher.ts:1008` | X |
| `generateMcpToken` | `components/Modals/commands/HTC/AllCommands.ts:1279` | `components/commandModalPanels2.tsx:516` | X |
| `restApi` | `components/Modals/commands/HTC/AllCommands.ts:1293` | `components/commandModalPanels2.tsx:532` | X |
| `logout` | `components/Modals/commands/HTC/AllCommands.ts:1299` | `components/commandModalPanels2.tsx:123` | X |
| `roundedTheme` | `components/Modals/commands/HTC/AllCommands.ts:1344` | `components/commandDispatcher.ts:339` | X |
| `sortBoard` | `components/Modals/commands/HTC/AllCommands.ts:181` | `components/commandModalPanels2.tsx:418` | B10 |
| `ShowFilterHtc` | `components/Modals/commands/HTC/AllCommands.ts:1380` | `components/commandModalPanels2.tsx:350` | B10 |
| `saveView` | `components/Modals/commands/HTC/AllCommands.ts:1386` | `components/commandModalPanels2.tsx:485` | B10 |
| `sortBoard` | `components/Modals/commands/HTC/AllCommands.ts:181` | `components/commandModalPanels2.tsx:418` | B10 |
| `ShowFilterHtc` | `components/Modals/commands/HTC/AllCommands.ts:1380` | `components/commandModalPanels2.tsx:350` | B10 |
| `undoLatest` | `components/Modals/commands/HTC/navigationCommands.ts:11` | `components/commandDispatcher.ts:267` | B20 |
| `GotoSnippets` | `components/Modals/commands/HTC/navigationCommands.ts:31` | `components/commandDispatcher.ts:532` | X |
| `GotoReports` | `components/Modals/commands/HTC/navigationCommands.ts:38` | `components/commandDispatcher.ts:577` | X |
| `GoToCalender` | `components/Modals/commands/HTC/navigationCommands.ts:44` | `components/commandDispatcher.ts:873` | X |
| `GoToAllTasks` | `components/Modals/commands/HTC/navigationCommands.ts:51` | `components/commandDispatcher.ts:628` | X |
| `GoToDueDates` | `components/Modals/commands/HTC/navigationCommands.ts:58` | `components/commandDispatcher.ts:880` | X |
| `GoToMyTasks` | `components/Modals/commands/HTC/navigationCommands.ts:65` | `components/commandDispatcher.ts:631` | X |
| `GoToDrafts` | `components/Modals/commands/HTC/navigationCommands.ts:73` | `components/commandDispatcher.ts:811` | X |
| `GoToStarred` | `components/Modals/commands/HTC/navigationCommands.ts:80` | `components/commandDispatcher.ts:815` | X |
| `GoToPinned` | `components/Modals/commands/HTC/navigationCommands.ts:87` | `components/commandDispatcher.ts:819` | X |
| `GoToAgents` | `components/Modals/commands/HTC/navigationCommands.ts:94` | `components/commandDispatcher.ts:823` | X |
| `GoToAgentChat` | `components/Modals/commands/HTC/navigationCommands.ts:101` | `components/commandDispatcher.ts:828` | X |
| `GotoReminders` | `components/Modals/commands/HTC/navigationCommands.ts:107` | `components/commandDispatcher.ts:546` | X |
| `GotoTaskArchived` | `components/Modals/commands/HTC/navigationCommands.ts:114` | `components/commandDispatcher.ts:625` | X |
| `GotoInboxArchive` | `components/Modals/commands/HTC/navigationCommands.ts:121` | `components/commandDispatcher.ts:543` | X |
| `archiveShowActiveBoardsOnly` | `components/Modals/commands/HTC/navigationCommands.ts:128` | `components/commandDispatcher.ts:424` | X |
| `archiveShowAllBoards` | `components/Modals/commands/HTC/navigationCommands.ts:134` | `components/commandDispatcher.ts:428` | X |
| `archiveShowArchivedBoardsOnly` | `components/Modals/commands/HTC/navigationCommands.ts:140` | `components/commandDispatcher.ts:432` | X |
| `GoToProjectInbox` | `components/Modals/commands/HTC/navigationCommands.ts:146` | `components/commandDispatcher.ts:1026` | X |
| `GoToTrash` | `components/Modals/commands/HTC/navigationCommands.ts:152` | `components/commandDispatcher.ts:622` | X |
| `toggleArchivedSearchResults` | `components/Modals/commands/HTC/navigationCommands.ts:161` | `components/commandDispatcher.ts:417` | X |
| `ShowFilterHtc` | `components/Modals/commands/HTC/navigationCommands.ts:178` | `components/commandModalPanels2.tsx:142` | B10 |
| `toggleTimeTracking` | `components/Modals/commands/HTC/navigationCommands.ts:194` | `components/commandDispatcher.ts:946` | U3 |
| `logTimeOnTask` | `components/Modals/commands/HTC/navigationCommands.ts:201` | `components/commandDispatcher.ts:562` | B15 |
| `goToTimeThisTask` | `components/Modals/commands/HTC/navigationCommands.ts:214` | `components/commandDispatcher.ts:552` | B15 |
| `goToTimeThisBoard` | `components/Modals/commands/HTC/navigationCommands.ts:225` | `components/commandDispatcher.ts:572` | B15 |
| `toggleBoardTimeTracking` | `components/Modals/commands/HTC/navigationCommands.ts:231` | `components/commandDispatcher.ts:592` | B15 |
| `GoToTimers` | `components/Modals/commands/HTC/navigationCommands.ts:244` | `components/commandDispatcher.ts:549` | X |
| `goToTimeMyWeek` | `components/Modals/commands/HTC/navigationCommands.ts:253` | `components/commandDispatcher.ts:595` | X |
| `bulkSelectAllOnBoard` | `components/Modals/commands/HTC/navigationCommands.ts:303` | `components/commandDispatcher.ts:55-65 (mode handoff)` | B19 |
| `bulkSelectAllInColumn` | `components/Modals/commands/HTC/navigationCommands.ts:310` | `components/commandDispatcher.ts:55-65 (mode handoff)` | B19 |
| `bulkClearSelection` | `components/Modals/commands/HTC/navigationCommands.ts:316` | `components/commandDispatcher.ts:55-65 (mode handoff)` | B19 |
| `agentChatNextAgent` | `components/Modals/commands/HTC/navigationCommands.ts:332` | `components/commandDispatcher.ts:841` | X |
| `agentChatPreviousAgent` | `components/Modals/commands/HTC/navigationCommands.ts:339` | `components/commandDispatcher.ts:845` | X |
| `agentChatSendMessage` | `components/Modals/commands/HTC/navigationCommands.ts:346` | `components/commandDispatcher.ts:849` | X |
| `agentChatOpenLinks` | `components/Modals/commands/HTC/navigationCommands.ts:353` | `components/commandDispatcher.ts:853` | X |
| `agentChatAddAgent` | `components/Modals/commands/HTC/navigationCommands.ts:360` | `components/commandDispatcher.ts:857` | X |
| `agentChatNextTeam` | `components/Modals/commands/HTC/navigationCommands.ts:366` | `components/commandDispatcher.ts:861` | X |
| `agentChatPreviousTeam` | `components/Modals/commands/HTC/navigationCommands.ts:373` | `components/commandDispatcher.ts:865` | X |
| `appShellTable` | `components/Modals/commands/HTC/navigationCommands.ts:401` | `components/commandDispatcher.ts:449` | X |
| `appShellCalendar` | `components/Modals/commands/HTC/navigationCommands.ts:408` | `components/commandDispatcher.ts:873` | X |
| `createTask` | `components/Modals/commands/HTC/taskCommands.ts:10` | `components/commandDispatcher.ts:640` | B9 |
| `setStartDate` | `components/Modals/commands/HTC/taskCommands.ts:24` | `components/commandModalPanels2.tsx:160` | B5 |
| `setRecurrence` | `components/Modals/commands/HTC/taskCommands.ts:31` | `components/commandModalPanels2.tsx:173` | B5 |
| `saveTaskTemplate` | `components/Modals/commands/HTC/taskCommands.ts:38` | `components/commandDispatcher.ts:634` | B6 |
| `blockedByPerson` | `components/Modals/commands/HTC/taskCommands.ts:52` | `components/commandModalPanels2.tsx:273` | U1 |
| `addSubTask` | `components/Modals/commands/HTC/taskCommands.ts:66` | `components/commandModalPanels2.tsx:500` | B9 |
| `viewSubTask` | `components/Modals/commands/HTC/taskCommands.ts:73` | `components/commandDispatcher.ts:704` | N2 |
| `addRelatedTask` | `components/Modals/commands/HTC/taskCommands.ts:80` | `components/commandModalPanels2.tsx:105-121` | B4 |
| `markBlockedBy` | `components/Modals/commands/HTC/taskCommands.ts:86` | `components/commandModalPanels2.tsx:105-121` | B4 |
| `markAsBlocking` | `components/Modals/commands/HTC/taskCommands.ts:92` | `components/commandModalPanels2.tsx:105-121` | B4 |
| `markDuplicateOf` | `components/Modals/commands/HTC/taskCommands.ts:98` | `components/commandModalPanels2.tsx:105-121` | B4 |
| `followtask` | `components/Modals/commands/HTC/taskCommands.ts:139` | `components/commandDispatcher.ts:358` | B7 |
| `unfollowtask` | `components/Modals/commands/HTC/taskCommands.ts:146` | `components/commandDispatcher.ts:361` | B7 |
| `openAiTaskWriter` | `components/Modals/commands/HTC/taskCommands.ts:153` | `components/commandDispatcher.ts:698` | N1 |
| `summarizeTicket` | `components/Modals/commands/HTC/taskCommands.ts:159` | `components/commandDispatcher.ts:701` | U9 |
| `markAsUnread` | `components/Modals/commands/HTC/taskCommands.ts:173` | `components/commandDispatcher.ts:517` | B2 |
| `summarizeComment` | `components/Modals/commands/HTC/taskCommands.ts:195` | `components/commandDispatcher.ts:716` | U10 |
| `createTaskFromComment` | `components/Modals/commands/HTC/taskCommands.ts:256` | `components/commandDispatcher.ts:757` | B9 |
| `branchInNewChat` | `components/Modals/commands/HTC/taskCommands.ts:262` | `components/commandDispatcher.ts:710` | U8 |
| `acceptTask` | `components/Modals/commands/HTC/taskCommands.ts:317` | `components/commandDispatcher.ts:373` | U6 |
| `declineTask` | `components/Modals/commands/HTC/taskCommands.ts:323` | `components/commandDispatcher.ts:376` | U4 |
| `declineAsDuplicateOf` | `components/Modals/commands/HTC/taskCommands.ts:329` | `components/commandModalPanels2.tsx:105-121` | B4 |
| `duplicateTask` | `components/Modals/commands/HTC/taskCommands.ts:335` | `components/commandDispatcher.ts:668` | B9 |
| `duplicateTaskToBoard` | `components/Modals/commands/HTC/taskCommands.ts:341` | `components/commandDispatcher.ts:695` | B9 |
| `copypublicformattedTaskLink` | `components/Modals/commands/HTC/taskCommands.ts:375` | `components/commandDispatcher.ts:778` | B8 |
| `copyPublicLink` | `components/Modals/commands/HTC/taskCommands.ts:382` | `components/commandDispatcher.ts:775` | B8 |
| `setReminder` | `components/Modals/commands/HTC/taskCommands.ts:421` | `components/commandDispatcher.ts:884` | B3 |
| `starTask` | `components/Modals/commands/HTC/taskCommands.ts:429` | `components/commandDispatcher.ts:790` | U2 |
| `shareTaskPublicly` | `components/Modals/commands/HTC/taskCommands.ts:436` | `components/commandModalPanels2.tsx:513` | B8 |
| `toggleHistory` | `components/Modals/commands/HTC/taskCommands.ts:471` | `components/commandDispatcher.ts:390` | B16 |
| `taskDescriptionVersions` | `components/Modals/commands/HTC/taskCommands.ts:480` | `components/commandDispatcher.ts:1038, components/commandModalPanels2.tsx:574` | B17 |
| `removeSubtask` | `components/Modals/commands/HTC/taskCommands.ts:506` | `components/commandDispatcher.ts:922` | N2 |
| `removeParenttask` | `components/Modals/commands/HTC/taskCommands.ts:514` | `components/commandDispatcher.ts:919` | U7 |
| `showInInbox` | `components/Modals/commands/HTC/taskCommands.ts:522` | `components/commandDispatcher.ts:769` | U5 |
| `subtaskSettings` | `components/Modals/commands/HTC/taskCommands.ts:528` | `components/commandModalPanels2.tsx:482` | B10 |
| `suggestReply` | `components/Modals/commands/HTC/taskCommands.ts:535` | `components/commandDispatcher.ts:925` | B18 |
| `speechToText` | `components/Modals/commands/HTC/taskCommands.ts:544` | `components/commandDispatcher.ts:934` | B18 |

Reviewed 151 historical unresolved declarations.
Unique command keys: 145.
Additional partial workflows retained: native AI Compose and search.
Search remains cached, not web ranked document search.
Web source `pages/api/search/document.ts:10-12,30-40`.
No existing partial row promoted to full parity.

## Native evidence contract

Acceptance: `GATES-cmdk2.md`, written before implementation.
Tests drive Ctrl+K, query text, Enter, and picker choices.
They observe durable payloads, task state, and real acknowledgements.
Negative cases cover missing identity/state and unavailable routes.
Core checks reject wrong account, project, and acknowledgement.
Existing keyboard tests retain timer unknown-outcome safeguards.
Exact-head GitHub receipt is required before completion.
No live authenticated account write is claimed.
