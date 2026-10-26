# Web keyboard shortcut parity

Web source root is `/home/valentin/projects/hypertask/src`.
Citations below are relative to that root.
`Ctrl/Cmd` means the platform command key.
Rows retain help-registry duplicates and conflicting labels.
Runtime handlers take precedence over help text.
Native means local input and queue behavior only.
It never means live server acceptance.

Acceptance: `GATES-keys2.md` and `GATES-keys3.md`.
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
| App shell surfaces | `6` | Search | Partial cached search; web document route requires session. R18. | `lib/constants/shortcuts.ts:60` |
| App shell surfaces | `7` | Running timers | Outside scope; no native destination. | `lib/constants/shortcuts.ts:61` |
| App shell surfaces | `[` | Collapse / expand sidebar | Native [ rail toggle. | `lib/constants/shortcuts.ts:62` |
| Global Shortcuts | `?` | Shortcuts | Native searchable shortcut sidebar. | `lib/constants/shortcuts.ts:73` |
| Global Shortcuts | `\\` | Settings | Native settings; outside this task. | `lib/constants/shortcuts.ts:74` |
| Global Shortcuts | `/` | Search | Partial cached search; web document route requires session. R18. | `lib/constants/shortcuts.ts:75` |
| Global Shortcuts | `P` | Set priority | Native picker and durable field write; cached catalogs only. | `lib/constants/shortcuts.ts:76` |
| Global Shortcuts | `S` | Set task size | Native picker and durable field write; cached catalogs only. | `lib/constants/shortcuts.ts:77` |
| Global Shortcuts | `T` | Add tags | Native picker and durable field write; cached catalogs only. | `lib/constants/shortcuts.ts:78` |
| Global Shortcuts | `E` | Archive | Native notification archive; not task completion. | `lib/constants/shortcuts.ts:79` |
| Global Shortcuts | `J` | Move focus down | Native arrows/J/K; task J/K advances/reverses playlist. | `lib/constants/shortcuts.ts:80` |
| Global Shortcuts | `K` | Move focus up | Native arrows/J/K; task J/K advances/reverses playlist. | `lib/constants/shortcuts.ts:81` |
| Global Shortcuts | `←+⭡+⭣+→` | Move focus | Native arrows/J/K; task J/K advances/reverses playlist. | `lib/constants/shortcuts.ts:82` |
| Global Shortcuts | `Ctrl/Cmd+K` | Open Hypertask Command | Native command center; cmdk branch owns further parity. | `lib/constants/shortcuts.ts:83` |
| Global Shortcuts | `Ctrl/Cmd+J` | Compose task / New Task | Native flagged AI compose note. Not plain quick entry. | `lib/constants/shortcuts.ts:84` |
| Global Shortcuts | `Ctrl/Cmd+Z` | Undo latest action | Blocked whole web undo stack; isolated bearer inverses differ. R17. | `lib/constants/shortcuts.ts:85` |
| Global Shortcuts | `;` | Use snippet | Blocked session-only settings.snippets catalog. R2. | `lib/constants/shortcuts.ts:86` |
| Global Shortcuts | `G then X` | Show or hide archived tasks | Blocked personal unsaved-view read/write; named settings differ. R5. | `lib/constants/shortcuts.ts:87` |
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
| Global Shortcuts | `Alt (Mac Ctrl)+0-9` | Switch boards | Blocked cookie-only favorite.index → project mapping. R3. | `lib/constants/shortcuts.ts:108` |
| Global Shortcuts | `Ctrl/Cmd+B` | Switch Kanban boards | Native searchable board switcher; cached boards only. | `lib/constants/shortcuts.ts:109` |
| Global Shortcuts | `D` | Set due date | Native picker and durable field write; cached catalogs only. | `lib/constants/shortcuts.ts:110` |
| Global Shortcuts | `Shift+M` | Move task to a different board | Native board then column picker; durable bearer MoveTask. | `lib/constants/shortcuts.ts:111` |
| Board | `C` | Add task | Blocked exact full-create/defaults/ranking contract; compose differs. R14 / creation-parity.md. | `lib/constants/shortcuts.ts:117` |
| Board | `Ctrl/Cmd+J` | Add task with AI Task Writer | Blocked exact full-create/defaults/ranking contract; compose differs. R14 / creation-parity.md. | `lib/constants/shortcuts.ts:118` |
| Board | `Ctrl/Cmd+Shift++` | Add sub-task | Blocked session-only orphan catalog and full-create fallback. Palette is not linker. R15. | `lib/constants/shortcuts.ts:119` |
| Board | `Ctrl/Cmd+Shift+C` | Add task at top | Blocked exact full-create/defaults/ranking contract; compose differs. R14 / creation-parity.md. | `lib/constants/shortcuts.ts:120` |
| Board | `Shift+C` | Add task at bottom | Skipped N/Shift+C; HTPR-7699. Existing creation blocker unchanged. | `lib/constants/shortcuts.ts:121` |
| Board | `ENTER` | Open task | Native Enter opens selected cached task. | `lib/constants/shortcuts.ts:122` |
| Board | `` | Next view | Blocked unsavedView/view_order/default_view_order/rendered tabs. R6. | `lib/constants/shortcuts.ts:126` |
| Board | `[` | Previous view | Blocked unsavedView/view_order/default_view_order/rendered tabs. R6. | `lib/constants/shortcuts.ts:127` |
| Board | `gg` | Move focus to top | Native gg and Shift+G G boundaries. | `lib/constants/shortcuts.ts:129` |
| Board | `GG` | Move focus to bottom | Native gg and Shift+G G boundaries. | `lib/constants/shortcuts.ts:130` |
| Board | `Shift+J, ⭣` | Move task down | Blocked session-only moveTask ranking; bearer has no position field. R4. | `lib/constants/shortcuts.ts:131` |
| Board | `Shift +K, ⭡` | Move task up | Blocked session-only moveTask ranking; bearer has no position field. R4. | `lib/constants/shortcuts.ts:132` |
| Board | `Shift+L, →` | Move task right | Native neighboring visible column; durable sectionId write. | `lib/constants/shortcuts.ts:133` |
| Board | `Shift +H, ←` | Move task left | Native neighboring visible column; durable sectionId write. | `lib/constants/shortcuts.ts:134` |
| Board | `Shift+H, ←` | Move task to the left | Native neighboring visible column; durable sectionId write. | `lib/constants/shortcuts.ts:135` |
| Board | `Shift+S` | Sort Kanban board | Blocked personal unsaved-view read/write; named settings differ. R5. | `lib/constants/shortcuts.ts:136` |
| Board | `Shift+CLICK` | Add a sort level (table column) | Outside scope. Table sort write unavailable. B2. | `lib/constants/shortcuts.ts:137` |
| Board | `Shift+T` | Toggle table layout | Added Shift+T for existing native board/table. Saved layout mutation remains B4. | `lib/constants/shortcuts.ts:138` |
| Board | `←+→` | Match any / all values (inside a filter's list) | Blocked personal unsaved-view read/write; named settings differ. R5. | `lib/constants/shortcuts.ts:139` |
| Board | `Ctrl/Cmd+Shift++` | Show task options | Blocked session-only orphan catalog and full-create fallback. Palette is not linker. R15. | `lib/constants/shortcuts.ts:140` |
| Board | `Shift+L, →` | Move task to the right | Native neighboring visible column; durable sectionId write. | `lib/constants/shortcuts.ts:141` |
| Board | `A` | Assign a user | Native picker and durable field write; cached catalogs only. | `lib/constants/shortcuts.ts:142` |
| Task View | `ENTER` | Activate description/comment or reply to comment | Native Enter activates selected task part. | `lib/constants/shortcuts.ts:148` |
| Task View | `Ctrl/Cmd+ENTER` | Send comment and advance in Inbox | Native durable send and inbox advance. | `lib/constants/shortcuts.ts:150` |
| Task View | `Ctrl/Cmd+Shift+ENTER` | Send comment and stay on task | Native durable send without advancing. | `lib/constants/shortcuts.ts:153` |
| Task View | `Ctrl/Cmd+J` | Write with AI | Blocked exact full-create/defaults/ranking contract; compose differs. R14 / creation-parity.md. | `lib/constants/shortcuts.ts:154` |
| Task View | `A` | Assign a user | Native picker and durable field write; cached catalogs only. | `lib/constants/shortcuts.ts:155` |
| Task View | `Shift+B` | Blocked by person | Native person picker and clear; server-gated write. U2. | `lib/constants/shortcuts.ts:156` |
| Task View | `W` | Start / stop timer | Native authoritative timer toggle; one effect attempt. U3. | `lib/constants/shortcuts.ts:157` |
| Task View | `B` | Log time | Blocked session-only all-user entries/canManage. MCP report restricts ordinary members. R9. | `lib/constants/shortcuts.ts:158` |
| Task View | `Ctrl/Cmd+M` | Add a comment | Native composer focus from inbox, board or task. | `lib/constants/shortcuts.ts:159` |
| Task View | `Shift+R` | Suggest reply with AI | Blocked cookie-only task-context reply HTML generation. R10. | `lib/constants/shortcuts.ts:160` |
| Task View | `Alt (Mac Ctrl)+0-9` | Switch boards | Blocked cookie-only favorite.index → project mapping. R3. | `lib/constants/shortcuts.ts:161` |
| Task View | `I` | Expand summary | Native full saved summary; no AI request. | `lib/constants/shortcuts.ts:162` |
| Task View | `O` | Expand single comment | Native comment expansion; latest remains open after collapse-all. | `lib/constants/shortcuts.ts:163` |
| Task View | `Shift+O` | Expand all comments | Native comment expansion; latest remains open after collapse-all. | `lib/constants/shortcuts.ts:164` |
| Task View | `Ctrl/Cmd+Shift+H` | Toggle history events / agent log | Blocked complete agent feed: link/options/selectedOption/selectedAt/selectedBy missing. Preference itself local. R11. | `lib/constants/shortcuts.ts:165` |
| Task View | `Ctrl/Cmd+O` | View all links | Blocked session-only stored/saved-source URL feed. Actual handler opens links. R8. | `lib/constants/shortcuts.ts:166` |
| Task View | `Ctrl/Cmd+M` | Write Comment | Native composer focus from inbox, board or task. | `lib/constants/shortcuts.ts:167` |
| Task View | `Ctrl+D` | Edit Description | Native Control+D specifically, including Mac Control; not Cmd+D. | `lib/constants/shortcuts.ts:168` |
| Task View | `F2` | Edit Title | Native F2 title editor. | `lib/constants/shortcuts.ts:169` |
| Task View | `Ctrl+Shift+3` | Delete comment | Native own-comment delete confirmation. | `lib/constants/shortcuts.ts:170` |
| Task View | `Ctrl/Cmd+Shift+V` | Paste as plain text | Native plain clipboard paste without link wrapping. | `lib/constants/shortcuts.ts:171` |
| Task View | `Ctrl+O` | View all sub-tasks | Blocked session-only stored/saved-source URL feed. Actual handler opens links. R8. | `lib/constants/shortcuts.ts:172` |
| Task View | `Ctrl/Cmd+S` | Share task | Blocked session-only sharing ID/type/live-state contract. R7. | `lib/constants/shortcuts.ts:173` |
| Task View | `Ctrl/Cmd+Shift++` | Show task options | Blocked session-only orphan catalog and full-create fallback. Palette is not linker. R15. | `lib/constants/shortcuts.ts:174` |
| Task View | `Ctrl/Cmd+Shift+;` | Private share link (URL only) | Native clipboard from cached uniqueIndex; refuses absent URL. | `lib/constants/shortcuts.ts:175` |
| Task View | `Ctrl/Cmd+;` | Private share link (Title + URL) | Native clipboard from cached uniqueIndex; refuses absent URL. | `lib/constants/shortcuts.ts:176` |
| Task View | `Ctrl/Cmd+Shift+.` | Public share link (URL only) | Blocked session-only sharing ID/type/live-state contract. R7. | `lib/constants/shortcuts.ts:177` |
| Task View | `Ctrl/Cmd+.` | Public share link (Title + URL) | Blocked session-only sharing ID/type/live-state contract. R7. | `lib/constants/shortcuts.ts:178` |
| Task View | `Ctrl/Cmd+Shift+I` | Copy task ID | Native ticket/title clipboard. | `lib/constants/shortcuts.ts:179` |
| Task View | `Ctrl/Cmd+I` | Copy task title and ID | Native ticket/title clipboard. | `lib/constants/shortcuts.ts:180` |
| Task View | `ALT+S` | Star task | Native task-star toggle; durable desired state. U1. | `lib/constants/shortcuts.ts:181` |
| Task View | `Ctrl+Shift+S` | Star comment | Native saved-comment write. | `lib/constants/shortcuts.ts:182` |
| Task View | `Ctrl+Shift+P` | Pin comment | Native saved-comment write. | `lib/constants/shortcuts.ts:183` |
| Task View | `F` | Follow task | Blocked session-only follower create/delete and follower-row ID. R1. | `lib/constants/shortcuts.ts:184` |
| Task View | `ALT+F` | Unfollow task | Blocked session-only follower create/delete and follower-row ID. R1. | `lib/constants/shortcuts.ts:185` |
| Task View | `Ctrl/Cmd+Shift+D` | Speech to text | Blocked session-only audio transcription; native capture absent. R12. | `lib/constants/shortcuts.ts:186` |
| Task View | `Ctrl/Cmd+Shift+F` | Dictate and Improve | Blocked session-only audio transcription; native capture absent. R12. | `lib/constants/shortcuts.ts:187` |
| Task View | `Ctrl/Cmd+Shift+,` | Discard draft | Native current editor draft discard. | `lib/constants/shortcuts.ts:188` |
| Saving new task | `Ctrl/Cmd+ENTER` | Save | Blocked exact full-create/defaults/ranking contract; compose differs. R14 / creation-parity.md. | `lib/constants/shortcuts.ts:194` |
| Saving new task | `Ctrl/Cmd+Alt/Opt+ENTER` | Save and close | Blocked exact full-create/defaults/ranking contract; compose differs. R14 / creation-parity.md. | `lib/constants/shortcuts.ts:195` |
| Saving new task | `Ctrl/Cmd+Alt/Opt+Shift+ENTER` | Save and create new task | Blocked exact full-create/defaults/ranking contract; compose differs. R14 / creation-parity.md. | `lib/constants/shortcuts.ts:196` |
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
| Format | `Ctrl/Cmd+Y` | Add Video Embed | Partial URL insertion; native browser-player rendering absent. No API blocker. R19. | `lib/constants/shortcuts.ts:224` |
| Format | `2xTAB` | Autocomplete sentence | Registry-only; no current double-Tab handler or API found. R20. | `lib/constants/shortcuts.ts:225` |
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
| Board | C | add task | Blocked exact full-create/defaults/ranking contract; compose differs. R14 / creation-parity.md. | `components/Global/ConstantsQuickTips.ts:49` |
| Board | N / Shift+C | quick add | Skipped N/Shift+C; HTPR-7699. Existing creation blocker unchanged. | `components/PageComponents/Kanban/KanbanSectionComponents/NewTaskButton.tsx:54` |
| Board | Ctrl+E | archive | Supported task archive | `components/Global/ConstantsQuickTips.ts:53` |
| Board | Ctrl+B | Change Board | Supported cached board picker | `components/Global/ConstantsQuickTips.ts:57` |
| Board | / | search | Partial cached search; web document route requires session. R18. | `components/Global/ConstantsQuickTips.ts:61` |
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
| Board | N / Shift+C | Quick entry in focused/selected column, bottom | Skipped N/Shift+C; HTPR-7699. Existing creation blocker unchanged. | `hooks/Homepage/useHandleKeyDownOperations.ts:93-118`, `hooks/Homepage/useSections.ts:108-141`, `hooks/MultiPages/useAddDeleteTaskInBoards.tsx:115-164` |
| Board | C, Ctrl/Cmd+Shift+C | Full create at top | Blocked exact full-create/defaults/ranking contract; compose differs. R14 / creation-parity.md. | `hooks/Homepage/useSections.ts:263-310` |
| Inbox/task | C | Full create task | Blocked exact full-create/defaults/ranking contract; compose differs. R14 / creation-parity.md. | `app/inbox/Inbox.tsx:395-398`, `app/detail/[...slug]/taskDetailEditingKeymap.ts:197-209` |
| Board | Shift+F | Filter commands | Blocked personal unsaved-view read/write; named settings differ. R5. | `hooks/Homepage/useHandleKeyDownOperations.ts:186` |
| Board | Shift+V | Save changed view | Blocked unsavedView/view_order/default_view_order/rendered tabs. R6. | `hooks/Homepage/useHandleKeyDownOperations.ts:178-180` |
| Board | G then V | Views picker | Blocked unsavedView/view_order/default_view_order/rendered tabs. R6. | `hooks/Homepage/useHandleKeyDownOperations.ts:209-223` |
| Board | G then X | Toggle archived visibility | Blocked personal unsaved-view read/write; named settings differ. R5. | `hooks/Homepage/useHandleKeyDownOperations.ts:226-231` |
| Board | Tab / Shift+Tab | New shell cycles rendered saved views | Blocked unsavedView/view_order/default_view_order/rendered tabs. R6. | `hooks/Homepage/Views/useViewCyclingShortcuts.ts:22-51` |
| Board | [ / ] | Legacy view cycling | Blocked unsavedView/view_order/default_view_order/rendered tabs. R6. | `hooks/Homepage/Views/useViewCyclingShortcuts.ts:22-24`, `hooks/Homepage/useAppShellSurfaceShortcuts.ts:25,119-127` |
| Board | H/Left, L/Right | Previous/next column focus | Supported | `hooks/Homepage/useHandleKeyDownOperations.ts:203-206,239-243` |
| Board | J/Down, K/Up | Down/up card focus | Supported | `hooks/Homepage/useHandleKeyDownOperations.ts:191-200` |
| Board | Ctrl/Cmd+F | Find in current view | Supported title/ticket search only | `hooks/Homepage/useHandleKeyDownOperations.ts:278-290` |
| Board | W | Toggle selected task timer, enabled boards only | Native authoritative timer toggle; one effect attempt. U3. | `hooks/Homepage/useHandleKeyDownOperations.ts:157-169` |
| Inbox | Ctrl/Cmd+F | Local inbox search | Supported title/ticket search | `app/inbox/Inbox.tsx:282-292,317-323` |
| Inbox | Shift+J/Down, Shift+K/Up | Extend selected rows | Supported cached rows | `hooks/MultiPages/useBulkActions.ts:132-164` |
| Inbox | H | Reminder | Supported for cached notification IDs, not arbitrary task reminders | `components/notifications/inboxSplit/RemindMeInbox.tsx:39-54` |
| Inbox | Shift+S | Subtask hint | Blocked session-only orphan catalog and full-create fallback. Palette is not linker. R15. | `lib/configs/inbox.config.ts:71` |
| Inbox | G then I | Default inbox split | Added. Native returns to the default inbox split. | `app/inbox/Inbox.tsx:343-356` |
| Inbox/board/task | Z, Ctrl/Cmd+Z | Last global undo outside text inputs | Blocked whole web undo stack; isolated bearer inverses differ. R17. | `app/inbox/Inbox.tsx:333-336`, `hooks/Homepage/useHandleKeyDownOperations.ts:182-184`, `app/detail/[...slug]/taskDetailNavigationKeymap.ts:28-34`, `hooks/General/useUndo.tsx:194-220` |
| Task | Ctrl/Cmd+Shift+Alt+Enter | Send, complete task, advance inbox | Supported existing comment save mode | `components/RTE/useTaskDetailEditorKeyboard.tsx:121-140`, `lib/taskDetailInboxFlow.ts:89-109` |
| Task | Up/Down, Tab/Shift+Tab | Description, comment and composer focus | Supported | `hooks/Task Detail/CommentAndDescriptionHooks/useCommentAndDescriptions.ts:607,662-701` |
| Task | R | React to focused comment | Supported cached comment reaction picker | `hooks/Task Detail/CommentAndDescriptionHooks/useCommentAndDescriptions.ts:573-576` |
| Task | L | Like focused comment | Supported | `hooks/Task Detail/CommentAndDescriptionHooks/useCommentAndDescriptions.ts:578-587` |
| Task | Ctrl/Cmd+Shift+A | AI suggest reply to focused comment | Blocked cookie-only task-context reply HTML generation. R10. | `hooks/Task Detail/CommentAndDescriptionHooks/useCommentAndDescriptions.ts:535-541` |
| Task | Ctrl/Cmd+, | Copy private title, ticket and URL | Supported semicolon alias | `app/detail/[...slug]/taskDetailNavigationKeymap.ts:165-179` |
| Task | Alt+T | Toggle timer | Native authoritative timer toggle; one effect attempt. U3. | `app/detail/[...slug]/taskDetailNavigationKeymap.ts:278-290` |
| Task | Alt+V | Dictation alias | Blocked session-only audio transcription; native capture absent. R12. | `app/detail/[...slug]/taskDetailEditingKeymap.ts:165-174` |
| Task | Ctrl+Tab | Suppress browser tab switch | No native action; intentionally no invented operation | `app/detail/[...slug]/taskDetailEditingKeymap.ts:125-134` |
| Task | Ctrl/Cmd+Shift++ | Link subtask | Blocked session-only orphan catalog and full-create fallback. Palette is not linker. R15. | `app/detail/[...slug]/taskDetailEditingKeymap.ts:213-218` |
| Task | Ctrl/Cmd+O | Links modal | Blocked session-only stored/saved-source URL feed. Actual handler opens links. R8. | `app/detail/[...slug]/taskDetailKeyboard.ts:57-60` |
| Task | ; | Comment snippet picker | Blocked session-only settings.snippets catalog. R2. | `app/detail/[...slug]/taskDetailKeyboard.ts:46-55` |
| AI sidebar | Ctrl/Cmd+Shift+J | Reset session alias, visible chat only | Added alongside Ctrl/Cmd+Shift+O; real new-chat action | `hooks/MultiPages/AIChat/aiChatKeyboard.ts:203-213` |
| AI sidebar | Ctrl+Q | Switch sidebar/workspace focus | Native exact modifiers; preserves prior workspace target | `hooks/MultiPages/AIChat/aiChatKeyboard.ts:83-114,139-200` |
| AI sidebar | Ctrl/Cmd+U | Upload attachment | Blocked web upload ticket/grant and staged file delivery. R13. | `hooks/MultiPages/AIChat/aiChatKeyboard.ts:115-118` |
| AI sidebar | Ctrl/Cmd+Shift+D | Start recording | Blocked session-only audio transcription; native capture absent. R12. | `hooks/MultiPages/AIChat/aiChatKeyboard.ts:119-124` |
| AI sidebar | Escape | Cancel stream | Supported existing stream cancel | `hooks/MultiPages/AIChat/aiChatKeyboard.ts:79-82` |
| AI sidebar | Shift+Enter | Newline instead of send | Supported native input | `hooks/MultiPages/AIChat/aiChatKeyboard.ts:65-78` |
| AI sidebar | /, Up/Down, Enter, Escape | Skill picker navigation | Supported cached bearer skills; not editor format catalog | `components/RTE/Extensions/SlashCommands/CommandsList.tsx:92-157` |
| Any applicable surface | G then T/H/I/;/S/P/E/R/A/M/D/U/B/C/# | Global routes | Inbox and task board only. Other destinations excluded/unavailable, never mapped to Board. | `lib/keyboard/globalGShortcuts.ts:33-49,52-66` |

## Remaining handler-only keys

| Surface | Web shortcut | Runtime action | Native status | Web source |
| --- | --- | --- | --- | --- |
| Inbox | Ctrl/Cmd+R | Open focused task for reply | Supported native composer focus | `components/notifications/inboxSplit/index.tsx:729-735` |
| Inbox | Alt+V | Open focused task for audio reply | Blocked session-only audio transcription; native capture absent. R12. | `components/notifications/inboxSplit/index.tsx:737-743` |
| Inbox | Ctrl/Cmd+Shift+O | Link subtask | Blocked session-only orphan catalog and full-create fallback. Palette is not linker. R15. | `components/notifications/inboxSplit/index.tsx:771-774` |
| Inbox | U | Toggle notification unread | Blocked seen=false/toggle mutation; bearer only sets true. R16. | `components/notifications/inboxSplit/index.tsx:776-789` |
| Inbox | Alt+S | Star notification's task | Native task-star toggle; durable desired state. U1. | `components/notifications/inboxSplit/index.tsx:798-800` |
| Inbox | gg / GG | First/last notification | Supported cached row boundaries | `components/notifications/inboxSplit/index.tsx:647-670` |
| Inbox | G then T | Focused notification's board | Supported legacy route; conflicts with global timers route | `components/notifications/inboxSplit/index.tsx:673-687` |
| Board selection | X, Escape | Toggle selection, clear selection | Supported native selection | `lib/contexts/Kanban/BulkSelectionContext.tsx:299-318` |
| Board selection | Ctrl/Cmd+A | Select board tasks, flag-gated | Native selection retained; no assumed server flag | `lib/contexts/Kanban/BulkSelectionContext.tsx:287-296` |
| Board selection | Ctrl/Cmd+E, A, T, M | Bulk archive, assign, tags, column | Supported existing bulk actions | `lib/contexts/Kanban/BulkSelectionContext.tsx:321-358` |
| Rich editor | Ctrl/Cmd+Shift+1, Ctrl/Cmd+Shift+2 | Heading overrides | Supported native format actions | `components/RTE/useTaskDetailEditorKeyboard.tsx:61-69` |
| Rich editor | Ctrl/Cmd+Shift+A | Upload attachment | Blocked web upload ticket/grant and staged file delivery. R13. | `components/RTE/useTaskDetailEditorKeyboard.tsx:70-73` |
| Comment editor | Ctrl/Cmd+Alt+Enter | Send without inbox advance | Added. Existing durable send-and-stay action. | `lib/taskDetailInboxFlow.ts:89-109`, `components/RTE/useTaskDetailEditorKeyboard.tsx:121-140` |
| Rich editor | Ctrl/Cmd+J | AI writer or flagged compose tab | Native flagged compose alternative. Legacy writer remains partial. | `components/RTE/useTaskDetailEditorKeyboard.tsx:156-159` |
| Rich editor | Escape | Save description or leave comment editor | Added. Description uses the existing durable save; comment Escape never publishes. Refused lossy saves retain editing, focus and draft. | `components/RTE/useTaskDetailEditorKeyboard.tsx:44-56` |
| Rich editor | Enter inside reply quote | Leave quoted block | Supported native reply-block handling | `components/RTE/Extensions/ReplyBlockquote.ts:201-220` |
| Rich editor | Ctrl/Cmd+Alt+0 | Normal text block | Supported native paragraph action | `components/RTE/Extensions/dBlock/dBlock.ts:82-85` |

Task creation remains deliberately unavailable for plain keys.
The exact defaults and authorization boundary are now traced in
[creation-parity.md](creation-parity.md), including N/Shift+C column targeting,
active-filter precedence, assignable IDs and exact top/bottom ranks.
Both exact save routes still need server sign-in work under HTPR-7699.
MCP append and AI compose cannot replace that user promise.
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
| Description Escape, comment Escape | `keyboard_escape_saves_description_but_never_publishes_comment` |
| Refused description Escape save | `keyboard_escape_keeps_description_when_lossless_save_is_refused` |
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


## Round 3 route audit

Audit root: `/home/valentin/projects/hypertask/src`.
All citations below use that root.
Duplicates reference the same audit entry.
`No` means today's desktop bearer cannot authorize it.
`Yes` does not mean live-server QA happened.
No cookies copied. No authorization widened.
N / Shift+C deliberately skipped: HTPR-7699.

### Unblocked contracts

| Audit | Keys / surfaces | Exact web handler and web route | Existing bearer equivalent / native result |
| --- | --- | --- | --- |
| U1 | Alt+S; task and inbox | Task `app/detail/[...slug]/taskDetailNavigationKeymap.ts:251-257`; inbox `components/notifications/inboxSplit/index.tsx:667-669`; `hooks/Task Detail/useTaskDetailGlobalStates.ts:610-624` calls saved-task toggle. POST `/api/savedContent/toggleSaved`, `pages/api/savedContent/toggleSaved.ts:17-20,26-37,67-83`. Direct route: **No**, CLI exception requires commentId. | **Yes**, POST `/api/mcp/tasks/card-actions`, `app/api/mcp/tasks/card-actions/route.ts:1-4`, `lib/mcp/tasks/cardActionHandler.ts:105-143,160-167`. Body `{task_id,project_id,action:"star",active}`. Private human-only task state, not comment stars. Read: authenticated user's private `savedContent`, `lib/mcp/operations/tasks/operation.ts:301-304,709-712`. Native toggles known cached state; unknown state refuses. Queue freezes desired boolean. Reconciliation uses actual `saved_content` IDs. |
| U2 | Shift+B; task | `app/detail/[...slug]/taskDetailNavigationKeymap.ts:80-94`; picker `components/Modals/BlockedByPerson/BlockedByPerson.tsx:53-69,81-98`. POST `/api/tasks/waiting-on`, `{taskId,userId:null|number}`. | **Yes**, POST `/api/mcp/tasks/waiting-on`, `app/api/mcp/tasks/waiting-on/route.ts:1-3`, `lib/mcp/operations/tasks/waiting-on/operation.ts:22-38,46-65`. Server retains HTPR-7723 gate. Same `lib/tasks/setWaitingOn.ts:30-41,51-63,72-89`. Native person picker includes Nobody (clear). No agents or assignment substitution. |
| U3 | W; board/task. Alt+T; task | Board `hooks/Homepage/useHandleKeyDownOperations.ts:157-174`; task `components/PageComponents/TaskDetail/TaskInfoColumn/TaskTime.tsx:79-100`; Alt+T `app/detail/[...slug]/taskDetailNavigationKeymap.ts:278-290`. Web GET `/api/time/task?taskId=…`, then POST `/api/time/start` or `/api/time/stop`, `hooks/Task Detail/useTimeTracking.ts:187-195,198-205,231-255`. | **Yes**, POST `/api/mcp/time/status`, `/time/start`, `/time/stop`; exports `app/api/mcp/time/status/route.ts:1-3`, `app/api/mcp/time/start/route.ts:1-3`, `app/api/mcp/time/stop/route.ts:1-3`. Bearer resolution `lib/mcp/operations/time/helpers.ts:8-25,53-68`; shared enabled/running state `lib/timeTracking.ts:555-579`. Disabled/no-running: no effect. Disabled/running: stop allowed. Same timer functions. Native W/Alt+T queues one account-fenced attempt. No automatic uncertain replay. Task Start/Stop control follows `components/PageComponents/TaskDetail/TaskInfoColumn/TaskTime.tsx:127,166-185`. Full time-log B remains R9. |

### Precise remaining blockers

| Audit | Keys / rows | Exact handler and app route | Bearer today / missing contract |
| --- | --- | --- | --- |
| R1 | F / Alt+F | `app/detail/[...slug]/taskDetailNavigationKeymap.ts:260-274`. POST `/api/follower/createFollower`, `{taskId,mentionById,…}`: `pages/api/follower/createFollower.ts:9-25`; POST `/api/follower/unFollowTask`, `{id}`: `pages/api/follower/unFollowTask.ts:10-18,25-29`. | **No**. Create verifies session cookie. Delete uses `getSessionUser`. Follower-row ID read also session-only: POST `/api/follower/getFollower`, `pages/api/follower/getFollower.ts:10-18,30-40`. MCP task-update fields contain no follower mutation: `lib/mcp/tasks/fields/types.ts:50-75`. |
| R2 | ; global/task snippet picker | Task `app/detail/[...slug]/taskDetailKeyboard.ts:46-55`; event `lib/snippets.ts:128`; catalog GET `/api/users/preferences`, `lib/snippets.ts:58-69`; writes `hooks/General/useSnippetManager.ts:22-35`. | **No**. Missing authenticated `settings.snippets` read. `app/api/users/preferences/route.ts:65-85,125-142` requires loaded/cookie user. MCP profile PATCH only displayName/photoURL: `lib/mcp/operations/user/profile/operation.ts:76-84`. No fabricated snippets. |
| R3 | Alt+0-9 / Mac Control+0-9 favorites | `components/ProviderGlobal/GloablProviders.tsx:724-739` finds `favorite.index === digit`, not array offset. GET `/api/favorites/getFavorites`, `pages/api/favorites/getFavorites.ts:5-18`; controller `utils/controllers/favorites/getAll.ts:24`. | **No**. Cookie-only favorite `{index,project}` mapping. MCP `/user/context` projects are not favorites: `lib/mcp/operations/user/context/operation.ts:159-191`. Arbitrary board ordering stays forbidden. |
| R4 | Shift+J/Down, Shift+K/Up ranking | `hooks/Homepage/useHandleKeyDownOperations.ts:360-420,497-503`; route caller `utils/api/global/apiHelpers/moveTaskHandler.ts:6-24`. PUT `/api/tasks/moveTask`, `pages/api/tasks/moveTask.ts:1-5`. | **No**. Session task-write wrapper `lib/api/task-writes/route.ts:31-39,101-106`. Missing bearer `ranking` or before/after position field. MCP update body `lib/mcp/tasks/fields/types.ts:50-75`; MCP cross-board move body `lib/mcp/operations/tasks/move/operation.ts:22-29,132-147`. SectionId move cannot reorder. Active sort also matters: handler line 379 refuses UpdatedAt ranking. |
| R5 | Shift+S sort; Shift+F filters; filter Left/Right; G then X archived | Sort/filter keys `hooks/Homepage/useHandleKeyDownOperations.ts:191-193`; archive `:231-235`. `hooks/Homepage/Views/useKanbanViews.ts:195-215` writes POST `/api/projects/views/unsaved-view` with `baseViewId`; route constant `lib/constants/APIRouteConstants.ts:34`. | **No** for transient mutation. Required fields: `board_sorting_mode/order/stack`, `board_filters`, `board_show_archived`, base active view. Session route `lib/api/project-writes/views/unsaved-view.ts:32-55,85-97`; wrapper auth R4. **Yes** for *named* MCP view settings, not equivalent: `lib/mcp/operations/view/[viewId]/operation.ts:261-268`. It updates a saved view, not unsaved personal state. No silently permanent sorting/filter changes. |
| R6 | Shift+V save changed view; G then V; Tab/Shift+Tab; legacy [ / ] view cycling | Save `hooks/Homepage/useHandleKeyDownOperations.ts:183-185`; G/V `:222-228`; cycling `hooks/Homepage/Views/useViewCyclingShortcuts.ts:21-51`. Rendered order `hooks/Homepage/Views/useOrderedViews.ts:17-26,44-57`, `useRenderedViews.ts:24-43,86-93`. POST `/api/projects/views/update-view` (`lib/constants/APIRouteConstants.ts:36`; `useKanbanViews.ts:344,383`). | **No** for exact transient save/order. Required `unsavedView`, `view_order`, `default_view_order`, active builtin ID, hidden/empty tab preferences. MCP GET `/api/mcp/view?projectId=…&include_settings=true` and POST `/view/{id}/apply` are **Yes**, but list select omits those fields: `lib/mcp/operations/view/operation.ts:111-133,196-227`. Apply only returns applied_view_id/is_default: `lib/mcp/views/services.ts:496-505,527-530`. Named views cannot establish actual rendered cycling order. Existing column Tab remains partial. |
| R7 | Ctrl/Cmd+S share; Ctrl/Cmd+. and Ctrl/Cmd+Shift+. public copy | Share/copy `app/detail/[...slug]/taskDetailNavigationKeymap.ts:185-204,239-247`. GET/POST/PUT `/api/share/createShareLink`, `pages/api/share/createShareLink.ts:12-21,64-95`. | **No**. `getSessionUser` at route lines 15-18. Missing authorized sharing ID, type and live/expired state. Public `/api/share/getSharedTask` needs an already-known shareId (`pages/api/share/getSharedTask.ts:13-18,28-38`); it cannot discover/create it. Never derive public URL from private uniqueIndex. |
| R8 | Ctrl/Cmd+O links; misleading help “sub-tasks” | Actual dispatch `app/detail/[...slug]/taskDetailKeyboard.ts:57-60`; fetch `components/Modals/LinksModal/index.tsx:83`. GET `/api/urls/fetchUrls?taskId=…&commentId=…`. | **No**. `pages/api/urls/fetchUrls.ts:10-24` requires session. Controller merges stored URLs and flag-controlled saved sources: `utils/controllers/urls/fetchUrls.ts:9-118`. MCP attachment URLs and description HTML cannot reproduce that feed. No guessed regex-only links modal. Help's subtask label is not a handler. |
| R9 | B time log | Actual key and enabled gate `components/PageComponents/TaskDetail/TaskInfoColumn/TaskTime.tsx:79-106`; full modal `components/Modals/TimeLog/TimeLogModal.tsx:105-129,143-165,173-195`. GET/POST/PATCH/DELETE `/api/time/entries`; hook GET `hooks/Task Detail/useTimeTracking.ts:455-473`. | **No** for full entries contract: `app/api/time/_lib.ts:11-20`, `app/api/time/entries/route.ts:40-58`. Missing all-member entries plus canManage. **Yes** for MCP `/time/log`, update/delete and report. Report is not equivalent for ordinary members: `lib/timeTracking.ts:333-349` returns all task users; `:370-390` report restricts nonadmins to self. No incomplete log presented as full task history. Timer shortcuts separately unblocked U3. |
| R10 | Shift+R / Ctrl/Cmd+Shift+A reply suggestion | `app/detail/[...slug]/taskDetailEditingKeymap.ts:49-65`; focused-comment `hooks/Task Detail/CommentAndDescriptionHooks/useCommentAndDescriptions.ts:535-541`; request `components/RTE/useTaskDetailEditorEvents.tsx:166-194`. POST `/api/ai/suggest-reply`, `{taskId}`. | **No**. `app/api/ai/suggest-reply/route.ts:160-170` uses cookie user. Missing authenticated reply-generation HTML contract. MCP improve edits supplied text; no replacement for task-context reply generation. |
| R11 | Ctrl/Cmd+Shift+H history / agent log | `app/detail/[...slug]/taskDetailEditingKeymap.ts:81-87`; local persisted boolean `store/index.ts:207-210`; merge `hooks/Task Detail/useTaskDetailGlobalStates.ts:207-223`, `lib/agentRuns/taskActivityFeed.ts:30-64`. | Preference itself needs **no route**. **Yes** for MCP comments `include_activity=true`, `lib/mcp/operations/comments/operation.ts:285-296,334-336`; **Yes** for GET `/api/mcp/tasks/{taskId}/agent-activity`. But exact merged feed needs `link/options/selectedOption/selectedAt/selectedBy`, `lib/agentRuns/model.ts:64-75`. MCP activity select/response omits them: `app/api/mcp/tasks/[taskId]/agent-activity/route.ts:53-72`. Cannot determine interactive elicitation visibility/selection (`taskActivityFeed.ts:48-52`). Remaining blocker is missing feed fields, not history preference auth. |
| R12 | Ctrl/Cmd+Shift+D/F dictation; Alt+V; inbox audio reply; AI recording | Task `app/detail/[...slug]/taskDetailEditingKeymap.ts:150-174`; AI `hooks/MultiPages/AIChat/aiChatKeyboard.ts:119-124`. POST `/api/ai/audio-transcript`. | **No**. `app/api/ai/audio-transcript/route.ts:102-104` session user required. Missing authorized audio/form-data transcription contract; native microphone capture also absent. No fake recording state. |
| R13 | AI Ctrl/Cmd+U; editor Ctrl/Cmd+Shift+A uploads | AI `hooks/MultiPages/AIChat/aiChatKeyboard.ts:115-118`; editor `components/RTE/useTaskDetailEditorKeyboard.tsx:70-73`; picker `components/Common/AttachmentsUpload/FileUploadHandler.tsx:14-18,91-94`. POST `/api/tasks/uploadUrl`, object-storage PUT, buffered POST `/api/tasks/n8nUpload`: `lib/storage/uploadViaApi.ts:95-121,125-140,159-178`. | **No** for web staged upload. Buffered route session auth `pages/api/tasks/n8nUpload.ts:102-126`. Missing authorized upload ticket/grant and native file picker/delivery. MCP `/tasks/attachments` is not a binary upload service; existing hosted URLs do not produce file bytes or upload grants. |
| R14 | C, Ctrl/Cmd+Shift+C full creation; new-task save chords; legacy Ctrl/Cmd+J writer | Full create `hooks/Homepage/useSections.ts:263-310`, task `app/detail/[...slug]/taskDetailEditingKeymap.ts:197-208`; new-task form `components/RTE/TiptapCreateTaskModal.tsx`. POST `/api/tasks/create`; GET/POST `/api/tasks/createGlobally`. | **No** for exact full form, defaults and ranking. Detailed route/field/auth trace: [creation-parity.md](creation-parity.md). Source `lib/api/task-writes/create.ts:17-55`, `lib/api/task-writes/create-global.ts:42,342-390`; session wrapper R4. CLI exception only selected endpoints, `lib/auth/desktopCliTokenUser.ts:7,53-77`, still HTPR-7699-dependent. Ctrl+J compose is a different flag variant. N/Shift+C explicitly skipped. |
| R15 | Ctrl/Cmd+Shift++ task linker; inbox Ctrl/Cmd+Shift+O; Shift+S inbox hint | Runtime `app/detail/[...slug]/taskDetailEditingKeymap.ts:212-218`; linker `components/Modals/SubtaskLinkingModal/SubtaskLinking.tsx:46-57,95-109,122-135`. GET `/api/tasks/searchOrphans`, POST `/api/tasks/addParent`; constants `lib/constants/APIRouteConstants.ts:29-30`. | **No** for exact orphan catalog and empty-result create flow. Session routes `pages/api/tasks/searchOrphans.ts:1-5`, `pages/api/tasks/addParent.ts:1-5`; wrapper R4. **Yes** for MCP `parent_task_id` write (`lib/mcp/tasks/fields/types.ts:70`), but no orphan catalog/searchQuery counterpart, nor full create fallback R14. Native command palette is not the linker. |
| R16 | U inbox unread | `components/notifications/inboxSplit/index.tsx:646-656`. GET `/api/notifications/markAsUnseen`; `lib/api/notification-writes/mark-unseen.ts:13-16,35-42,58-64,73-75`. | **No**. Session wrapper R4. Missing false/toggle seen write. MCP POST `/inbox/seen` only sets true: `app/api/mcp/inbox/seen/route.ts:79-85`. Existing seen-on-open is not U parity. |
| R17 | Z / Ctrl/Cmd+Z global undo | `hooks/General/useUndo.tsx:194-220`, `utils/undoActions/helperFuncs.ts:21-68`. POST `/api/notifications/inbox-zero/undo`, `/api/notifications/(un)archiveBulk`, `/api/savedContent/toggleSaved`. | **No** for whole action-history contract. Session-only routes; saved route's CLI exception only comment rows (`pages/api/savedContent/toggleSaved.ts:17-20`). MCP `/inbox/unarchive` exists but cannot restore the entire heterogeneous web undo stack. Editor undo remains supported. |
| R18 | / and 6 server document search | Shell `hooks/Homepage/useAppShellSurfaceShortcuts.ts:69-76,125`; registry `lib/constants/shortcuts.ts:60,75`; query POST `/api/search/document`, `pages/api/search/document.ts:10-12,30-40`. | **No**. Session route requires searchQuery/projectIds/archive/contextProjectId. Controller `utils/controllers/search/searchDocuments.ts:16` supplies ranked document hits. **Yes** for MCP `/tasks/search` (`app/api/mcp/tasks/search/route.ts:1-3`), but task results differ from full web document/grouping contract. Cached task search remains partial, never called server-document parity. |
| R19 | Ctrl/Cmd+Y video embed | Registry `lib/constants/shortcuts.ts:224`; native URL dialog already inserts HTML. | **No API blocker**. Native rendering limitation, not missing auth. Full browser player cannot be claimed from HTML insertion. Rich-editor/media backend remains outside shortcut dispatch. Existing partial status retained explicitly. |
| R20 | 2xTab sentence completion | Help registry `lib/constants/shortcuts.ts:225`. No runtime double-Tab completion handler found in current source. Actual task editor map `components/RTE/useTaskDetailEditorKeyboard.tsx`; actual Tab focus `hooks/Task Detail/CommentAndDescriptionHooks/useCommentAndDescriptions.ts:607,638-704`. | **No handler or route to port**. Registry-only claim, not proven AI API blocker. Never invent an AI completion response or steal Tab navigation. |

Session authorization evidence: `lib/auth/getSessionUser.ts:21-26,51-79` reads
cookies / browser sessions, never a generic Authorization bearer. The explicit
CLI bridge is endpoint-scoped (`lib/auth/desktopCliTokenUser.ts:7,53-77`).
Desktop can physically address `/api/*`; authorization remains server-owned.
MCP alternatives were checked individually, not blanket-dismissed.
Out-of-scope global destinations remain excluded, not “blocked APIs”.

### Round 3 native checks

- `keyboard_alt_s_toggles_private_task_star_on_every_surface_and_reconciles_real_ids`
- `keyboard_shift_b_picks_only_people_and_clear_without_assigning_them`
- `keyboard_timer_aliases_read_authoritative_state_and_stop_disabled_running_timers`
- `keyboard_timer_unknown_effect_is_never_automatically_replayed`
- `keyboard_new_task_actions_never_escape_text_inputs_or_extra_modifiers`

Evidence: `GATES-keys3.md`; exact-head GitHub CI only.
