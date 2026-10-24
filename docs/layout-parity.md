# HTPR-7381 B: Graphite layout evidence

F011 is unchanged: POST `createGlobally` remains server-blocked under HTPR-7699.
No build, test, app, test copy, or owner-laptop command is run on the VPS.

## Acceptance and measurement

Acceptance was recorded before implementation in `internal/LOOK-B-GATES.md`.
CI runs the existing dense 1440x900/all-theme coverage plus Graphite 1920x1009
inbox, board, and task dumps. `layout-dumps` also contains `parity/`: a synthetic
three-column/two-card, one-notification, empty-description/no-comment fixture.
This preserves the dense fixture's badge, label, metadata and comment coverage,
while allowing comparison without mistaking different content cardinality for a
layout defect. Text content is never compared or copied from the live account.

Unmodified comparer:
`/home/valentin/projects/hypertask-desktop-wt-public/internal/gates/layout-compare.py`.
Live input: that worktree's `internal/live-graphite-1920` (QA board 7283/task 60253,
1920x1009, scale 1). All geometry, font and color fields remain compared.

First CI baseline (dense fixture): `91cfe30ccf63200ef4c65d99cbb802793f8406c5`,
[40086213038](https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40086213038),
all five applicable jobs successful. Downloaded `layout-dumps` into
`evidence/look-before`. Raw comparer output: inbox **39**, board **41**, task
**36**, total **116**. The supplied older `mismatch-baseline.txt` is not used as
the measured before count (it reports 184).

## Web source of changed values

Source repository `hypertask`, revision
`6f9901764d355aba7bfc76e4e62ff255e0b68b31` (source read only).

| Change | Web source |
| --- | --- |
| Collapsed rail controls inherit 16px/24px; chevron button is 16x20 and muted | `src/components/PageComponents/Kanban/HeaderComponents/AppShellRail.tsx:152-164,172-191` (no local 14px override on the buttons); live rail font measurement confirms inherited base |
| Rail inbox begins at y=82: 8px top + 20px chevron + 4px margin + 34px boards + 4px wrapper gap + 12px section margin | `src/components/PageComponents/Kanban/HeaderComponents/AppShellRail.tsx:152-159,172,191` |
| Inbox remains a constrained, centered 1440px surface; header inset 12+52px left, 12+20px right, 9px gap | `src/styles/globals.scss:52-62,72-94`; `src/app/inbox/Inbox.tsx:978-988`. Fix auto-width flex overflow, not the measured width by hardcoding a screen-specific value |
| Board plus inherits the base 16px/24px rather than column-title 13px/19.5px | `src/components/PageComponents/Kanban/KanbanSectionComponents/section.tsx:600-607` limits dense font to the title/count, not the plus |
| Board title content wraps inside the card rather than overflowing its width | `src/components/PageComponents/Kanban/KanbanTaskComponents/TaskDraggableContainer.tsx:94-96`; `src/components/PageComponents/Kanban/KanbanTaskComponents/KanbanTaskCard.tsx:181` |
| Quick-tip label capitalization; 8px key/label gap; 2px flex gap + 4px separator + two 4px margins (16px inter-tip spacing) | `src/components/Global/BottomSettings_QuickTips.tsx:375,390-418` |
| `footer.text` measures the first hint span, not the whole hints row | `src/components/Global/BottomSettings_QuickTips.tsx:413-415`; live dump selector `#gear ... > span:nth-child(2)` |

No extra rail controls are removed to game the comparer: the live collector only
samples icons 0-4, although the web rail also contains calendar, AI, search,
agents, invitation, announcements, feedback and settings. Their native-only dump
keys are evidence coverage differences, not invented UI.

## CI evidence

Further run IDs, raw before/after comparisons, and any content-dependent
residuals are recorded below after downloading the actual CI artifacts.

Structural baseline: `9ee408af16afb54afae207951375a3d47c0045fd`,
[40087161031](https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40087161031),
all five applicable jobs successful. `layout-dumps` artifact ID `12557423496`,
digest `sha256:aef25f4e2ee40cbf49ac6ee03952805c4c13e0e7abc5175cc678df4a6881843c`.
Downloaded into `evidence/look-structural-before`; unmodified comparer on its
`parity/` subdirectory reports inbox **32**, board **27**, task **24**, total
**83**. This is the same fixture used for subsequent structural comparisons.

First fix artifact (`8cec052b0cfca40dcc97dcf6490c20842f49aec3`, run
[40088049145](https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40088049145)):
dense **74**, structural **42** differences. All Rust/platform jobs passed;
the layout assertion incorrectly applied Graphite's label-width range to Dia's
Inter font (115px). The range is now theme-scoped, retaining the 100-110px
Graphite assertion and checking that other themes still measure the label, not
the entire row. No application theme values were changed to satisfy that check.

Board footer declaration includes the web's `N / Shift+C` quick-add hint between
add-task and archive (`src/components/Global/BottomSettings_QuickTips.tsx:125-127,241`).
This is a cosmetic hint-list change only, with a CI declaration-order regression
test; it does not implement or change shortcuts or the blocked F011 save route.

Second fix artifact: `50c87bbd8d2f32a91994cf1348f05f217288b923`,
[40089051349](https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40089051349),
all five applicable jobs successful. Artifact `12558386340`, digest
`sha256:a5cab35705275673fb9d0ab8636fa7ab01fb2dd52c4aa1efd4cda682258677b1`.
Dense **116 → 74** (25/26/23); matched structure **83 → 42** (19/12/11).
The unchanged comparer reports **29 → 0 style-field differences**. Raw counts
are not reduced by filtering its output or changing its tolerance.

The 42 remaining raw differences consist of:

- 24 native-only rail keys, because the live collector samples only icons 0-4.
- Nine fixture-dependent boxes: inbox author/title/kind/ticket, board title/ticket
  and second card height, task ticket and composer (the synthetic detail task is
  in the inbox, so its existing snooze/remove-notification row is present).
- Nine small same-label text-metric differences: six tab boxes (including the
  active alias) and three first-footer-hint boxes. These remain visible, not
  silently waived. `gates/layout-fonts.mjs` measures the pinned web font and
  source CSS in CI Chromium against the actual native dumps to investigate them.
  The probe is not a replacement for the live comparison or a whole-app web QA.

CI logs prove `footer::tests::board_tips_include_the_web_quick_add_hint_in_declaration_order`
and `tests::cli_validates_all_contract_flags` pass on Windows, macOS and Linux;
`LAYOUT_DUMP_OK` proves all four themes, both resolutions, held-window behavior,
exit-after-ready and failed-write handling passed.

The CI Chromium probe at `bb76ad36178a9c59c7671600a441fa27ab945834`,
[40090306655](https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40090306655),
reproduced the frozen widths exactly: footer **106**, Important **96**, Agents
**72**, versus native **109/93/70**. The stricter gate failed honestly. Thus these
were not waived as inevitable platform noise: the embedded font faces need
correction. The web asset is pinned by its SHA-256, sourced from
`src/lib/fonts/ibmPlexSans.ts:3` and `src/styles/tailwindThemes/graphite.css:171-172`.

`gates/layout-font-assets.py` generates static weight 400/500/600/700 faces only
in CI. It checks the full Google-family source against every glyph metric in the
web's Latin subset, and checks that no character from any existing embedded
face is lost. Only the resulting full-coverage font artifacts are candidates
for desktop embedding; the narrow web subset is not substituted wholesale.

Font generation proof: run
[40091185028](https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40091185028),
`web-source-fonts/source.json`: all four weights retain **891** mapped characters,
cover all **232** web-subset characters with **zero** advance/bearing differences,
and lose **zero** existing characters. The four full-coverage `.ttf` files were
downloaded from that CI artifact and embedded without local conversion or build.
The previous filenames were mislabeled (400/500/600/700 contained OS/2 weights
500/400/700/600); a Rust CI regression now checks their declared weights.

## Font investigation disposition (not pixel-perfect completion)

The generated full faces and the exact Latin web faces have identical declared
advances/bearings, but Chromium measures them differently: footer 108 vs 106,
Important 92 vs 96, Agents 70 vs 72. Run
[40092934298](https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40092934298)
records those values alongside native 109/93/70. Disabling browser kerning did
not change the web-subset widths. The one-line native metrics-hinting experiment
at run [40093732560](https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40093732560)
also left all three native boxes unchanged. Neither experiment closed a live
difference, so the regenerated desktop fonts, metadata-only test, and renderer
vendoring were removed rather than shipping unrelated or ineffective changes.

The font oracle now compares **equivalent full-coverage faces** in Chromium and
native, with the original 1.5px tolerance. It retains the actual web-subset and
frozen widths in the artifact, and does **not** assert that those nine live
boxes match. This corrects a cross-font-variant oracle, not the live comparison:
its raw 42 differences and nine unresolved text-metric boxes remain explicit.
No screenshot coordinate, font size, padding, comparer tolerance, or dump value
was changed to hide them. Exact closure needs a separately scoped native font
metrics/rasterizer investigation, beyond the requested small UI edits.

## Verified partial-result receipt

UI SHA `f9ee7e4fbe2b8acc4d546f1486fd56e0c5879902` passed all five applicable jobs
in [40094950702](https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40094950702).
Artifact `12560028723` digest
`sha256:fa0fd7e208c23aa14a185c0f25f027eaf0bbb9151bdf7a66db7b2811e21fc5e6`
was downloaded and compared again: dense **116 → 74**, structural **83 → 42**,
with **29 → 0** style-field mismatches. Nine same-label metric boxes are still
unresolved, so acceptance B3 is abandoned/unmet, not passed. B1/B2/B4 are met;
three met, one handoff required. PR 16 remains draft. The final documentation-only
descendant's exact SHA, CI and freshly downloaded comparison are recorded in the
[handoff receipt](https://github.com/hypertask-ai/hypertask-desktop/pull/16#issuecomment-6384475006).

F011 remains server-blocked under HTPR-7699 and untouched. No app/build/test/test
copy was run on this VPS and no owner-laptop command was used. The recommended
next decision is deeper native font-metric work rather than source-unsupported
UI compensation; this receipt must not be presented as complete part-B parity.
