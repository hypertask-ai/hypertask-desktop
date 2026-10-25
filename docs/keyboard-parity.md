# Web keyboard shortcut parity

Web source root is `/home/valentin/projects/hypertask/src`.
Citations below are relative to that root.
`Ctrl/Cmd` means the platform command key.
Rows retain help-registry duplicates and conflicting labels.
Runtime handlers take precedence over help text.
Native means local input and queue behavior only.
It never means live server acceptance.

Acceptance and exact-head proof live in `GATES-keys2.md`.
B references identify `docs/p2-parity.md` blocker rows.
Formatting limits remain in `docs/editor-formatting.md`.
AI transport limits remain in `docs/ai-parity.md`.
Unsupported actions never silently perform different actions.

## Web registry

Source is `lib/constants/shortcuts.ts`.
Flags add shell, compose and legacy variants.
Excluded destinations remain labeled, not silently omitted.

| Surface | Web shortcut | Web action | Native status | Web source |
| --- | --- | --- | --- | --- |
| App shell surfaces | `1` | Inbox | Native shell destination. | `lib/constants/shortcuts.ts:54` |
| App shell surfaces | `2` | Board | Native shell destination. | `lib/constants/shortcuts.ts:55` |
| App shell surfaces | `3` | Table | Native 3 navigation; table editing outside scope. | `lib/constants/shortcuts.ts:56` |
| App shell surfaces | `4` | Calendar | Outside scope; no native destination. | `lib/constants/shortcuts.ts:57` |
| App shell surfaces | `5` | Toggle AI chat | Native AI panel toggle; OEM ? alias included. | `lib/constants/shortcuts.ts:58` |
| App shell surfaces | `` | Toggle AI chat (alternative) | Native AI panel toggle; OEM ? alias included. | `lib/constants/shortcuts.ts:59` |
| App shell surfaces | `6` | Search | Partial. Opens cached task search, not server document search. B8. | `lib/constants/shortcuts.ts:60` |
| App shell surfaces | `7` | Running timers | Outside scope; no native destination. | `lib/constants/shortcuts.ts:61` |
| App shell surfaces | `[` | Collapse / expand sidebar | Native [ rail toggle. | `lib/constants/shortcuts.ts:62` |
| Global Shortcuts | `?` | Shortcuts | Native searchable shortcut sidebar. | `lib/constants/shortcuts.ts:73` |
| Global Shortcuts | `\\` | Settings | Native settings; outside this task. | `lib/constants/shortcuts.ts:74` |
| Global Shortcuts | `/` | Search | Partial. Opens cached task search, not server document search. B8. | `lib/constants/shortcuts.ts:75` |
| Global Shortcuts | `P` | Set priority | Native picker and durable field write; cached catalogs only. | `lib/constants/shortcuts.ts:76` |
| Global Shortcuts | `S` | Set task size | Native picker and durable field write; cached catalogs only. | `lib/constants/shortcuts.ts:77` |
| Global Shortcuts | `T` | Add tags | Native picker and durable field write; cached catalogs only. | `lib/constants/shortcuts.ts:78` |
| Global Shortcuts | `E` | Archive | Native notification archive; not task completion. | `lib/constants/shortcuts.ts:79` |
| Global Shortcuts | `J` | Move focus down | Native arrows/J/K; task J/K advances/reverses playlist. | `lib/constants/shortcuts.ts:80` |
| Global Shortcuts | `K` | Move focus up | Native arrows/J/K; task J/K advances/reverses playlist. | `lib/constants/shortcuts.ts:81` |
| Global Shortcuts | `←+⭡+⭣+→` | Move focus | Native arrows/J/K; task J/K advances/reverses playlist. | `lib/constants/shortcuts.ts:82` |
| Global Shortcuts | `Ctrl/Cmd+K` | Open Hypertask Command | Native command center; cmdk branch owns further parity. | `lib/constants/shortcuts.ts:83` |
| Global Shortcuts | `Ctrl/Cmd+J` | Compose task / New Task | Native flagged AI compose note. Not plain quick entry. | `lib/constants/shortcuts.ts:84` |
| Global Shortcuts | `Ctrl/Cmd+Z` | Undo latest action | Editor undo supported. Global action undo unavailable. B9. | `lib/constants/shortcuts.ts:85` |
| Global Shortcuts | `;` | Use snippet | Blocked. No authoritative personal snippet catalog. B8. | `lib/constants/shortcuts.ts:86` |
| Global Shortcuts | `G then X` | Show or hide archived tasks | Blocked. Missing active-view visibility contract. B2. | `lib/constants/shortcuts.ts:87` |
| Global Shortcuts | `ESC` | Exit (go back) | Native closes editor/modal and returns to originating selection. | `lib/constants/shortcuts.ts:88` |
| Global Shortcuts | `Ctrl/Cmd+E` | Archive task or comment | Native task Archive. Help overstates comment archive; no such entity inferred. | `lib/constants/shortcuts.ts:89` |
| Global Shortcuts | `#` | Delete task | Native task delete confirmation and durable status write. | `lib/constants/shortcuts.ts:90` |
| Global Shortcuts | `G then B` | Go to task board | Native selected-task board and inbox navigation. | `lib/constants/shortcuts.ts:91` |
| Global Shortcuts | `G then R` | Go to inbox archives | Outside scope; no matching native destination. B9. | `lib/constants/shortcuts.ts:92` |
| Global Shortcuts | `G then T` | Go to running timers | Outside scope; no matching native destination. B9. | `lib/constants/shortcuts.ts:93` |
| Global Shortcuts | `G then H` | Go to reminders | Outside scope; no matching native destination. B9. | `lib/constants/shortcuts.ts:94` |
| Global Shortcuts | `G then #` | Go to trash | Outside scope; no matching native destination. B9. | `lib/constants/shortcuts.ts:95` |
| Global Shortcuts | `G then A` | Go to all tasks | Outside scope; no matching native destination. B9. | `lib/constants/shortcuts.ts:96` |
| Global Shortcuts | `G then M` | Go to my tasks | Outside scope; no matching native destination. B9. | `lib/constants/shortcuts.ts:97` |
| Global Shortcuts | `G then I` | Go to inbox | Native selected-task board and inbox navigation. | `lib/constants/shortcuts.ts:98` |
| Global Shortcuts | `G then ;` | Go to snippets | Outside scope; no matching native destination. B9. | `lib/constants/shortcuts.ts:99` |
| Global Shortcuts | `G then E` | Go to archived tasks | Outside scope; no matching native destination. B9. | `lib/constants/shortcuts.ts:100` |
| Global Shortcuts | `G then S` | Go to starred | Outside scope; no matching native destination. B9. | `lib/constants/shortcuts.ts:101` |
| Global Shortcuts | `G then P` | Go to pinned | Outside scope; no matching native destination. B9. | `lib/constants/shortcuts.ts:102` |
| Global Shortcuts | `G then D` | Go to drafts | Outside scope; no matching native destination. B9. | `lib/constants/shortcuts.ts:103` |
| Global Shortcuts | `G then U` | Go to scheduled tasks | Outside scope; no matching native destination. B9. | `lib/constants/shortcuts.ts:104` |
| Global Shortcuts | `G then C` | Go to calendar | Outside scope; no matching native destination. B9. | `lib/constants/shortcuts.ts:105` |
| Global Shortcuts | `TAB` | Cycle focus forward | Native split, column or task-part cycling. | `lib/constants/shortcuts.ts:106` |
| Global Shortcuts | `Shift+TAB` | Cycle focus backward | Native split, column or task-part cycling. | `lib/constants/shortcuts.ts:107` |
| Global Shortcuts | `Alt (Mac Ctrl)+0-9` | Switch boards | Blocked. Favorite ordering absent; never use arbitrary cached board order. B8. | `lib/constants/shortcuts.ts:108` |
| Global Shortcuts | `Ctrl/Cmd+B` | Switch Kanban boards | Native searchable board switcher; cached boards only. | `lib/constants/shortcuts.ts:109` |
| Global Shortcuts | `D` | Set due date | Native picker and durable field write; cached catalogs only. | `lib/constants/shortcuts.ts:110` |
| Global Shortcuts | `Shift+M` | Move task to a different board | Native board then column picker; durable bearer MoveTask. | `lib/constants/shortcuts.ts:111` |
| Board | `C` | Add task | Blocked. Full create lifecycle, active-view defaults and ranking unavailable. B3/B11. Shift+C runtime quick entry is not compose. | `lib/constants/shortcuts.ts:117` |
| Board | `Ctrl/Cmd+J` | Add task with AI Task Writer | Partial. Native compose alternative; legacy description-AI create form unavailable. B11. | `lib/constants/shortcuts.ts:118` |
| Board | `Ctrl/Cmd+Shift++` | Add sub-task | Blocked. Full create lifecycle, active-view defaults and ranking unavailable. B3/B11. Shift+C runtime quick entry is not compose. | `lib/constants/shortcuts.ts:119` |
| Board | `Ctrl/Cmd+Shift+C` | Add task at top | Blocked. Full create lifecycle, active-view defaults and ranking unavailable. B3/B11. Shift+C runtime quick entry is not compose. | `lib/constants/shortcuts.ts:120` |
| Board | `Shift+C` | Add task at bottom | Blocked. Full create lifecycle, active-view defaults and ranking unavailable. B3/B11. Shift+C runtime quick entry is not compose. | `lib/constants/shortcuts.ts:121` |
| Board | `ENTER` | Open task | Native Enter opens selected cached task. | `lib/constants/shortcuts.ts:122` |
| Board | `` | Next view | Legacy shell only. Saved-view cycling unavailable. B4. | `lib/constants/shortcuts.ts:126` |
| Board | `[` | Previous view | Legacy shell only. Saved-view cycling unavailable. B4. | `lib/constants/shortcuts.ts:127` |
| Board | `gg` | Move focus to top | Native gg and Shift+G G boundaries. | `lib/constants/shortcuts.ts:129` |
| Board | `GG` | Move focus to bottom | Native gg and Shift+G G boundaries. | `lib/constants/shortcuts.ts:130` |
| Board | `Shift+J, ⭣` | Move task down | Blocked. Bearer task ranking unavailable. B1. | `lib/constants/shortcuts.ts:131` |
| Board | `Shift +K, ⭡` | Move task up | Blocked. Bearer task ranking unavailable. B1. | `lib/constants/shortcuts.ts:132` |
| Board | `Shift+L, →` | Move task right | Native neighboring visible column; durable sectionId write. | `lib/constants/shortcuts.ts:133` |
| Board | `Shift +H, ←` | Move task left | Native neighboring visible column; durable sectionId write. | `lib/constants/shortcuts.ts:134` |
| Board | `Shift+H, ←` | Move task to the left | Native neighboring visible column; durable sectionId write. | `lib/constants/shortcuts.ts:135` |
| Board | `Shift+S` | Sort Kanban board | Blocked. Missing active-view sort contract. B2/B4. | `lib/constants/shortcuts.ts:136` |
| Board | `Shift+CLICK` | Add a sort level (table column) | Outside scope. Table sort write unavailable. B2. | `lib/constants/shortcuts.ts:137` |
| Board | `Shift+T` | Toggle table layout | Added Shift+T for existing native board/table. Saved layout mutation remains B4. | `lib/constants/shortcuts.ts:138` |
| Board | `←+→` | Match any / all values (inside a filter's list) | Blocked. Authoritative active-view filter state unavailable. B2. | `lib/constants/shortcuts.ts:139` |
| Board | `Ctrl/Cmd+Shift++` | Show task options | Native commands shortcut. Task subtask-linking handler differs and remains unavailable. | `lib/constants/shortcuts.ts:140` |
| Board | `Shift+L, →` | Move task to the right | Native neighboring visible column; durable sectionId write. | `lib/constants/shortcuts.ts:141` |
| Board | `A` | Assign a user | Native picker and durable field write; cached catalogs only. | `lib/constants/shortcuts.ts:142` |
| Task View | `ENTER` | Activate description/comment or reply to comment | Native Enter activates selected task part. | `lib/constants/shortcuts.ts:148` |
| Task View | `Ctrl/Cmd+ENTER` | Send comment and advance in Inbox | Native durable send and inbox advance. | `lib/constants/shortcuts.ts:150` |
| Task View | `Ctrl/Cmd+Shift+ENTER` | Send comment and stay on task | Native durable send without advancing. | `lib/constants/shortcuts.ts:153` |
| Task View | `Ctrl/Cmd+J` | Write with AI | Partial. Native compose alternative; legacy description-AI create form unavailable. B11. | `lib/constants/shortcuts.ts:154` |
| Task View | `A` | Assign a user | Native picker and durable field write; cached catalogs only. | `lib/constants/shortcuts.ts:155` |
| Task View | `Shift+B` | Blocked by person | Blocked. Person-blocking model/write unavailable. B10. | `lib/constants/shortcuts.ts:156` |
| Task View | `W` | Start / stop timer | Blocked. Board time-enabled state and entry flow unavailable. B10. | `lib/constants/shortcuts.ts:157` |
| Task View | `B` | Log time | Blocked. Board time-enabled state and entry flow unavailable. B10. | `lib/constants/shortcuts.ts:158` |
| Task View | `Ctrl/Cmd+M` | Add a comment | Native composer focus from inbox, board or task. | `lib/constants/shortcuts.ts:159` |
| Task View | `Shift+R` | Suggest reply with AI | Blocked. Existing improve does not generate replies. B10. | `lib/constants/shortcuts.ts:160` |
| Task View | `Alt (Mac Ctrl)+0-9` | Switch boards | Blocked. Favorite ordering absent; never use arbitrary cached board order. B8. | `lib/constants/shortcuts.ts:161` |
| Task View | `I` | Expand summary | Native full saved summary; no AI request. | `lib/constants/shortcuts.ts:162` |
| Task View | `O` | Expand single comment | Native comment expansion; latest remains open after collapse-all. | `lib/constants/shortcuts.ts:163` |
| Task View | `Shift+O` | Expand all comments | Native comment expansion; latest remains open after collapse-all. | `lib/constants/shortcuts.ts:164` |
| Task View | `Ctrl/Cmd+Shift+H` | Toggle history events / agent log | Blocked. Cached comments omit merged agent activity feed and history preference. B10. | `lib/constants/shortcuts.ts:165` |
| Task View | `Ctrl/Cmd+O` | View all links | Missing native links control and complete web link feed. B10. | `lib/constants/shortcuts.ts:166` |
| Task View | `Ctrl/Cmd+M` | Write Comment | Native composer focus from inbox, board or task. | `lib/constants/shortcuts.ts:167` |
| Task View | `Ctrl+D` | Edit Description | Native Control+D specifically, including Mac Control; not Cmd+D. | `lib/constants/shortcuts.ts:168` |
| Task View | `F2` | Edit Title | Native F2 title editor. | `lib/constants/shortcuts.ts:169` |
| Task View | `Ctrl+Shift+3` | Delete comment | Native own-comment delete confirmation. | `lib/constants/shortcuts.ts:170` |
| Task View | `Ctrl/Cmd+Shift+V` | Paste as plain text | Native plain clipboard paste without link wrapping. | `lib/constants/shortcuts.ts:171` |
| Task View | `Ctrl+O` | View all sub-tasks | Help conflict. Actual Ctrl+O opens links; subtask view unavailable. B11. | `lib/constants/shortcuts.ts:172` |
| Task View | `Ctrl/Cmd+S` | Share task | Blocked. Authoritative public share contract unavailable. B8. | `lib/constants/shortcuts.ts:173` |
| Task View | `Ctrl/Cmd+Shift++` | Show task options | Native commands shortcut. Task subtask-linking handler differs and remains unavailable. | `lib/constants/shortcuts.ts:174` |
| Task View | `Ctrl/Cmd+Shift+;` | Private share link (URL only) | Native clipboard from cached uniqueIndex; refuses absent URL. | `lib/constants/shortcuts.ts:175` |
| Task View | `Ctrl/Cmd+;` | Private share link (Title + URL) | Native clipboard from cached uniqueIndex; refuses absent URL. | `lib/constants/shortcuts.ts:176` |
| Task View | `Ctrl/Cmd+Shift+.` | Public share link (URL only) | Blocked. Authoritative public share contract unavailable. B8. | `lib/constants/shortcuts.ts:177` |
| Task View | `Ctrl/Cmd+.` | Public share link (Title + URL) | Blocked. Authoritative public share contract unavailable. B8. | `lib/constants/shortcuts.ts:178` |
| Task View | `Ctrl/Cmd+Shift+I` | Copy task ID | Native ticket/title clipboard. | `lib/constants/shortcuts.ts:179` |
| Task View | `Ctrl/Cmd+I` | Copy task title and ID | Native ticket/title clipboard. | `lib/constants/shortcuts.ts:180` |
| Task View | `ALT+S` | Star task | Blocked. Saved-task toggle unavailable; not comment saved state. B8. | `lib/constants/shortcuts.ts:181` |
| Task View | `Ctrl+Shift+S` | Star comment | Native saved-comment write. | `lib/constants/shortcuts.ts:182` |
| Task View | `Ctrl+Shift+P` | Pin comment | Native saved-comment write. | `lib/constants/shortcuts.ts:183` |
| Task View | `F` | Follow task | Blocked. Follower write unavailable. B8. | `lib/constants/shortcuts.ts:184` |
| Task View | `ALT+F` | Unfollow task | Blocked. Follower write unavailable. B8. | `lib/constants/shortcuts.ts:185` |
| Task View | `Ctrl/Cmd+Shift+D` | Speech to text | Blocked. Native microphone upload unavailable. B11. | `lib/constants/shortcuts.ts:186` |
| Task View | `Ctrl/Cmd+Shift+F` | Dictate and Improve | Blocked. Native microphone upload unavailable. B11. | `lib/constants/shortcuts.ts:187` |
| Task View | `Ctrl/Cmd+Shift+,` | Discard draft | Native current editor draft discard. | `lib/constants/shortcuts.ts:188` |
| Saving new task | `Ctrl/Cmd+ENTER` | Save | Blocked new-task form. Comment/editor saving is separate. B11. | `lib/constants/shortcuts.ts:194` |
| Saving new task | `Ctrl/Cmd+Alt/Opt+ENTER` | Save and close | Blocked new-task form. Comment/editor saving is separate. B11. | `lib/constants/shortcuts.ts:195` |
| Saving new task | `Ctrl/Cmd+Alt/Opt+Shift+ENTER` | Save and create new task | Blocked new-task form. Comment/editor saving is separate. B11. | `lib/constants/shortcuts.ts:196` |
| AI Chat | `Ctrl/Cmd+Shift+?` | Toggle AI chat | Native AI panel toggle; OEM ? alias included. | `lib/constants/shortcuts.ts:202` |
| AI Chat | `Ctrl/Cmd+Shift+O` | Reset session | Added workspace dispatch and Cmd alias; existing local new-chat action. | `lib/constants/shortcuts.ts:203` |
| AI Chat | `Ctrl+Q` | Switch focus: AI chat / workspace | Native exact Ctrl+Q focus toggle. | `lib/constants/shortcuts.ts:204` |
| AI Chat | `Enter` | Send message | Native AI sidebar Enter-to-send. | `lib/constants/shortcuts.ts:205` |
| Agent Chat | `Alt/Opt+⭣` | Next agent | Outside scope; agent room, not AI sidebar. | `lib/constants/shortcuts.ts:211` |
| Agent Chat | `Alt/Opt+⭡` | Previous agent | Outside scope; agent room, not AI sidebar. | `lib/constants/shortcuts.ts:212` |
| Agent Chat | `Ctrl/Cmd+ENTER` | Send message | Outside scope; agent room, not AI sidebar. | `lib/constants/shortcuts.ts:213` |
| Agent Chat | `Ctrl+O` | Open all links in latest reply | Outside scope; agent room, not AI sidebar. | `lib/constants/shortcuts.ts:214` |
| Agent Chat | `@` | Search tasks to reference | Outside scope; agent room, not AI sidebar. | `lib/constants/shortcuts.ts:215` |
| Agent Chat | `Alt/Opt+Shift+⭣` | Next team (any page) | Outside scope; agent room, not AI sidebar. | `lib/constants/shortcuts.ts:216` |
| Agent Chat | `Alt/Opt+Shift+⭡` | Previous team (any page) | Outside scope; agent room, not AI sidebar. | `lib/constants/shortcuts.ts:217` |
| Format | `/` | Tiptap menu | Native editor slash formatting catalog. | `lib/constants/shortcuts.ts:223` |
| Format | `Ctrl/Cmd+Y` | Add Video Embed | Partial. Native video URL dialog and HTML insertion; no player rendering. | `lib/constants/shortcuts.ts:224` |
| Format | `2xTAB` | Autocomplete sentence | Blocked AI completion; no invented Tab result. | `lib/constants/shortcuts.ts:225` |
| Format | `Shift+ENTER` | Add a line break | Native rich editor action. See editor-formatting.md limits. | `lib/constants/shortcuts.ts:226` |
| Format | `Ctrl/Cmd+I` | Italicize | Native rich editor action. See editor-formatting.md limits. | `lib/constants/shortcuts.ts:227` |
| Format | `Ctrl/Cmd+U` | Underline | Native rich editor action. See editor-formatting.md limits. | `lib/constants/shortcuts.ts:228` |
| Format | `Ctrl/Cmd+Shift+B` | Quote | Native rich editor action. See editor-formatting.md limits. | `lib/constants/shortcuts.ts:229` |
| Format | `Ctrl/Cmd+Shift+H` | Highlight | Native rich editor action. See editor-formatting.md limits. | `lib/constants/shortcuts.ts:230` |
| Format | `Ctrl/Cmd+E` | Code | Native rich editor action. See editor-formatting.md limits. | `lib/constants/shortcuts.ts:231` |
| Format | `Ctrl/Cmd+Shift+8` | Bullet list | Native rich editor action. See editor-formatting.md limits. | `lib/constants/shortcuts.ts:232` |
| Format | `Ctrl/Cmd+Z` | Undo | Native rich editor action. See editor-formatting.md limits. | `lib/constants/shortcuts.ts:233` |
| Format | `Ctrl/Cmd+Shift+Z` | Redo | Native rich editor action. See editor-formatting.md limits. | `lib/constants/shortcuts.ts:234` |
| Format | `Ctrl/Cmd+Alt/Opt+0` | Apply normal text style | Native rich editor action. See editor-formatting.md limits. | `lib/constants/shortcuts.ts:235` |
| Format | `Ctrl/Cmd+Alt/Opt+1` | Apply heading style 1 | Native rich editor heading. | `lib/constants/shortcuts.ts:236` |
| Format | `Ctrl/Cmd+Alt/Opt+2` | Apply heading style 2 | Native rich editor heading. | `lib/constants/shortcuts.ts:237` |
| Format | `Ctrl/Cmd+Alt/Opt+3` | Apply heading style 3 | Native rich editor heading. | `lib/constants/shortcuts.ts:238` |
| Format | `Ctrl/Cmd+Alt/Opt+4` | Apply heading style 4 | Native rich editor heading. | `lib/constants/shortcuts.ts:239` |
| Format | `Ctrl/Cmd+Alt/Opt+5` | Apply heading style 5 | Native rich editor heading. | `lib/constants/shortcuts.ts:240` |
| Format | `Ctrl/Cmd+Alt/Opt+6` | Apply heading style 6 | Native rich editor heading. | `lib/constants/shortcuts.ts:241` |
| Format | `Ctrl/Cmd+Alt/Opt+Shift+C` | Code block | Native rich editor action. See editor-formatting.md limits. | `lib/constants/shortcuts.ts:242` |
| Format | `Ctrl/Cmd+Shift+C` | Code | Native rich editor action. See editor-formatting.md limits. | `lib/constants/shortcuts.ts:243` |
| Inbox | `Ctrl/Cmd+Shift+A` | Select ALL | Native inbox selection; all/current split stay distinct. | `lib/constants/shortcuts.ts:249` |
| Inbox | `Ctrl/Cmd+A` | Select all from current split | Native inbox selection; all/current split stay distinct. | `lib/constants/shortcuts.ts:250` |
| Inbox | `X` | Select message | Native inbox selection; all/current split stay distinct. | `lib/constants/shortcuts.ts:251` |
| Inbox | `Shift+E` | Bulk Archive | Native selected notification action; existing limits preserved. | `lib/constants/shortcuts.ts:252` |
| Inbox | `Shift+H` | Bulk Reminder | Native selected notification action; existing limits preserved. | `lib/constants/shortcuts.ts:253` |
| Inbox | `E` | Archive | Native notification archive; not task completion. | `lib/constants/shortcuts.ts:254` |
| Inbox | `ENTER` | Open task | Native Enter opens selected cached task. | `lib/constants/shortcuts.ts:255` |
| Inbox | `J` | Move focus down | Native arrows/J/K; task J/K advances/reverses playlist. | `lib/constants/shortcuts.ts:256` |
| Inbox | `K` | Move focus up | Native arrows/J/K; task J/K advances/reverses playlist. | `lib/constants/shortcuts.ts:257` |
| Inbox | `Ctrl/Cmd+E` | Archive task | Native task status Archive, not notification archive. | `lib/constants/shortcuts.ts:258` |

## Footer hints

Footer labels come from `components/Global/ConstantsQuickTips.ts`.
Quick add appears on the board's add button.
Hints do not prove native support.

| Surface | Web shortcut | Footer action | Native status | Web source |
| --- | --- | --- | --- | --- |
| Inbox | Ctrl+K | command center | Supported | `components/Global/ConstantsQuickTips.ts:138` |
| Inbox | J | down | Supported | `components/Global/ConstantsQuickTips.ts:142` |
| Inbox | K | up | Supported | `components/Global/ConstantsQuickTips.ts:146` |
| Inbox | Enter | open task | Supported | `components/Global/ConstantsQuickTips.ts:150` |
| Inbox | Tab | cycle inbox splits | Supported | `components/Global/ConstantsQuickTips.ts:154` |
| Inbox | E | Archive | Notification only | `components/Global/ConstantsQuickTips.ts:158` |
| Inbox | Ctrl+E | Mark task as done | Task only | `components/Global/ConstantsQuickTips.ts:162` |
| Board | Ctrl+K | command center | Supported | `components/Global/ConstantsQuickTips.ts:33` |
| Board | J | down | Supported | `components/Global/ConstantsQuickTips.ts:37` |
| Board | K | up | Supported | `components/Global/ConstantsQuickTips.ts:41` |
| Board | Tab | next column | Supported legacy hint. Shell view-cycle differs below. | `components/Global/ConstantsQuickTips.ts:45` |
| Board | C | add task | Blocked B3/B11, not AI compose | `components/Global/ConstantsQuickTips.ts:49` |
| Board | N / Shift+C | quick add | Blocked B3; not silently mapped to compose | `components/PageComponents/Kanban/KanbanSectionComponents/NewTaskButton.tsx:54` |
| Board | Ctrl+E | archive | Supported task archive | `components/Global/ConstantsQuickTips.ts:53` |
| Board | Ctrl+B | Change Board | Supported cached board picker | `components/Global/ConstantsQuickTips.ts:57` |
| Board | / | search | Partial cached search | `components/Global/ConstantsQuickTips.ts:61` |
| Task | Ctrl+K | command center | Supported | `components/Global/ConstantsQuickTips.ts:68` |
| Task | A | assign | Supported | `components/Global/ConstantsQuickTips.ts:72` |
| Task | M | change status(move) | Supported | `components/Global/ConstantsQuickTips.ts:76` |
| Task | P | priority | Supported | `components/Global/ConstantsQuickTips.ts:80` |
| Task | S | task size | Supported | `components/Global/ConstantsQuickTips.ts:84` |
| Task | T | tags | Supported | `components/Global/ConstantsQuickTips.ts:88` |
| Task | Ctrl+E | archive | Supported | `components/Global/ConstantsQuickTips.ts:92` |
| Task | Escape | back | Supported originating selection | `components/Global/ConstantsQuickTips.ts:96` |
| Task | Tab | cycle | Supported task-part focus | `components/Global/ConstantsQuickTips.ts:100` |

## Additional runtime bindings and conflicts

Actual handlers include keys absent from help.
Conflicting hints stay explicit below.

| Surface | Web shortcut | Runtime action | Native status | Web source |
| --- | --- | --- | --- | --- |
| Board | N / Shift+C | Quick entry in focused/selected column, bottom | Blocked B3. Active-view defaults and exact ranking unavailable. | `hooks/Homepage/useHandleKeyDownOperations.ts:93-118`, `hooks/Homepage/useSections.ts:108-141`, `hooks/MultiPages/useAddDeleteTaskInBoards.tsx:115-164` |
| Board | C, Ctrl/Cmd+Shift+C | Full create at top | Blocked B3/B11; plain bearer create cannot preserve defaults/ranking | `hooks/Homepage/useSections.ts:263-310` |
| Inbox/task | C | Full create task | Blocked B11; not compose | `app/inbox/Inbox.tsx:395-398`, `app/detail/[...slug]/taskDetailEditingKeymap.ts:197-209` |
| Board | Shift+F | Filter commands | Blocked active-view filters B2 | `hooks/Homepage/useHandleKeyDownOperations.ts:186` |
| Board | Shift+V | Save changed view | Blocked unsaved-view state/write B2/B4 | `hooks/Homepage/useHandleKeyDownOperations.ts:178-180` |
| Board | G then V | Views picker | Blocked full rendered view order B4 | `hooks/Homepage/useHandleKeyDownOperations.ts:209-223` |
| Board | G then X | Toggle archived visibility | Blocked B2 | `hooks/Homepage/useHandleKeyDownOperations.ts:226-231` |
| Board | Tab / Shift+Tab | New shell cycles rendered saved views | Partial. Native retains hinted column focus; no guessed saved views. | `hooks/Homepage/Views/useViewCyclingShortcuts.ts:22-51` |
| Board | [ / ] | Legacy view cycling | Blocked B4; new shell reuses [ rail, ] AI | `hooks/Homepage/Views/useViewCyclingShortcuts.ts:22-24`, `hooks/Homepage/useAppShellSurfaceShortcuts.ts:25,119-127` |
| Board | H/Left, L/Right | Previous/next column focus | Supported | `hooks/Homepage/useHandleKeyDownOperations.ts:203-206,239-243` |
| Board | J/Down, K/Up | Down/up card focus | Supported | `hooks/Homepage/useHandleKeyDownOperations.ts:191-200` |
| Board | Ctrl/Cmd+F | Find in current view | Supported title/ticket search only | `hooks/Homepage/useHandleKeyDownOperations.ts:278-290` |
| Board | W | Toggle selected task timer, enabled boards only | Blocked enabled-state read B10 | `hooks/Homepage/useHandleKeyDownOperations.ts:157-169` |
| Inbox | Ctrl/Cmd+F | Local inbox search | Supported title/ticket search | `app/inbox/Inbox.tsx:282-292,317-323` |
| Inbox | Shift+J/Down, Shift+K/Up | Extend selected rows | Supported cached rows | `hooks/MultiPages/useBulkActions.ts:132-164` |
| Inbox | H | Reminder | Supported for cached notification IDs, not arbitrary task reminders | `components/notifications/inboxSplit/RemindMeInbox.tsx:39-54` |
| Inbox | Shift+S | Subtask hint | Unavailable subtask flow B11; hint only | `lib/configs/inbox.config.ts:71` |
| Inbox | G then I | Default inbox split | Added. Native returns to the default inbox split. | `app/inbox/Inbox.tsx:343-356` |
| Inbox/board/task | Z, Ctrl/Cmd+Z | Last global undo outside text inputs | Blocked B9; editor undo supported separately | `app/inbox/Inbox.tsx:333-336`, `hooks/Homepage/useHandleKeyDownOperations.ts:182-184`, `app/detail/[...slug]/taskDetailNavigationKeymap.ts:28-34`, `hooks/General/useUndo.tsx:194-220` |
| Task | Ctrl/Cmd+Shift+Alt+Enter | Send, complete task, advance inbox | Supported existing comment save mode | `components/RTE/useTaskDetailEditorKeyboard.tsx:121-140`, `lib/taskDetailInboxFlow.ts:89-109` |
| Task | Up/Down, Tab/Shift+Tab | Description, comment and composer focus | Supported | `hooks/Task Detail/CommentAndDescriptionHooks/useCommentAndDescriptions.ts:607,662-701` |
| Task | R | React to focused comment | Supported cached comment reaction picker | `hooks/Task Detail/CommentAndDescriptionHooks/useCommentAndDescriptions.ts:573-576` |
| Task | L | Like focused comment | Supported | `hooks/Task Detail/CommentAndDescriptionHooks/useCommentAndDescriptions.ts:578-587` |
| Task | Ctrl/Cmd+Shift+A | AI suggest reply to focused comment | Blocked reply generation B10; not select-all | `hooks/Task Detail/CommentAndDescriptionHooks/useCommentAndDescriptions.ts:535-541` |
| Task | Ctrl/Cmd+, | Copy private title, ticket and URL | Supported semicolon alias | `app/detail/[...slug]/taskDetailNavigationKeymap.ts:165-179` |
| Task | Alt+T | Toggle timer | Blocked B10; actual handler differs from W help | `app/detail/[...slug]/taskDetailNavigationKeymap.ts:278-290` |
| Task | Alt+V | Dictation alias | Blocked microphone path B11 | `app/detail/[...slug]/taskDetailEditingKeymap.ts:165-174` |
| Task | Ctrl+Tab | Suppress browser tab switch | No native action; intentionally no invented operation | `app/detail/[...slug]/taskDetailEditingKeymap.ts:125-134` |
| Task | Ctrl/Cmd+Shift++ | Link subtask | Blocked subtask linker. Help also advertises options. Native palette is not linker parity. | `app/detail/[...slug]/taskDetailEditingKeymap.ts:213-218` |
| Task | Ctrl/Cmd+O | Links modal | Unavailable native links flow; not help's subtask view | `app/detail/[...slug]/taskDetailKeyboard.ts:57-60` |
| Task | ; | Comment snippet picker | Blocked personal snippet catalog B8 | `app/detail/[...slug]/taskDetailKeyboard.ts:46-55` |
| AI sidebar | Ctrl/Cmd+Shift+J | Reset session alias, visible chat only | Added alongside Ctrl/Cmd+Shift+O; real new-chat action | `hooks/MultiPages/AIChat/aiChatKeyboard.ts:203-213` |
| AI sidebar | Ctrl+Q | Switch sidebar/workspace focus | Native exact modifiers; preserves prior workspace target | `hooks/MultiPages/AIChat/aiChatKeyboard.ts:83-114,139-200` |
| AI sidebar | Ctrl/Cmd+U | Upload attachment | Blocked file upload path; no fabricated upload | `hooks/MultiPages/AIChat/aiChatKeyboard.ts:115-118` |
| AI sidebar | Ctrl/Cmd+Shift+D | Start recording | Blocked microphone path | `hooks/MultiPages/AIChat/aiChatKeyboard.ts:119-124` |
| AI sidebar | Escape | Cancel stream | Supported existing stream cancel | `hooks/MultiPages/AIChat/aiChatKeyboard.ts:79-82` |
| AI sidebar | Shift+Enter | Newline instead of send | Supported native input | `hooks/MultiPages/AIChat/aiChatKeyboard.ts:65-78` |
| AI sidebar | /, Up/Down, Enter, Escape | Skill picker navigation | Supported cached bearer skills; not editor format catalog | `components/RTE/Extensions/SlashCommands/CommandsList.tsx:92-157` |
| Any applicable surface | G then T/H/I/;/S/P/E/R/A/M/D/U/B/C/# | Global routes | Inbox and task board only. Other destinations excluded/unavailable, never mapped to Board. | `lib/keyboard/globalGShortcuts.ts:33-49,52-66` |

## Remaining handler-only keys

| Surface | Web shortcut | Runtime action | Native status | Web source |
| --- | --- | --- | --- | --- |
| Inbox | Ctrl/Cmd+R | Open focused task for reply | Supported native composer focus | `components/notifications/inboxSplit/index.tsx:729-735` |
| Inbox | Alt+V | Open focused task for audio reply | Blocked microphone flow B11 | `components/notifications/inboxSplit/index.tsx:737-743` |
| Inbox | Ctrl/Cmd+Shift+O | Link subtask | Blocked subtask flow B11. Visible AI reset has separate context. | `components/notifications/inboxSplit/index.tsx:771-774` |
| Inbox | U | Toggle notification unread | Blocked unread mutation B10; native seen-only write is not a toggle | `components/notifications/inboxSplit/index.tsx:776-789` |
| Inbox | Alt+S | Star notification's task | Blocked saved-task mutation B8 | `components/notifications/inboxSplit/index.tsx:798-800` |
| Inbox | gg / GG | First/last notification | Supported cached row boundaries | `components/notifications/inboxSplit/index.tsx:647-670` |
| Inbox | G then T | Focused notification's board | Supported legacy route; conflicts with global timers route | `components/notifications/inboxSplit/index.tsx:673-687` |
| Board selection | X, Escape | Toggle selection, clear selection | Supported native selection | `lib/contexts/Kanban/BulkSelectionContext.tsx:299-318` |
| Board selection | Ctrl/Cmd+A | Select board tasks, flag-gated | Native selection retained; no assumed server flag | `lib/contexts/Kanban/BulkSelectionContext.tsx:287-296` |
| Board selection | Ctrl/Cmd+E, A, T, M | Bulk archive, assign, tags, column | Supported existing bulk actions | `lib/contexts/Kanban/BulkSelectionContext.tsx:321-358` |
| Rich editor | Ctrl/Cmd+Shift+1, Ctrl/Cmd+Shift+2 | Heading overrides | Supported native format actions | `components/RTE/useTaskDetailEditorKeyboard.tsx:61-69` |
| Rich editor | Ctrl/Cmd+Shift+A | Upload attachment | Blocked staged upload flow; not outside-editor selection | `components/RTE/useTaskDetailEditorKeyboard.tsx:70-73` |
| Comment editor | Ctrl/Cmd+Alt+Enter | Send without inbox advance | Added. Existing durable send-and-stay action. | `lib/taskDetailInboxFlow.ts:89-109`, `components/RTE/useTaskDetailEditorKeyboard.tsx:121-140` |
| Rich editor | Ctrl/Cmd+J | AI writer or flagged compose tab | Native flagged compose alternative. Legacy writer remains partial. | `components/RTE/useTaskDetailEditorKeyboard.tsx:156-159` |
| Rich editor | Escape | Save description or leave comment editor | Partial. Native leaves editing; does not auto-publish description on Escape. | `components/RTE/useTaskDetailEditorKeyboard.tsx:44-56` |
| Rich editor | Enter inside reply quote | Leave quoted block | Supported native reply-block handling | `components/RTE/Extensions/ReplyBlockquote.ts:201-220` |
| Rich editor | Ctrl/Cmd+Alt+0 | Normal text block | Supported native paragraph action | `components/RTE/Extensions/dBlock/dBlock.ts:82-85` |

Task creation remains deliberately unavailable for plain keys.
Its defaults require active filters and exact ranks.
AI compose cannot replace that user promise.
No fake quick-add UI was introduced.

## Native input evidence

| Input contract | GPUI behavior test |
| --- | --- |
| Shift+T board/table | `keyboard_shift_t_toggles_existing_board_table_without_changing_board` |
| Ctrl/Cmd+Shift+O/J from workspace | `keyboard_ai_reset_aliases_work_from_workspace_and_focus_a_clean_composer` |
| Same reset aliases inside AI | `keyboard_reset_platform_aliases_create_distinct_chats_and_clear_the_draft` |
| Exact Ctrl+Q | `keyboard_ctrl_q_ignores_extra_modifiers_and_preserves_workspace_focus` |
| A/M/P/S/T/D, all three surfaces | `keyboard_task_field_shortcuts_commit_the_selected_target_on_every_surface` |
| J/K/arrows/Tab/Shift+Tab/gg/GG/Enter/Escape | `keyboard_board_navigation_aliases_focus_columns_rows_and_open_the_selected_task` |
| Ctrl/Cmd+E, all three surfaces | `keyboard_archive_command_and_platform_aliases_archive_tasks_not_notifications` |
| Plain keys inside F2/Ctrl+M/Ctrl+D editors | `keyboard_plain_shortcuts_do_not_escape_title_comment_or_description_inputs` |
| /, 6, Ctrl/Cmd+B | `keyboard_search_and_board_switch_shortcuts_use_existing_controls` |
| G then I default split | `keyboard_g_i_returns_to_the_default_inbox_split` |
| Comment Ctrl/Cmd+Alt+Enter | `keyboard_alt_enter_sends_an_inbox_reply_without_archiving_or_advancing` |
| Every formatting chord and slash choice | Existing `every_format_keyboard_and_slash_choice_is_a_real_editor_action` |
| I summary | Existing `task_summary_i_click_escape_preserve_full_markdown_without_requesting_ai` |
| O/Shift+O comment expansion | Existing `comment_single_all_expansion_keeps_latest_open_and_does_not_fire_in_editor` |
| Shift+M board/column choice | Existing `move_board_shortcut_uses_search_then_destination_column_and_never_guesses` |
| Inbox E durability | Existing GitHub native X11 `DESKTOP_INPUT_ARCHIVE_VERIFIED` receipt |


`crates/ht-desktop/src/keyboard_parity_tests.rs` drives keystrokes.
Field assertions check target IDs and durable payloads.
Navigation checks focused rows, columns and opened tasks.
Archive checks distinguish tasks from inbox notifications.
AI reset checks a clean focused composer.
Editor guards prevent plain keys triggering task actions.
Existing feature tests cover reminder, comment and formatting keys.
Exact GitHub run evidence belongs in `GATES-keys2.md`.
