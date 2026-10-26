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

### Rejected footer reconstruction

The first hosted unit replaced absolute separator dots with source-equivalent in-flow spacing. CI 40379885550 passed the new regression on Ubuntu, Windows and macOS, but actual footer.text coordinates stayed Inbox 380, Board 272, Task 323. No ledger row closed. That reconstruction and its test are removed. No label-specific offset or guessed font compensation replaces them.

### Task surface corrections

Data shape stays the existing task selection index and theme. Graphite description selection paints only the source's 4px left edge, not a bright outline around every side. Graphite description and properties corners use 4px, not 5px. Properties use the source's #363b44 border instead of the generic #33373e. These genuine screenshot corrections were not original ledger identities, so they earn zero original-row reductions.

| Source value | Component authority | Token authority | Native regression |
| --- | --- | --- | --- |
| Description 4px left edge only; selected #707788, otherwise transparent; 4px corners; top 20px, horizontal 16px | src/components/PageComponents/TaskDetail/CommentAndDescription/DescriptionContainer/index.tsx:139-151 | src/styles/tailwindThemes/graphite.css:90-93,153-155; src/lib/configs/taskDetail.config.ts:6; src/styles/tailwind-entry.css:10-11 | task_graphite_description_selection_and_property_surfaces_follow_web |
| Description footer effective bottom reserve 4 + 8 + 12px, preserving content positions | DescriptionContainer/index.tsx:139; DescriptionSubTasks/DescriptionSubTasks.tsx:52,103 | src/lib/configs/taskDetail.config.ts:248; src/styles/tailwind-entry.css:10-11 | same test and existing task_web_header_title_description_meta_and_composer_flow |
| Properties 4px corners, 312px width, #363b44 border, #262a30 background | src/components/PageComponents/TaskDetail/MainPageComponents/TaskInfoColumnContainer.tsx:6-10 | src/styles/tailwindThemes/graphite.css:20,153-155 | same test |

The regression checks actual painted border widths on all four sides, selected and unselected colors, radii, surface widths, and unchanged description author placement. The 4px draft marker remains aligned with the outer border edge; the new regression pins its position and #c2cfa5 color from DescriptionContainer/index.tsx:141,151. Existing long-title, long-description, comments, draft-marker and composer-flow tests remain unchanged. Other themes retain their existing styles. No shortcut mapping, keyboard handler, command center implementation, or AI sidebar changes.

Web paths are relative to /home/valentin/projects/hypertask. Description component short paths are under src/components/PageComponents/TaskDetail/CommentAndDescription/DescriptionContainer.

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

The remaining 43rd identity is Task footer.text's measured x coordinate. The rejected source-equivalent footer reconstruction left that row unchanged. Its 1.625px x delta exceeds the immutable 1.5px threshold. Source sizing is BottomSettings_QuickTips.tsx:375,391,402,407,413,418; no source value authorizes a label-specific compensating offset. Even closing it leaves 42 original rows. The 20-fewer-row acceptance target stays unmet, not waived.

## One-off hosted comparison

The separate gates/visual-artifact-proof.py takes before/after run IDs at invocation. It downloads exact-run windows-visual3 artifacts, validates capture SHA, compares with the unchanged oracle, reports all original counts, checks every sidebar pixel, and rejects changes to protected code, fixtures or thresholds. It never pins an expiring artifact in permanent CI. The separate Visual artifact comparison workflow runs manually or after a PR description edit, selecting already-completed base/head CI runs without polling; .github/workflows/ci.yml is unchanged. Its receipt distinguishes valid evidence from the unmet product target.
