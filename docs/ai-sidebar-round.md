# AI sidebar parity round

## Frozen acceptance and baseline

Acceptance preceded implementation in GATES-aisidebar.md, commit a5c7971.
Main 2ceb3255911725f8e780ede25c2ec4f415b0a269.
CI https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40369851241.
Artifact windows-visual3 uses unchanged gates/fixtures/visual3.
Frozen web images and JSON remain read-only under
/home/valentin/projects/hypertask-desktop-wt-public/internal/live-graphite-1920.
All images are 1920x1009. Sidebar crop is [1500,0,1920,1009].
PR https://github.com/hypertask-ai/hypertask-desktop/pull/50 fixed the gutter,
not the remaining rows below. Round 2 and 3 ledgers remain unchanged.

One inventory row means one screen and one element/property class.
Repeated prompt defects count once per screen, not four times.
Content/state and font raster differences remain counted.
Geometry tolerance remains 1.5px. Style tolerance remains 0.5.
One-pixel horizontal residues remain disclosed, not compensated.

## Before inventory

Each C row applies independently to Inbox, Board, Task.
Bounds are [x,y,width,height]. Font weights are numeric.
Colors are RGB unless a source token is shown.

| Row | Property | Frozen web | Main Windows | Decision |
| --- | --- | --- | --- | --- |
| C01 | Starter stack vertical position | Inbox/Board y394,436,478,520; Task summary y377 and prompts y419,459,499,539 | Inbox/Board y396,438,480,522; Task summary y374 and prompts y418,460,502,544 | Open. Flow follows composer and task line boxes, not absolute offsets. |
| C02 | Starter corner radius | 4px | 6px | Open. Existing source projection also incorrectly uses 6px. No projection or fixture changed. |
| C03 | Composer y/height | [1513,908,399,93] | [1514,913,398,88] | Open. Preserve real input/action flow. |
| C04 | Placeholder color | #adb5bd, dominant opaque web pixels (173,181,189) | #8e9093, source native input tint (142,144,147) | Correct source tint. |
| C05 | Model label weight | 500, 13px/19.5px | 400, 13px/19.5px | Correct to 500. |
| C06 | Model label-chevron spacing | 4px margin | 5px gap | Correct to 4px. |
| C07 | Scope label weight | 500, 13px/19.5px | 400, 13px/19.5px | Correct to 500. |
| C08 | Scope icon-label spacing | 4px margin | 5px gap | Correct to 4px. |
| C09 | Scope icon box | 13x13 | 16x16 | Correct to 13x13. |
| C10 | Scope icon path/state | SquareKanban, 1.75 stroke; frozen board scope | Columns2, 1.5 stroke; native selected task context | Correct glyph for actual native state. Frozen state remains different, so retain row. |
| C11 | Scope trailing control | 16px ChevronDown | 16px X while context included | Open action mismatch. Do not paint a dropdown without implementing it. |
| C12 | Attachment action | 18px paperclip around x1797 | Missing | Open action gap. No decorative button. |
| C13 | Recording action group | 18px microphone around x1825 plus dropdown | Missing | Open action gap. |
| C14 | Add-context action | @ around x1867 | Missing | Open action gap. |
| C15 | Selected model content | Haiku 5.5 | Gemini Flash Lite | Retain account/state difference. Do not alter capture data or model selection. |
| C16 | Scope label content | Inbox Current board; Board/Task QA Team Board | QABO-5 plus truncated task title | Retain selected-scope difference. Do not relabel a task context as a board context. |
| C17 | Starter text/order | Four web-selected suggestions per screen | Four different native-seeded suggestions | Retain data/state difference. |
| C18 | Font raster and equal-label painted bounds | Task New AI Chat bright glyph bounds [1518,30,79,11] | [1519,31,79,10], colored DirectWrite edge pixels | Open. No string-specific offsets or global typography edits. |
| C19 | Send icon stroke | 16x16, 1.75 stroke | 16x16, 1.5 stroke | Open. Shared asset unchanged. |
| C20 | Scope control sizing/placement | Natural-width leading scope; separate right action group | Scope flexes across remaining row to the send control | Open with missing actions and selected-scope differences. |

| Task-only row | Property | Frozen web | Main Windows | Decision |
| --- | --- | --- | --- | --- |
| T01 | Summary fill | #c668ff at 10%; opaque center pixel (43,36,55) | Transparent, center (26,29,33) | Correct source tint. |
| T02 | Summary border | #c668ff at 70%; observed edge (152,83,195) | #707788 = (112,119,136) | Correct source tint. |
| T03 | Summary leading icon | 14x14 purple Sparkles | Missing | Restore existing action's icon. |
| T04 | Summary label inset | 9px border/padding + 14px icon + 8px gap = 31px | 9px, no icon or gap | Restore source layout. |
| T05 | Summary line height | 12px text / 16px line | 12px text / 18px line | Correct source text-xs. |
| T06 | Summary height | 34px, including 1px borders and 8px padding | 36px | Natural source line-box correction. |
| T07 | Task prompt line/height | 12px/16px, 32px tall | 12px/18px, 34px tall | Correct source text-xs. Inbox/Board remain text-meta 12px/18px. |

Inbox-only I01. Header title is QA Team Board on web,
New AI Chat natively. Board/Task header labels already match.
This remains selected-scope/session state, not a new fixed title.

Before counts are Inbox 21, Board 20, Task 27, total 68.
C04..C09 are eighteen candidate closures across three screens.
T01..T07 are seven candidate closures. No closure claimed before CI.

### Tolerated residues and unchanged matches

- Welcome x1545/w335 web, x1546/w334 native. Delta 1px.
- Composer x1513/w399 web, x1514/w398 native. Delta 1px.
- At y300, web x1502/x1503 both (43,46,50).
  Native x1502=(60,63,66), x1503=(26,29,33).
  Resize geometry already matches 4px gutter and 5x24 grip.
  Track/grip antialiasing belongs to retained C18 raster residuals.
- Header h48, y0, 14px/21px text, 16px action icons.
  Header action slots, welcome 40px inset, 8px prompt gaps,
  #363b44 prompt fill and #262a30 composer fill already match.
- All three screens retain 8px composer insets and radius8.
  No extra sidebar element exists beyond the incorrect X control.

## Source authority

All component paths below are relative to /home/valentin/projects/hypertask.
No dimensions come from the native implementation or visual guesses.

| Change | Web source |
| --- | --- |
| C04 placeholder #adb5bd; actual message 14px, weight400, line21 | src/styles/tiptap.module.scss:188-205; src/hooks/MultiPages/AIChat/useAiTiptap.ts:76-81 |
| C05 model weight500, C06 margin4; 13px dense, line-normal, padding8/4; chevron16 | src/components/Global/ModelSelectorDropdown.tsx:493,511,522-527 |
| C07 scope weight500, C08 margin4, C09 13px icon and state-selected path | src/components/AI_CHAT/AI_Tiptap_Container.tsx:608-637; src/components/AI_CHAT/ChatScopeChip.tsx:16,24-49 |
| Task fill10%, border70%, 8px padding/gap, text-xs and icon14 | src/components/AI_CHAT/WelcomeScreen.tsx:314-338; purple literal also src/components/Common/IconsGlobal.tsx:20 |
| Inbox/Board text-meta remains distinct from task text-xs | src/components/AI_CHAT/WelcomeScreen.tsx:335,398-407 |
| Composer dynamic input and max380 remain unchanged | src/components/AI_CHAT/AI_Tiptap_Container.tsx:298-305,326-375; src/styles/tiptap.module.scss:45-53 |
| Missing actions remain missing, not invented | src/components/AI_CHAT/AiChatComposerActionRow.tsx:146-152 |

Data shape remains the existing Scope, Chat and Theme.
Only their presentation changes. No new chat/session state.
New SVGs are sidebar-specific assets, registered in main.rs.
The shared main-content icon assets remain unchanged.
Transport, keyboard and command-center behavior remain unchanged.

Native regression names:
sidebar_web_control_labels_icons_and_spacing.
sidebar_task_summary_web_tint_icon_and_compact_prompt_geometry.
The latter inspects painted GPUI quads, including alpha and borders,
and clicks the real summary action to retain offline refusal behavior.

## Exact-head verification

Pending hosted CI and final capture measurement.

### First hosted correction

Run https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40380658015
compiled on all platforms. Both new layout tests failed before drawing.
Exact error was `missing field status` in the new in-memory task seed.
The test seeds now include the required Normal status.
Existing capture fixtures remain unchanged.
Source review also replaced remembered glyph paths with the actual
Lucide symbols imported by the cited web components.
Native asset tests now pin those paths and the 1.75 stroke.
Summary alpha now uses exact source 0.7 and 0.1,
not byte-quantized approximations.
