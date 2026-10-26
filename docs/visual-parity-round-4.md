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
| Label 14px, 21px line, normal weight | src/components/Global/BottomSettings_QuickTips.tsx:413 | src/styles/tailwindThemes/graphite.css:169-172; tailwind.config.ts:173; src/styles/tailwind-entry.css:10-11 | same test |

Web paths are relative to /home/valentin/projects/hypertask. No shortcut mapping, keyboard handler, or command center implementation changes.
