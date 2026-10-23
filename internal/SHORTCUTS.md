# Keyboard shortcut parity (HTPR-7381)

Implementation work on branch `keys`: the table statuses below supersede the original source-only audit counts. “Done” means the native handler exists; the final laptop receipt is required for runtime claims. Unsupported commands are hidden, not silent no-ops. Source-only research receipts below are historical, not proof of this implementation.


## Acceptance ledger (written before research)

Scope: source-only audit of inbox, Kanban board, task page, global commands and AI sidebar; no calendar/table-view audit, code changes, builds or tests. Exactly two Markdown deliverables.

- [x] G1: Inventory every in-scope web shortcut and every command-center command; group by screen, missing first; each entry has keys, behavior, web source and native status/source. Count rows from the finished inventory.
  EVIDENCE: Source AST inspection output: SHORTCUT_REGISTRY_COVERAGE 142/142; PALETTE_SOURCE_COVERAGE 295/295 exact declaration locations. Finished Markdown measurement: SHORTCUT_COUNTS 249 {missing:224, partial:18, done:7}; PALETTE 295; MISSING_FIRST_ORDER_OK. Read handler/footer conflicts against the cited web and native dispatchers; duplicate help rows and excluded screen-only rows are explicitly reconciled.
- [x] G2: Document AI components, complete HTTP request/stream contracts, auth, models, context, suggestions, history/new-chat and composer, with source citations.
  EVIDENCE: Read component/send/stream/session/question/model/composer implementations. Output: MODEL_OPTION_COVERAGE 24/24 IDs present; AI_TOPIC_RECONCILIATION 21 required contract/topic anchors present; removed-endpoint control detected. Initial model extraction returned 0/0 and was rejected as non-evidence; rerun requires nonempty AST extraction and produced 24/24. Request and SSE examples were checked against server schemas/framing, not just frontend labels.
- [x] G3: Trace desktop token creation/use through both API-router authentication paths; state supported endpoints and the smallest required server change, with citations.
  EVIDENCE: Read CLI exchange, MCP JWT creation, OAuth claim prefilter, getSessionUser/currentUser, App Router session/question/audio routes, Pages Router mentions and proxy. Output: AUTH_CHAIN_CONFIRMED MCP audience mcp-api; stream OAuth-claims prefilter; session cookie lookup; native stream session bootstrap. Plan corrected after reading ensureNativeChatTurn: stream can create the user-owned session itself; history remains a separate cookie-auth barrier.
- [x] G4: Supply a smallest-first native implementation plan and verify citations, counts and the two-file-only research outputs using source/document inspection commands, not builds or tests.
  EVIDENCE: Read-only document inspection output: SHORTCUTS SOURCE_CITATIONS_VALID 1111; AI-SIDEBAR SOURCE_CITATIONS_VALID 223; CITATIONS_TOTAL 1334; TRACKED_DIFF_BYTES 0; both absolute OUTPUTS_EXIST paths printed. Scope oracle confirmed no calendar/table-operation sections. Plan gives auth → stream → focus/Stop → picker → context → suggestions → history. No code, builds or tests run. git check-ignore confirms both requested outputs are ignored by .gitignore:7 (internal/); empty git status is not proof the documents are absent.

## Conventions and scope

Web citations are relative to `/home/valentin/projects/hypertask`; native citations are relative to this repository. Ctrl means Cmd on macOS except bindings marked **literal Ctrl**; the AI focus shortcut Ctrl+Q is always literal Control. SHIFT/CTRL capitalization is only presentation; arrows separated by / are alternatives, not simultaneous chords. `G → X` is sequential, not a chord. Count = context/action rows, not physical keys; equivalent aliases share a row. Palette definitions are counted separately to avoid double-counting their direct shortcuts.

**done** = matching source handler; **partial** = handler exists but behavior/aliases/context differ; **missing** = no matching implementation. Static source parity, not runtime QA. Missing rows cite the exhaustive native dispatcher, `crates/ht-desktop/src/ui.rs:477`; native footer text at `crates/ht-desktop/src/ui.rs:668` is not implementation. All native text-input bindings are at `crates/ht-desktop/src/input.rs:684`.

The web help registry is included even when advertised behavior conflicts with handlers; caveats below distinguish those cases. Task-targeted “global” keys require a focused task, not arbitrary inbox rows. Calendar/table **screen operations** are excluded; their global navigation/menu entries remain so the command-center inventory is complete. Agent-only shortcut help is outside the ordinary AI sidebar, but its conditional palette commands are cataloged.

## Global Shortcuts

| ID | Keys | Action / caveat | Web file:line | Native status / file:line |
|---|---|---|---|---|
| S001 | Tab / Shift+Tab | Switch search/compose tabs in new-task command window (flagged) | `src/components/Modals/commands/HTC/commands.tsx:672` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S002 | Enter / Shift+Enter | Send compose prompt / insert newline in command-center AI writer | `src/components/Modals/commands/HTC/ComposeTaskWriter.tsx:415` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S003 | Tab / Shift+Tab; Enter / Space | Move between compose prompt and board chip; open board picker | `src/components/Modals/commands/HTC/ComposeTaskWriter.tsx:408` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S004 | ? | Shortcuts | `src/lib/constants/shortcuts.ts:73` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S005 | \\ | Settings | `src/lib/constants/shortcuts.ts:74` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S006 | / | Search | `src/lib/constants/shortcuts.ts:75` | **partial** — cached tasks only; `crates/ht-desktop/src/interactions.rs:180` |
| S007 | P | Set priority | `src/lib/constants/shortcuts.ts:76` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |
| S008 | S | Set task size | `src/lib/constants/shortcuts.ts:77` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |
| S009 | T | Add tags | `src/lib/constants/shortcuts.ts:78` | **partial** — cached tag toggling persists through Write; picker closes after one toggle rather than supporting the web multi-select session; `crates/ht-desktop/src/interactions.rs:930` |
| S010 | Ctrl+K | Open Hypertask Command | `src/lib/constants/shortcuts.ts:83` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |
| S011 | Ctrl+J | Compose task / New Task (feature-flagged) | `src/lib/constants/shortcuts.ts:84` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S012 | Ctrl+Z | Undo latest action | `src/lib/constants/shortcuts.ts:85` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S013 | ; | Use snippet | `src/lib/constants/shortcuts.ts:86` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S014 | G → X | Show or hide archived tasks | `src/lib/constants/shortcuts.ts:87` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S015 | Ctrl+E | Archive task or comment | `src/lib/constants/shortcuts.ts:89` | **partial** — task archive only; no comment archive; `crates/ht-desktop/src/interactions.rs:180` |
| S016 | # | Delete task | `src/lib/constants/shortcuts.ts:90` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |
| S017 | G → B | Go to task board | `src/lib/constants/shortcuts.ts:91` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |
| S018 | G → R | Go to inbox archives | `src/lib/constants/shortcuts.ts:92` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S019 | G → T | Go to running timers | `src/lib/constants/shortcuts.ts:93` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S020 | G → H | Go to reminders | `src/lib/constants/shortcuts.ts:94` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S021 | G → # | Go to trash | `src/lib/constants/shortcuts.ts:95` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S022 | G → A | Go to all tasks | `src/lib/constants/shortcuts.ts:96` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S023 | G → M | Go to my tasks | `src/lib/constants/shortcuts.ts:97` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S024 | G → I | Go to inbox | `src/lib/constants/shortcuts.ts:98` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |
| S025 | G → ; | Go to snippets | `src/lib/constants/shortcuts.ts:99` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S026 | G → E | Go to archived tasks | `src/lib/constants/shortcuts.ts:100` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S027 | G → S | Go to starred | `src/lib/constants/shortcuts.ts:101` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S028 | G → P | Go to pinned | `src/lib/constants/shortcuts.ts:102` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S029 | G → D | Go to drafts | `src/lib/constants/shortcuts.ts:103` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S030 | G → U | Go to scheduled tasks | `src/lib/constants/shortcuts.ts:104` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S031 | G → C | Go to calendar | `src/lib/constants/shortcuts.ts:105` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S032 | Alt+0–9 (macOS Ctrl+0–9) | Switch boards | `src/lib/constants/shortcuts.ts:108` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S033 | Ctrl+B | Switch Kanban boards | `src/lib/constants/shortcuts.ts:109` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |
| S034 | D | Set due date | `src/lib/constants/shortcuts.ts:110` | **partial** — Today/Tomorrow/Next week/clear presets; no calendar date grid; `crates/ht-desktop/src/interactions.rs:180` |
| S035 | SHIFT+M | Move task to a different board | `src/lib/constants/shortcuts.ts:111` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S036 | Alt+Shift+Down / Up | Next / previous team | `src/components/ProviderGlobal/GloablProviders.tsx:995` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S037 | J / Down; K / Up; Enter; Esc | Navigate, execute or dismiss command-center results | `src/components/Modals/commands/HTC/commands.tsx:482` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |
| S038 | E | Archive — only some screens/aliases implemented; see reconciliation | `src/lib/constants/shortcuts.ts:79` | **partial** — context-specific task/notification behavior; `crates/ht-desktop/src/interactions.rs:180` |
| S039 | J | Move focus down — only some screens/aliases implemented; see reconciliation | `src/lib/constants/shortcuts.ts:80` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |
| S040 | K | Move focus up — only some screens/aliases implemented; see reconciliation | `src/lib/constants/shortcuts.ts:81` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |
| S041 | Left / Up / Down / Right | Move focus — only some screens/aliases implemented; see reconciliation | `src/lib/constants/shortcuts.ts:82` | **partial** — board columns and task vertical cycle; no filter-list UI; `crates/ht-desktop/src/interactions.rs:180` |
| S042 | ESC | Exit (go back) — only some screens/aliases implemented; see reconciliation | `src/lib/constants/shortcuts.ts:88` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |
| S043 | TAB | Cycle focus forward — only some screens/aliases implemented; see reconciliation | `src/lib/constants/shortcuts.ts:106` | **partial** — native columns, not saved web views; `crates/ht-desktop/src/interactions.rs:180` |
| S044 | SHIFT+TAB | Cycle focus backward — only some screens/aliases implemented; see reconciliation | `src/lib/constants/shortcuts.ts:107` | **partial** — native columns, not saved web views; `crates/ht-desktop/src/interactions.rs:180` |

## App shell surfaces

| ID | Keys | Action / caveat | Web file:line | Native status / file:line |
|---|---|---|---|---|
| S045 | 1 | Inbox | `src/lib/constants/shortcuts.ts:54` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |
| S046 | 2 | Board | `src/lib/constants/shortcuts.ts:55` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |
| S047 | 3 | Table | `src/lib/constants/shortcuts.ts:56` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S048 | 4 | Calendar | `src/lib/constants/shortcuts.ts:57` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S049 | 5 | Toggle AI chat | `src/lib/constants/shortcuts.ts:58` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S050 | ] | Toggle AI chat (alternative) | `src/lib/constants/shortcuts.ts:59` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S051 | 6 | Search | `src/lib/constants/shortcuts.ts:60` | **partial** — cached tasks only; `crates/ht-desktop/src/interactions.rs:180` |
| S052 | 7 | Running timers | `src/lib/constants/shortcuts.ts:61` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S053 | [ | Collapse / expand sidebar | `src/lib/constants/shortcuts.ts:62` | **missing** — no native feature/UI/write path yet; hidden from the command center |

## Inbox

| ID | Keys | Action / caveat | Web file:line | Native status / file:line |
|---|---|---|---|---|
| S054 | Ctrl+SHIFT+A | Select ALL | `src/lib/constants/shortcuts.ts:249` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |
| S055 | Ctrl+A | Select all from current split | `src/lib/constants/shortcuts.ts:250` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |
| S056 | X | Select message | `src/lib/constants/shortcuts.ts:251` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |
| S057 | SHIFT+E | Bulk Archive — native Shift+E wrongly falls through to single archive, not bulk | `src/lib/constants/shortcuts.ts:252` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |
| S058 | SHIFT+H | Bulk Reminder | `src/lib/constants/shortcuts.ts:253` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S059 | Ctrl+E | Archive task | `src/lib/constants/shortcuts.ts:258` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |
| S060 | Ctrl+F | Focus local inbox search | `src/app/inbox/Inbox.tsx:317` | **partial** — cached inbox task-search modal, not inline filtering; `crates/ht-desktop/src/interactions.rs:180` |
| S061 | Esc | Clear/close local search, otherwise back | `src/app/inbox/Inbox.tsx:286` | **partial** — closes cached search modal; no inline search; `crates/ht-desktop/src/interactions.rs:180` |
| S062 | Z | Undo latest action | `src/app/inbox/Inbox.tsx:333` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S063 | Shift+J / Down; Shift+K / Up | Extend bulk selection while moving | `src/hooks/MultiPages/useBulkActions.ts:132` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |
| S064 | Ctrl+R | Open task in reply mode | `src/components/notifications/inboxSplit/index.tsx:729` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |
| S065 | Alt+V | Open task with dictation | `src/components/notifications/inboxSplit/index.tsx:738` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S066 | Ctrl+Shift+O | Link subtask on selected notification | `src/components/notifications/inboxSplit/index.tsx:771` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S067 | U | Toggle notification seen/unseen | `src/components/notifications/inboxSplit/index.tsx:777` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S068 | Alt+S | Star/unstar selected task | `src/components/notifications/inboxSplit/index.tsx:798` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S069 | G → G / Shift+G → Shift+G | Jump to first/last notification boundary | `src/components/notifications/inboxSplit/index.tsx:648` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |
| S070 | G → T | Focused task board (local handler; global G→T timers can win) | `src/components/notifications/inboxSplit/index.tsx:674` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |
| S071 | C | Create task | `src/app/inbox/Inbox.tsx:395` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S072 | E | Archive | `src/lib/constants/shortcuts.ts:254` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |
| S073 | ENTER | Open task | `src/lib/constants/shortcuts.ts:255` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |
| S074 | J / Down | Move focus down | `src/lib/constants/shortcuts.ts:256` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |
| S075 | K / Up | Move focus up | `src/lib/constants/shortcuts.ts:257` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |
| S076 | Tab / Shift+Tab | Cycle inbox splits | `src/app/inbox/Inbox.tsx:400` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |

## Board

| ID | Keys | Action / caveat | Web file:line | Native status / file:line |
|---|---|---|---|---|
| S077 | Enter / Esc | Submit / cancel inline quick-entry task | `src/components/Common/newTask.tsx:83` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S078 | Esc | Clear bulk task selection | `src/lib/contexts/Kanban/BulkSelectionContext.tsx:315` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |
| S079 | A / T / M / Ctrl+E | Bulk assign / tags / move / archive when cards selected | `src/lib/contexts/Kanban/BulkSelectionContext.tsx:323` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |
| S080 | C | Add task | `src/lib/constants/shortcuts.ts:117` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S081 | Ctrl+J | Add task with AI Task Writer | `src/lib/constants/shortcuts.ts:118` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S082 | Ctrl+SHIFT++ | Add sub-task | `src/lib/constants/shortcuts.ts:119` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S083 | Ctrl+SHIFT+C | Add task at top | `src/lib/constants/shortcuts.ts:120` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S084 | SHIFT+C | Add task at bottom — advertised; current Shift+C handler opens quick entry like N | `src/lib/constants/shortcuts.ts:121` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S085 | ] | Next view | `src/lib/constants/shortcuts.ts:126` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S086 | [ | Previous view | `src/lib/constants/shortcuts.ts:127` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S087 | gg | Move focus to top | `src/lib/constants/shortcuts.ts:129` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |
| S088 | GG | Move focus to bottom | `src/lib/constants/shortcuts.ts:130` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |
| S089 | Shift+J / Shift+Down | Move task down | `src/lib/constants/shortcuts.ts:131` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S090 | Shift+K / Shift+Up | Move task up | `src/lib/constants/shortcuts.ts:132` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S091 | SHIFT+S | Sort Kanban board | `src/lib/constants/shortcuts.ts:136` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S092 | SHIFT+T | Toggle table layout | `src/lib/constants/shortcuts.ts:138` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S093 | Left / Right | Match any / all values (inside a filter's list) | `src/lib/constants/shortcuts.ts:139` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S094 | Ctrl+SHIFT++ | Show task options | `src/lib/constants/shortcuts.ts:140` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S095 | A | Assign a user | `src/lib/constants/shortcuts.ts:142` | **partial** — human assignees only; native API does not assign agents; `crates/ht-desktop/src/interactions.rs:180` |
| S096 | N / Shift+C | Quick-entry input in active column | `src/hooks/Homepage/useHandleKeyDownOperations.ts:93` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S097 | Ctrl+F | Focus local task search/filter | `src/hooks/Homepage/useHandleKeyDownOperations.ts:278` | **partial** — cached board search modal, not inline filter controls; `crates/ht-desktop/src/interactions.rs:180` |
| S098 | Shift+F | Open filters | `src/hooks/Homepage/useHandleKeyDownOperations.ts:186` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S099 | Shift+V | Save unsaved board view | `src/hooks/Homepage/useHandleKeyDownOperations.ts:178` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S100 | Z | Undo latest action | `src/hooks/Homepage/useHandleKeyDownOperations.ts:182` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S101 | G → V | Open saved views | `src/hooks/Homepage/useHandleKeyDownOperations.ts:217` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S102 | G → X | Toggle archived tasks | `src/hooks/Homepage/useHandleKeyDownOperations.ts:226` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S103 | W | Toggle timer when tracking enabled | `src/hooks/Homepage/useHandleKeyDownOperations.ts:157` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S104 | H / Left; L / Right | Focus previous/next column | `src/hooks/Homepage/useHandleKeyDownOperations.ts:203` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |
| S105 | Esc | Dismiss commands/refocus card | `src/hooks/Homepage/useHandleKeyDownOperations.ts:126` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |
| S106 | X | Toggle card selection | `src/components/PageComponents/Kanban/KanbanTaskComponents/task.tsx:357` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |
| S107 | Ctrl+A | Select all board tasks (feature flag) | `src/lib/contexts/Kanban/BulkSelectionContext.tsx:287` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |
| S108 | E | Edit focused task, NOT archive | `src/components/PageComponents/Kanban/KanbanTaskComponents/task.tsx:433` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |
| S109 | S | Set task size | `src/lib/keyboard/taskShortcuts.ts:86` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |
| S110 | P | Set priority | `src/lib/keyboard/taskShortcuts.ts:94` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |
| S111 | T | Set tags | `src/lib/keyboard/taskShortcuts.ts:122` | **partial** — cached tag toggling persists through Write; picker closes after one toggle rather than supporting the web multi-select session; `crates/ht-desktop/src/interactions.rs:930` |
| S112 | D | Set due date | `src/lib/keyboard/taskShortcuts.ts:113` | **partial** — date presets, not a full date grid; `crates/ht-desktop/src/interactions.rs:180` |
| S113 | M | Open move-to-column picker | `src/components/PageComponents/Kanban/KanbanTaskComponents/task.tsx:586` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |
| S114 | Shift+M | Move to another board | `src/components/PageComponents/Kanban/KanbanTaskComponents/task.tsx:609` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S115 | Ctrl+M | Open task and start reply | `src/components/PageComponents/Kanban/KanbanTaskComponents/task.tsx:635` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |
| S116 | Ctrl+E | Archive task | `src/components/PageComponents/Kanban/KanbanTaskComponents/task.tsx:394` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |
| S117 | # / Shift+3 | Delete task | `src/lib/keyboard/taskShortcuts.ts:70` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |
| S118 | Ctrl+S | Share task | `src/lib/keyboard/taskShortcuts.ts:130` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S119 | Ctrl+Shift+; | Copy private URL | `src/components/PageComponents/Kanban/KanbanTaskComponents/task.tsx:662` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |
| S120 | Ctrl+; / Ctrl+, | Copy private title + URL | `src/components/PageComponents/Kanban/KanbanTaskComponents/task.tsx:671` | **partial** — plain-text title + URL only; GPUI clipboard API has no HTML format; web defaults to a rich anchor; `crates/ht-desktop/src/interactions.rs:560` |
| S121 | Ctrl+Shift+. | Copy public URL | `src/components/PageComponents/Kanban/KanbanTaskComponents/task.tsx:686` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S122 | Ctrl+. | Copy public title + URL | `src/components/PageComponents/Kanban/KanbanTaskComponents/task.tsx:691` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S123 | Ctrl+I | Copy title + ticket number | `src/components/PageComponents/Kanban/KanbanTaskComponents/task.tsx:696` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |
| S124 | Ctrl+Shift+I | Copy ticket number | `src/components/PageComponents/Kanban/KanbanTaskComponents/task.tsx:700` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |
| S125 | Ctrl+Shift+O | Create subtask, additional actual binding | `src/components/PageComponents/Kanban/KanbanTaskComponents/task.tsx:724` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S126 | Alt+S | Star/unstar task | `src/lib/keyboard/taskShortcuts.ts:145` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S127 | F | Follow task | `src/components/PageComponents/Kanban/KanbanTaskComponents/task.tsx:743` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S128 | Alt+F | Unfollow task | `src/components/PageComponents/Kanban/KanbanTaskComponents/task.tsx:748` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S129 | Alt+V | Open task and dictate | `src/components/PageComponents/Kanban/KanbanTaskComponents/task.tsx:753` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S130 | F2 | Rename task | `src/lib/keyboard/taskShortcuts.ts:148` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |
| S131 | Shift+L / Shift+Right | Move task right — native Shift+H/L only, no arrow aliases | `src/lib/constants/shortcuts.ts:133` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |
| S132 | Shift+H / Shift+Left | Move task left — native Shift+H/L only, no arrow aliases | `src/lib/constants/shortcuts.ts:134` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |
| S133 | Tab / Shift+Tab | Columns in legacy shell; saved views in new shell (native columns only) | `src/hooks/Homepage/Views/useViewCyclingShortcuts.ts:21` | **partial** — native columns; saved web views absent; `crates/ht-desktop/src/interactions.rs:180` |
| S134 | ENTER | Open task | `src/lib/constants/shortcuts.ts:122` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |
| S135 | J / Down; K / Up | Move task focus down/up | `src/hooks/Homepage/useHandleKeyDownOperations.ts:191` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |

## Task View

| ID | Keys | Action / caveat | Web file:line | Native status / file:line |
|---|---|---|---|---|
| S136 | H | Remind me / set reminder (palette-advertised shortcut) | `src/components/Modals/commands/HTC/taskCommands.ts:164` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S137 | U | Mark task unread (palette-advertised shortcut) | `src/components/Modals/commands/HTC/taskCommands.ts:171` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S138 | Ctrl+J | Write with AI | `src/lib/constants/shortcuts.ts:154` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S139 | A | Assign a user | `src/lib/constants/shortcuts.ts:155` | **partial** — human assignees only; `crates/ht-desktop/src/interactions.rs:180` |
| S140 | SHIFT+B | Blocked by person | `src/lib/constants/shortcuts.ts:156` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S141 | W | Start / stop timer | `src/lib/constants/shortcuts.ts:157` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S142 | B | Log time | `src/lib/constants/shortcuts.ts:158` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S143 | SHIFT+R | Suggest reply with AI | `src/lib/constants/shortcuts.ts:160` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S144 | Alt+0–9 (macOS Ctrl+0–9) | Switch boards | `src/lib/constants/shortcuts.ts:161` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S145 | I | Expand summary | `src/lib/constants/shortcuts.ts:162` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S146 | O | Expand single comment | `src/lib/constants/shortcuts.ts:163` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S147 | SHIFT+O | Expand all comments | `src/lib/constants/shortcuts.ts:164` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S148 | Ctrl+SHIFT+H | Toggle task history / agent log | `src/lib/constants/shortcuts.ts:165` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S149 | Ctrl+O | View all links | `src/lib/constants/shortcuts.ts:166` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S150 | F2 | Edit Title | `src/lib/constants/shortcuts.ts:169` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |
| S151 | CTRL+SHIFT+3 | Delete comment | `src/lib/constants/shortcuts.ts:170` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S152 | Ctrl+SHIFT+V | Paste as plain text | `src/lib/constants/shortcuts.ts:171` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S153 | CTRL+O | View all sub-tasks — advertised conflict with Ctrl+O links; see reconciliation | `src/lib/constants/shortcuts.ts:172` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S154 | Ctrl+S | Share task | `src/lib/constants/shortcuts.ts:173` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S155 | Ctrl+SHIFT++ | Show task options | `src/lib/constants/shortcuts.ts:174` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S156 | Ctrl+SHIFT+; | Private share link (URL only) | `src/lib/constants/shortcuts.ts:175` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |
| S157 | Ctrl+; | Private share link (Title + URL) | `src/lib/constants/shortcuts.ts:176` | **partial** — plain-text title + URL only; GPUI clipboard API has no HTML format; web defaults to a rich anchor; `crates/ht-desktop/src/interactions.rs:560` |
| S158 | Ctrl+SHIFT+. | Public share link (URL only) | `src/lib/constants/shortcuts.ts:177` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S159 | Ctrl+. | Public share link (Title + URL) | `src/lib/constants/shortcuts.ts:178` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S160 | Ctrl+SHIFT+I | Copy task ID | `src/lib/constants/shortcuts.ts:179` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |
| S161 | Ctrl+I | Copy task title and ID | `src/lib/constants/shortcuts.ts:180` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |
| S162 | ALT+S | Star task | `src/lib/constants/shortcuts.ts:181` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S163 | CTRL+SHIFT+S | Star comment | `src/lib/constants/shortcuts.ts:182` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S164 | CTRL+SHIFT+P | Pin comment | `src/lib/constants/shortcuts.ts:183` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S165 | F | Follow task | `src/lib/constants/shortcuts.ts:184` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S166 | ALT+F | Unfollow task | `src/lib/constants/shortcuts.ts:185` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S167 | Ctrl+SHIFT+D | Speech to text | `src/lib/constants/shortcuts.ts:186` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S168 | Ctrl+SHIFT+F | Dictate and Improve | `src/lib/constants/shortcuts.ts:187` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S169 | Ctrl+SHIFT+, | Discard draft | `src/lib/constants/shortcuts.ts:188` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |
| S170 | Ctrl+E | Archive task | `src/app/detail/[...slug]/taskDetailNavigationKeymap.ts:36` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |
| S171 | E | Remove notification / advance inbox flow | `src/app/detail/[...slug]/taskDetailNavigationKeymap.ts:106` | **partial** — notification removal and inbox playlist advance; no unread-mode state; `crates/ht-desktop/src/interactions.rs:180` |
| S172 | J | Next task in playlist/inbox, NOT comment scroll | `src/app/detail/[...slug]/taskDetailNavigationKeymap.ts:125` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |
| S173 | K | Previous task in playlist/inbox | `src/app/detail/[...slug]/taskDetailNavigationKeymap.ts:228` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |
| S174 | S | Task size | `src/app/detail/[...slug]/taskDetailNavigationKeymap.ts:13` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |
| S175 | T | Tags | `src/app/detail/[...slug]/taskDetailNavigationKeymap.ts:50` | **partial** — cached tag toggling persists through Write; picker closes after one toggle rather than supporting the web multi-select session; `crates/ht-desktop/src/interactions.rs:930` |
| S176 | P | Priority | `src/app/detail/[...slug]/taskDetailEditingKeymap.ts:281` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |
| S177 | Shift+M | Move to board | `src/app/detail/[...slug]/taskDetailEditingKeymap.ts:40` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S178 | D | Due date | `src/app/detail/[...slug]/taskDetailEditingKeymap.ts:221` | **partial** — date presets, not a full date grid; `crates/ht-desktop/src/interactions.rs:180` |
| S179 | # / Shift+3 | Delete task | `src/app/detail/[...slug]/taskDetailNavigationKeymap.ts:97` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |
| S180 | C | Create task at top of current section | `src/app/detail/[...slug]/taskDetailEditingKeymap.ts:196` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S181 | Shift+H / Left; Shift+L / Right | Move task to neighboring column | `src/app/detail/[...slug]/taskDetailEditingKeymap.ts:91` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |
| S182 | Alt+T | Toggle timer, additional actual binding | `src/app/detail/[...slug]/taskDetailNavigationKeymap.ts:278` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S183 | Alt+V | Dictate, actual alias | `src/app/detail/[...slug]/taskDetailEditingKeymap.ts:165` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S184 | R | React to focused comment | `src/hooks/Task Detail/CommentAndDescriptionHooks/useCommentAndDescriptions.ts:573` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S185 | L | Fast-like focused comment | `src/hooks/Task Detail/CommentAndDescriptionHooks/useCommentAndDescriptions.ts:578` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S186 | Ctrl+Shift+A | Attach file in focused task editor | `src/hooks/Task Detail/CommentAndDescriptionHooks/useCommentAndDescriptions.ts:535` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S187 | Ctrl+Tab | Consumed placeholder, no action beyond preventing default | `src/app/detail/[...slug]/taskDetailEditingKeymap.ts:125` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S188 | Ctrl+, | Copy private title + URL, actual alias | `src/app/detail/[...slug]/taskDetailNavigationKeymap.ts:169` | **partial** — plain-text title + URL only; GPUI clipboard API has no HTML format; web defaults to a rich anchor; `crates/ht-desktop/src/interactions.rs:560` |
| S189 | ENTER | Activate description/comment or reply to comment — native always edits title | `src/lib/constants/shortcuts.ts:148` | **partial** — description/composer activation; quoted reply editor absent; `crates/ht-desktop/src/interactions.rs:180` |
| S190 | Ctrl+ENTER | Send comment and advance in Inbox — native saves comment but never advances inbox | `src/lib/constants/shortcuts.ts:149` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |
| S191 | Ctrl+SHIFT+ENTER | Send comment and stay on task — native saves comment; Shift distinction absent | `src/lib/constants/shortcuts.ts:153` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |
| S192 | Ctrl+M | Add a comment — native plain single-line comment only | `src/lib/constants/shortcuts.ts:159` | **partial** — plain text composer; rich Tiptap editing/attachments absent; `crates/ht-desktop/src/interactions.rs:180` |
| S193 | literal Ctrl+D | Edit Description — native only scrolls to description, does not edit | `src/lib/constants/shortcuts.ts:168` | **partial** — plain text description editing, not rich HTML round-trip; `crates/ht-desktop/src/interactions.rs:180` |
| S194 | M | Open move-to-column picker (native only toggles moving state) | `src/app/detail/[...slug]/taskDetailEditingKeymap.ts:248` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |
| S195 | Tab / Shift+Tab; Down / Up | Cycle description/comments/composer (native only moving toggle/comment scroll) | `src/hooks/Task Detail/CommentAndDescriptionHooks/useCommentAndDescriptions.ts:607` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |
| S196 | Ctrl+Alt+Shift+Enter | Send and complete in inbox, mode dependent elsewhere (native only saves) | `src/lib/taskDetailInboxFlow.ts:99` | **done** — `crates/ht-desktop/src/shortcuts.rs:51`, `crates/ht-desktop/src/interactions.rs:180` (behavior verification pending) |

## Saving new task

| ID | Keys | Action / caveat | Web file:line | Native status / file:line |
|---|---|---|---|---|
| S197 | Esc | Close create-task modal; while title editing, leave edit mode first | `src/components/Modals/CreateTaskGloballyModal/CreateTaskModalBody.tsx:185` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S198 | Tab | From title move to description | `src/components/Modals/CreateTaskGloballyModal/TaskTitleModal.tsx:33` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S199 | Tab / arrows | Cycle create-task controls; handler currently sends both arrow directions to ArrowRightHandler | `src/components/Modals/CreateTaskGloballyModal/CreateTaskModalBody.tsx:224` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S200 | Enter | Activate selected create-task control | `src/components/Modals/CreateTaskGloballyModal/CreateTaskModalBody.tsx:226` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S201 | A | Assign new task | `src/components/Modals/CreateTaskGloballyModal/CreateTaskModalBody.tsx:193` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S202 | P | Priority on new task | `src/components/Modals/CreateTaskGloballyModal/CreateTaskModalBody.tsx:195` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S203 | D | Due date on new task | `src/components/Modals/CreateTaskGloballyModal/CreateTaskModalBody.tsx:200` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S204 | M / Shift+M | Column / board picker for new task | `src/components/Modals/CreateTaskGloballyModal/CreateTaskModalBody.tsx:204` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S205 | S / T | Size / tags for new task | `src/components/Modals/CreateTaskGloballyModal/CreateTaskModalBody.tsx:209` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S206 | Ctrl+D | Edit new-task description | `src/components/Modals/CreateTaskGloballyModal/CreateTaskModalBody.tsx:228` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S207 | Ctrl+J | AI writer for new task; sidebar/compose interception when enabled | `src/components/Modals/CreateTaskGloballyModal/CreateTaskModalBody.tsx:48` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S208 | Ctrl+Shift+A | Attach file in create-task modal | `src/components/Modals/CreateTaskGloballyModal/CreateTaskModalBody.tsx:187` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S209 | Ctrl+Shift+D / Alt+V | Dictate new-task description | `src/components/Modals/CreateTaskGloballyModal/TaskTitleModal.tsx:54` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S210 | Ctrl+Shift+F | Dictate and improve new-task description | `src/components/Modals/CreateTaskGloballyModal/TaskTitleModal.tsx:63` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S211 | Ctrl+ENTER | Save | `src/lib/constants/shortcuts.ts:194` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S212 | Ctrl+Alt+ENTER | Save and close | `src/lib/constants/shortcuts.ts:195` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S213 | Ctrl+Alt+SHIFT+ENTER | Save and create new task | `src/lib/constants/shortcuts.ts:196` | **missing** — no native feature/UI/write path yet; hidden from the command center |

## Format

| ID | Keys | Action / caveat | Web file:line | Native status / file:line |
|---|---|---|---|---|
| S214 | / | Tiptap menu | `src/lib/constants/shortcuts.ts:223` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S215 | Ctrl+Y | Add Video Embed | `src/lib/constants/shortcuts.ts:224` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S216 | 2xTAB | Autocomplete sentence | `src/lib/constants/shortcuts.ts:225` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S217 | SHIFT+ENTER | Add a line break | `src/lib/constants/shortcuts.ts:226` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S218 | Ctrl+I | Italicize | `src/lib/constants/shortcuts.ts:227` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S219 | Ctrl+U | Underline | `src/lib/constants/shortcuts.ts:228` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S220 | Ctrl+SHIFT+B | Quote | `src/lib/constants/shortcuts.ts:229` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S221 | Ctrl+SHIFT+H | Highlight | `src/lib/constants/shortcuts.ts:230` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S222 | Ctrl+E | Code | `src/lib/constants/shortcuts.ts:231` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S223 | Ctrl+SHIFT+8 | Bullet list | `src/lib/constants/shortcuts.ts:232` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S224 | Ctrl+Z | Undo | `src/lib/constants/shortcuts.ts:233` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S225 | Ctrl+SHIFT+Z | Redo | `src/lib/constants/shortcuts.ts:234` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S226 | Ctrl+Alt+0 | Apply normal text style | `src/lib/constants/shortcuts.ts:235` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S227 | Ctrl+Alt+1 | Apply heading style 1 | `src/lib/constants/shortcuts.ts:236` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S228 | Ctrl+Alt+2 | Apply heading style 2 | `src/lib/constants/shortcuts.ts:237` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S229 | Ctrl+Alt+3 | Apply heading style 3 | `src/lib/constants/shortcuts.ts:238` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S230 | Ctrl+Alt+4 | Apply heading style 4 | `src/lib/constants/shortcuts.ts:239` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S231 | Ctrl+Alt+5 | Apply heading style 5 | `src/lib/constants/shortcuts.ts:240` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S232 | Ctrl+Alt+6 | Apply heading style 6 | `src/lib/constants/shortcuts.ts:241` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S233 | Ctrl+Alt+SHIFT+C | Code block | `src/lib/constants/shortcuts.ts:242` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S234 | Ctrl+SHIFT+C | Code | `src/lib/constants/shortcuts.ts:243` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S235 | Ctrl+B | Bold inside editor; board switcher elsewhere | `src/components/RTE/Tiptap.ts:135` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S236 | Ctrl+Shift+1 / 2 | Heading 1/2, explicit override | `src/components/RTE/useTaskDetailEditorKeyboard.tsx:62` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S237 | Ctrl+Shift+A | Upload attachment in task editor | `src/components/RTE/useTaskDetailEditorKeyboard.tsx:70` | **missing** — no native feature/UI/write path yet; hidden from the command center |

## AI Chat

| ID | Keys | Action / caveat | Web file:line | Native status / file:line |
|---|---|---|---|---|
| S238 | Up / Down; Enter / Space; Tab; Esc | Navigate/select/leave conversation chooser (flagged agent sidebar) | `src/components/AI_CHAT/AgentSidebarChatCommands.tsx:85` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S239 | Ctrl+Shift+/ (Ctrl+?) | Toggle AI chat | `src/lib/constants/shortcuts.ts:202` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S240 | Ctrl+SHIFT+O | Reset session | `src/lib/constants/shortcuts.ts:203` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S241 | CTRL+Q | Switch focus: AI chat / workspace | `src/lib/constants/shortcuts.ts:204` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S242 | Enter | Send message | `src/lib/constants/shortcuts.ts:205` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S243 | Ctrl+Shift+J | Open if closed; New AI Chat if already open | `src/hooks/MultiPages/AIChat/aiChatKeyboard.ts:203` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S244 | Esc | Cancel active streamed reply | `src/hooks/MultiPages/AIChat/aiChatKeyboard.ts:79` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S245 | Shift+Enter | Newline, do not send | `src/hooks/MultiPages/AIChat/aiChatKeyboard.ts:65` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S246 | Ctrl+U | Open attachment chooser | `src/hooks/MultiPages/AIChat/aiChatKeyboard.ts:115` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S247 | Ctrl+Shift+D | Start/stop microphone | `src/hooks/MultiPages/AIChat/aiChatKeyboard.ts:119` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S248 | @; Up / Down; Enter; Esc | Search, navigate, select or dismiss context mention | `src/components/RTE/Components/AI_Chat/MentionData.js:8` | **missing** — no native feature/UI/write path yet; hidden from the command center |
| S249 | / | Skill/slash menu | `src/hooks/MultiPages/AIChat/useAiTiptap.ts:90` | **missing** — no native feature/UI/write path yet; hidden from the command center |

## Handler and footer reconciliation

- Footer sources: `src/components/Global/ConstantsQuickTips.ts:31` (board), `src/components/Global/ConstantsQuickTips.ts:66` (task), `src/components/Global/ConstantsQuickTips.ts:136` (inbox). N / Shift+C is injected by `src/components/Global/BottomSettings_QuickTips.tsx:126` and `src/components/Global/BottomSettings_QuickTips.tsx:241`. Footer labels are evidence of advertised behavior, not independent dispatch.
- Ctrl+K capture: `src/hooks/General/useCommandCenterShortcut.ts:14`. Global G sequences: `src/lib/keyboard/globalGShortcuts.ts:32`. Settings/search/AI: `src/components/ProviderGlobal/GloablProviders.tsx:1073`. Global capture may win over older local G→T task-board mappings; global G→T means timers.
- Task-card target guards: `src/lib/keyboard/taskShortcuts.ts:35`; no bare shortcuts while editing/in dialogs. Native j/k/Tab are modifier-insensitive (`crates/ht-desktop/src/ui.rs:492`), so Ctrl+K moving selection is a collision, NOT a command-center implementation.
- Top insertion Ctrl+Shift+C and current C/Shift+C variants: `src/hooks/Homepage/useSections.ts:263`. Help's “Shift+C at bottom” is stale compared with quick-entry interception. Help gives Ctrl+Shift++ both subtask and options; actual task handler links subtasks (`src/app/detail/[...slug]/taskDetailEditingKeymap.ts:212`). Those advertised behaviors remain separate inventory rows.
- Task J/K change tasks, not comments (`src/app/detail/[...slug]/taskDetailNavigationKeymap.ts:125`, `src/app/detail/[...slug]/taskDetailNavigationKeymap.ts:228`). Native j/k only scroll comment anchors (`crates/ht-desktop/src/ui.rs:588`). Native global movement is partial: inbox/board up/down work, horizontal arrows do not. Native Esc only cancels edit/move or leaves task; no inbox back/modal stack. Native E only archives an inbox notification.
- Help advertises both links and subtasks on Ctrl+O. Earlier links handler (`src/app/detail/[...slug]/taskDetailKeyboard.ts:57`) wins on Windows; both advertisements are recorded, not claimed simultaneously executable. W/B timer/log-time are advertised/palette bindings; task keymap additionally has Alt+T. Literal Ctrl+D remains Ctrl even on Mac.
- Rich-text rows cover app-advertised keys/overrides, not every OS editing key or inherited Tiptap default. Native has only plain-text selection/Home/End/delete/Ctrl+A/C/V/X (`crates/ht-desktop/src/input.rs:684`); paste flattens newlines (`crates/ht-desktop/src/input.rs:147`).

## Complete command-center catalog

Each row is a declared command definition. No direct key means **Ctrl+K → search the command name → Enter**, not an invented hotkey. All native palette entries are **missing**, even if an underlying mouse action exists. Calendar/table destination/settings names are retained only for global menu completeness, not screen implementation scope.

Composition/order and availability: `src/components/Modals/commands/HTC/AllCommands.ts:1300`; Task/Kanban/comment context, selection, mobile, feature flags, ownership and project settings affect visibility. Conditional definitions below are a union, not a claim all appear together. Dispatcher sets the selected mode (`src/components/commandDispatcher.ts:76`); modes with no immediate side effect can open modal content rather than being dead commands. Settings entries open the named section; provider/feature entries do not themselves toggle a secret or feature.

### Global / mobile only

| ID / command key | Keys | Command / behavior | Web file:line | Native status / file:line |
|---|---|---|---|---|
| C001 `reloadApp` | — (palette only) | Reload app — `ReloadApp` | `src/components/Modals/commands/HTC/AllCommands.ts:13` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |

### Inbox

| ID / command key | Keys | Command / behavior | Web file:line | Native status / file:line |
|---|---|---|---|---|
| C002 `swipeThroughUnread` | — (palette only) | Swipe through unread — `SwipeThroughUnread` | `src/components/Modals/commands/HTC/AllCommands.ts:57` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C003 `clearInboxToZero` | — (palette only) | Clear inbox to zero — `ClearInboxToZero` | `src/components/Modals/commands/HTC/AllCommands.ts:63` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C004 `archiveAllReadNotifications` | — (palette only) | Archive all read — `ArchiveAllReadNotifications` | `src/components/Modals/commands/HTC/AllCommands.ts:69` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C005 `archiveReactionNotifications` | — (palette only) | Archive reactions — `ArchiveReactionNotifications` | `src/components/Modals/commands/HTC/AllCommands.ts:75` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |

### Global / team & billing

| ID / command key | Keys | Command / behavior | Web file:line | Native status / file:line |
|---|---|---|---|---|
| C006 `createTeam` | — (palette only) | Create team — `CreateTeam` | `src/components/Modals/commands/HTC/AllCommands.ts:87` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C007 `teamSettings` | — (palette only) | Team settings — `TeamSettings` | `src/components/Modals/commands/HTC/AllCommands.ts:93` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C008 `ManageTeamMembers` | — (palette only) | Manage team members — `ManageTeamMembers` | `src/components/Modals/commands/HTC/AllCommands.ts:100` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C009 `billing` | — (palette only) | Billing — `Billing` | `src/components/Modals/commands/HTC/AllCommands.ts:107` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C010 `manageSubscriptions` | — (palette only) | Manage subscription — `ManageSubscriptions` | `src/components/Modals/commands/HTC/AllCommands.ts:114` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C011 `manageTeamAIAPIKeys` | — (palette only) | Manage API keys — `ManageTeamAIAPIKeys` | `src/components/Modals/commands/HTC/AllCommands.ts:121` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |

### Board

| ID / command key | Keys | Command / behavior | Web file:line | Native status / file:line |
|---|---|---|---|---|
| C012 `toggleBoardZoom` | — (palette only) | commandOptions.boardZoomedOut ? "Zoom board in" : "Zoom board out" — `ToggleBoardZoom` | `src/components/Modals/commands/HTC/AllCommands.ts:136` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C013 `createTask` | C | Create task — `CreateTask` | `src/components/Modals/commands/HTC/AllCommands.ts:146` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C014 `createTaskWithAiWriter` | CTRL+J | Create task with AI Task Writer — `CreateTaskWithAiWriter` | `src/components/Modals/commands/HTC/AllCommands.ts:153` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C015 `newTaskFromTemplate` | — (palette only) | New task from template — `NewTaskFromTemplate` | `src/components/Modals/commands/HTC/AllCommands.ts:160` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C016 `generateStatusUpdate` | — (palette only) | Generate status update — `GenerateStatusUpdate` | `src/components/Modals/commands/HTC/AllCommands.ts:167` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C017 `sendFeedback` | — (palette only) | Send feedback — `SendFeedback` | `src/components/Modals/commands/HTC/AllCommands.ts:174` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C018 `sortBoard` | SHIFT+S | Sort board — `SortKanbanBoard` | `src/components/Modals/commands/HTC/AllCommands.ts:180` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C019 `toggleFilterValueMatch` | — (palette only) | Filter values: match any or all — `ToggleFilterValueMatch` | `src/components/Modals/commands/HTC/AllCommands.ts:189` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C020 `toggleStaleness` | — (palette only) | commandOptions.projectOptions?.stalenessEnabled ? "Turn off staleness for this board" : "Turn on staleness for this board" — `ToggleStaleness` | `src/components/Modals/commands/HTC/AllCommands.ts:199` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C021 `toggleStalenessView` | — (palette only) | commandOptions.projectOptions?.stalenessViewEnabled ? "Turn off staleness for this view" : "Turn on staleness for this view" — `ToggleStalenessView` | `src/components/Modals/commands/HTC/AllCommands.ts:207` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C022 `toggleAutoArchive` | — (palette only) | commandOptions.projectOptions?.autoArchiveEnabled ? "Turn off auto-archive for this board" : "Turn on auto-archive for this board (6 months idle)" — `ToggleAutoArchive` | `src/components/Modals/commands/HTC/AllCommands.ts:215` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C023 `boardVelocityReport` | — (palette only) | Board velocity report — `GotoBoardVelocityReport` | `src/components/Modals/commands/HTC/AllCommands.ts:223` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C024 `sortByTimeInColumn` | — (palette only) | Sort by time in column — `SortByTimeInColumn` | `src/components/Modals/commands/HTC/AllCommands.ts:230` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C025 `sortByLastComment` | — (palette only) | Sort by last comment — `SortByLastComment` | `src/components/Modals/commands/HTC/AllCommands.ts:236` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C026 `autoAssignColumn` | — (palette only) | Auto-assign for this column — `AutoAssignColumn` | `src/components/Modals/commands/HTC/AllCommands.ts:246` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C027 `manageColumns` | — (palette only) | Manage board columns — `ManageColumn` | `src/components/Modals/commands/HTC/AllCommands.ts:254` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C028 `renameColumn` | — (palette only) | Rename board column — `RenameColumn` | `src/components/Modals/commands/HTC/AllCommands.ts:260` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C029 `hideColumn` | — (palette only) | Hide board column — `HideColumn` | `src/components/Modals/commands/HTC/AllCommands.ts:266` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C030 `addColumn` | — (palette only) | Add board column — `AddColumn` | `src/components/Modals/commands/HTC/AllCommands.ts:272` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C031 `deleteColumn` | — (palette only) | Delete board column — `DeleteColumn` | `src/components/Modals/commands/HTC/AllCommands.ts:278` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C032 `inviteBoard` | — (palette only) | Invite people — `InviteMember` | `src/components/Modals/commands/HTC/AllCommands.ts:284` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C033 `manageMembers` | — (palette only) | Manage members — `ManageMembers` | `src/components/Modals/commands/HTC/AllCommands.ts:290` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C034 `goToViews` | G → V | Go to views — `ShowBoardViews` | `src/components/Modals/commands/HTC/AllCommands.ts:296` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C035 `manageViews` | — (palette only) | Manage views — `ManageViews` | `src/components/Modals/commands/HTC/AllCommands.ts:303` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C036 `createSmartSplit` | — (palette only) | Add smart split — `CreateSmartSplit` | `src/components/Modals/commands/HTC/AllCommands.ts:311` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C037 `toggleBoardLayout` | SHIFT+T | Switch to table layout — `ToggleBoardLayout` | `src/components/Modals/commands/HTC/AllCommands.ts:320` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C038 `configureTableColumns` | — (palette only) | Table columns — `ConfigureTableColumns` | `src/components/Modals/commands/HTC/AllCommands.ts:327` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C039 `manageLabels` | — (palette only) | Manage tags — `ManageLabels` | `src/components/Modals/commands/HTC/AllCommands.ts:333` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C040 `createCustomField` | — (palette only) | Create custom field… — `CreateCustomField` | `src/components/Modals/commands/HTC/AllCommands.ts:339` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C041 `manageCustomFields` | — (palette only) | Manage custom fields… — `ManageCustomFields` | `src/components/Modals/commands/HTC/AllCommands.ts:345` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C042 `toggleEmptyColumns` | — (palette only) | Hide empty board columns — `ToggleEmptyColumns` | `src/components/Modals/commands/HTC/AllCommands.ts:352` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C043 `toggleArchivedOnBoard` | G → X | `Hide / Show archived tasks on board` — `ToggleArchivedOnBoard` | `src/components/Modals/commands/HTC/AllCommands.ts:362` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C044 `renameBoard` | — (palette only) | Rename board — `EditBoard` | `src/components/Modals/commands/HTC/AllCommands.ts:371` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C045 `copyBoardLink` | — (palette only) | Copy board join link — `BoardJoinResetLink` | `src/components/Modals/commands/HTC/AllCommands.ts:377` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C046 `resetBoardLink` | — (palette only) | Reset board join link — `BoardJoinResetLink` | `src/components/Modals/commands/HTC/AllCommands.ts:383` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C047 `manageTeams` | — (palette only) | Manage boards — `ManageTeams` | `src/components/Modals/commands/HTC/AllCommands.ts:389` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C048 `createBoard` | — (palette only) | Create board — `NewBoard` | `src/components/Modals/commands/HTC/AllCommands.ts:395` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C049 `boardCreationAssistant` | — (palette only) | Board creation assistant — `BoardCreationAssistant` | `src/components/Modals/commands/HTC/AllCommands.ts:401` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C050 `archiveBoards` | — (palette only) | Archive board — `ArchiveBoard` | `src/components/Modals/commands/HTC/AllCommands.ts:408` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C051 `deleteBoard` | — (palette only) | Delete board — `DeleteBoard` | `src/components/Modals/commands/HTC/AllCommands.ts:414` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C052 `toggleRailExpanded` | [ | Collapse / expand sidebar — `ToggleRailExpanded` | `src/components/Modals/commands/HTC/AllCommands.ts:420` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C053 `toggleAppShellRail` | — (palette only) | commandOptions.appShellRailOn ? "Old Hypertask Design" : "New Hypertask Design" — `ToggleAppShellRail` | `src/components/Modals/commands/HTC/AllCommands.ts:427` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |

### AI sidebar

| ID / command key | Keys | Command / behavior | Web file:line | Native status / file:line |
|---|---|---|---|---|
| C054 `aiChatInterface` | — (palette only) | Chat with AI — `AIChatInterface` | `src/components/Modals/commands/HTC/AllCommands.ts:441` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C055 `pinAiChatOpen` | — (palette only) | Pin AI chat open — `PinAIChatOpen` | `src/components/Modals/commands/HTC/AllCommands.ts:447` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C056 `fullScreenAiChat` | — (palette only) | Open AI chat full screen — `FullScreenAIChat` | `src/components/Modals/commands/HTC/AllCommands.ts:453` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C057 `branchInNewChat` | — (palette only) | Ask AI about this — `BranchInNewChat` | `src/components/Modals/commands/HTC/AllCommands.ts:459` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C058 `toggleAIChatView` | — (palette only) | Switch AI chat mode — `ToggleAIChatView` | `src/components/Modals/commands/HTC/AllCommands.ts:465` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C059 `createAgent` | — (palette only) | Create agent — `CreateAgent` | `src/components/Modals/commands/HTC/AllCommands.ts:471` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C060 `manageAgents` | — (palette only) | Manage agents — `ManageAgents` | `src/components/Modals/commands/HTC/AllCommands.ts:477` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C061 `disabledAgents` | — (palette only) | Disabled agents — `DisabledAgents` | `src/components/Modals/commands/HTC/AllCommands.ts:484` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C062 `deleteAllChats` | — (palette only) | Delete all chat sessions — `DeleteAllChats` | `src/components/Modals/commands/HTC/AllCommands.ts:490` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |

### Global / appearance

| ID / command key | Keys | Command / behavior | Web file:line | Native status / file:line |
|---|---|---|---|---|
| C063 `systemTheme` | — (palette only) | Follow system — `ToggleSystemTheme` | `src/components/Modals/commands/HTC/AllCommands.ts:502` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C064 `lightTheme` | — (palette only) | Light · Porcelain — `ToggleWhiteTheme` | `src/components/Modals/commands/HTC/AllCommands.ts:508` | **done** — `crates/ht-desktop/src/command_center.rs:19`, `crates/ht-desktop/src/interactions.rs:180` (supported native action; behavior pending) |
| C065 `darkTheme` | — (palette only) | Dark · Graphite — `ToggleDarkTheme` | `src/components/Modals/commands/HTC/AllCommands.ts:514` | **done** — `crates/ht-desktop/src/command_center.rs:19`, `crates/ht-desktop/src/interactions.rs:180` (supported native action; behavior pending) |
| C066 `amoledTheme` | — (palette only) | OLED black · AMOLED — `ToggleAmoledTheme` | `src/components/Modals/commands/HTC/AllCommands.ts:520` | **done** — `crates/ht-desktop/src/command_center.rs:19`, `crates/ht-desktop/src/interactions.rs:180` (supported native action; behavior pending) |
| C067 `paperTheme` | — (palette only) | Paper · Dia — `ToggleDiaTheme` | `src/components/Modals/commands/HTC/AllCommands.ts:526` | **done** — `crates/ht-desktop/src/command_center.rs:19`, `crates/ht-desktop/src/interactions.rs:180` (supported native action; behavior pending) |
| C068 `profilePicture` | — (palette only) | Profile picture — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:532` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C069 `switchAccount` | — (palette only) | Switch account — `SwitchAccount` | `src/components/Modals/commands/HTC/AllCommands.ts:539` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C070 `manageFavorites` | — (palette only) | Manage favorites — `ManageFavorites` | `src/components/Modals/commands/HTC/AllCommands.ts:545` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C071 `calendarSettings` | — (palette only) | Calendar settings — `CalendarSettings` | `src/components/Modals/commands/HTC/AllCommands.ts:551` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C072 `toggleCalendarWeekends` | — (palette only) | Show weekends — `ToggleCalendarWeekends` | `src/components/Modals/commands/HTC/AllCommands.ts:558` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C073 `calendarWeekStartsMonday` | — (palette only) | Week starts on Monday — `CalendarWeekStartsMonday` | `src/components/Modals/commands/HTC/AllCommands.ts:564` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C074 `calendarWeekStartsSunday` | — (palette only) | Week starts on Sunday — `CalendarWeekStartsSunday` | `src/components/Modals/commands/HTC/AllCommands.ts:570` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C075 `subscribeGoogleCalendar` | — (palette only) | Subscribe in Google Calendar — `SubscribeGoogleCalendar` | `src/components/Modals/commands/HTC/AllCommands.ts:576` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |

### Global / snippets

| ID / command key | Keys | Command / behavior | Web file:line | Native status / file:line |
|---|---|---|---|---|
| C076 `useSnippet` | ; | Use snippet — `UseSnippet` | `src/components/Modals/commands/HTC/AllCommands.ts:590` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C077 `createSnippet` | — (palette only) | Create snippet — `CreateSnippet` | `src/components/Modals/commands/HTC/AllCommands.ts:597` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C078 `createSnippetFromDraft` | — (palette only) | Create snippet from draft — `CreateSnippetFromDraft` | `src/components/Modals/commands/HTC/AllCommands.ts:603` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |

### Global / settings

| ID / command key | Keys | Command / behavior | Web file:line | Native status / file:line |
|---|---|---|---|---|
| C079 `settingsAnnouncementsAnnouncementAlerts` | — (palette only) | Settings: Announcement alerts — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:615` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C080 `settingsCalendarKeepTasksUpdated` | — (palette only) | Settings: Keep tasks updated — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:622` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C081 `settingsCalendarShowWeekends` | — (palette only) | Settings: Show weekends — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:629` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C082 `settingsApiKeysOpenaiApiKey` | — (palette only) | Settings: OpenAI API key — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:636` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C083 `settingsApiKeysAnthropicApiKey` | — (palette only) | Settings: Anthropic API key — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:643` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C084 `settingsApiKeysGoogleApiKey` | — (palette only) | Settings: Google API key — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:650` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C085 `settingsApiKeysDeepseekApiKey` | — (palette only) | Settings: DeepSeek API key — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:657` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C086 `settingsApiKeysMoonshotKimiApiKey` | — (palette only) | Settings: Moonshot (Kimi) API key — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:664` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C087 `settingsApiKeysZAiGlmApiKey` | — (palette only) | Settings: Z.ai (GLM) API key — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:671` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C088 `settingsApiKeysAlibabaQwenApiKey` | — (palette only) | Settings: Alibaba (Qwen) API key — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:678` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C089 `settingsApiKeysOpenrouterApiKey` | — (palette only) | Settings: OpenRouter API key — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:685` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C090 `settingsApiKeysVercelAiGatewayApiKey` | — (palette only) | Settings: Vercel AI Gateway API key — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:692` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C091 `settingsApiKeysCustomEndpointOperatesUnderAnEuUsDataAgreement` | — (palette only) | Settings: Custom endpoint EU/US agreement — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:699` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C092 `settingsAiModelsGdprSafeMode` | — (palette only) | Settings: GDPR safe mode — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:706` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C093 `settingsAiModelsOpenai` | — (palette only) | Settings: OpenAI — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:713` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C094 `settingsAiModelsAnthropic` | — (palette only) | Settings: Anthropic — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:720` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C095 `settingsAiModelsGoogle` | — (palette only) | Settings: Google — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:727` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C096 `settingsAiModelsDeepseek` | — (palette only) | Settings: DeepSeek — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:734` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C097 `settingsAiModelsMoonshotKimi` | — (palette only) | Settings: Moonshot (Kimi) — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:741` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C098 `settingsAiModelsZAiGlm` | — (palette only) | Settings: Z.ai (GLM) — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:748` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C099 `settingsAiModelsAlibabaQwen` | — (palette only) | Settings: Alibaba (Qwen) — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:755` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C100 `settingsAiModelsOpenrouter` | — (palette only) | Settings: OpenRouter — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:762` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C101 `settingsAiFeaturesAiChat` | — (palette only) | Settings: AI chat — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:769` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C102 `settingsAiFeaturesTaskWriter` | — (palette only) | Settings: Task writer — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:776` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C103 `settingsAiFeaturesWriteWithAi` | — (palette only) | Settings: Write with AI — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:783` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C104 `settingsAiFeaturesImproveWriting` | — (palette only) | Settings: Improve writing — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:790` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C105 `settingsAiFeaturesAskAi` | — (palette only) | Settings: Ask AI — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:797` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C106 `settingsAiFeaturesBoardGeneration` | — (palette only) | Settings: Board generation — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:804` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C107 `settingsAiFeaturesImageGeneration` | — (palette only) | Settings: Image generation — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:811` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C108 `settingsAiFeaturesHyperai` | — (palette only) | Settings: @HyperAI — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:818` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C109 `settingsAiFeaturesTaskSummaries` | — (palette only) | Settings: Task summaries — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:825` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C110 `settingsAiFeaturesQuestionSuggestions` | — (palette only) | Settings: Question suggestions — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:832` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C111 `settingsAiFeaturesDictation` | — (palette only) | Settings: Dictation — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:839` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C112 `settingsBoardMemoryLearnFromAiCorrections` | — (palette only) | Settings: Learn from AI corrections — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:846` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C113 `settingsBoardGeneralBoardNotifications` | — (palette only) | Settings: Board notifications — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:853` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C114 `settingsBoardGeneralShowTotalTimeOnTasks` | — (palette only) | Settings: Show total time on tasks — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:860` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C115 `settingsBoardPlanningEnableCycles` | — (palette only) | Settings: Enable cycles — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:867` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C116 `settingsBoardStalenessStaleness` | — (palette only) | Settings: Staleness — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:874` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C117 `settingsBoardStalenessStaleReminders` | — (palette only) | Settings: Stale reminders — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:881` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C118 `settingsBoardStalenessAutoArchive` | — (palette only) | Settings: Auto-archive — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:888` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C119 `settingsNotificationsPushNotificationsOnThisDevice` | — (palette only) | Settings: Push Notifications on this device — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:895` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C120 `settingsNotificationsMentionsNotifications` | — (palette only) | Settings: Mentions notifications — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:902` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C121 `settingsNotificationsCommentsNotifications` | — (palette only) | Settings: Comments notifications — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:909` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C122 `settingsNotificationsAssignmentsNotifications` | — (palette only) | Settings: Assignments notifications — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:916` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C123 `settingsNotificationsTaskUpdatesNotifications` | — (palette only) | Settings: Task updates notifications — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:923` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C124 `settingsNotificationsDueDatesNotifications` | — (palette only) | Settings: Due dates notifications — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:930` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C125 `settingsTaskPageOpenAiChatByDefault` | — (palette only) | Settings: Open AI chat by default — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:937` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C126 `settingsTaskPageCollapseComments` | — (palette only) | Settings: Collapse Comments — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:944` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C127 `settingsTaskPagePlayGifs` | — (palette only) | Settings: Play GIFs — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:951` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C128 `settingsTaskPageSuggestDescriptionsFromTaskTitles` | — (palette only) | Settings: Suggest descriptions from task titles — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:958` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C129 `settingsTaskPageShowTaskHistory` | — (palette only) | Settings: Show task history — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:965` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C130 `settingsTaskPageReadReceipts` | — (palette only) | Settings: Read receipts — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:972` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C131 `settingsInboxArchiveAndGoToNextTaskAfterSendingAComment` | — (palette only) | Settings: Archive after commenting — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:979` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C132 `settingsInboxDisplayAvatarInInbox` | — (palette only) | Settings: Display avatar in inbox — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:986` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C133 `settingsLearnQuickTips` | — (palette only) | Settings: Quick tips — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:993` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C134 `settingsGeneral` | — (palette only) | Settings: General — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:1000` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C135 `settingsAppearance` | — (palette only) | Settings: Appearance — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:1007` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C136 `settingsTaskPage` | — (palette only) | Settings: Task page — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:1014` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C137 `settingsNotifications` | — (palette only) | Settings: Notifications — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:1021` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C138 `settingsAccounts` | — (palette only) | Settings: Accounts — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:1028` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C139 `settingsBilling` | — (palette only) | Settings: Billing — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:1035` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C140 `settingsPlans` | — (palette only) | Settings: Plans — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:1042` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C141 `settingsAiUsage` | — (palette only) | Settings: AI usage — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:1049` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C142 `settingsMemberUsage` | — (palette only) | Settings: Usage by member — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:1056` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C143 `settingsAiModels` | — (palette only) | Settings: AI models — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:1063` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C144 `settingsAiFeatures` | — (palette only) | Settings: AI features — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:1070` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C145 `settingsDefaultModels` | — (palette only) | Settings: Default models — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:1077` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C146 `settingsApiKeys` | — (palette only) | Settings: Bring your own key — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:1084` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C147 `settingsMembers` | — (palette only) | Settings: Members — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:1091` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C148 `settingsTeamAgents` | — (palette only) | Settings: Agents (team) — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:1098` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C149 `settingsSkills` | — (palette only) | Settings: Skills — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:1105` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C150 `settingsBoardGeneral` | — (palette only) | Settings: Time tracking (board) — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:1112` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C151 `settingsBoardTicketPrefix` | — (palette only) | Settings: Ticket prefix (board) — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:1119` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C152 `settingsBoardMembers` | — (palette only) | Settings: Members (board) — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:1126` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C153 `settingsBoardAgents` | — (palette only) | Settings: Agents (board) — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:1133` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C154 `settingsBoardAi` | — (palette only) | Settings: Custom instructions — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:1140` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C155 `settingsBoardSkills` | — (palette only) | Settings: Skills (board) — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:1147` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C156 `settingsBoardFiles` | — (palette only) | Settings: Files — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:1154` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C157 `settingsSlack` | — (palette only) | Settings: Slack — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:1161` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C158 `settingsMcp` | — (palette only) | Settings: MCP — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:1168` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C159 `settingsCli` | — (palette only) | Settings: CLI — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:1175` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C160 `settingsApi` | — (palette only) | Settings: REST API — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:1182` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C161 `settingsShortcuts` | — (palette only) | Settings: Shortcuts — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:1189` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C162 `settingsLearn` | — (palette only) | Settings: Learn Hypertask — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:1196` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |

### Global / help

| ID / command key | Keys | Command / behavior | Web file:line | Native status / file:line |
|---|---|---|---|---|
| C163 `shortcuts` | ? | Keyboard shortcuts — `Shortcut` | `src/components/Modals/commands/HTC/AllCommands.ts:1209` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C164 `settings` | \\ | Settings — `Setting` | `src/components/Modals/commands/HTC/AllCommands.ts:1216` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C165 `boardSettings` | — (palette only) | Board settings — `BoardSettings` | `src/components/Modals/commands/HTC/AllCommands.ts:1223` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C166 `helpCenter` | — (palette only) | Help center — `HelpCenter` | `src/components/Modals/commands/HTC/AllCommands.ts:1229` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C167 `onboardingTour` | — (palette only) | Onboarding tour — `GoToOnboarding` | `src/components/Modals/commands/HTC/AllCommands.ts:1235` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C168 `quickTips` | — (palette only) | Quick tips — `QuickTips` | `src/components/Modals/commands/HTC/AllCommands.ts:1241` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C169 `latestUpdates` | — (palette only) | What's new — `ShowAnnouncements` | `src/components/Modals/commands/HTC/AllCommands.ts:1247` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C170 `goToWelcome` | — (palette only) | Welcome tutorial — `GoToWelcome` | `src/components/Modals/commands/HTC/AllCommands.ts:1253` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C171 `startKanbanTutorial` | — (palette only) | Start Kanban tutorial — `StartKanbanTutorial` | `src/components/Modals/commands/HTC/AllCommands.ts:1259` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C172 `startTaskWriterTutorial` | — (palette only) | Start task writer tutorial — `StartTaskWriterTutorial` | `src/components/Modals/commands/HTC/AllCommands.ts:1265` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C173 `generateMcpToken` | — (palette only) | MCP — `GenerateMcpToken` | `src/components/Modals/commands/HTC/AllCommands.ts:1271` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C174 `cliInstall` | — (palette only) | CLI — `CliInstall` | `src/components/Modals/commands/HTC/AllCommands.ts:1278` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C175 `restApi` | — (palette only) | REST API — `RestApi` | `src/components/Modals/commands/HTC/AllCommands.ts:1285` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C176 `logout` | — (palette only) | Sign out — `Logout` | `src/components/Modals/commands/HTC/AllCommands.ts:1291` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |

### Global / appearance (flagged)

| ID / command key | Keys | Command / behavior | Web file:line | Native status / file:line |
|---|---|---|---|---|
| C177 `roundedTheme` | — (palette only) | Material (follows device) — `ToggleRoundedTheme` | `src/components/Modals/commands/HTC/AllCommands.ts:1328` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |

### Task / focused card

| ID / command key | Keys | Command / behavior | Web file:line | Native status / file:line |
|---|---|---|---|---|
| C178 `createTask` | C | Create task — `CreateTask` | `src/components/Modals/commands/HTC/taskCommands.ts:8` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C179 `setDueDate` | D | Set due date — `SetDueDate` | `src/components/Modals/commands/HTC/taskCommands.ts:15` | **done** — `crates/ht-desktop/src/command_center.rs:19`, `crates/ht-desktop/src/interactions.rs:180` (supported native action; behavior pending) |
| C180 `setStartDate` | — (palette only) | Set start date — `SetStartDate` | `src/components/Modals/commands/HTC/taskCommands.ts:22` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C181 `setRecurrence` | — (palette only) | Repeat task — `SetRecurrence` | `src/components/Modals/commands/HTC/taskCommands.ts:29` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C182 `saveTaskTemplate` | — (palette only) | Save task as template — `SaveTaskTemplate` | `src/components/Modals/commands/HTC/taskCommands.ts:36` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C183 `assignUser` | A | Assign task — `OpenAssignModal` | `src/components/Modals/commands/HTC/taskCommands.ts:43` | **done** — `crates/ht-desktop/src/command_center.rs:19`, `crates/ht-desktop/src/interactions.rs:180` (supported native action; behavior pending) |
| C184 `blockedByPerson` | SHIFT+B | Blocked by person… — `OpenBlockedByModal` | `src/components/Modals/commands/HTC/taskCommands.ts:50` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C185 `assignToMe` | — (palette only) | Assign to me — `AssignToMe` | `src/components/Modals/commands/HTC/taskCommands.ts:57` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C186 `addSubTask` | CTRL+SHIFT++ | Create subtask — `CreateSubTask` | `src/components/Modals/commands/HTC/taskCommands.ts:64` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C187 `viewSubTask` | CTRL+O | View subtasks — `ViewSubTasks` | `src/components/Modals/commands/HTC/taskCommands.ts:71` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C188 `addRelatedTask` | — (palette only) | Add related task — `AddRelatedTask` | `src/components/Modals/commands/HTC/taskCommands.ts:78` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C189 `markBlockedBy` | — (palette only) | Mark blocked by — `MarkBlockedBy` | `src/components/Modals/commands/HTC/taskCommands.ts:84` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C190 `markAsBlocking` | — (palette only) | Mark as blocking — `MarkAsBlocking` | `src/components/Modals/commands/HTC/taskCommands.ts:90` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C191 `markDuplicateOf` | — (palette only) | Mark duplicate of — `MarkDuplicateOf` | `src/components/Modals/commands/HTC/taskCommands.ts:96` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C192 `labelmodal` | T | Set tags — `LabelModal` | `src/components/Modals/commands/HTC/taskCommands.ts:102` | **done** — `crates/ht-desktop/src/command_center.rs:19`, `crates/ht-desktop/src/interactions.rs:180` (supported native action; behavior pending) |
| C193 `estimateModal` | S | Set task size — `EstimateModal` | `src/components/Modals/commands/HTC/taskCommands.ts:109` | **done** — `crates/ht-desktop/src/command_center.rs:19`, `crates/ht-desktop/src/interactions.rs:180` (supported native action; behavior pending) |
| C194 `priorityModal` | P | Set priority — `PriorityModal` | `src/components/Modals/commands/HTC/taskCommands.ts:116` | **done** — `crates/ht-desktop/src/command_center.rs:19`, `crates/ht-desktop/src/interactions.rs:180` (supported native action; behavior pending) |
| C195 `moveToColumn` | M | Move task to column — `MoveToColumn` | `src/components/Modals/commands/HTC/taskCommands.ts:123` | **done** — `crates/ht-desktop/src/command_center.rs:19`, `crates/ht-desktop/src/interactions.rs:180` (supported native action; behavior pending) |
| C196 `movetasktodifferentboard` | SHIFT+M | Move task to board — `MoveTaskToBoard` | `src/components/Modals/commands/HTC/taskCommands.ts:130` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C197 `followtask` | F | Follow task — `FollowTask` | `src/components/Modals/commands/HTC/taskCommands.ts:137` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C198 `unfollowtask` | ALT+F | Unfollow task — `UnFollowTask` | `src/components/Modals/commands/HTC/taskCommands.ts:144` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C199 `openAiTaskWriter` | — (palette only) | Open AI task writer — `OpenAiTaskWriter` | `src/components/Modals/commands/HTC/taskCommands.ts:151` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C200 `summarizeTicket` | — (palette only) | Summarize ticket — `SummarizeTicket` | `src/components/Modals/commands/HTC/taskCommands.ts:157` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C201 `remindMe` | H | Remind me — `RemindMe` | `src/components/Modals/commands/HTC/taskCommands.ts:164` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C202 `markAsUnread` | U | Mark unread — `MarkUnread` | `src/components/Modals/commands/HTC/taskCommands.ts:171` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C203 `renameTask` | — (palette only) | Rename task — `RenameTask` | `src/components/Modals/commands/HTC/taskCommands.ts:297` | **done** — `crates/ht-desktop/src/command_center.rs:19`, `crates/ht-desktop/src/interactions.rs:180` (supported native action; behavior pending) |
| C204 `acceptTask` | — (palette only) | Accept task — `AcceptTask` | `src/components/Modals/commands/HTC/taskCommands.ts:305` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C205 `declineTask` | — (palette only) | Decline task — `DeclineTask` | `src/components/Modals/commands/HTC/taskCommands.ts:311` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C206 `declineAsDuplicateOf` | — (palette only) | Decline as duplicate of… — `DeclineAsDuplicateOf` | `src/components/Modals/commands/HTC/taskCommands.ts:317` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C207 `duplicateTask` | — (palette only) | Duplicate task — `DuplicateTask` | `src/components/Modals/commands/HTC/taskCommands.ts:323` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C208 `duplicateTaskToBoard` | — (palette only) | Copy task to another board — `DuplicateTaskToBoard` | `src/components/Modals/commands/HTC/taskCommands.ts:329` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C209 `archiveTask` | CTRL+E | `Unarchive / Archive task` — `ArchiveTask` | `src/components/Modals/commands/HTC/taskCommands.ts:335` | **done** — `crates/ht-desktop/src/command_center.rs:19`, `crates/ht-desktop/src/interactions.rs:180` (supported native action; behavior pending) |
| C210 `deleteTask` | SHIFT+3 | Delete task — `DeleteTask` | `src/components/Modals/commands/HTC/taskCommands.ts:342` | **done** — `crates/ht-desktop/src/command_center.rs:19`, `crates/ht-desktop/src/interactions.rs:180` (supported native action; behavior pending) |
| C211 `copyformattedTaskLink` | CTRL+: | Copy private task link (title + URL) — `CopyFormattedURLTask` | `src/components/Modals/commands/HTC/taskCommands.ts:349` | **done** — `crates/ht-desktop/src/command_center.rs:19`, `crates/ht-desktop/src/interactions.rs:180` (supported native action; behavior pending) |
| C212 `copyLink` | CTRL+SHIFT+: | Copy private task URL — `CopyUrlTask` | `src/components/Modals/commands/HTC/taskCommands.ts:356` | **done** — `crates/ht-desktop/src/command_center.rs:19`, `crates/ht-desktop/src/interactions.rs:180` (supported native action; behavior pending) |
| C213 `copypublicformattedTaskLink` | CTRL+. | Copy public task link (title + URL) — `CopyPublicFormattedUrlTask` | `src/components/Modals/commands/HTC/taskCommands.ts:363` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C214 `copyPublicLink` | CTRL+SHIFT+. | Copy public task URL — `CopyPublicUrlTask` | `src/components/Modals/commands/HTC/taskCommands.ts:370` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C215 `setReminder` | H | Set reminder — `SetReminder` | `src/components/Modals/commands/HTC/taskCommands.ts:409` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C216 `starTask` | ALT+S | `Unstar / Star task` — `StarTask` | `src/components/Modals/commands/HTC/taskCommands.ts:417` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C217 `shareTaskPublicly` | CTRL+S | Share task — `ShareTaskPublic` | `src/components/Modals/commands/HTC/taskCommands.ts:424` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C218 `copyTaskId` | CTRL+SHIFT+I | Copy task ID — `CopyTaskID` | `src/components/Modals/commands/HTC/taskCommands.ts:437` | **done** — `crates/ht-desktop/src/command_center.rs:19`, `crates/ht-desktop/src/interactions.rs:180` (supported native action; behavior pending) |
| C219 `copyTaskTitleId` | CTRL+I | Copy task title and ID — `CopyTaskTitleAndID` | `src/components/Modals/commands/HTC/taskCommands.ts:444` | **done** — `crates/ht-desktop/src/command_center.rs:19`, `crates/ht-desktop/src/interactions.rs:180` (supported native action; behavior pending) |
| C220 `copyBranchName` | — (palette only) | Copy branch name — `CopyBranchName` | `src/components/Modals/commands/HTC/taskCommands.ts:451` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C221 `toggleHistory` | CTRL+SHIFT+H | `Hide / Show agent log` — `ToggleHistory` | `src/components/Modals/commands/HTC/taskCommands.ts:459` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C222 `taskDescriptionVersions` | — (palette only) | Description version history — `TaskDescriptionVersions` | `src/components/Modals/commands/HTC/taskCommands.ts:468` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C223 `toggleAllComments` | SHIFT+O | Expand / collapse all comments — `ToggleAllComments` | `src/components/Modals/commands/HTC/taskCommands.ts:476` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C224 `removeTaskNotification` | E | Remove task notification — `RemoveTaskNotification` | `src/components/Modals/commands/HTC/taskCommands.ts:485` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C225 `removeSubtask` | — (palette only) | Remove a sub-task — `RemoveSubtask` | `src/components/Modals/commands/HTC/taskCommands.ts:494` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C226 `removeParenttask` | — (palette only) | Remove as sub-task — `RemoveParent` | `src/components/Modals/commands/HTC/taskCommands.ts:502` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C227 `showInInbox` | — (palette only) | Move task to inbox — `ShowInInbox` | `src/components/Modals/commands/HTC/taskCommands.ts:510` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C228 `subtaskSettings` | — (palette only) | Sub-task Settings — `SubtaskSettings` | `src/components/Modals/commands/HTC/taskCommands.ts:516` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C229 `suggestReply` | SHIFT+R | Suggest reply — `SuggestReply` | `src/components/Modals/commands/HTC/taskCommands.ts:523` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C230 `speechToText` | CTRL+SHIFT+D | Speech to text — `SpeechToText` | `src/components/Modals/commands/HTC/taskCommands.ts:532` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |

### Task / focused comment

| ID / command key | Keys | Command / behavior | Web file:line | Native status / file:line |
|---|---|---|---|---|
| C231 `editcomment` | — (palette only) | Edit comment — `EditComment` | `src/components/Modals/commands/HTC/taskCommands.ts:186` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C232 `summarizeComment` | — (palette only) | Summarize comment — `SummarizeComment` | `src/components/Modals/commands/HTC/taskCommands.ts:193` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C233 `fastLikeComment` | — (palette only) | Fast like — `FastLikeComment` | `src/components/Modals/commands/HTC/taskCommands.ts:200` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C234 `replyToComment` | ENTER | Reply to comment — `ReplyToComment` | `src/components/Modals/commands/HTC/taskCommands.ts:206` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C235 `reactToComment` | R | React to comment — `ReactToComment` | `src/components/Modals/commands/HTC/taskCommands.ts:213` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C236 `pinComment` | CTRL+SHIFT+P | `Unpin / Pin comment` — `PinComment` | `src/components/Modals/commands/HTC/taskCommands.ts:220` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C237 `copyCommentContent` | — (palette only) | Copy comment content — `CopyCommentContent` | `src/components/Modals/commands/HTC/taskCommands.ts:227` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C238 `copyCommentLink` | — (palette only) | Copy comment URL — `CopyCommentURL` | `src/components/Modals/commands/HTC/taskCommands.ts:233` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C239 `starComment` | CTRL+SHIFT+S | `Unstar / Star comment` — `StarComment` | `src/components/Modals/commands/HTC/taskCommands.ts:239` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C240 `createTaskFromComment` | — (palette only) | Create task from comment — `CreateTaskFromComment` | `src/components/Modals/commands/HTC/taskCommands.ts:246` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C241 `branchInNewChat` | — (palette only) | Branch in new chat — `BranchInNewChat` | `src/components/Modals/commands/HTC/taskCommands.ts:252` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C242 `copyCommentToAiChat` | — (palette only) | Copy comment to AI chat — `CopyCommentToAiChat` | `src/components/Modals/commands/HTC/taskCommands.ts:259` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C243 `deletemessage` | — (palette only) | Delete comment — `DeleteMessage` | `src/components/Modals/commands/HTC/taskCommands.ts:266` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |

### Global navigation

| ID / command key | Keys | Command / behavior | Web file:line | Native status / file:line |
|---|---|---|---|---|
| C244 `undoLatest` | CTRL+Z | Undo latest action — `UndoLatest` | `src/components/Modals/commands/HTC/navigationCommands.ts:10` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C245 `copyCurrentPageUrl` | — (palette only) | Copy current page URL — `CopyViewURL` | `src/components/Modals/commands/HTC/navigationCommands.ts:17` | **done** — `crates/ht-desktop/src/command_center.rs:19`, `crates/ht-desktop/src/interactions.rs:180` (supported native action; behavior pending) |
| C246 `GotoInbox` | G → I | Go to inbox — `GotoInbox` | `src/components/Modals/commands/HTC/navigationCommands.ts:23` | **done** — `crates/ht-desktop/src/command_center.rs:19`, `crates/ht-desktop/src/interactions.rs:180` (supported native action; behavior pending) |
| C247 `GotoSnippets` | G → ; | Go to snippets — `GotoSnippets` | `src/components/Modals/commands/HTC/navigationCommands.ts:30` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C248 `GotoReports` | — (palette only) | Go to reports — `GotoReports` | `src/components/Modals/commands/HTC/navigationCommands.ts:37` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C249 `GoToCalender` | G → C | Go to calendar — `GoToCalender` | `src/components/Modals/commands/HTC/navigationCommands.ts:43` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C250 `GoToAllTasks` | G → A | Go to all tasks — `GoToAllTasks` | `src/components/Modals/commands/HTC/navigationCommands.ts:50` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C251 `GoToDueDates` | G → U | Go to scheduled tasks — `GoToDueDates` | `src/components/Modals/commands/HTC/navigationCommands.ts:57` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C252 `GoToMyTasks` | G → M | Go to my tasks — `GoToMyTasks` | `src/components/Modals/commands/HTC/navigationCommands.ts:64` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C253 `GoToDrafts` | G → D | Go to drafts — `GoToDrafts` | `src/components/Modals/commands/HTC/navigationCommands.ts:72` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C254 `GoToStarred` | G → S | Go to starred — `GoToStarred` | `src/components/Modals/commands/HTC/navigationCommands.ts:79` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C255 `GoToPinned` | G → P | Go to pinned — `GoToPinned` | `src/components/Modals/commands/HTC/navigationCommands.ts:86` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C256 `GoToAgents` | — (palette only) | Go to agents — `GoToAgents` | `src/components/Modals/commands/HTC/navigationCommands.ts:93` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C257 `GoToAgentChat` | — (palette only) | Agent Chat — `GoToAgentChat` | `src/components/Modals/commands/HTC/navigationCommands.ts:100` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C258 `GotoReminders` | G → H | Go to reminders — `GotoReminders` | `src/components/Modals/commands/HTC/navigationCommands.ts:106` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C259 `GotoTaskArchived` | G → E | Go to archived tasks — `GotoTaskArchives` | `src/components/Modals/commands/HTC/navigationCommands.ts:113` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C260 `GotoInboxArchive` | G → R | Go to archived inbox — `GotoInboxArchives` | `src/components/Modals/commands/HTC/navigationCommands.ts:120` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C261 `archiveShowActiveBoardsOnly` | — (palette only) | Archive: show active boards only — `ArchiveShowActiveBoardsOnly` | `src/components/Modals/commands/HTC/navigationCommands.ts:127` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C262 `archiveShowAllBoards` | — (palette only) | Archive: show all boards — `ArchiveShowAllBoards` | `src/components/Modals/commands/HTC/navigationCommands.ts:133` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C263 `archiveShowArchivedBoardsOnly` | — (palette only) | Archive: show archived boards only — `ArchiveShowArchivedBoardsOnly` | `src/components/Modals/commands/HTC/navigationCommands.ts:139` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C264 `GoToProjectInbox` | — (palette only) | Go to project inbox — `GotoProjectInbox` | `src/components/Modals/commands/HTC/navigationCommands.ts:145` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C265 `GoToTrash` | G → # | Go to trash — `GotoTrash` | `src/components/Modals/commands/HTC/navigationCommands.ts:151` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C266 `toggleArchivedSearchResults` | G → X | `Hide / Show archived search results` — `ToggleArchivedSearchResults` | `src/components/Modals/commands/HTC/navigationCommands.ts:160` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C267 `searchTask` | / | Search — `SearchTask` | `src/components/Modals/commands/HTC/navigationCommands.ts:169` | **done** — `crates/ht-desktop/src/command_center.rs:19`, `crates/ht-desktop/src/interactions.rs:180` (supported native action; behavior pending) |
| C268 `ShowFilterHtc` | SHIFT+F | Filters — `ShowFilterHTC` | `src/components/Modals/commands/HTC/navigationCommands.ts:177` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |

### Global / task time

| ID / command key | Keys | Command / behavior | Web file:line | Native status / file:line |
|---|---|---|---|---|
| C269 `toggleTimeTracking` | W | Start or stop timer — `ToggleTimeTracking` | `src/components/Modals/commands/HTC/navigationCommands.ts:193` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C270 `logTimeOnTask` | B | Log time — `LogTimeOnTask` | `src/components/Modals/commands/HTC/navigationCommands.ts:200` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C271 `goToTimeThisTask` | — (palette only) | Time: this task — `GoToTimeThisTask` | `src/components/Modals/commands/HTC/navigationCommands.ts:213` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C272 `goToTimeThisBoard` | — (palette only) | Time: this board — `GoToTimeThisBoard` | `src/components/Modals/commands/HTC/navigationCommands.ts:224` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C273 `toggleBoardTimeTracking` | — (palette only) | Toggle time tracking for this board — `ToggleBoardTimeTracking` | `src/components/Modals/commands/HTC/navigationCommands.ts:230` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C274 `GoToTimers` | G → T | Time: running timers — `GoToTimers` | `src/components/Modals/commands/HTC/navigationCommands.ts:243` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C275 `goToTimeMyWeek` | — (palette only) | Time: my week — `GoToTimeMyWeek` | `src/components/Modals/commands/HTC/navigationCommands.ts:252` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |

### Board / bulk selection

| ID / command key | Keys | Command / behavior | Web file:line | Native status / file:line |
|---|---|---|---|---|
| C276 `bulkMoveToColumn` | M | commandOptions.kanbanSelectAllEnabled ? "Move to column" : "Move selected tasks to column" — `MoveToColumn` | `src/components/Modals/commands/HTC/navigationCommands.ts:273` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C277 `bulkAssignUser` | A | commandOptions.kanbanSelectAllEnabled ? "Assign" : "Assign selected tasks" — `OpenAssignModal` | `src/components/Modals/commands/HTC/navigationCommands.ts:280` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C278 `bulkSetLabels` | T | commandOptions.kanbanSelectAllEnabled ? "Set tags" : "Set tags on selected tasks" — `LabelModal` | `src/components/Modals/commands/HTC/navigationCommands.ts:287` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C279 `bulkArchive` | CTRL+E | commandOptions.kanbanSelectAllEnabled ? "Archive" : "Archive selected tasks" — `ArchiveTask` | `src/components/Modals/commands/HTC/navigationCommands.ts:294` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C280 `bulkSelectAllOnBoard` | CTRL+A | Select all on board — `SelectAllOnBoard` | `src/components/Modals/commands/HTC/navigationCommands.ts:302` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C281 `bulkSelectAllInColumn` | — (palette only) | Select all in this column — `SelectAllInColumn` | `src/components/Modals/commands/HTC/navigationCommands.ts:309` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C282 `bulkClearSelection` | ESC | Clear selection — `ClearTaskSelection` | `src/components/Modals/commands/HTC/navigationCommands.ts:315` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |

### AI sidebar / agent chat (conditional)

| ID / command key | Keys | Command / behavior | Web file:line | Native status / file:line |
|---|---|---|---|---|
| C283 `agentChatNextAgent` | CTRL+TAB | Next agent — `AgentChatNextAgent` | `src/components/Modals/commands/HTC/navigationCommands.ts:331` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C284 `agentChatPreviousAgent` | CTRL+SHIFT+TAB | Previous agent — `AgentChatPreviousAgent` | `src/components/Modals/commands/HTC/navigationCommands.ts:338` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C285 `agentChatSendMessage` | CTRL+ENTER | Send message — `AgentChatSendMessage` | `src/components/Modals/commands/HTC/navigationCommands.ts:345` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C286 `agentChatOpenLinks` | CTRL+O | Open all links in latest reply — `AgentChatOpenLinks` | `src/components/Modals/commands/HTC/navigationCommands.ts:352` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C287 `agentChatAddAgent` | — (palette only) | Add agent — `AgentChatAddAgent` | `src/components/Modals/commands/HTC/navigationCommands.ts:359` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C288 `agentChatNextTeam` | ALT+SHIFT+⭣ | Next team — `AgentChatNextTeam` | `src/components/Modals/commands/HTC/navigationCommands.ts:365` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C289 `agentChatPreviousTeam` | ALT+SHIFT+⭡ | Previous team — `AgentChatPreviousTeam` | `src/components/Modals/commands/HTC/navigationCommands.ts:372` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |

### Global / app shell (conditional)

| ID / command key | Keys | Command / behavior | Web file:line | Native status / file:line |
|---|---|---|---|---|
| C290 `appShellInbox` | 1 | App shell: Inbox — `GotoInbox` | `src/components/Modals/commands/HTC/navigationCommands.ts:386` | **done** — `crates/ht-desktop/src/command_center.rs:19`, `crates/ht-desktop/src/interactions.rs:180` (supported native action; behavior pending) |
| C291 `appShellBoard` | 2 | App shell: Board — `GoToBoardSurface` | `src/components/Modals/commands/HTC/navigationCommands.ts:393` | **done** — `crates/ht-desktop/src/command_center.rs:19`, `crates/ht-desktop/src/interactions.rs:180` (supported native action; behavior pending) |
| C292 `appShellTable` | 3 | App shell: Table — `GoToTableSurface` | `src/components/Modals/commands/HTC/navigationCommands.ts:400` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C293 `appShellCalendar` | 4 | App shell: Calendar — `GoToCalender` | `src/components/Modals/commands/HTC/navigationCommands.ts:407` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C294 `appShellAIChat` | 5 | App shell: Toggle AI chat — `AIChatInterface` | `src/components/Modals/commands/HTC/navigationCommands.ts:414` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |
| C295 `appShellAIChatAlternative` | ] | App shell: Toggle AI chat (alternative) — `AIChatInterface` | `src/components/Modals/commands/HTC/navigationCommands.ts:421` | **missing / hidden** — native capability unavailable; never rendered as an inert entry |

## Measured counts (native implementation, runtime pending)

- 249 direct-shortcut context/action rows: 67 done in source, 26 partial, 156 missing.
- 295 palette definitions: 22 supported entries, 273 definitions hidden. Cached-board navigation entries and persisted usage/frecency scoring are implemented; runtime receipt pending.
- Behavior suite: 105 isolated laptop cases; the original baseline produced 5 PASS / 4 FAIL at `d63d692` (`/tmp/htbehavior-baseline-complete/results.txt`). Its offscreen mouse injection was subsequently tightened; do not treat the baseline mouse PASS as proof of a visible target.
- Current acceptance evidence remains pending until the final SHA's CI and laptop round finish. No build, test suite, or app copy runs on the VPS.

## Explicit remaining capability gaps

Creation/subtasks/relationships need provisional-ID reconciliation and native creation controls. Undo needs durable inverse writes. Assignment supports humans, not agent IDs. Cached search is not server-wide search or the web's inline filters. Date presets are not a full calendar picker. Saved views, sorting/reordering, board management and favorite navigation have no native state/UI yet. Rich comments, quotes, history, collapse, attachments and clipboard rich formatting are absent. Star/follow, reminders, unread toggling, public sharing and AI/voice features have no implemented native write/UI path. Timer APIs exist but native timer state/controls are not implemented. Settings/help/tutorials and out-of-scope table/calendar/AI-sidebar commands remain hidden. This is not complete parity with all 249 inventory rows.

## Hidden palette command keys

`reloadApp`, `swipeThroughUnread`, `clearInboxToZero`, `archiveAllReadNotifications`, `archiveReactionNotifications`, `createTeam`, `teamSettings`, `ManageTeamMembers`, `billing`, `manageSubscriptions`, `manageTeamAIAPIKeys`, `toggleBoardZoom`, `createTask`, `createTaskWithAiWriter`, `newTaskFromTemplate`, `generateStatusUpdate`, `sendFeedback`, `sortBoard`, `toggleFilterValueMatch`, `toggleStaleness`, `toggleStalenessView`, `toggleAutoArchive`, `boardVelocityReport`, `sortByTimeInColumn`, `sortByLastComment`, `autoAssignColumn`, `manageColumns`, `renameColumn`, `hideColumn`, `addColumn`, `deleteColumn`, `inviteBoard`, `manageMembers`, `goToViews`, `manageViews`, `createSmartSplit`, `toggleBoardLayout`, `configureTableColumns`, `manageLabels`, `createCustomField`, `manageCustomFields`, `toggleEmptyColumns`, `toggleArchivedOnBoard`, `renameBoard`, `copyBoardLink`, `resetBoardLink`, `manageTeams`, `createBoard`, `boardCreationAssistant`, `archiveBoards`, `deleteBoard`, `toggleRailExpanded`, `toggleAppShellRail`, `aiChatInterface`, `pinAiChatOpen`, `fullScreenAiChat`, `branchInNewChat`, `toggleAIChatView`, `createAgent`, `manageAgents`, `disabledAgents`, `deleteAllChats`, `systemTheme`, `profilePicture`, `switchAccount`, `manageFavorites`, `calendarSettings`, `toggleCalendarWeekends`, `calendarWeekStartsMonday`, `calendarWeekStartsSunday`, `subscribeGoogleCalendar`, `useSnippet`, `createSnippet`, `createSnippetFromDraft`, `settingsAnnouncementsAnnouncementAlerts`, `settingsCalendarKeepTasksUpdated`, `settingsCalendarShowWeekends`, `settingsApiKeysOpenaiApiKey`, `settingsApiKeysAnthropicApiKey`, `settingsApiKeysGoogleApiKey`, `settingsApiKeysDeepseekApiKey`, `settingsApiKeysMoonshotKimiApiKey`, `settingsApiKeysZAiGlmApiKey`, `settingsApiKeysAlibabaQwenApiKey`, `settingsApiKeysOpenrouterApiKey`, `settingsApiKeysVercelAiGatewayApiKey`, `settingsApiKeysCustomEndpointOperatesUnderAnEuUsDataAgreement`, `settingsAiModelsGdprSafeMode`, `settingsAiModelsOpenai`, `settingsAiModelsAnthropic`, `settingsAiModelsGoogle`, `settingsAiModelsDeepseek`, `settingsAiModelsMoonshotKimi`, `settingsAiModelsZAiGlm`, `settingsAiModelsAlibabaQwen`, `settingsAiModelsOpenrouter`, `settingsAiFeaturesAiChat`, `settingsAiFeaturesTaskWriter`, `settingsAiFeaturesWriteWithAi`, `settingsAiFeaturesImproveWriting`, `settingsAiFeaturesAskAi`, `settingsAiFeaturesBoardGeneration`, `settingsAiFeaturesImageGeneration`, `settingsAiFeaturesHyperai`, `settingsAiFeaturesTaskSummaries`, `settingsAiFeaturesQuestionSuggestions`, `settingsAiFeaturesDictation`, `settingsBoardMemoryLearnFromAiCorrections`, `settingsBoardGeneralBoardNotifications`, `settingsBoardGeneralShowTotalTimeOnTasks`, `settingsBoardPlanningEnableCycles`, `settingsBoardStalenessStaleness`, `settingsBoardStalenessStaleReminders`, `settingsBoardStalenessAutoArchive`, `settingsNotificationsPushNotificationsOnThisDevice`, `settingsNotificationsMentionsNotifications`, `settingsNotificationsCommentsNotifications`, `settingsNotificationsAssignmentsNotifications`, `settingsNotificationsTaskUpdatesNotifications`, `settingsNotificationsDueDatesNotifications`, `settingsTaskPageOpenAiChatByDefault`, `settingsTaskPageCollapseComments`, `settingsTaskPagePlayGifs`, `settingsTaskPageSuggestDescriptionsFromTaskTitles`, `settingsTaskPageShowTaskHistory`, `settingsTaskPageReadReceipts`, `settingsInboxArchiveAndGoToNextTaskAfterSendingAComment`, `settingsInboxDisplayAvatarInInbox`, `settingsLearnQuickTips`, `settingsGeneral`, `settingsAppearance`, `settingsTaskPage`, `settingsNotifications`, `settingsAccounts`, `settingsBilling`, `settingsPlans`, `settingsAiUsage`, `settingsMemberUsage`, `settingsAiModels`, `settingsAiFeatures`, `settingsDefaultModels`, `settingsApiKeys`, `settingsMembers`, `settingsTeamAgents`, `settingsSkills`, `settingsBoardGeneral`, `settingsBoardTicketPrefix`, `settingsBoardMembers`, `settingsBoardAgents`, `settingsBoardAi`, `settingsBoardSkills`, `settingsBoardFiles`, `settingsSlack`, `settingsMcp`, `settingsCli`, `settingsApi`, `settingsShortcuts`, `settingsLearn`, `shortcuts`, `settings`, `boardSettings`, `helpCenter`, `onboardingTour`, `quickTips`, `latestUpdates`, `goToWelcome`, `startKanbanTutorial`, `startTaskWriterTutorial`, `generateMcpToken`, `cliInstall`, `restApi`, `logout`, `roundedTheme`, `setStartDate`, `setRecurrence`, `saveTaskTemplate`, `blockedByPerson`, `assignToMe`, `addSubTask`, `viewSubTask`, `addRelatedTask`, `markBlockedBy`, `markAsBlocking`, `markDuplicateOf`, `movetasktodifferentboard`, `followtask`, `unfollowtask`, `openAiTaskWriter`, `summarizeTicket`, `remindMe`, `markAsUnread`, `acceptTask`, `declineTask`, `declineAsDuplicateOf`, `duplicateTask`, `duplicateTaskToBoard`, `copypublicformattedTaskLink`, `copyPublicLink`, `setReminder`, `starTask`, `shareTaskPublicly`, `copyBranchName`, `toggleHistory`, `taskDescriptionVersions`, `toggleAllComments`, `removeTaskNotification`, `removeSubtask`, `removeParenttask`, `showInInbox`, `subtaskSettings`, `suggestReply`, `speechToText`, `editcomment`, `summarizeComment`, `fastLikeComment`, `replyToComment`, `reactToComment`, `pinComment`, `copyCommentContent`, `copyCommentLink`, `starComment`, `createTaskFromComment`, `copyCommentToAiChat`, `deletemessage`, `undoLatest`, `GotoSnippets`, `GotoReports`, `GoToCalender`, `GoToAllTasks`, `GoToDueDates`, `GoToMyTasks`, `GoToDrafts`, `GoToStarred`, `GoToPinned`, `GoToAgents`, `GoToAgentChat`, `GotoReminders`, `GotoTaskArchived`, `GotoInboxArchive`, `archiveShowActiveBoardsOnly`, `archiveShowAllBoards`, `archiveShowArchivedBoardsOnly`, `GoToProjectInbox`, `GoToTrash`, `toggleArchivedSearchResults`, `ShowFilterHtc`, `toggleTimeTracking`, `logTimeOnTask`, `goToTimeThisTask`, `goToTimeThisBoard`, `toggleBoardTimeTracking`, `GoToTimers`, `goToTimeMyWeek`, `bulkMoveToColumn`, `bulkAssignUser`, `bulkSetLabels`, `bulkArchive`, `bulkSelectAllOnBoard`, `bulkSelectAllInColumn`, `bulkClearSelection`, `agentChatNextAgent`, `agentChatPreviousAgent`, `agentChatSendMessage`, `agentChatOpenLinks`, `agentChatAddAgent`, `agentChatNextTeam`, `agentChatPreviousTeam`, `appShellTable`, `appShellCalendar`, `appShellAIChat`, `appShellAIChatAlternative`

## Verification receipt

All four acceptance gates are met (0 unmet, 0 abandoned). Commands were read-only source/document inspection, except writes to the two requested Markdown files. The ledger is manual because correctness of source interpretation is not decided by row/citation counters; counters prove coverage/existence, not semantics. Gate linter output: `LINT OK (5 warning(s))`, all warnings explicitly concern manual gates/mostly-manual ledger. No check runner, build or application test suite was executed.

Measured inventory: **249 shortcut context/action rows / 224 missing**, plus **295 palette definitions / 295 missing** separately. Source registry coverage 142/142 and palette declaration coverage 295/295; 24/24 model-option IDs. All 1,334 source file:line citations checked for an existing file and valid line range. Source semantics, auth branches, persistence bootstrap and UI caveats were reviewed in the reads recorded above. These are static findings, not browser/live API proof.

`internal/` is ignored in this checkout; the documents exist on disk but do not appear in ordinary `git status`. Nothing was staged, committed, pushed, built or tested.
