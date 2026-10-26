# Visual parity round 4

## Acceptance checks, before implementation

1. Baseline is main 2ceb32559117 and its public CI windows-visual3 capture, with unchanged gates/fixtures/visual3. Retain all original ledger identities. Starting inventories are Inbox 14, Board 14, Task 15, total 43. Require at least 20 fewer rows from product changes alone.
2. Compare exact-head hosted Windows screenshots and native layout dumps with the read-only frozen /home/valentin/projects/hypertask-desktop-wt-public/internal/live-graphite-1920 references. Preserve gates/layout-compare.py and every threshold. No fixture, reference, hiding, or content shortcuts.
3. Every correction cites component and token file:line from /home/valentin/projects/hypertask/src. Each correction adds or updates a native layout regression pinning the source value. Unfixable rows retain identity and exact source-backed reason.
4. AI sidebar crop x1500..1919 must have zero changed pixels on Inbox, Board, Task versus main capture. No AI sidebar, keyboard shortcut, or command center code changes.
5. Verify only on public GitHub-hosted runners. No VPS builds, tests, lint, app launches, test copies, or laptop access. One-off comparisons remain separate from permanent CI and post their results on the draft PR.
6. Fetch and rebase origin/main before each push. Every required job and artifact must verify against the final exact HEAD through the supplied CI receipt script. Report job conclusions, test names, capture deltas, SHA, PR and CI URLs.

## Execution units

1. Establish immutable main artifact identity and enumerate all remaining row identities.
2. Fix largest source-backed layout group and native regression; commit and obtain hosted proof.
3. Continue source-backed groups with hosted before/after checks until reduction reaches 20, or document exact blockers without dropping rows.
4. Exact-head full CI receipt, unchanged-sidebar image proof, protected-path diff, and PR evidence.

Principle Prove It Works chooses actual hosted captures and executable native regressions. Principle Sequence Verifiable Units chooses component commits and exact-head hosted verification. Self-review only; unavailable Cursor-only tools use repository-native evidence instead.

Owned files are this document, inbox/board/task portions of crates/ht-desktop/src/ui.rs and their native layout tests, plus narrowly required source-backed main-content styling. AI/sidebar, shortcuts, command center, fixtures, references, thresholds and other helpers' files are excluded.

## Baseline inventory limit

Main CI https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40369851241 succeeded at 2ceb3255911725f8e780ede25c2ec4f415b0a269. Downloaded windows-visual3 memory.json records that Revision and BuildRevision, 1920x1009 clients, 96 DPI, and executable B6CD3F8A6FD86E6E89C26514A3BD9E34316C36623ECAD1895B389581161D81BF.

Only one measured editable row remains, Task footer.text. Twenty-four measured presence rows are rail.icon.5 through rail.icon.12 on each screen. The frozen collector sampled only five rail icons; actual rail controls exist in the web and native screenshots. Deleting them or suppressing their capture keys would violate the no-hiding and unchanged-reference requirements. Eighteen screenshot-only rows comprise six classes per screen, all preserved. Four classes belong to the protected AI sidebar, one is account state, and one is the original footer rasterization class. A 20-row product reduction is therefore impossible within this scope. Do not report these rows as closed by filtering or renaming them.

### Footer source correction

Data shape stays the existing ordered hint tuples. The layout now models the source's actual sequence of hint group, in-flow separator, hint group. Absolute decorative dots plus a guessed fixed total gap do not model the source flex items' sizing and centering.

| Source value | Component authority | Token authority | Native regression |
| --- | --- | --- | --- |
| Outer 8px padding; flexible clipped inner row; separate 18px close reserve | src/components/Global/BottomSettings_QuickTips.tsx:371,375,425 | src/styles/tailwind-entry.css:10-11 | footer_web_in_flow_separators_and_close_reserve |
| Inner gap 2px; group gap 8px; in-flow 4px separator with 4px side margins and #8e9093 | src/components/Global/BottomSettings_QuickTips.tsx:375,391,418 | src/styles/tailwind-entry.css:10-11 | same test, each separator on all three screens |
| Key gap 4px; padding 6.25px; 20px height; 12px font, 14px line, weight 600, radius 2px | src/components/Global/BottomSettings_QuickTips.tsx:402,407; src/components/Common/kbd.tsx:13 | src/styles/tailwind-entry.css:10-11; tailwind.config.ts:171 | same test |
| Label 14px, 21px line, normal weight | src/components/Global/BottomSettings_QuickTips.tsx:413 | src/styles/tailwindThemes/graphite.css:169-172; tailwind.config.ts:174; src/styles/tailwind-entry.css:10-11 | same test |

Web paths are relative to /home/valentin/projects/hypertask. No shortcut mapping, keyboard handler, or command center implementation changes.

### Exact retained row identities and blockers

Each row below applies independently to Inbox, Board and Task. No row is removed from the denominator.

| Identity | Rows | Exact source-backed reason retained |
| --- | ---: | --- |
| rail.icon.5 | 3 | Calendar is real UI, AppShellRail.tsx:268-279. Frozen JSON did not sample it; deleting UI or capture keys is forbidden. |
| rail.icon.6 | 3 | AI Chat is real UI, AppShellRail.tsx:281-290. Same collector omission, not a layout defect. |
| rail.icon.7 | 3 | Search is real UI, AppShellRail.tsx:292-314. Same collector omission. |
| rail.icon.8 | 3 | Agents is real UI, AppShellRail.tsx:349-362. Same collector omission. |
| rail.icon.9 | 3 | Invite is real UI, AppShellRail.tsx:363-373. Same collector omission. |
| rail.icon.10 | 3 | News is real UI, AppShellRail.tsx:374-389. Same collector omission. |
| rail.icon.11 | 3 | Feedback is real UI, AppShellRail.tsx:390-400. Same collector omission. |
| rail.icon.12 | 3 | Settings is real UI, AppShellRail.tsx:401-406. Same collector omission. |
| AI composer top/height | 3 | Protected AI sidebar. Source flows context, input and actual controls, AI_Tiptap_Container.tsx:298-305,323-324; AiChatComposerActionRow.tsx:146-152. Fixed-height padding would conceal missing actions. |
| AI attachment | 3 | Protected AI sidebar action, AiChatComposerActionRow.tsx:148. Decorative icons without an action are forbidden. |
| AI recording | 3 | Protected AI sidebar action, AiChatComposerActionRow.tsx:149. Same scope and missing-action blocker. |
| AI context-add | 3 | Protected AI sidebar action, AiChatComposerActionRow.tsx:150. Same scope and missing-action blocker. |
| AI model/scope/prompts | 3 | Protected synthetic account state. WelcomeScreen.tsx:166-176 derives task/demo/random suggestions; AI_Tiptap_Container.tsx:323-324 renders actual contextList. Fixture changes are forbidden. |
| Footer glyph rasterization | 3 | Source family is IBM Plex Sans, graphite.css:169-172; 14px normal label, BottomSettings_QuickTips.tsx:413. Glyph coverage differs between native DirectWrite and frozen Chromium. Geometry convergence alone does not prove pixel identity. No replacement font or text-specific compensation. |

AppShellRail.tsx is src/components/PageComponents/Kanban/HeaderComponents/AppShellRail.tsx. AI component names are under src/components/AI_CHAT. graphite.css is src/styles/tailwindThemes/graphite.css. All source paths remain under /home/valentin/projects/hypertask/src.

The remaining 43rd identity is Task footer.text's measured x coordinate. The native regression pins genuine source structure rather than hard-coding the frozen label's position. Source-equivalent total spacing need not close that row; the hosted capture decides. Even closing it leaves 42 original rows. The 20-fewer-row acceptance target stays unmet, not waived.

## One-off hosted comparison

The separate gates/visual-artifact-proof.py takes before/after run IDs at invocation. It downloads exact-run windows-visual3 artifacts, validates capture SHA, compares with the unchanged oracle, reports all original counts, checks every sidebar pixel, and rejects changes to protected code, fixtures or thresholds. It never pins an expiring artifact in permanent CI. The separate Visual artifact comparison workflow runs manually or after a PR description edit, selecting already-completed base/head CI runs without polling; .github/workflows/ci.yml is unchanged. Its receipt distinguishes valid evidence from the unmet product target.
