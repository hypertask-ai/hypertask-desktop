# Command center round 4 audit

Web root: `/home/valentin/projects/hypertask/src`.
Web checkout: `84ee35e8afe00cc67c79c8ad6162563363bfc19d`.
Acceptance: [GATES-cmdk4.md](../GATES-cmdk4.md).
Baseline native main: `c5fbfcc` after rebase.
Historical inventories remain unchanged below their round headings.
This refresh supersedes their unresolved-row dispositions.

## Audit and counts

The current TypeScript declarations and dispatcher were read again.
The AST audit found 299 declarations, 297 keys.
The runtime query handover adds one inventory row.
All 300 existing declaration locations and labels match.
No new web command was added since round 3.
The three historical ledgers were cross-checked by key.
The table below covers all 75 unresolved declarations.
The historical crosswalk retains repeated rows separately.

| Measure | Before | After |
| --- | --- | --- |
| Fully working fixed keys | 84 | 85 |
| Fully working declarations, including duplicate branch declaration | 85 | 86 |
| Available fixed keys, including partial workflows | 90 | 90 |
| Unresolved declarations | 75 | 74 |
| Excluded declarations | 140 | 140 |

These counts describe palette workflows, not live-account authorization.
Dynamic board rows are excluded from fixed-key counts.
Search navigation remains working; search-engine parity remains partial.

## Implemented workflow

`removeSubtask` now works with uncached child details.
The parent projection supplies every displayed field.
Source: `lib/mcp/tasks/mappers.ts:218-231,255`.
Web rows display ticket number and title only.
Source: `components/Modals/SubtaskLinkingModal/RemoveSubtask.tsx:128-136`.
Web submits the selected child ID only.
Source: `components/Modals/SubtaskLinkingModal/RemoveSubtask.tsx:37-49`.
The existing bearer update accepts nullable `parent_task_id`.
Source: `lib/mcp/tasks/fields/types.ts:70`.
It writes that value through the existing single-task writer.
Source: `lib/mcp/tasks/fields/text.ts:15-35`.

Native `BoardWrite::RemoveSubtask` retains parent and child identities.
The queue validates projected membership and cached contradictions.
It reuses `/tasks/update`, rather than inventing child snapshots.
It checks the current account before delivery.
Success requires the selected child, with no remaining parent.
Only acknowledgement removes the parent's projected child row.
Both parent and child are then refetched.
Refusal or invalid acknowledgement leaves the parent list intact.
Missing nullable server aliases explicitly clear merged cached aliases.
Source omission contract: `lib/mcp/tasks/mappers.ts:248-255`.

The label remains **Remove a sub-task**.
The group remains **Task**, without shortcut or icon.
It follows notification removal and precedes parent unlinking.
It appears with Task context and nonempty child data.
Sources: `components/Modals/commands/HTC/taskCommands.ts:495-520`.
Desktop icons remain absent for all palette rows.
Source: `components/Modals/commands/HTC/CommandList.tsx:36-44`.
No board, inbox, task, or AI layout changed.

## Current unresolved declaration recheck

Each row retains the web label expression and effective order.
Group order follows `components/Modals/commands/HTC/AllCommands.ts:1321-1388`.
Task order follows `components/Modals/commands/HTC/taskCommands.ts:390-560`.
Hints and visibility are unchanged from current web declarations.
Desktop icon is **none** throughout this table.
B/N codes name exact missing contracts below.
No code means an existing partial workflow, explained below.

| Group / slot | Web key | Label | Hint | Visibility | Effect | Before | Round 4 | Declaration |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Inbox / 2 | `clearInboxToZero` | "Clear inbox to zero" | — | all desktop contexts | Panel mode ClearInboxToZero (see exact contract below) | Omitted, B1 | Omitted, B1 | `components/Modals/commands/HTC/AllCommands.ts:64` |
| Inbox / 3 | `archiveAllReadNotifications` | "Archive all read" | — | all desktop contexts | Panel mode ArchiveAllReadNotifications (see exact contract below) | Omitted, B1 | Omitted, B1 | `components/Modals/commands/HTC/AllCommands.ts:70` |
| Inbox / 4 | `archiveReactionNotifications` | "Archive reactions" | — | all desktop contexts | Panel mode ArchiveReactionNotifications (see exact contract below) | Omitted, B1 | Omitted, B1 | `components/Modals/commands/HTC/AllCommands.ts:76` |
| Board / 1 | `toggleBoardZoom` | commandOptions.boardZoomedOut ? "Zoom board in" : "Zoom board out" | — | all desktop contexts; current board; commandOptions.boardZoomedOut !== undefined | { const boardId = _currentProject?.id; if (boardId !== undefined) { setBoardZoomedOutByProject((current) => toggleProjectBoardZoom(current, boardId) ); } boardCloseHandler(); return; } `components/commandDispatcher.ts:277` | Native gap, boardZoomedOut state and compact-board renderer absent; visual3-owned layout | Native gap, boardZoomedOut state and compact-board renderer absent; visual3-owned layout | `components/Modals/commands/HTC/AllCommands.ts:137` |
| Board / 2 | `createTask` | "Create task" | ["C"] | all desktop contexts; current board | toggleCreateTaskGlobally(); boardCloseHandler(); `components/commandDispatcher.ts:640` | Omitted, B9 | Omitted, B9 | `components/Modals/commands/HTC/AllCommands.ts:147` |
| Board / 3 | `createTaskWithAiWriter` | "Create task with AI Task Writer" | ["CTRL", "J"] | all desktop contexts; current board | toggleCreateTaskGlobally(undefined, { defaultEditMode: "Description-ai", defaultFocus: "Description", }); boardCloseHandler(); `components/commandDispatcher.ts:644` | Partial, retained (see limits) | Partial, retained (see limits) | `components/Modals/commands/HTC/AllCommands.ts:154` |
| Board / 4 | `newTaskFromTemplate` | "New task from template" | — | all desktop contexts; current board | Open NewTaskFromTemplate panel `components/commandModalPanels2.tsx:191` | Omitted, B6 | Omitted, B6 | `components/Modals/commands/HTC/AllCommands.ts:161` |
| Board / 5 | `generateStatusUpdate` | "Generate status update" | — | all desktop contexts; current board | generateStatusUpdateHandler(); `components/commandDispatcher.ts:637` | Omitted, B11 | Omitted, B11 | `components/Modals/commands/HTC/AllCommands.ts:168` |
| Board / 7 | `sortBoard` | "Sort board" | ["SHIFT", "S"] | all desktop contexts; current board | Open SortKanbanBoard panel `components/commandModalPanels2.tsx:418` | Omitted, B10 | Omitted, B10 | `components/Modals/commands/HTC/AllCommands.ts:181` |
| Board / 8 | `toggleFilterValueMatch` | "Filter values: match any or all" | — | all desktop contexts; current board; commandOptions.context === "Task" &#124;&#124; commandOptions.context === "Kanban" | setShowCommands((prev) => ({ ...prev, mode: CommandMode.ShowFilterHTC })); setCommandMode(CommandMode.ShowFilterHTC); `components/commandDispatcher.ts:330` | Omitted, B10 | Omitted, B10 | `components/Modals/commands/HTC/AllCommands.ts:190` |
| Board / 9 | `toggleStaleness` | commandOptions.projectOptions?.stalenessEnabled ? "Turn off staleness for this board" : "Turn on staleness for this board" | — | all desktop contexts; current board; commandOptions.context === "Task" &#124;&#124; commandOptions.context === "Kanban" | void toggleStalenessHandler(); `components/commandDispatcher.ts:949` | Omitted, B11 | Omitted, B11 | `components/Modals/commands/HTC/AllCommands.ts:200` |
| Board / 10 | `toggleStalenessView` | commandOptions.projectOptions?.stalenessViewEnabled ? "Turn off staleness for this view" : "Turn on staleness for this view" | — | all desktop contexts; current board; commandOptions.context === "Task" &#124;&#124; commandOptions.context === "Kanban" | void toggleStalenessViewHandler(); `components/commandDispatcher.ts:952` | Omitted, B11 | Omitted, B11 | `components/Modals/commands/HTC/AllCommands.ts:208` |
| Board / 11 | `toggleAutoArchive` | commandOptions.projectOptions?.autoArchiveEnabled ? "Turn off auto-archive for this board" : "Turn on auto-archive for this board (6 months idle)" | — | all desktop contexts; current board; commandOptions.context === "Task" &#124;&#124; commandOptions.context === "Kanban" | void toggleAutoArchiveHandler(); `components/commandDispatcher.ts:955` | Omitted, B11 | Omitted, B11 | `components/Modals/commands/HTC/AllCommands.ts:216` |
| Board / 12 | `boardVelocityReport` | "Board velocity report" | — | all desktop contexts; current board; commandOptions.context === "Task" &#124;&#124; commandOptions.context === "Kanban" | { const velocityProjectId = _currentProject?.id ?? Number(pathname?.match(/project-(\d+)/)?.[1] ?? NaN); if (Number.isInteger(velocityProjectId) && velocityProjectId > 0) { GoToHandler(/report/project-${velocityProjectId}/velocity); } else { toast.error("Open a board first to see its velocity report"); } break; } `components/commandDispatcher.ts:581` | Omitted, B11 | Omitted, B11 | `components/Modals/commands/HTC/AllCommands.ts:224` |
| Board / 13 | `sortByTimeInColumn` | "Sort by time in column" | — | all desktop contexts; current board; commandOptions.context === "Task" &#124;&#124; commandOptions.context === "Kanban" | void sortByStalenessHandler("TimeInColumn"); `components/commandDispatcher.ts:958` | Omitted, B11 | Omitted, B11 | `components/Modals/commands/HTC/AllCommands.ts:231` |
| Board / 14 | `sortByLastComment` | "Sort by last comment" | — | all desktop contexts; current board; commandOptions.context === "Task" &#124;&#124; commandOptions.context === "Kanban" | void sortByStalenessHandler("TimeWithoutComment"); `components/commandDispatcher.ts:961` | Omitted, B11 | Omitted, B11 | `components/Modals/commands/HTC/AllCommands.ts:237` |
| Board / 15 | `autoAssignColumn` | "Auto-assign for this column" | — | all desktop contexts; current board; commandOptions.context === "Kanban" | Open AutoAssignColumn panel `components/commandModalPanels2.tsx:289` | Omitted, B11 | Omitted, B11 | `components/Modals/commands/HTC/AllCommands.ts:247` |
| Board / 16 | `manageColumns` | "Manage board columns" | — | all desktop contexts; current board | Open ManageColumn panel `components/commandModalPanels2.tsx:350` | Omitted, B12 | Omitted, B12 | `components/Modals/commands/HTC/AllCommands.ts:255` |
| Board / 18 | `hideColumn` | "Hide board column" | — | all desktop contexts; current board | void hideActiveColumn(); boardCloseHandler(); `components/commandDispatcher.ts:409` | Omitted, B10 | Omitted, B10 | `components/Modals/commands/HTC/AllCommands.ts:267` |
| Board / 23 | `goToViews` | "Go to views" | ["G", null, "V"] | all desktop contexts; current board | Open ShowBoardViews panel `components/commandModalPanels2.tsx:494` | Omitted, B10 | Omitted, B10 | `components/Modals/commands/HTC/AllCommands.ts:297` |
| Board / 24 | `manageViews` | "Manage views" | — | all desktop contexts; current board | Open ManageViews panel `components/commandModalPanels2.tsx:485` | Omitted, B10 | Omitted, B10 | `components/Modals/commands/HTC/AllCommands.ts:304` |
| Board / 25 | `createSmartSplit` | "Add smart split" | — | all desktop contexts; current board; commandOptions.context === "Task" &#124;&#124; commandOptions.context === "Kanban" | Open CreateSmartSplit panel `components/commandModalPanels2.tsx:488` | Omitted, B19 | Omitted, B10; context-gated, not selection-flag-gated | `components/Modals/commands/HTC/AllCommands.ts:312` |
| Board / 28 | `manageLabels` | "Manage tags" | — | all desktop contexts; current board | Open ManageLabels panel `components/commandModalPanels2.tsx:421` | Omitted, B12 | Omitted, B12 | `components/Modals/commands/HTC/AllCommands.ts:334` |
| Board / 31 | `toggleEmptyColumns` | "Hide empty board columns" | — | all desktop contexts; current board | if (_currentProject) { const current = getActiveEmptySectionSettingFromProject(_currentProject); saveEmptySectionsAPI( _currentProject, current === "Hidden" ? "Show" : "Hidden" ); } boardCloseHandler(); `components/commandDispatcher.ts:398` | Omitted, B10 | Omitted, B10 | `components/Modals/commands/HTC/AllCommands.ts:353` |
| Board / 32 | `toggleArchivedOnBoard` | ${commandOptions.showArchivedOnBoard ? "Hide" : "Show"} archived tasks on board | ["G", null, "X"] | all desktop contexts; current board; commandOptions.searchOptions | toggleShowArchivedOnBoard(); boardCloseHandler(); `components/commandDispatcher.ts:413` | Omitted, B10 | Omitted, B10 | `components/Modals/commands/HTC/AllCommands.ts:363` |
| Board / 34 | `copyBoardLink` | "Copy board join link" | — | all desktop contexts; current board | Open BoardJoinResetLink panel `components/commandModalPanels2.tsx:360` | Omitted, B13 | Omitted, B13 | `components/Modals/commands/HTC/AllCommands.ts:378` |
| Board / 35 | `resetBoardLink` | "Reset board join link" | — | all desktop contexts; current board | Open BoardJoinResetLink panel `components/commandModalPanels2.tsx:360` | Omitted, B13 | Omitted, B13 | `components/Modals/commands/HTC/AllCommands.ts:384` |
| Board / 40 | `deleteBoard` | "Delete board" | — | all desktop contexts; current board | if (!_currentProject &#124;&#124; currentUser.id.toString() !== _currentProject.ownerId.toString()) { toast.error("Only board owner can manage this board"); boardCloseHandler(); return; } `components/commandDispatcher.ts:291` | Omitted, B13 | Omitted, B13 | `components/Modals/commands/HTC/AllCommands.ts:415` |
| AI / 3 | `fullScreenAiChat` | "Open AI chat full screen" | — | all desktop contexts | if (isMbl &#124;&#124; pathname?.startsWith("/login")) break; router.push( buildFullScreenChatPath( ${window.location.pathname}${window.location.search}${window.location.hash} ) ); boardCloseHandler(); `components/commandDispatcher.ts:743` | Partial, retained (see limits) | Partial, retained (see limits) | `components/Modals/commands/HTC/AllCommands.ts:454` |
| AI / 9 | `deleteAllChats` | "Delete all chat sessions" | — | all desktop contexts | deleteAllChats(); `components/commandDispatcher.ts:1031` | Omitted, N3 | Omitted, N3 | `components/Modals/commands/HTC/AllCommands.ts:491` |
| Snippets / 1 | `useSnippet` | "Use snippet" | [";"] | all desktop contexts | boardCloseHandler(); if (!openSnippetPicker()) { toast.error("Focus an editor before using a snippet"); } `components/commandDispatcher.ts:651` | Omitted, B14 | Omitted, B14 | `components/Modals/commands/HTC/AllCommands.ts:591` |
| Snippets / 2 | `createSnippet` | "Create snippet" | — | all desktop contexts | putSnippetDraftHandoff( currentUser?.id, mode === CommandMode.CreateSnippetFromDraft ? getLastFocusedTiptapEditor()?.getHTML() ?? "" : "" ); boardCloseHandler(); router.push("/snippets"); `components/commandDispatcher.ts:657` | Omitted, B14 | Omitted, B14 | `components/Modals/commands/HTC/AllCommands.ts:598` |
| Snippets / 3 | `createSnippetFromDraft` | "Create snippet from draft" | — | all desktop contexts | putSnippetDraftHandoff( currentUser?.id, mode === CommandMode.CreateSnippetFromDraft ? getLastFocusedTiptapEditor()?.getHTML() ?? "" : "" ); boardCloseHandler(); router.push("/snippets"); `components/commandDispatcher.ts:658` | Omitted, B14 | Omitted, B14 | `components/Modals/commands/HTC/AllCommands.ts:604` |
| Task / base only | `createTask` | "Create task" | ["C"] | Base declaration not selected by getTaskCommands; Board has own declaration | toggleCreateTaskGlobally(); boardCloseHandler(); `components/commandDispatcher.ts:640` | Omitted, B9 | Omitted, B9 | `components/Modals/commands/HTC/taskCommands.ts:10` |
| Task / 8 | `setStartDate` | "Set start date" | — | Task context | Open SetStartDate panel `components/commandModalPanels2.tsx:160` | Omitted, B5 | Omitted, B5 | `components/Modals/commands/HTC/taskCommands.ts:24` |
| Task / 9 | `setRecurrence` | "Repeat task" | — | Task context | Open SetRecurrence panel `components/commandModalPanels2.tsx:173` | Omitted, B5 | Omitted, B5 | `components/Modals/commands/HTC/taskCommands.ts:31` |
| Task / 10 | `saveTaskTemplate` | "Save task as template" | — | Task context | saveTaskTemplateHandler(); `components/commandDispatcher.ts:634` | Omitted, B6 | Omitted, B6 | `components/Modals/commands/HTC/taskCommands.ts:38` |
| Task / 47 | `addSubTask` | "Create subtask" | ["CTRL", "SHIFT", "+"] | Task context | Open CreateSubTask panel `components/commandModalPanels2.tsx:500` | Omitted, B9 | Omitted, B9 | `components/Modals/commands/HTC/taskCommands.ts:66` |
| Task / 48 | `viewSubTask` | "View subtasks" | ["CTRL", "O"] | Task context | viewSubTasksHandler(); `components/commandDispatcher.ts:704` | Omitted, N2 | Omitted, N2 | `components/Modals/commands/HTC/taskCommands.ts:73` |
| Task / 16 | `addRelatedTask` | "Add related task" | — | Task context | Panel mode AddRelatedTask (see exact contract below) | Omitted, B4 | Omitted, B4 | `components/Modals/commands/HTC/taskCommands.ts:80` |
| Task / 17 | `markBlockedBy` | "Mark blocked by" | — | Task context | Panel mode MarkBlockedBy (see exact contract below) | Omitted, B4 | Omitted, B4 | `components/Modals/commands/HTC/taskCommands.ts:86` |
| Task / 18 | `markAsBlocking` | "Mark as blocking" | — | Task context | Panel mode MarkAsBlocking (see exact contract below) | Omitted, B4 | Omitted, B4 | `components/Modals/commands/HTC/taskCommands.ts:92` |
| Task / 19 | `markDuplicateOf` | "Mark duplicate of" | — | Task context | Panel mode MarkDuplicateOf (see exact contract below) | Omitted, B4 | Omitted, B4 | `components/Modals/commands/HTC/taskCommands.ts:98` |
| Task / 24 | `followtask` | "Follow task" | ["F"] | Task context | followTaskHandler(); `components/commandDispatcher.ts:358` | Omitted, B7 | Omitted, B7 | `components/Modals/commands/HTC/taskCommands.ts:139` |
| Task / 25 | `unfollowtask` | "Unfollow task" | ["ALT", "F"] | Task context | unFollowTaskHandler(); `components/commandDispatcher.ts:361` | Omitted, B7 | Omitted, B7 | `components/Modals/commands/HTC/taskCommands.ts:146` |
| Task / 49 | `openAiTaskWriter` | "Open AI task writer" | — | Task context; not Kanban card | openAiWriterHandler(); `components/commandDispatcher.ts:698` | Omitted, N1 | Omitted, N1 | `components/Modals/commands/HTC/taskCommands.ts:153` |
| Task / 51 | `remindMe` | "Remind me" | ["H"] | Inbox context; also My Tasks Task context | Open RemindMe panel `components/commandModalPanels2.tsx:464` | Partial, retained (see limits) | Partial, retained (see limits) | `components/Modals/commands/HTC/taskCommands.ts:166` |
| Task / base only | `markAsUnread` | "Mark unread" | ["U"] | Inbox context | markUnread(); `components/commandDispatcher.ts:517` | Omitted, B2 | Omitted, B2 | `components/Modals/commands/HTC/taskCommands.ts:173` |
| Comment / 7 | `blockerComment` | commentProps.isBlocker ? "Unmark as blocker" : "Mark as blocker" | — | focused comment; commentProps.blockerCommentEnabled; commentProps; both account flag and comment capability | commentFunctionHandler("BlockerComment"); `components/commandDispatcher.ts:799` | Omitted, B21 | Omitted, B21 | `components/Modals/commands/HTC/taskCommands.ts:230` |
| Comment / 11 | `createTaskFromComment` | "Create task from comment" | — | focused comment; commentProps | commentFunctionHandler("CreateTaskFromComment"); `components/commandDispatcher.ts:757` | Omitted, B9 | Omitted, B9 | `components/Modals/commands/HTC/taskCommands.ts:256` |
| Task / 3 | `declineAsDuplicateOf` | "Decline as duplicate of…" | — | Task context; not archived | Open DeclineAsDuplicateOf panel `components/commandModalPanels2.tsx:113` | Omitted, B4 | Omitted, B4 | `components/Modals/commands/HTC/taskCommands.ts:329` |
| Task / 33 | `duplicateTask` | "Duplicate task" | — | Task context | duplicateTaskHandler(); `components/commandDispatcher.ts:668` | Omitted, B9 | Omitted, B9 | `components/Modals/commands/HTC/taskCommands.ts:335` |
| Task / 34 | `duplicateTaskToBoard` | "Copy task to another board" | — | Task context | duplicateTaskHandler(true); `components/commandDispatcher.ts:695` | Omitted, B9 | Omitted, B9 | `components/Modals/commands/HTC/taskCommands.ts:341` |
| Task / 28 | `copypublicformattedTaskLink` | "Copy public task link (title + URL)" | shareKeys("publicFormatted", ["CTRL", "."]) | Task context | copyURLFunctionHandler("PublicFormatted"); `components/commandDispatcher.ts:778` | Omitted, B8 | Omitted, B8 | `components/Modals/commands/HTC/taskCommands.ts:375` |
| Task / 29 | `copyPublicLink` | "Copy public task URL" | shareKeys("publicUrl", ["CTRL", "SHIFT", "."]) | Task context | copyURLFunctionHandler("Public"); `components/commandDispatcher.ts:775` | Omitted, B8 | Omitted, B8 | `components/Modals/commands/HTC/taskCommands.ts:382` |
| Task / 21 | `setReminder` | "Set reminder" | ["H"] | Task context; not Kanban card; !taskProps?.isKanban; commandOptions?.context === "Task" | setReminderHandler(); `components/commandDispatcher.ts:884` | Omitted, B3 | Omitted, B3 | `components/Modals/commands/HTC/taskCommands.ts:421` |
| Task / 23 | `shareTaskPublicly` | "Share task" | ["CTRL", "S"] | Task context; commandOptions?.context === "Task" | Open ShareTaskPublic panel `components/commandModalPanels2.tsx:513` | Omitted, B8 | Omitted, B8 | `components/Modals/commands/HTC/taskCommands.ts:436` |
| Task / 35 | `toggleHistory` | ${taskProps?.showHistory ? "Hide" : "Show"} agent log | ["CTRL", "SHIFT", "H"] | Task context; commandOptions?.context === "Task" | setShowTaskHistory((prev) => !prev); boardCloseHandler(); `components/commandDispatcher.ts:390` | Omitted, B16 | Omitted, B16 | `components/Modals/commands/HTC/taskCommands.ts:471` |
| Task / 36 | `taskDescriptionVersions` | "Description version history" | — | Task context; not Kanban card; !taskProps?.isKanban; commandOptions?.context === "Task" | Open TaskDescriptionVersions panel `components/commandModalPanels2.tsx:574` | Omitted, B17 | Omitted, B17 | `components/Modals/commands/HTC/taskCommands.ts:480` |
| Task / 38 | `removeTaskNotification` | "Remove task notification" | ["E"] | Task context; hasNotifications; taskProps?.hasNotifications; commandOptions?.context === "Task" | removeNotificationHandler(); `components/commandDispatcher.ts:379` | Partial, retained (see limits) | Partial, retained (see limits) | `components/Modals/commands/HTC/taskCommands.ts:497` |
| Task / 39 | `removeSubtask` | "Remove a sub-task" | — | Task context; hasSubtasks; taskProps?.hasSubtasks; commandOptions?.context === "Task" | removeSubtaskHandler(); `components/commandDispatcher.ts:922` | Partial, cached complete child list required | Working; projected children, no detail-cache prerequisite | `components/Modals/commands/HTC/taskCommands.ts:506` |
| Task / 43 | `subtaskSettings` | "Sub-task Settings" | — | Task context; commandOptions?.context === "Task" | Open SubtaskSettings panel `components/commandModalPanels2.tsx:482` | Omitted, B10 | Omitted, B10 | `components/Modals/commands/HTC/taskCommands.ts:528` |
| Task / 44 | `suggestReply` | "Suggest reply" | ["SHIFT", "R"] | Task context; not Kanban card; !taskProps?.isKanban; commandOptions?.context === "Task" | if (!inViewObject.taskId) { toast.error("Open a task first to suggest a reply"); boardCloseHandler(); break; } boardCloseHandler(); window.dispatchEvent(new CustomEvent(AI_SUGGEST_REPLY_EVENT)); `components/commandDispatcher.ts:925` | Omitted, B18 | Omitted, B18 | `components/Modals/commands/HTC/taskCommands.ts:535` |
| Task / 45 | `speechToText` | "Speech to text" | ["CTRL", "SHIFT", "D"] | Task context; not Kanban card; !taskProps?.isKanban; commandOptions?.context === "Task" | commentAudioSpeechToText(); `components/commandDispatcher.ts:934` | Omitted, B18 | Omitted, B18 | `components/Modals/commands/HTC/taskCommands.ts:544` |
| Navigate / 1 | `undoLatest` | "Undo latest action" | ["CTRL", "Z"] | all desktop contexts | void undoLatest() .then((didUndo) => { if (!didUndo) toast("Nothing to undo"); }) .finally(boardCloseHandler); `components/commandDispatcher.ts:267` | Omitted, B20 | Omitted, B20 | `components/Modals/commands/HTC/navigationCommands.ts:11` |
| Navigate / 23 | `toggleArchivedSearchResults` | ${commandOptions.searchOptions.includeArchived ? "Hide" : "Show"} archived search results | ["G", null, "X"] | all desktop contexts; commandOptions.searchOptions | callbackHandler?.( !paletteContextOptions?.searchOptions?.includeArchived, "ToggleArchivedSearchResults" ); boardCloseHandler(); `components/commandDispatcher.ts:417` | Omitted, B22 | Omitted, B22 | `components/Modals/commands/HTC/navigationCommands.ts:161` |
| Navigate / 24 | `searchTask` | "Search" | ["/"] | all desktop contexts | GoToHandler(/search?searchTerm=${encodeURIComponent(action ?? "")}); `components/commandDispatcher.ts:526` | Working navigation; partial search engine (B22) | Working navigation; partial search engine (B22) | `components/Modals/commands/HTC/navigationCommands.ts:170` |
| Navigate / 25 | `ShowFilterHtc` | "Filters" | ["SHIFT", "F"] | all desktop contexts | Open ShowFilterHTC panel `components/commandModalPanels2.tsx:142` | Omitted, B10 | Omitted, B10 | `components/Modals/commands/HTC/navigationCommands.ts:178` |
| Time / 2 | `logTimeOnTask` | "Log time" | ["B"] | Task context; timeTrackingEnabled; commandOptions.context === "Task" && commandOptions.taskOptions?.timeTrackingEnabled | { const url = timeQuickLogTaskUrl( inViewObject.taskId, inViewObject.taskProjectId, _currentProject?.id ); GoToHandler(url ?? "/time"); } `components/commandDispatcher.ts:562` | Omitted, B15 | Omitted, B15 | `components/Modals/commands/HTC/navigationCommands.ts:201` |
| Time / 4 | `goToTimeThisTask` | "Time: this task" | — | Task context; timeTrackingEnabled; commandOptions.context === "Task" && commandOptions.taskOptions?.timeTrackingEnabled | { const url = timeQuickLogTaskUrl( inViewObject.taskId, inViewObject.taskProjectId, _currentProject?.id ); if (url) GoToHandler(url); } `components/commandDispatcher.ts:552` | Omitted, B15 | Omitted, B15 | `components/Modals/commands/HTC/navigationCommands.ts:214` |
| Time / 5 | `goToTimeThisBoard` | "Time: this board" | — | Task or Kanban context; commandOptions.context === "Task" &#124;&#124; commandOptions.context === "Kanban" | if (_currentProject?.id) { GoToHandler(/time?board=${_currentProject.id}); } `components/commandDispatcher.ts:572` | Omitted, B15 | Omitted, B15 | `components/Modals/commands/HTC/navigationCommands.ts:225` |
| Time / 6 | `toggleBoardTimeTracking` | "Toggle time tracking for this board" | — | Task or Kanban context; commandOptions.context === "Task" &#124;&#124; commandOptions.context === "Kanban" | void toggleBoardTimeTrackingHandler(); `components/commandDispatcher.ts:592` | Omitted, B15 | Omitted, B15 | `components/Modals/commands/HTC/navigationCommands.ts:231` |
| Selected tasks / 5 | `bulkSelectAllOnBoard` | "Select all on board" | ["CTRL", "A"] | bulkSelectionCount > 0; compact labels/count group only with kanbanSelectAllEnabled; commandOptions.kanbanSelectAllEnabled | Flag-gated select all / select column / clear selection `components/commandDispatcher.ts:52` | Omitted, B19 | Omitted, B19 | `components/Modals/commands/HTC/navigationCommands.ts:303` |
| Selected tasks / 6 | `bulkSelectAllInColumn` | "Select all in this column" | — | bulkSelectionCount > 0; compact labels/count group only with kanbanSelectAllEnabled; commandOptions.kanbanSelectAllEnabled | Flag-gated select all / select column / clear selection `components/commandDispatcher.ts:52` | Omitted, B19 | Omitted, B19 | `components/Modals/commands/HTC/navigationCommands.ts:310` |
| Selected tasks / 7 | `bulkClearSelection` | "Clear selection" | ["ESC"] | bulkSelectionCount > 0; compact labels/count group only with kanbanSelectAllEnabled; commandOptions.kanbanSelectAllEnabled | Flag-gated select all / select column / clear selection `components/commandDispatcher.ts:52` | Omitted, B19 | Omitted, B19 | `components/Modals/commands/HTC/navigationCommands.ts:316` |

## Exact missing routes and fields

Session authentication does not accept arbitrary CLI bearers.
Source: `lib/auth/getSessionUser.ts:21-25,70-79`.
The CLI exception lists four named endpoint contracts.
Source: `lib/auth/desktopCliTokenUser.ts:7,53-77`.
Generated SQLite fields do not prove writable bearer fields.
The authoritative update fields remain narrowly listed.
Source: `lib/mcp/tasks/fields/types.ts:50-75`.

| Contract | Exact missing route or field | Current source evidence |
| --- | --- | --- |
| B1 | Authorized Inbox Zero `/api/notifications/inbox-zero/preview` and `/execute`, including rules and `previewVersion`. Cached-ID archive is not equivalent. | `components/notifications/InboxZeroSheet.tsx:97-112`; `hooks/Inbox/useInboxZeroActions.ts:53-78`; `app/api/notifications/inbox-zero/preview/route.ts:14-18`; `app/api/notifications/inbox-zero/execute/route.ts:61-65`. |
| B2 | Authorized false/toggle seen write, `/api/notifications/markAsUnseen`. Bearer `/inbox/seen` only sets true. | `components/generalCommandActions.ts:219-227`; `lib/api/notification-writes/mark-unseen.ts:13-28,35-42,58-75`; `app/api/mcp/inbox/seen/route.ts:79-85`. |
| B3 | Authorized arbitrary-task `/api/queues/inboxReminder`, with `remindTask` and reminder option. Existing bearer duration reminders require notification IDs. | `pages/api/queues/inboxReminder.ts:27-43`; `components/Modals/RemindMe/RemindMeComponent.tsx:86-111`; `lib/mcp/operations/inbox/archive/operation.ts:50-87,105-108`. |
| B4 | Authorized `/api/tasks/searchAll` recent/all-project picker and `/api/tasks/addRelations`. A description link is not a relation. | `lib/api/task-writes/search-all.ts:9-27`; `pages/api/tasks/addRelations.ts:1-5`; `lib/api/task-writes/route.ts:31-38,101-106`; `components/commandModalPanels2.tsx:105-121`. |
| B5 | Authorized `/api/tasks/setStartDate` and `/api/tasks/setRecurrence`. Bearer update lacks both fields. | `pages/api/tasks/setStartDate.ts:1-5`; `lib/api/task-writes/start-date.ts:18-28`; `pages/api/tasks/setRecurrence.ts:1-5`; `lib/api/task-writes/recurrence.ts:19-32`; `lib/mcp/tasks/fields/types.ts:50-75`. |
| B6 | Authorized `/api/task-templates` catalog/save plus exact task creation. | `components/useCommandsState.ts:325-329`; `components/boardCommandActions.ts:39-57`; `app/api/task-templates/route.ts:20-24,72-76`. |
| B7 | Authorized `/api/follower/createFollower`, `/api/follower/unFollowTask`, and follower-row lookup. Bearer follower user IDs are not follower-row IDs. | `pages/api/follower/createFollower.ts:9-25`; `pages/api/follower/unFollowTask.ts:10-29`; `pages/api/follower/getFollower.ts:10-18`; `lib/mcp/tasks/mappers.ts:283-286`. |
| B8 | Authorized `/api/share/createShareLink` GET/POST/PUT and share ID/type/expiry. Private links cannot substitute. | `pages/api/share/createShareLink.ts:12-23`; `hooks/MultiPages/HTC/useHTCTaskAndComments.ts:130-141`; `components/commandDispatcher.ts:775-780`. |
| B9 | Authorized POST `/api/tasks/create` and `/api/tasks/createGlobally`, including exact ranking, defaults, duplicate source and comment handoff. GET defaults is unlocked, not POST. | `lib/api/task-writes/create.ts:20-35`; `lib/api/task-writes/create-global.ts:46-58`; `pages/api/tasks/createGlobally.ts:31,113-115`; `lib/api/task-writes/route.ts:35-38,101-106`. |
| B10 | Authorized personal view reads/writes `/api/projects/views/unsaved-view`, `/update-view`, `/switch-view`, `/create-view`, `/delete-rename-view`; missing active `baseViewId`, filters, visibility, sorting stack and subtask settings. | `lib/constants/APIRouteConstants.ts:33-38`; `hooks/Homepage/Views/useKanbanViews.ts:195-215,257-267,279-356,387-429,436-458,490-527`; `hooks/SubTask/useSubTask.ts:13-35`. |
| B11 | Authorized `/api/projects/staleness`, `/api/projects/auto-archive`, `/api/sections/auto-assign`, `/api/reports/status-update`, velocity report; missing policy and report fields. Personal staleness/sort also need B10. | `components/boardCommandActions.ts:72-75,89-102,127-133,182-187,210-245`; `app/api/projects/staleness/route.ts:6-12`; `app/api/projects/auto-archive/route.ts:8-14`; `lib/mcp/operations/projects/operation.ts:24-43`. |
| B12 | Full column management needs B10 visibility. Tag management lacks equivalent update/delete `/api/labels/updateLabel`; bearer exports only GET/POST creation. Existing individual column forms remain working. | `utils/api/global/apiHelpers/getAllManageColumnsAPI.ts:9`; `utils/api/global/apiHelpers/updateLabelsAPI.ts:3-20`; `app/api/mcp/projects/[projectId]/labels/route.ts:1-4`. |
| B13 | Authorized board invitation `inviteLink`/`key` read/reset `/api/invite/generatePublicInvite`, and board delete contract. Existing rename/archive are not delete. | `components/Modals/commands/inviteMember.tsx:335-356`; `components/Modals/commands/HTC/AllCommands.ts:378-388,415-419`; `app/api/mcp/projects/[projectId]/route.ts:1-3`; `lib/mcp/operations/projects/[projectId]/operation.ts:31-42`. |
| B14 | Authorized `/api/users/preferences` `settings.snippets` catalog/write, plus focused-editor insertion. Bearer profile only accepts name/photo. | `lib/snippets.ts:58-69`; `hooks/General/useSnippetManager.ts:22-35`; `app/api/users/preferences/route.ts:65-85`; `lib/mcp/operations/user/profile/operation.ts:76-84`. |
| B15 | Authorized `/api/time/entries` full-member `canManage`/entries and board `/api/projects/time-tracking`. Bearer report lacks `canManage`; timer start/stop is already working. Report destinations remain outside requested surfaces. | `app/api/time/entries/route.ts:40-58`; `app/api/time/_lib.ts:11-20`; `components/generalCommandActions.ts:125-140`; `lib/mcp/operations/time/report/operation.ts:66-84`. |
| B16 | Complete task agent feed fields `runId`, `type`, `text`, `link`, `options`, `selectedOption`, selection actor/time, and account quiet-activity flag. The local boolean needs no route; toggling an incomplete feed cannot match its result. | `lib/agentRuns/model.ts:64-75`; `lib/agentRuns/taskActivityFeed.ts:30-64`; `hooks/Task Detail/useTaskDetailGlobalStates.ts:207-223`; `pages/api/comments/getByTask.ts:21-27`. |
| B17 | Description version `current.contentText`, `hasMore`, actor display name/type, and exact plain-text preview. Bearer versions provide Markdown content and actor IDs only. Existing restore writer does not supply missing reads. | `components/Modals/TaskDescriptionHistory/TaskDescriptionHistoryModal.tsx:19-30`; `lib/mcp/operations/tasks/description-versions/operation.ts:53-76`. |
| B18 | Authorized `/api/ai/suggest-reply` and `/api/ai/audio-transcript`, plus microphone capture. Text improve cannot replace reply generation. | `components/RTE/useTaskDetailEditorEvents.tsx:166-194`; `app/api/ai/suggest-reply/route.ts:162-170`; `app/api/ai/audio-transcript/route.ts:101-106`. |
| B19 | Authoritative `kanbanSelectAllEnabled` account capability in palette context. Native selection alone cannot establish that flag. Smart split is context-gated and needs B10, not this flag. | `components/Modals/commands/HTC/navigationCommands.ts:301-322`; `components/commandDispatcher.ts:52-59`; `components/Modals/commands/HTC/AllCommands.ts:309-319`; `lib/mcp/operations/user/context/operation.ts:17-35`. |
| B20 | Whole inverse action history, including `/api/notifications/inbox-zero/undo`, synthetic restoration and saved toggles. Native text undo or one inbox inverse is not equivalent. | `components/commandDispatcher.ts:267-273`; `hooks/General/useUndo.tsx:194-220`; `utils/undoActions/helperFuncs.ts:21-68`. |
| B21 | Authoritative account `blockerCommentEnabled` and focused-comment capability. `blockerCommentId` exists; it is not falsely marked absent. | `components/Modals/commands/HTC/taskCommands.ts:228-234`; `components/Modals/commands/HTC/AllCommands.ts:1333-1338`; `lib/mcp/tasks/mappers.ts:242`. |
| B22 | Exact ranked `/api/search/document` engine and archived-search context/results. Native title search does not supply document ranking/attachments. Search handover remains working. | `pages/api/search/document.ts:10-12,30-40`; `app/search/SearchComp.tsx:134-136,270-279`; `components/commandDispatcher.ts:417-421,526-528`. |
| N1 | Focused editor identity and authoritative `editWithAiEnabled`/`commentInlineAi` choices. Writer auth is available, not the blocker. Creation Compose is not focused-description/comment rewrite. | `app/api/ai/task-writer/route.ts:52-61`; `app/detail/[...slug]/useTaskDetailCommentActions.tsx:172-174`; `hooks/Task Detail/useTaskDetailGlobalStates.ts:97-98,448-476`. |
| N2 | View-subtasks needs authorized complete `/api/urls/fetchUrls` links feed. Child removal is now working and has no remaining cold-cache blocker. | `hooks/MultiPages/HTC/useHTCTaskAndComments.ts:223-225`; `components/Modals/LinksModal/index.tsx:83`; `pages/api/urls/fetchUrls.ts:10-24`. |
| N3 | Authorized atomic DELETE `/api/ai-chat/delete-session?delete=ALL`. Enumerating cached UUID sessions cannot promise ALL. | `components/generalCommandActions.ts:110-123`; `app/api/ai-chat/delete-session/route.ts:27-46`; `lib/mcp/ai/sessionHandler.ts:191-200`. |
| Native zoom gap | `boardZoomedOut` state and compact-board renderer. No missing server route claimed. Board layout is explicitly outside this palette-only change. | `components/Modals/commands/HTC/AllCommands.ts:133-144`; `components/commandDispatcher.ts:277-286`. |

### Other partial workflows

- `createTaskWithAiWriter`: generation/default reads work; exact save/rank remains B9.
- `remindMe`: cached positive-ID duration reminders work; arbitrary task/options remain B3.
- `removeTaskNotification`: positive cached notification archive works; exact synthetic inbox restoration/archive and full playlist behavior remain incomplete. Web selects the task's inbox entry and navigates, including flag-dependent synthetic membership. Sources: `app/detail/[...slug]/useTaskDetailCommandActions.tsx:55`; `hooks/Task Detail/useArchiveAndNavigate.ts:53-125`; `lib/mcp/operations/inbox/archive/operation.ts:25-29,50-75`.
- `fullScreenAiChat`: existing native mode opens full screen and preserves workspace; durable session/history remains partial. Missing authorized POST `/api/ai-chat/create-session` and complete server transcript workflow. Sources: `components/commandDispatcher.ts:743-750`; `lib/aiChatDisplayMode.ts:14-22`; `app/api/ai-chat/create-session/route.ts:23-33`. No new AI layout added.
- `searchTask`: navigation works; B22 remains. Neither search command is relabeled as full search parity.

## Historical unresolved-row crosswalk

Every round-2 historical unresolved declaration remains represented.
Round-1 rows resolve through these same current keys.
Previously shipped U1-U10 and round-3 keys remain retained.
X means scope exclusion, not an authorization claim.
Current metadata appears in the main 300-row inventory.
All declaration locations and labels were checked against current AST.

| Historical key | Current disposition | Current web declaration |
| --- | --- | --- |
| `reloadApp` | Excluded destination; no new UI | `components/Modals/commands/HTC/AllCommands.ts:14` |
| `clearInboxToZero` | Omitted, B1 | `components/Modals/commands/HTC/AllCommands.ts:64` |
| `archiveAllReadNotifications` | Omitted, B1 | `components/Modals/commands/HTC/AllCommands.ts:70` |
| `archiveReactionNotifications` | Omitted, B1 | `components/Modals/commands/HTC/AllCommands.ts:76` |
| `createTeam` | Excluded destination; no new UI | `components/Modals/commands/HTC/AllCommands.ts:88` |
| `teamSettings` | Excluded destination; no new UI | `components/Modals/commands/HTC/AllCommands.ts:94` |
| `ManageTeamMembers` | Excluded destination; no new UI | `components/Modals/commands/HTC/AllCommands.ts:101` |
| `billing` | Excluded destination; no new UI | `components/Modals/commands/HTC/AllCommands.ts:108` |
| `manageSubscriptions` | Excluded destination; no new UI | `components/Modals/commands/HTC/AllCommands.ts:115` |
| `manageTeamAIAPIKeys` | Excluded destination; no new UI | `components/Modals/commands/HTC/AllCommands.ts:122` |
| `toggleBoardZoom` | Native gap, boardZoomedOut state and compact-board renderer absent; visual3-owned layout | `components/Modals/commands/HTC/AllCommands.ts:137` |
| `createTask` | Omitted, B9 | `components/Modals/commands/HTC/AllCommands.ts:147` |
| `newTaskFromTemplate` | Omitted, B6 | `components/Modals/commands/HTC/AllCommands.ts:161` |
| `generateStatusUpdate` | Omitted, B11 | `components/Modals/commands/HTC/AllCommands.ts:168` |
| `sortBoard` | Omitted, B10 | `components/Modals/commands/HTC/AllCommands.ts:181` |
| `toggleFilterValueMatch` | Omitted, B10 | `components/Modals/commands/HTC/AllCommands.ts:190` |
| `toggleStaleness` | Omitted, B11 | `components/Modals/commands/HTC/AllCommands.ts:200` |
| `toggleStalenessView` | Omitted, B11 | `components/Modals/commands/HTC/AllCommands.ts:208` |
| `toggleAutoArchive` | Omitted, B11 | `components/Modals/commands/HTC/AllCommands.ts:216` |
| `boardVelocityReport` | Omitted, B11 | `components/Modals/commands/HTC/AllCommands.ts:224` |
| `sortByTimeInColumn` | Omitted, B11 | `components/Modals/commands/HTC/AllCommands.ts:231` |
| `sortByLastComment` | Omitted, B11 | `components/Modals/commands/HTC/AllCommands.ts:237` |
| `autoAssignColumn` | Omitted, B11 | `components/Modals/commands/HTC/AllCommands.ts:247` |
| `manageColumns` | Omitted, B12 | `components/Modals/commands/HTC/AllCommands.ts:255` |
| `hideColumn` | Omitted, B10 | `components/Modals/commands/HTC/AllCommands.ts:267` |
| `goToViews` | Omitted, B10 | `components/Modals/commands/HTC/AllCommands.ts:297` |
| `manageViews` | Omitted, B10 | `components/Modals/commands/HTC/AllCommands.ts:304` |
| `createSmartSplit` | Omitted, B10 | `components/Modals/commands/HTC/AllCommands.ts:312` |
| `toggleBoardLayout` | Excluded destination; no new UI | `components/Modals/commands/HTC/AllCommands.ts:321` |
| `configureTableColumns` | Excluded destination; no new UI | `components/Modals/commands/HTC/AllCommands.ts:328` |
| `manageLabels` | Omitted, B12 | `components/Modals/commands/HTC/AllCommands.ts:334` |
| `createCustomField` | Excluded destination; no new UI | `components/Modals/commands/HTC/AllCommands.ts:340` |
| `manageCustomFields` | Excluded destination; no new UI | `components/Modals/commands/HTC/AllCommands.ts:346` |
| `toggleEmptyColumns` | Omitted, B10 | `components/Modals/commands/HTC/AllCommands.ts:353` |
| `toggleArchivedOnBoard` | Omitted, B10 | `components/Modals/commands/HTC/AllCommands.ts:363` |
| `copyBoardLink` | Omitted, B13 | `components/Modals/commands/HTC/AllCommands.ts:378` |
| `resetBoardLink` | Omitted, B13 | `components/Modals/commands/HTC/AllCommands.ts:384` |
| `manageTeams` | Excluded destination; no new UI | `components/Modals/commands/HTC/AllCommands.ts:390` |
| `createBoard` | Excluded destination; no new UI | `components/Modals/commands/HTC/AllCommands.ts:396` |
| `boardCreationAssistant` | Excluded destination; no new UI | `components/Modals/commands/HTC/AllCommands.ts:402` |
| `deleteBoard` | Omitted, B13 | `components/Modals/commands/HTC/AllCommands.ts:415` |
| `toggleAppShellRail` | Excluded destination; no new UI | `components/Modals/commands/HTC/AllCommands.ts:428` |
| `branchInNewChat` | Working, retained | `components/Modals/commands/HTC/AllCommands.ts:460` |
| `createAgent` | Excluded destination; no new UI | `components/Modals/commands/HTC/AllCommands.ts:472` |
| `manageAgents` | Excluded destination; no new UI | `components/Modals/commands/HTC/AllCommands.ts:478` |
| `disabledAgents` | Excluded destination; no new UI | `components/Modals/commands/HTC/AllCommands.ts:485` |
| `deleteAllChats` | Omitted, N3 | `components/Modals/commands/HTC/AllCommands.ts:491` |
| `systemTheme` | Working | `components/Modals/commands/HTC/AllCommands.ts:503` |
| `profilePicture` | Excluded destination; no new UI | `components/Modals/commands/HTC/AllCommands.ts:533` |
| `switchAccount` | Excluded destination; no new UI | `components/Modals/commands/HTC/AllCommands.ts:540` |
| `manageFavorites` | Excluded destination; no new UI | `components/Modals/commands/HTC/AllCommands.ts:546` |
| `calendarSettings` | Excluded destination; no new UI | `components/Modals/commands/HTC/AllCommands.ts:552` |
| `toggleCalendarWeekends` | Excluded destination; no new UI | `components/Modals/commands/HTC/AllCommands.ts:559` |
| `calendarWeekStartsMonday` | Excluded destination; no new UI | `components/Modals/commands/HTC/AllCommands.ts:565` |
| `calendarWeekStartsSunday` | Excluded destination; no new UI | `components/Modals/commands/HTC/AllCommands.ts:571` |
| `subscribeGoogleCalendar` | Excluded destination; no new UI | `components/Modals/commands/HTC/AllCommands.ts:577` |
| `useSnippet` | Omitted, B14 | `components/Modals/commands/HTC/AllCommands.ts:591` |
| `createSnippet` | Omitted, B14 | `components/Modals/commands/HTC/AllCommands.ts:598` |
| `createSnippetFromDraft` | Omitted, B14 | `components/Modals/commands/HTC/AllCommands.ts:604` |
| `onboardingTour` | Excluded destination; no new UI | `components/Modals/commands/HTC/AllCommands.ts:1257` |
| `latestUpdates` | Excluded destination; no new UI | `components/Modals/commands/HTC/AllCommands.ts:1269` |
| `goToWelcome` | Excluded destination; no new UI | `components/Modals/commands/HTC/AllCommands.ts:1275` |
| `startKanbanTutorial` | Excluded destination; no new UI | `components/Modals/commands/HTC/AllCommands.ts:1281` |
| `startTaskWriterTutorial` | Excluded destination; no new UI | `components/Modals/commands/HTC/AllCommands.ts:1287` |
| `generateMcpToken` | Excluded destination; no new UI | `components/Modals/commands/HTC/AllCommands.ts:1293` |
| `restApi` | Excluded destination; no new UI | `components/Modals/commands/HTC/AllCommands.ts:1307` |
| `logout` | Excluded destination; no new UI | `components/Modals/commands/HTC/AllCommands.ts:1313` |
| `roundedTheme` | Excluded destination; no new UI | `components/Modals/commands/HTC/AllCommands.ts:1358` |
| `sortBoard` | Omitted, B10 | `components/Modals/commands/HTC/AllCommands.ts:181` |
| `ShowFilterHtc` | Omitted, B10 | `components/Modals/commands/HTC/navigationCommands.ts:178` |
| `saveView` | See current runtime Board registry/context | `components/Modals/commands/HTC/AllCommands.ts:1375-1388` |
| `sortBoard` | Omitted, B10 | `components/Modals/commands/HTC/AllCommands.ts:181` |
| `ShowFilterHtc` | Omitted, B10 | `components/Modals/commands/HTC/navigationCommands.ts:178` |
| `undoLatest` | Omitted, B20 | `components/Modals/commands/HTC/navigationCommands.ts:11` |
| `GotoSnippets` | Excluded destination; no new UI | `components/Modals/commands/HTC/navigationCommands.ts:31` |
| `GotoReports` | Excluded destination; no new UI | `components/Modals/commands/HTC/navigationCommands.ts:38` |
| `GoToCalender` | Excluded destination; no new UI | `components/Modals/commands/HTC/navigationCommands.ts:44` |
| `GoToAllTasks` | Excluded destination; no new UI | `components/Modals/commands/HTC/navigationCommands.ts:51` |
| `GoToDueDates` | Excluded destination; no new UI | `components/Modals/commands/HTC/navigationCommands.ts:58` |
| `GoToMyTasks` | Excluded destination; no new UI | `components/Modals/commands/HTC/navigationCommands.ts:65` |
| `GoToDrafts` | Excluded destination; no new UI | `components/Modals/commands/HTC/navigationCommands.ts:73` |
| `GoToStarred` | Excluded destination; no new UI | `components/Modals/commands/HTC/navigationCommands.ts:80` |
| `GoToPinned` | Excluded destination; no new UI | `components/Modals/commands/HTC/navigationCommands.ts:87` |
| `GoToAgents` | Excluded destination; no new UI | `components/Modals/commands/HTC/navigationCommands.ts:94` |
| `GoToAgentChat` | Excluded destination; no new UI | `components/Modals/commands/HTC/navigationCommands.ts:101` |
| `GotoReminders` | Excluded destination; no new UI | `components/Modals/commands/HTC/navigationCommands.ts:107` |
| `GotoTaskArchived` | Excluded destination; no new UI | `components/Modals/commands/HTC/navigationCommands.ts:114` |
| `GotoInboxArchive` | Excluded destination; no new UI | `components/Modals/commands/HTC/navigationCommands.ts:121` |
| `archiveShowActiveBoardsOnly` | Excluded destination; no new UI | `components/Modals/commands/HTC/navigationCommands.ts:128` |
| `archiveShowAllBoards` | Excluded destination; no new UI | `components/Modals/commands/HTC/navigationCommands.ts:134` |
| `archiveShowArchivedBoardsOnly` | Excluded destination; no new UI | `components/Modals/commands/HTC/navigationCommands.ts:140` |
| `GoToProjectInbox` | Excluded destination; no new UI | `components/Modals/commands/HTC/navigationCommands.ts:146` |
| `GoToTrash` | Excluded destination; no new UI | `components/Modals/commands/HTC/navigationCommands.ts:152` |
| `toggleArchivedSearchResults` | Omitted, B22 | `components/Modals/commands/HTC/navigationCommands.ts:161` |
| `ShowFilterHtc` | Omitted, B10 | `components/Modals/commands/HTC/navigationCommands.ts:178` |
| `toggleTimeTracking` | Working, retained | `components/Modals/commands/HTC/navigationCommands.ts:194` |
| `logTimeOnTask` | Omitted, B15 | `components/Modals/commands/HTC/navigationCommands.ts:201` |
| `goToTimeThisTask` | Omitted, B15 | `components/Modals/commands/HTC/navigationCommands.ts:214` |
| `goToTimeThisBoard` | Omitted, B15 | `components/Modals/commands/HTC/navigationCommands.ts:225` |
| `toggleBoardTimeTracking` | Omitted, B15 | `components/Modals/commands/HTC/navigationCommands.ts:231` |
| `GoToTimers` | Excluded destination; no new UI | `components/Modals/commands/HTC/navigationCommands.ts:244` |
| `goToTimeMyWeek` | Excluded destination; no new UI | `components/Modals/commands/HTC/navigationCommands.ts:253` |
| `bulkSelectAllOnBoard` | Omitted, B19 | `components/Modals/commands/HTC/navigationCommands.ts:303` |
| `bulkSelectAllInColumn` | Omitted, B19 | `components/Modals/commands/HTC/navigationCommands.ts:310` |
| `bulkClearSelection` | Omitted, B19 | `components/Modals/commands/HTC/navigationCommands.ts:316` |
| `agentChatNextAgent` | Excluded destination; no new UI | `components/Modals/commands/HTC/navigationCommands.ts:332` |
| `agentChatPreviousAgent` | Excluded destination; no new UI | `components/Modals/commands/HTC/navigationCommands.ts:339` |
| `agentChatSendMessage` | Excluded destination; no new UI | `components/Modals/commands/HTC/navigationCommands.ts:346` |
| `agentChatOpenLinks` | Excluded destination; no new UI | `components/Modals/commands/HTC/navigationCommands.ts:353` |
| `agentChatAddAgent` | Excluded destination; no new UI | `components/Modals/commands/HTC/navigationCommands.ts:360` |
| `agentChatNextTeam` | Excluded destination; no new UI | `components/Modals/commands/HTC/navigationCommands.ts:366` |
| `agentChatPreviousTeam` | Excluded destination; no new UI | `components/Modals/commands/HTC/navigationCommands.ts:373` |
| `appShellTable` | Excluded destination; no new UI | `components/Modals/commands/HTC/navigationCommands.ts:401` |
| `appShellCalendar` | Excluded destination; no new UI | `components/Modals/commands/HTC/navigationCommands.ts:408` |
| `createTask` | Omitted, B9 | `components/Modals/commands/HTC/AllCommands.ts:147` |
| `setStartDate` | Omitted, B5 | `components/Modals/commands/HTC/taskCommands.ts:24` |
| `setRecurrence` | Omitted, B5 | `components/Modals/commands/HTC/taskCommands.ts:31` |
| `saveTaskTemplate` | Omitted, B6 | `components/Modals/commands/HTC/taskCommands.ts:38` |
| `blockedByPerson` | Working, retained | `components/Modals/commands/HTC/taskCommands.ts:52` |
| `addSubTask` | Omitted, B9 | `components/Modals/commands/HTC/taskCommands.ts:66` |
| `viewSubTask` | Omitted, N2 | `components/Modals/commands/HTC/taskCommands.ts:73` |
| `addRelatedTask` | Omitted, B4 | `components/Modals/commands/HTC/taskCommands.ts:80` |
| `markBlockedBy` | Omitted, B4 | `components/Modals/commands/HTC/taskCommands.ts:86` |
| `markAsBlocking` | Omitted, B4 | `components/Modals/commands/HTC/taskCommands.ts:92` |
| `markDuplicateOf` | Omitted, B4 | `components/Modals/commands/HTC/taskCommands.ts:98` |
| `followtask` | Omitted, B7 | `components/Modals/commands/HTC/taskCommands.ts:139` |
| `unfollowtask` | Omitted, B7 | `components/Modals/commands/HTC/taskCommands.ts:146` |
| `openAiTaskWriter` | Omitted, N1 | `components/Modals/commands/HTC/taskCommands.ts:153` |
| `summarizeTicket` | Working, retained | `components/Modals/commands/HTC/taskCommands.ts:159` |
| `markAsUnread` | Omitted, B2 | `components/Modals/commands/HTC/taskCommands.ts:173` |
| `summarizeComment` | Working, retained | `components/Modals/commands/HTC/taskCommands.ts:195` |
| `createTaskFromComment` | Omitted, B9 | `components/Modals/commands/HTC/taskCommands.ts:256` |
| `branchInNewChat` | Working, retained | `components/Modals/commands/HTC/AllCommands.ts:460` |
| `acceptTask` | Working, retained | `components/Modals/commands/HTC/taskCommands.ts:317` |
| `declineTask` | Working, retained | `components/Modals/commands/HTC/taskCommands.ts:323` |
| `declineAsDuplicateOf` | Omitted, B4 | `components/Modals/commands/HTC/taskCommands.ts:329` |
| `duplicateTask` | Omitted, B9 | `components/Modals/commands/HTC/taskCommands.ts:335` |
| `duplicateTaskToBoard` | Omitted, B9 | `components/Modals/commands/HTC/taskCommands.ts:341` |
| `copypublicformattedTaskLink` | Omitted, B8 | `components/Modals/commands/HTC/taskCommands.ts:375` |
| `copyPublicLink` | Omitted, B8 | `components/Modals/commands/HTC/taskCommands.ts:382` |
| `setReminder` | Omitted, B3 | `components/Modals/commands/HTC/taskCommands.ts:421` |
| `starTask` | Working, retained | `components/Modals/commands/HTC/taskCommands.ts:429` |
| `shareTaskPublicly` | Omitted, B8 | `components/Modals/commands/HTC/taskCommands.ts:436` |
| `toggleHistory` | Omitted, B16 | `components/Modals/commands/HTC/taskCommands.ts:471` |
| `taskDescriptionVersions` | Omitted, B17 | `components/Modals/commands/HTC/taskCommands.ts:480` |
| `removeSubtask` | Working; projected children | `components/Modals/commands/HTC/taskCommands.ts:506` |
| `removeParenttask` | Working, retained | `components/Modals/commands/HTC/taskCommands.ts:514` |
| `showInInbox` | Working, retained | `components/Modals/commands/HTC/taskCommands.ts:522` |
| `subtaskSettings` | Omitted, B10 | `components/Modals/commands/HTC/taskCommands.ts:528` |
| `suggestReply` | Omitted, B18 | `components/Modals/commands/HTC/taskCommands.ts:535` |
| `speechToText` | Omitted, B18 | `components/Modals/commands/HTC/taskCommands.ts:544` |

## Verification evidence

GitHub CI alone compiles and runs executable proof.
No VPS build/test/lint/app launch or laptop access.
The initial cold-child native test is committed first.
Exact-head CI and final receipt belong on the draft PR.
PR: https://github.com/hypertask-ai/hypertask-desktop/pull/59.

Native tests:

- `cmdk4_remove_subtask_uses_projected_children_without_loading_their_details`.
- `cmdk4_child_picker_revalidates_relationship_before_enter`.
- `cmdk4_child_removal_rejects_missing_identity_and_conflicting_parent`.
- `cmdk3_remove_child_picker_unlinks_selected_child_not_parent`.

Durable queue and exact-request tests:

- `projected_subtask_removal_survives_restart_and_updates_only_acknowledged_child`.
- `projected_subtask_removal_rejects_foreign_children_and_changed_account`.
- `projected_subtask_removal_retains_parent_on_refusal_or_wrong_acknowledgement`.

Existing complete registry/order/context and palette style tests remain required.
Prove It Works selects exact-head CI and stored results.
Sequence Verifiable Units preserves audit, regression and implementation commits.
Source/diff self-review only; no independent reviewer claimed.
