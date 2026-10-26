# Command center web-to-native inventory

Written before production changes. Web source root: `/home/valentin/projects/hypertask/src`. Registry order comes from `components/Modals/commands/HTC/AllCommands.ts:1300-1356`. Native support means local action support, not authenticated server proof.

| Web command | Label | Web source | Native status before fix / planned gap closure |
| --- | --- | --- | --- |
| `reloadApp` | Reload app | `components/Modals/commands/HTC/AllCommands.ts:14` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `swipeThroughUnread` | Swipe through unread | `components/Modals/commands/HTC/AllCommands.ts:58` | Existing action; audit order, context, label, shortcut, group. |
| `clearInboxToZero` | Clear inbox to zero | `components/Modals/commands/HTC/AllCommands.ts:64` | Blocked. No matching native operation/authorization contract. Omit, never fake. |
| `archiveAllReadNotifications` | Archive all read | `components/Modals/commands/HTC/AllCommands.ts:70` | Blocked. No matching native operation/authorization contract. Omit, never fake. |
| `archiveReactionNotifications` | Archive reactions | `components/Modals/commands/HTC/AllCommands.ts:76` | Blocked. No matching native operation/authorization contract. Omit, never fake. |
| `createTeam` | Create team | `components/Modals/commands/HTC/AllCommands.ts:88` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `teamSettings` | Team settings | `components/Modals/commands/HTC/AllCommands.ts:94` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `ManageTeamMembers` | Manage team members | `components/Modals/commands/HTC/AllCommands.ts:101` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `billing` | Billing | `components/Modals/commands/HTC/AllCommands.ts:108` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `manageSubscriptions` | Manage subscription | `components/Modals/commands/HTC/AllCommands.ts:115` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `manageTeamAIAPIKeys` | Manage API keys | `components/Modals/commands/HTC/AllCommands.ts:122` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `toggleBoardZoom` | Create task | `components/Modals/commands/HTC/AllCommands.ts:137` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `createTask` | Create task | `components/Modals/commands/HTC/AllCommands.ts:147` | Blocked. No matching native operation/authorization contract. Omit, never fake. |
| `createTaskWithAiWriter` | Create task with AI Task Writer | `components/Modals/commands/HTC/AllCommands.ts:154` | Gap. Reuse native Compose AI draft workflow; no claim of live creation. |
| `newTaskFromTemplate` | New task from template | `components/Modals/commands/HTC/AllCommands.ts:161` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `generateStatusUpdate` | Generate status update | `components/Modals/commands/HTC/AllCommands.ts:168` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `sendFeedback` | Send feedback | `components/Modals/commands/HTC/AllCommands.ts:175` | Existing action; audit order, context, label, shortcut, group. |
| `sortBoard` | Sort board | `components/Modals/commands/HTC/AllCommands.ts:181` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `toggleFilterValueMatch` | Filter values: match any or all | `components/Modals/commands/HTC/AllCommands.ts:190` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `toggleStaleness` | Board velocity report | `components/Modals/commands/HTC/AllCommands.ts:200` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `toggleStalenessView` | Board velocity report | `components/Modals/commands/HTC/AllCommands.ts:208` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `toggleAutoArchive` | Board velocity report | `components/Modals/commands/HTC/AllCommands.ts:216` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `boardVelocityReport` | Board velocity report | `components/Modals/commands/HTC/AllCommands.ts:224` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `sortByTimeInColumn` | Sort by time in column | `components/Modals/commands/HTC/AllCommands.ts:231` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `sortByLastComment` | Sort by last comment | `components/Modals/commands/HTC/AllCommands.ts:237` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `autoAssignColumn` | Auto-assign for this column | `components/Modals/commands/HTC/AllCommands.ts:247` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `manageColumns` | Manage board columns | `components/Modals/commands/HTC/AllCommands.ts:255` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `renameColumn` | Rename board column | `components/Modals/commands/HTC/AllCommands.ts:261` | Existing action; audit order, context, label, shortcut, group. |
| `hideColumn` | Hide board column | `components/Modals/commands/HTC/AllCommands.ts:267` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `addColumn` | Add board column | `components/Modals/commands/HTC/AllCommands.ts:273` | Existing action; audit order, context, label, shortcut, group. |
| `deleteColumn` | Delete board column | `components/Modals/commands/HTC/AllCommands.ts:279` | Existing action; audit order, context, label, shortcut, group. |
| `inviteBoard` | Invite people | `components/Modals/commands/HTC/AllCommands.ts:285` | Existing action; audit order, context, label, shortcut, group. |
| `manageMembers` | Manage members | `components/Modals/commands/HTC/AllCommands.ts:291` | Gap. Open existing board-members settings. |
| `goToViews` | Go to views | `components/Modals/commands/HTC/AllCommands.ts:297` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `manageViews` | Manage views | `components/Modals/commands/HTC/AllCommands.ts:304` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `createSmartSplit` | Add smart split | `components/Modals/commands/HTC/AllCommands.ts:312` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `toggleBoardLayout` | Switch to table layout | `components/Modals/commands/HTC/AllCommands.ts:321` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `configureTableColumns` | Table columns | `components/Modals/commands/HTC/AllCommands.ts:328` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `manageLabels` | Manage tags | `components/Modals/commands/HTC/AllCommands.ts:334` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `createCustomField` | Create custom field… | `components/Modals/commands/HTC/AllCommands.ts:340` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `manageCustomFields` | Manage custom fields… | `components/Modals/commands/HTC/AllCommands.ts:346` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `toggleEmptyColumns` | Hide empty board columns | `components/Modals/commands/HTC/AllCommands.ts:353` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `toggleArchivedOnBoard` | ${commandOptions.showArchivedOnBoard ? "Hide" : "Show"} archived tasks on board | `components/Modals/commands/HTC/AllCommands.ts:363` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `renameBoard` | Rename board | `components/Modals/commands/HTC/AllCommands.ts:372` | Existing action; audit order, context, label, shortcut, group. |
| `copyBoardLink` | Copy board join link | `components/Modals/commands/HTC/AllCommands.ts:378` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `resetBoardLink` | Reset board join link | `components/Modals/commands/HTC/AllCommands.ts:384` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `manageTeams` | Manage boards | `components/Modals/commands/HTC/AllCommands.ts:390` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `createBoard` | Create board | `components/Modals/commands/HTC/AllCommands.ts:396` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `boardCreationAssistant` | Board creation assistant | `components/Modals/commands/HTC/AllCommands.ts:402` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `archiveBoards` | Archive board | `components/Modals/commands/HTC/AllCommands.ts:409` | Existing action; audit order, context, label, shortcut, group. |
| `deleteBoard` | Delete board | `components/Modals/commands/HTC/AllCommands.ts:415` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `toggleRailExpanded` | Collapse / expand sidebar | `components/Modals/commands/HTC/AllCommands.ts:421` | Gap. Reuse native sidebar expansion toggle. |
| `toggleAppShellRail` | Chat with AI | `components/Modals/commands/HTC/AllCommands.ts:428` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `aiChatInterface` | Chat with AI | `components/Modals/commands/HTC/AllCommands.ts:442` | Gap. Reuse native AI visibility toggle. |
| `pinAiChatOpen` | Pin AI chat open | `components/Modals/commands/HTC/AllCommands.ts:448` | Existing action; audit order, context, label, shortcut, group. |
| `fullScreenAiChat` | Open AI chat full screen | `components/Modals/commands/HTC/AllCommands.ts:454` | Existing action; audit order, context, label, shortcut, group. |
| `branchInNewChat` | Ask AI about this | `components/Modals/commands/HTC/AllCommands.ts:460` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `toggleAIChatView` | Switch AI chat mode | `components/Modals/commands/HTC/AllCommands.ts:466` | Gap. Reuse native sidebar/floating display modes. |
| `createAgent` | Create agent | `components/Modals/commands/HTC/AllCommands.ts:472` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `manageAgents` | Manage agents | `components/Modals/commands/HTC/AllCommands.ts:478` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `disabledAgents` | Disabled agents | `components/Modals/commands/HTC/AllCommands.ts:485` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `deleteAllChats` | Delete all chat sessions | `components/Modals/commands/HTC/AllCommands.ts:491` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `systemTheme` | Follow system | `components/Modals/commands/HTC/AllCommands.ts:503` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `lightTheme` | Light · Porcelain | `components/Modals/commands/HTC/AllCommands.ts:509` | Existing action; audit order, context, label, shortcut, group. |
| `darkTheme` | Dark · Graphite | `components/Modals/commands/HTC/AllCommands.ts:515` | Existing action; audit order, context, label, shortcut, group. |
| `amoledTheme` | OLED black · AMOLED | `components/Modals/commands/HTC/AllCommands.ts:521` | Existing action; audit order, context, label, shortcut, group. |
| `paperTheme` | Paper · Dia | `components/Modals/commands/HTC/AllCommands.ts:527` | Existing action; audit order, context, label, shortcut, group. |
| `profilePicture` | Profile picture | `components/Modals/commands/HTC/AllCommands.ts:533` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `switchAccount` | Switch account | `components/Modals/commands/HTC/AllCommands.ts:540` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `manageFavorites` | Manage favorites | `components/Modals/commands/HTC/AllCommands.ts:546` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `calendarSettings` | Calendar settings | `components/Modals/commands/HTC/AllCommands.ts:552` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `toggleCalendarWeekends` | Show weekends | `components/Modals/commands/HTC/AllCommands.ts:559` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `calendarWeekStartsMonday` | Week starts on Monday | `components/Modals/commands/HTC/AllCommands.ts:565` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `calendarWeekStartsSunday` | Week starts on Sunday | `components/Modals/commands/HTC/AllCommands.ts:571` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `subscribeGoogleCalendar` | Subscribe in Google Calendar | `components/Modals/commands/HTC/AllCommands.ts:577` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `useSnippet` | Use snippet | `components/Modals/commands/HTC/AllCommands.ts:591` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `createSnippet` | Create snippet | `components/Modals/commands/HTC/AllCommands.ts:598` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `createSnippetFromDraft` | Create snippet from draft | `components/Modals/commands/HTC/AllCommands.ts:604` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `settingsAnnouncementsAnnouncementAlerts` | Settings: Announcement alerts | `components/Modals/commands/HTC/AllCommands.ts:616` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsCalendarKeepTasksUpdated` | Settings: Keep tasks updated | `components/Modals/commands/HTC/AllCommands.ts:623` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsCalendarShowWeekends` | Settings: Show weekends | `components/Modals/commands/HTC/AllCommands.ts:630` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsApiKeysOpenaiApiKey` | Settings: OpenAI API key | `components/Modals/commands/HTC/AllCommands.ts:637` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsApiKeysAnthropicApiKey` | Settings: Anthropic API key | `components/Modals/commands/HTC/AllCommands.ts:644` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsApiKeysGoogleApiKey` | Settings: Google API key | `components/Modals/commands/HTC/AllCommands.ts:651` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsApiKeysDeepseekApiKey` | Settings: DeepSeek API key | `components/Modals/commands/HTC/AllCommands.ts:658` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsApiKeysMoonshotKimiApiKey` | Settings: Moonshot (Kimi) API key | `components/Modals/commands/HTC/AllCommands.ts:665` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsApiKeysZAiGlmApiKey` | Settings: Z.ai (GLM) API key | `components/Modals/commands/HTC/AllCommands.ts:672` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsApiKeysAlibabaQwenApiKey` | Settings: Alibaba (Qwen) API key | `components/Modals/commands/HTC/AllCommands.ts:679` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsApiKeysOpenrouterApiKey` | Settings: OpenRouter API key | `components/Modals/commands/HTC/AllCommands.ts:686` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsApiKeysVercelAiGatewayApiKey` | Settings: Vercel AI Gateway API key | `components/Modals/commands/HTC/AllCommands.ts:693` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsApiKeysCustomEndpointOperatesUnderAnEuUsDataAgreement` | Settings: Custom endpoint EU/US agreement | `components/Modals/commands/HTC/AllCommands.ts:700` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsAiModelsGdprSafeMode` | Settings: GDPR safe mode | `components/Modals/commands/HTC/AllCommands.ts:707` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsAiModelsOpenai` | Settings: OpenAI | `components/Modals/commands/HTC/AllCommands.ts:714` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsAiModelsAnthropic` | Settings: Anthropic | `components/Modals/commands/HTC/AllCommands.ts:721` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsAiModelsGoogle` | Settings: Google | `components/Modals/commands/HTC/AllCommands.ts:728` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsAiModelsDeepseek` | Settings: DeepSeek | `components/Modals/commands/HTC/AllCommands.ts:735` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsAiModelsMoonshotKimi` | Settings: Moonshot (Kimi) | `components/Modals/commands/HTC/AllCommands.ts:742` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsAiModelsZAiGlm` | Settings: Z.ai (GLM) | `components/Modals/commands/HTC/AllCommands.ts:749` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsAiModelsAlibabaQwen` | Settings: Alibaba (Qwen) | `components/Modals/commands/HTC/AllCommands.ts:756` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsAiModelsOpenrouter` | Settings: OpenRouter | `components/Modals/commands/HTC/AllCommands.ts:763` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsAiFeaturesAiChat` | Settings: AI chat | `components/Modals/commands/HTC/AllCommands.ts:770` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsAiFeaturesTaskWriter` | Settings: Task writer | `components/Modals/commands/HTC/AllCommands.ts:777` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsAiFeaturesWriteWithAi` | Settings: Write with AI | `components/Modals/commands/HTC/AllCommands.ts:784` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsAiFeaturesImproveWriting` | Settings: Improve writing | `components/Modals/commands/HTC/AllCommands.ts:791` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsAiFeaturesAskAi` | Settings: Ask AI | `components/Modals/commands/HTC/AllCommands.ts:798` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsAiFeaturesBoardGeneration` | Settings: Board generation | `components/Modals/commands/HTC/AllCommands.ts:805` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsAiFeaturesImageGeneration` | Settings: Image generation | `components/Modals/commands/HTC/AllCommands.ts:812` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsAiFeaturesHyperai` | Settings: @HyperAI | `components/Modals/commands/HTC/AllCommands.ts:819` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsAiFeaturesTaskSummaries` | Settings: Task summaries | `components/Modals/commands/HTC/AllCommands.ts:826` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsAiFeaturesQuestionSuggestions` | Settings: Question suggestions | `components/Modals/commands/HTC/AllCommands.ts:833` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsAiFeaturesDictation` | Settings: Dictation | `components/Modals/commands/HTC/AllCommands.ts:840` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsBoardMemoryLearnFromAiCorrections` | Settings: Learn from AI corrections | `components/Modals/commands/HTC/AllCommands.ts:847` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsBoardGeneralBoardNotifications` | Settings: Board notifications | `components/Modals/commands/HTC/AllCommands.ts:854` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsBoardGeneralShowTotalTimeOnTasks` | Settings: Show total time on tasks | `components/Modals/commands/HTC/AllCommands.ts:861` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsBoardPlanningEnableCycles` | Settings: Enable cycles | `components/Modals/commands/HTC/AllCommands.ts:868` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsBoardStalenessStaleness` | Settings: Staleness | `components/Modals/commands/HTC/AllCommands.ts:875` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsBoardStalenessStaleReminders` | Settings: Stale reminders | `components/Modals/commands/HTC/AllCommands.ts:882` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsBoardStalenessAutoArchive` | Settings: Auto-archive | `components/Modals/commands/HTC/AllCommands.ts:889` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsNotificationsPushNotificationsOnThisDevice` | Settings: Push Notifications on this device | `components/Modals/commands/HTC/AllCommands.ts:896` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsNotificationsMentionsNotifications` | Settings: Mentions notifications | `components/Modals/commands/HTC/AllCommands.ts:903` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsNotificationsCommentsNotifications` | Settings: Comments notifications | `components/Modals/commands/HTC/AllCommands.ts:910` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsNotificationsAssignmentsNotifications` | Settings: Assignments notifications | `components/Modals/commands/HTC/AllCommands.ts:917` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsNotificationsTaskUpdatesNotifications` | Settings: Task updates notifications | `components/Modals/commands/HTC/AllCommands.ts:924` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsNotificationsDueDatesNotifications` | Settings: Due dates notifications | `components/Modals/commands/HTC/AllCommands.ts:931` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsTaskPageOpenAiChatByDefault` | Settings: Open AI chat by default | `components/Modals/commands/HTC/AllCommands.ts:938` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsTaskPageCollapseComments` | Settings: Collapse Comments | `components/Modals/commands/HTC/AllCommands.ts:945` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsTaskPagePlayGifs` | Settings: Play GIFs | `components/Modals/commands/HTC/AllCommands.ts:952` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsTaskPageSuggestDescriptionsFromTaskTitles` | Settings: Suggest descriptions from task titles | `components/Modals/commands/HTC/AllCommands.ts:959` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsTaskPageShowTaskHistory` | Settings: Show task history | `components/Modals/commands/HTC/AllCommands.ts:966` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsTaskPageReadReceipts` | Settings: Read receipts | `components/Modals/commands/HTC/AllCommands.ts:973` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsInboxArchiveAndGoToNextTaskAfterSendingAComment` | Settings: Archive after commenting | `components/Modals/commands/HTC/AllCommands.ts:980` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsInboxDisplayAvatarInInbox` | Settings: Display avatar in inbox | `components/Modals/commands/HTC/AllCommands.ts:987` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsLearnQuickTips` | Settings: Quick tips | `components/Modals/commands/HTC/AllCommands.ts:994` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsGeneral` | Settings: General | `components/Modals/commands/HTC/AllCommands.ts:1001` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsAppearance` | Settings: Appearance | `components/Modals/commands/HTC/AllCommands.ts:1008` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsTaskPage` | Settings: Task page | `components/Modals/commands/HTC/AllCommands.ts:1015` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsNotifications` | Settings: Notifications | `components/Modals/commands/HTC/AllCommands.ts:1022` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsAccounts` | Settings: Accounts | `components/Modals/commands/HTC/AllCommands.ts:1029` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsBilling` | Settings: Billing | `components/Modals/commands/HTC/AllCommands.ts:1036` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsPlans` | Settings: Plans | `components/Modals/commands/HTC/AllCommands.ts:1043` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsAiUsage` | Settings: AI usage | `components/Modals/commands/HTC/AllCommands.ts:1050` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsMemberUsage` | Settings: Usage by member | `components/Modals/commands/HTC/AllCommands.ts:1057` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsAiModels` | Settings: AI models | `components/Modals/commands/HTC/AllCommands.ts:1064` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsAiFeatures` | Settings: AI features | `components/Modals/commands/HTC/AllCommands.ts:1071` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsDefaultModels` | Settings: Default models | `components/Modals/commands/HTC/AllCommands.ts:1078` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsApiKeys` | Settings: Bring your own key | `components/Modals/commands/HTC/AllCommands.ts:1085` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsMembers` | Settings: Members | `components/Modals/commands/HTC/AllCommands.ts:1092` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsTeamAgents` | Settings: Agents (team) | `components/Modals/commands/HTC/AllCommands.ts:1099` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsSkills` | Settings: Skills | `components/Modals/commands/HTC/AllCommands.ts:1106` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsBoardGeneral` | Settings: Time tracking (board) | `components/Modals/commands/HTC/AllCommands.ts:1113` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsBoardTicketPrefix` | Settings: Ticket prefix (board) | `components/Modals/commands/HTC/AllCommands.ts:1120` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsBoardMembers` | Settings: Members (board) | `components/Modals/commands/HTC/AllCommands.ts:1127` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsBoardAgents` | Settings: Agents (board) | `components/Modals/commands/HTC/AllCommands.ts:1134` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsBoardAi` | Settings: Custom instructions | `components/Modals/commands/HTC/AllCommands.ts:1141` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsBoardSkills` | Settings: Skills (board) | `components/Modals/commands/HTC/AllCommands.ts:1148` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsBoardFiles` | Settings: Files | `components/Modals/commands/HTC/AllCommands.ts:1155` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsSlack` | Settings: Slack | `components/Modals/commands/HTC/AllCommands.ts:1162` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsMcp` | Settings: MCP | `components/Modals/commands/HTC/AllCommands.ts:1169` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsCli` | Settings: CLI | `components/Modals/commands/HTC/AllCommands.ts:1176` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsApi` | Settings: REST API | `components/Modals/commands/HTC/AllCommands.ts:1183` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsShortcuts` | Settings: Shortcuts | `components/Modals/commands/HTC/AllCommands.ts:1190` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settingsLearn` | Settings: Learn Hypertask | `components/Modals/commands/HTC/AllCommands.ts:1197` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `shortcuts` | Keyboard shortcuts | `components/Modals/commands/HTC/AllCommands.ts:1210` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `settings` | Settings | `components/Modals/commands/HTC/AllCommands.ts:1217` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `boardSettings` | Board settings | `components/Modals/commands/HTC/AllCommands.ts:1224` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `helpCenter` | Help center | `components/Modals/commands/HTC/AllCommands.ts:1230` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `onboardingTour` | Onboarding tour | `components/Modals/commands/HTC/AllCommands.ts:1236` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `quickTips` | Quick tips | `components/Modals/commands/HTC/AllCommands.ts:1242` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `latestUpdates` | What's new | `components/Modals/commands/HTC/AllCommands.ts:1248` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `goToWelcome` | Welcome tutorial | `components/Modals/commands/HTC/AllCommands.ts:1254` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `startKanbanTutorial` | Start Kanban tutorial | `components/Modals/commands/HTC/AllCommands.ts:1260` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `startTaskWriterTutorial` | Start task writer tutorial | `components/Modals/commands/HTC/AllCommands.ts:1266` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `generateMcpToken` | MCP | `components/Modals/commands/HTC/AllCommands.ts:1272` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `cliInstall` | CLI | `components/Modals/commands/HTC/AllCommands.ts:1279` | Existing supported settings/help subset retained; unsupported destinations omitted. Outside new feature scope. |
| `restApi` | REST API | `components/Modals/commands/HTC/AllCommands.ts:1286` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `logout` | Sign out | `components/Modals/commands/HTC/AllCommands.ts:1292` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `roundedTheme` | Material (follows device) | `components/Modals/commands/HTC/AllCommands.ts:1328` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `sortBoard` | Dynamic / inherited | `components/Modals/commands/HTC/AllCommands.ts:1361` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `ShowFilterHtc` | Dynamic / inherited | `components/Modals/commands/HTC/AllCommands.ts:1362` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `saveView` | Dynamic / inherited | `components/Modals/commands/HTC/AllCommands.ts:1368` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `sortBoard` | Dynamic / inherited | `components/Modals/commands/HTC/AllCommands.ts:1378` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `ShowFilterHtc` | Dynamic / inherited | `components/Modals/commands/HTC/AllCommands.ts:1379` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `undoLatest` | Undo latest action | `components/Modals/commands/HTC/navigationCommands.ts:11` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `copyCurrentPageUrl` | Copy current page URL | `components/Modals/commands/HTC/navigationCommands.ts:18` | Existing action; audit order, context, label, shortcut, group. |
| `GotoInbox` | Go to inbox | `components/Modals/commands/HTC/navigationCommands.ts:24` | Existing action; audit order, context, label, shortcut, group. |
| `GotoSnippets` | Go to snippets | `components/Modals/commands/HTC/navigationCommands.ts:31` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `GotoReports` | Go to reports | `components/Modals/commands/HTC/navigationCommands.ts:38` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `GoToCalender` | Go to calendar | `components/Modals/commands/HTC/navigationCommands.ts:44` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `GoToAllTasks` | Go to all tasks | `components/Modals/commands/HTC/navigationCommands.ts:51` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `GoToDueDates` | Go to scheduled tasks | `components/Modals/commands/HTC/navigationCommands.ts:58` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `GoToMyTasks` | Go to my tasks | `components/Modals/commands/HTC/navigationCommands.ts:65` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `GoToDrafts` | Go to drafts | `components/Modals/commands/HTC/navigationCommands.ts:73` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `GoToStarred` | Go to starred | `components/Modals/commands/HTC/navigationCommands.ts:80` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `GoToPinned` | Go to pinned | `components/Modals/commands/HTC/navigationCommands.ts:87` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `GoToAgents` | Go to agents | `components/Modals/commands/HTC/navigationCommands.ts:94` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `GoToAgentChat` | Agent Chat | `components/Modals/commands/HTC/navigationCommands.ts:101` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `GotoReminders` | Go to reminders | `components/Modals/commands/HTC/navigationCommands.ts:107` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `GotoTaskArchived` | Go to archived tasks | `components/Modals/commands/HTC/navigationCommands.ts:114` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `GotoInboxArchive` | Go to archived inbox | `components/Modals/commands/HTC/navigationCommands.ts:121` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `archiveShowActiveBoardsOnly` | Archive: show active boards only | `components/Modals/commands/HTC/navigationCommands.ts:128` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `archiveShowAllBoards` | Archive: show all boards | `components/Modals/commands/HTC/navigationCommands.ts:134` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `archiveShowArchivedBoardsOnly` | Archive: show archived boards only | `components/Modals/commands/HTC/navigationCommands.ts:140` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `GoToProjectInbox` | Go to project inbox | `components/Modals/commands/HTC/navigationCommands.ts:146` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `GoToTrash` | Go to trash | `components/Modals/commands/HTC/navigationCommands.ts:152` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `toggleArchivedSearchResults` | ${commandOptions.searchOptions.includeArchived ? "Hide" : "Show"} archived search results | `components/Modals/commands/HTC/navigationCommands.ts:161` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `searchTask` | Search | `components/Modals/commands/HTC/navigationCommands.ts:170` | Existing action; audit order, context, label, shortcut, group. |
| `ShowFilterHtc` | Filters | `components/Modals/commands/HTC/navigationCommands.ts:178` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `toggleTimeTracking` | Start or stop timer | `components/Modals/commands/HTC/navigationCommands.ts:194` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `logTimeOnTask` | Log time | `components/Modals/commands/HTC/navigationCommands.ts:201` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `goToTimeThisTask` | Time: this task | `components/Modals/commands/HTC/navigationCommands.ts:214` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `goToTimeThisBoard` | Time: this board | `components/Modals/commands/HTC/navigationCommands.ts:225` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `toggleBoardTimeTracking` | Toggle time tracking for this board | `components/Modals/commands/HTC/navigationCommands.ts:231` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `GoToTimers` | Time: running timers | `components/Modals/commands/HTC/navigationCommands.ts:244` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `goToTimeMyWeek` | Time: my week | `components/Modals/commands/HTC/navigationCommands.ts:253` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `bulkMoveToColumn` | Select all on board | `components/Modals/commands/HTC/navigationCommands.ts:274` | Existing action; audit order, context, label, shortcut, group. |
| `bulkAssignUser` | Select all on board | `components/Modals/commands/HTC/navigationCommands.ts:281` | Existing action; audit order, context, label, shortcut, group. |
| `bulkSetLabels` | Select all on board | `components/Modals/commands/HTC/navigationCommands.ts:288` | Existing action; audit order, context, label, shortcut, group. |
| `bulkArchive` | Select all on board | `components/Modals/commands/HTC/navigationCommands.ts:295` | Existing action; audit order, context, label, shortcut, group. |
| `bulkSelectAllOnBoard` | Select all on board | `components/Modals/commands/HTC/navigationCommands.ts:303` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `bulkSelectAllInColumn` | Select all in this column | `components/Modals/commands/HTC/navigationCommands.ts:310` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `bulkClearSelection` | Clear selection | `components/Modals/commands/HTC/navigationCommands.ts:316` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `agentChatNextAgent` | Next agent | `components/Modals/commands/HTC/navigationCommands.ts:332` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `agentChatPreviousAgent` | Previous agent | `components/Modals/commands/HTC/navigationCommands.ts:339` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `agentChatSendMessage` | Send message | `components/Modals/commands/HTC/navigationCommands.ts:346` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `agentChatOpenLinks` | Open all links in latest reply | `components/Modals/commands/HTC/navigationCommands.ts:353` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `agentChatAddAgent` | Add agent | `components/Modals/commands/HTC/navigationCommands.ts:360` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `agentChatNextTeam` | Next team | `components/Modals/commands/HTC/navigationCommands.ts:366` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `agentChatPreviousTeam` | Previous team | `components/Modals/commands/HTC/navigationCommands.ts:373` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `appShellInbox` | App shell: Inbox | `components/Modals/commands/HTC/navigationCommands.ts:387` | Existing action; audit order, context, label, shortcut, group. |
| `appShellBoard` | App shell: Board | `components/Modals/commands/HTC/navigationCommands.ts:394` | Existing action; audit order, context, label, shortcut, group. |
| `appShellTable` | App shell: Table | `components/Modals/commands/HTC/navigationCommands.ts:401` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `appShellCalendar` | App shell: Calendar | `components/Modals/commands/HTC/navigationCommands.ts:408` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `appShellAIChat` | App shell: Toggle AI chat | `components/Modals/commands/HTC/navigationCommands.ts:415` | Gap. Reuse native AI visibility toggle. |
| `appShellAIChatAlternative` | App shell: Toggle AI chat (alternative) | `components/Modals/commands/HTC/navigationCommands.ts:422` | Gap. Reuse native AI visibility toggle. |
| `createTask` | Create task | `components/Modals/commands/HTC/taskCommands.ts:9` | Blocked. No matching native operation/authorization contract. Omit, never fake. |
| `setDueDate` | Set due date | `components/Modals/commands/HTC/taskCommands.ts:16` | Existing action; audit order, context, label, shortcut, group. |
| `setStartDate` | Set start date | `components/Modals/commands/HTC/taskCommands.ts:23` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `setRecurrence` | Repeat task | `components/Modals/commands/HTC/taskCommands.ts:30` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `saveTaskTemplate` | Save task as template | `components/Modals/commands/HTC/taskCommands.ts:37` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `assignUser` | Assign task | `components/Modals/commands/HTC/taskCommands.ts:44` | Existing action; audit order, context, label, shortcut, group. |
| `blockedByPerson` | Blocked by person… | `components/Modals/commands/HTC/taskCommands.ts:51` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `assignToMe` | Assign to me | `components/Modals/commands/HTC/taskCommands.ts:58-63` | Added for active task context with cached account membership. Durable explicit-assign intent preserves other people and agents, then opens the assignment picker. |
| `addSubTask` | Create subtask | `components/Modals/commands/HTC/taskCommands.ts:65` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `viewSubTask` | View subtasks | `components/Modals/commands/HTC/taskCommands.ts:72` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `addRelatedTask` | Add related task | `components/Modals/commands/HTC/taskCommands.ts:79` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `markBlockedBy` | Mark blocked by | `components/Modals/commands/HTC/taskCommands.ts:85` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `markAsBlocking` | Mark as blocking | `components/Modals/commands/HTC/taskCommands.ts:91` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `markDuplicateOf` | Mark duplicate of | `components/Modals/commands/HTC/taskCommands.ts:97` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `labelmodal` | Set tags | `components/Modals/commands/HTC/taskCommands.ts:103` | Existing action; audit order, context, label, shortcut, group. |
| `estimateModal` | Set task size | `components/Modals/commands/HTC/taskCommands.ts:110` | Existing action; audit order, context, label, shortcut, group. |
| `priorityModal` | Set priority | `components/Modals/commands/HTC/taskCommands.ts:117` | Existing action; audit order, context, label, shortcut, group. |
| `moveToColumn` | Move task to column | `components/Modals/commands/HTC/taskCommands.ts:124` | Existing action; audit order, context, label, shortcut, group. |
| `movetasktodifferentboard` | Move task to board | `components/Modals/commands/HTC/taskCommands.ts:131` | Gap. Correct native moveTaskToBoard key and label; existing move form. |
| `followtask` | Follow task | `components/Modals/commands/HTC/taskCommands.ts:138` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `unfollowtask` | Unfollow task | `components/Modals/commands/HTC/taskCommands.ts:145` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `openAiTaskWriter` | Open AI task writer | `components/Modals/commands/HTC/taskCommands.ts:152` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `summarizeTicket` | Summarize ticket | `components/Modals/commands/HTC/taskCommands.ts:158` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `remindMe` | Remind me | `components/Modals/commands/HTC/taskCommands.ts:165` | Gap. Reuse native inbox reminder picker and queue. |
| `markAsUnread` | Mark unread | `components/Modals/commands/HTC/taskCommands.ts:172` | Blocked. No matching native operation/authorization contract. Omit, never fake. |
| `editcomment` | Edit comment | `components/Modals/commands/HTC/taskCommands.ts:187` | Existing action; audit order, context, label, shortcut, group. |
| `summarizeComment` | Summarize comment | `components/Modals/commands/HTC/taskCommands.ts:194` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `fastLikeComment` | Fast like | `components/Modals/commands/HTC/taskCommands.ts:201` | Existing action; audit order, context, label, shortcut, group. |
| `replyToComment` | Reply to comment | `components/Modals/commands/HTC/taskCommands.ts:207` | Existing action; audit order, context, label, shortcut, group. |
| `reactToComment` | React to comment | `components/Modals/commands/HTC/taskCommands.ts:214` | Existing action; audit order, context, label, shortcut, group. |
| `pinComment` | ${commentProps.isPinned ? "Unpin" : "Pin"} comment | `components/Modals/commands/HTC/taskCommands.ts:221` | Existing action; audit order, context, label, shortcut, group. |
| `copyCommentContent` | Copy comment content | `components/Modals/commands/HTC/taskCommands.ts:228` | Existing action; audit order, context, label, shortcut, group. |
| `copyCommentLink` | Copy comment URL | `components/Modals/commands/HTC/taskCommands.ts:234` | Existing action; audit order, context, label, shortcut, group. |
| `starComment` | ${commentProps.isStarred ? "Unstar" : "Star"} comment | `components/Modals/commands/HTC/taskCommands.ts:240` | Existing action; audit order, context, label, shortcut, group. |
| `createTaskFromComment` | Create task from comment | `components/Modals/commands/HTC/taskCommands.ts:247` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `branchInNewChat` | Branch in new chat | `components/Modals/commands/HTC/taskCommands.ts:253` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `copyCommentToAiChat` | Copy comment to AI chat | `components/Modals/commands/HTC/taskCommands.ts:260` | Existing action; audit order, context, label, shortcut, group. |
| `deletemessage` | Delete comment | `components/Modals/commands/HTC/taskCommands.ts:267` | Gap. Correct native deleteComment key and label; existing owner-only delete action. |
| `renameTask` | Rename task | `components/Modals/commands/HTC/taskCommands.ts:298` | Existing action; audit order, context, label, shortcut, group. |
| `acceptTask` | Accept task | `components/Modals/commands/HTC/taskCommands.ts:306` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `declineTask` | Decline task | `components/Modals/commands/HTC/taskCommands.ts:312` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `declineAsDuplicateOf` | Decline as duplicate of… | `components/Modals/commands/HTC/taskCommands.ts:318` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `duplicateTask` | Duplicate task | `components/Modals/commands/HTC/taskCommands.ts:324` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `duplicateTaskToBoard` | Copy task to another board | `components/Modals/commands/HTC/taskCommands.ts:330` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `archiveTask` | ${taskProps?.isArchived ? "Unarchive" : "Archive"} task | `components/Modals/commands/HTC/taskCommands.ts:336` | Existing action; audit order, context, label, shortcut, group. |
| `deleteTask` | Delete task | `components/Modals/commands/HTC/taskCommands.ts:343` | Existing action; audit order, context, label, shortcut, group. |
| `copyformattedTaskLink` | Copy private task link (title + URL) | `components/Modals/commands/HTC/taskCommands.ts:350` | Existing action; audit order, context, label, shortcut, group. |
| `copyLink` | Copy private task URL | `components/Modals/commands/HTC/taskCommands.ts:357` | Existing action; audit order, context, label, shortcut, group. |
| `copypublicformattedTaskLink` | Copy public task link (title + URL) | `components/Modals/commands/HTC/taskCommands.ts:364` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `copyPublicLink` | Copy public task URL | `components/Modals/commands/HTC/taskCommands.ts:371` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `setReminder` | Set reminder | `components/Modals/commands/HTC/taskCommands.ts:410` | Blocked. Native reminder targets inbox notifications, not arbitrary tasks. Omit rather than expose a no-op. |
| `starTask` | ${taskProps?.isStarred ? "Unstar" : "Star"} task | `components/Modals/commands/HTC/taskCommands.ts:418` | Blocked. No matching native operation/authorization contract. Omit, never fake. |
| `shareTaskPublicly` | Share task | `components/Modals/commands/HTC/taskCommands.ts:425` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `copyTaskId` | Copy task ID | `components/Modals/commands/HTC/taskCommands.ts:438` | Existing action; audit order, context, label, shortcut, group. |
| `copyTaskTitleId` | Copy task title and ID | `components/Modals/commands/HTC/taskCommands.ts:445` | Existing action; audit order, context, label, shortcut, group. |
| `copyBranchName` | Copy branch name | `components/Modals/commands/HTC/taskCommands.ts:452` | Gap. Implement from loaded ticket/title using the web branch-name utility. |
| `toggleHistory` | ${taskProps?.showHistory ? "Hide" : "Show"} agent log | `components/Modals/commands/HTC/taskCommands.ts:460` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `taskDescriptionVersions` | Description version history | `components/Modals/commands/HTC/taskCommands.ts:469` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `toggleAllComments` | Expand / collapse all comments | `components/Modals/commands/HTC/taskCommands.ts:477` | Gap. Correct native expandCollapseAllComments key and label; existing expand action. |
| `removeTaskNotification` | Remove task notification | `components/Modals/commands/HTC/taskCommands.ts:486` | Gap. Existing inbox archive action supports tasks with cached notifications. |
| `removeSubtask` | Remove a sub-task | `components/Modals/commands/HTC/taskCommands.ts:495` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `removeParenttask` | Remove as sub-task | `components/Modals/commands/HTC/taskCommands.ts:503` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `showInInbox` | Move task to inbox | `components/Modals/commands/HTC/taskCommands.ts:511` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `subtaskSettings` | Sub-task Settings | `components/Modals/commands/HTC/taskCommands.ts:517` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `suggestReply` | Suggest reply | `components/Modals/commands/HTC/taskCommands.ts:524` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |
| `speechToText` | Speech to text | `components/Modals/commands/HTC/taskCommands.ts:533` | Unsupported or outside inbox/board/task/AI-sidebar/shortcuts scope. No matching native action; omit. |

## Native-only rows removed or corrected

- `composeTask` / New Task becomes the web board AI-writer command.
- `moveTaskToBoard` becomes `movetasktodifferentboard`.
- `expandCollapseAllComments` becomes `toggleAllComments`.
- `deleteComment` becomes `deletemessage`.
- Comment actions use the Comment group, not Task.
- App shell uses App shell surfaces, not App shell.

## Presentation sources

- Desktop rows have no icons. `components/Modals/commands/HTC/CommandList.tsx:33-45` renders icons only on mobile.
- Rows are 36px, 6px horizontal margins, 12px padding, 2px radius, 12px gap. `CommandList.tsx:35`.
- Keycaps use 5px/2px padding and 3px gaps. `CommandList.tsx:74-112`.
- New badge uses purple, 5px/2px padding. `CommandList.tsx:68-71`.
- Group headings use 16px padding, 10px top, 4px bottom, uppercase. Scroll height is 364px. `CommandGroup.tsx:23-26`.
- Modal uses 560px minimum width, top 24%, 5px radius. Search icon is 13px. `commands.tsx:781-804`.
- Board insertion follows the first context group, or Comment plus Task. `commands.tsx:308-315`.
- Usage sorts within groups. Frequently used/Get started apply outside Task context; selected tasks suppress recommendations. `commands.tsx:315-390`.
- AI toggle, pin, fullscreen, mode actions. `components/commandDispatcher.ts:724-754`.
- Sidebar expansion action. `components/commandDispatcher.ts:462-465`.
- AI writer opens an AI description editor. `components/commandDispatcher.ts:639-648`. Native Compose remains a partial offline equivalent.

## Remaining limits

Unsupported web commands stay omitted. The inventory does not claim complete web parity. Fullscreen is a native display mode, not web routing. No live account credentials or remote delivery were tested. Paired web/native pixel identity remains unproven.

## Implemented local gaps

- Final context ordering includes all producers. Task fields, comments, board forms, and AI rows no longer split their groups.
- Corrected web command keys, labels, keycap spelling, and App shell surfaces group.
- Added AI chat toggle, sidebar/floating mode, both AI shell commands, sidebar expansion, board member settings, and inbox reminder rows.
- Swipe through unread remains available in all supported contexts. Its existing native action reads the inbox and preserves the origin.
- Remove task notification appears only with a cached notification. Arbitrary task reminders remain blocked.
- Board navigation uses `gotoBoard-ID` and `Go to board: TITLE`, excludes the current board, and caps at 50. `components/Modals/commands/HTC/commands.tsx:207-215`.
- Usage sorts inside each group. Task context suppresses recommendation groups. Bulk context leads and suppresses recommendations. Other contexts use Frequently used or the supported Get started subset.
- Desktop keeps the header search icon and no mobile-only row icons. Added supported New badges and sidebar-aware centering.
- Purple is `#4455BB`, also explicit in `components/PageComponents/Trial/TrialCard.tsx:65-72`. New badge uses `components/Modals/commands/HTC/CommandList.tsx:68-71`.
- Rail shortcut is `[`. `lib/constants/railToggleKey.ts:1`.
- Header is 44px. `components/Common/CommonModalComponents/index.tsx:141`. Footer geometry is `index.tsx:211-221`.

## Regression evidence

Before fix, [CI 40236357225](https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40236357225) at `384298cbf6c495155341e9c7bd6b725b6425948c` fails `cmdk_task_commands_match_web_order_labels_and_groups` on Linux, Windows, and macOS. The macOS result is 181 passed, 1 failed. Actual first row is native `expandCollapseAllComments`; expected first row is web `assignUser`. The output also exposes the wrong move key, rename keycap, and copy keycaps.

The table records pre-fix support. Implemented gaps above still require exact-head CI. Saved-comment toggle labels use cached `savedContent`; remote refresh remains required after pending actions. AI writer remains a native draft workflow, not the web creation modal. Full pixel identity is not claimed.

## Additional supported state

- Pin/Unpin and Star/Unstar labels read cached comment `savedContent`, matching `hooks/Task Detail/useTaskDetailGlobalStates.ts:692-703`.
- Copy branch name uses the loaded ticket and title. The ASCII slug, 50-character limit, and trailing-dash removal match `utils/branchName.ts:1-13`. The clipboard action matches `components/generalCommandActions.ts:61-77` and `components/commandDispatcher.ts:786-788`.
- Card-command clicks retain Task context without turning ordinary Board Ctrl+K into a task menu. Web context selects Task when a current card exists. `components/PageComponents/Kanban/KanbanHomepageComponents/Homepage.tsx:775-789`.
- Archived task assignment/column edits remain omitted because the native operation rejects archived targets. The web registry exposes them; this capability gap is not promoted to parity.
- Comment-to-AI requires a cached creator ID. Without that input, the native quote action cannot run, so no inert row is shown.

## CI-discovered data boundaries

The first fix run catches a missing `uniqueIndex` panic in comment-command
assembly. Optional cached links now use `get`, and the comment URL row stays
absent without its input. The archived-task notification expectation was wrong:
`crates/ht-core/src/db.rs:382-383` intentionally excludes archived tasks from
native inbox rows. Tests now cover the notification command while active and
its omission after archiving. No core filtering rule was changed.

The initial fix run is https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40237015308.
Its macOS result is 187 passed, two failed. Both failures have targeted input
coverage in the follow-up. GPUI heading letter spacing has no native style
property; web `tracking-wider` remains a presentation gap. No exact pixel
parity claim is made.


## Remaining-gap continuation

Acceptance is `docs/gaps-acceptance.md`. Source re-read at web revision
`10377b20e8c338357fda306b025ef16699c9b4ab`.

- Assign to me follows `components/generalCommandActions.ts:80-93`: add the
  current person, then open assignment. Existing assignment is not toggled off.
  Unknown account, uncached membership, archived task and bulk context omit the
  row instead of advertising an action that cannot run. The New badge matches
  `components/Modals/commands/HTC/taskCommands.ts:58-63`.
- Self assignment sends only task_id, user_id and intent=assign to the existing
  bearer `/assignees/assign` route, not a replacement human list. Server
  `lib/mcp/operations/assignees/assign/operation.ts:348-395` validates membership
  and applies explicit intent. This preserves people added since the local
  cache was read. Its acknowledgement retains the full human/agent list.
  Restart, denial rollback and concurrent uncached-person tests cover this
  contract. It does not claim a live authenticated server round trip.
- Full create and plain quick entry remain server-blocked, not unknown rules.
  `docs/creation-parity.md` records exact filter precedence, validated defaults,
  target selection, rank generation, both save routes and HTPR-7699's narrow
  authorization boundary. GET defaults does not authorize POST createGlobally.
- The three additional bulk-selection commands are feature-flagged in
  `components/Modals/commands/HTC/navigationCommands.ts:301-322`. Without the
  account's authoritative flag, their presence is not inferred from the native
  keyboard selection capability. The existing unflagged four-command group stays.
- Existing B1-B11 server/read-model blockers and scope exclusions are unchanged.
  Native links/history/upload flows and command-heading letter spacing remain
  documented gaps, not silently relabeled as implemented or CI-proven parity.

The regression uses real Ctrl+K input and observes assignment counts, retained
people/agents, the durable payload, repeated-action idempotence and picker state.
Negative input cases cover unknown identity, absent membership and archives.
Exact-head CI evidence belongs in `docs/gaps-evidence.md`.

## Round 2 re-audit

The historical Blocked/Unsupported rows above are not current verdicts.
See [cmdk-round2.md](cmdk-round2.md) for every unresolved declaration,
exact handlers/routes, bearer comparisons, native gaps, and exclusions.
Ten command keys now reuse native operations: person blocker,
star/unstar, timer toggle, accept, decline, move to inbox, parent unlink,
comment branch, ticket summary, and comment summary.
Unknown cached state stays omitted; creation remains HTPR-7699.
