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
layout defect. Text content is never compared. Task, comment and user content is synthetic;
generic tab labels mirror the measurement so their width differences are visible.

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

## Historical font handoff (superseded by the authorized continuation below)

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

### Historical blocking oracle correction

Agreement with an equivalent full font is diagnostic evidence only, not a
substitute for the actual frozen web-subset contract. The final CI gate retains
**both** assertions with the original 1.5px tolerance: equivalent-full-font and
actual-web-font widths must match native. At that handoff the nine same-label differences failed the latter. The earlier
equivalent-only green receipt was not full parity proof. B3 **and B4** were
unmet/handoff required until the owner authorized the deeper continuation below.
That continuation keeps both assertions and closes the nine boxes; the old
equivalent-only receipt is not used as its proof.

## Authorized native-font continuation

The owner authorized deeper font/loading/shaping work for HTPR-7381. Acceptance
is recorded in `internal/LOOK-FONT-GATES.md`; the earlier B3/B4 handoff is history,
not the current scope limit. F011 remains unchanged.

The font authority is still `src/lib/fonts/ibmPlexSans.ts:3-10` and
`src/styles/tailwindThemes/graphite.css:169-172` at web revision
`6f9901764d355aba7bfc76e4e62ff255e0b68b31`. Footer sizing/weight/line height comes
from `src/components/Global/BottomSettings_QuickTips.tsx:413-415`; tab sizing,
spacing and weight classes from
`src/components/notifications/inboxSplit/SplitTitle.tsx:33-54` and
`tailwind.config.ts:167-178`. No source-unsupported UI size or offset is added.

CI [40098005431](https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40098005431)
generates unhinted full-coverage faces, retaining every mapped character and
matching every web-subset advance/bearing. Its `web-source-fonts/freetype.json`
proves normal FreeType advances of **106/70/46/7** for the footer label,
Important, Agents and the 11px count; unhinted design widths are
**108.2476/66.6677/43.7358/6.5999**. The exact Latin subset produces identical
normal advances. Full-source TrueType hint programs caused the browser variant
difference, while native layout previously used fractional design advances.
Rounding design advances alone cannot reproduce FreeType auto-hinting.

The embedded full faces originate from CI font artifacts (with family-name
preservation corrected in run 40099324788 below), with the web
asset's unhinted behavior and correctly named weight classes. Linux's vendored
GPUI backend retains HarfRust shaping and uses FreeType normal advances for the
actual selected glyphs, preserving kerning, fallback, variable weight and bidi
cluster order. Glyph metrics are cached per face,
size and glyph; fixed-pitch keycaps retain fractional spacing. Shaping
adjustments share the normal hinted pixel grid, and glyph positions preserve
FreeType 26.6 precision. No text-specific compensation is used. Both original strict
browser assertions remain unchanged. Final runtime/comparison proof follows.

A further source review found that the extra task-composer row is not a fixture
exception: desktop `NewCommentComponent.tsx:447-485` does **not** mount TaskOptions;
that row is mounted only by the phone branch at `:527` (its implementation is
`:631-655`). The native-only row is removed, retaining the existing H/E keyboard
actions and their write assertions. A replacement regression checks that the
desktop composer exists without either phone-only control, then exercises both
shortcuts. The CI sparse Graphite dump additionally requires the live 154px
composer height. No fixture content or comparer tolerance is changed.

Font subsetting must also retain name IDs 16/17: IBM Plex Sans Medium and
SemiBold use those typographic-family names to remain members of IBM Plex Sans.
The generator now retains all name IDs and checks the family, and the embedded
asset regression checks family names, OS/2 weights and absence of hint programs.

Corrected embedding source: CI
[40099324788](https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40099324788),
SHA `e0367d979b9385071621be848521abf4fa95b880`, `web-source-fonts` artifact:
all four faces report family IBM Plex Sans, **891** mapped characters, no lost
existing characters and no web advance/bearing differences. The new embedded
asset regression fails on the earlier incorrectly subsetted faces in that run's
macOS job (`167 passed; 1 failed`, missing typographic family), demonstrating a
real negative control; the four corrected faces are downloaded without local
conversion. Only runtime font/shader sources and tests are vendored; the
unrelated upstream Criterion benchmark sources and direct dev dependency are
omitted. Cargo still locks the optional upstream benchmark feature graph.

The patched backend is an explicit workspace member so Cargo can run its
text-system unit target. Its upstream GPU unit target is not auto-selected on
all hosts: Linux CI explicitly selects the text-system tests (including both
new metric/cluster regressions). Original workspace tests, real layout dumps
and both strict Chromium assertions remain required and unchanged.

Intermediate native dump proof (run
[40100723328](https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40100723328),
SHA `74684c577d5fc09754ea88f6651f220f12cae685`): matched **42 → 37**, dense
**74 → 71**. All six tab boxes close and the composer is 154px, but the three
footer positions remain 2–4px off. New section/board-title boxes are synthetic
label differences (YESTERDAY versus LAST WEEK, Synthetic Board versus QA Team
Board), not new source-supported UI changes. Native text tests exposed a
fractional kerning/ceil drift, so shaping adjustments now use the same pixel
grid as normal FreeType advances. Fixed-pitch keycap fonts retain fractional
advances (`src/components/Common/kbd.tsx:11-18`, the web kbd element, and the
Tailwind preflight imported by `src/styles/tailwind-entry.css:7-10`;
`BottomSettings_QuickTips.tsx:405-408` supplies the 12px size). CI records the
actual browser keycap widths as additional diagnostics. The inherited wrap-hit
regression now injects its historical one-ulp boundary explicitly instead of
assuming that font rounding will still produce that error naturally; its
out-of-width hit assertion remains mandatory.


## Verified native-font completion

Runtime SHA **dde82c20b363a0bcdc0c0f5a08280072726567c4**, CI
[40102196576](https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40102196576):
Ubuntu, Windows x64, macOS ARM64, Windows ARM64 and Windows memory receipt/budget
all **success**. The historical ram2-only baseline is skipped by its unchanged
branch condition. The final descendant's exact SHA, run and
every job result are bound in the [final receipt](https://github.com/hypertask-ai/hypertask-desktop/pull/16#issuecomment-6385476101).

`layout-dumps` artifact **12563074476**, digest
`sha256:49c7f27ac90162c68c4b5fc5bb2f5ac6acdecd1bc70f6e4d0ae108978c1c3ffc`,
was downloaded into `evidence/look-pixel-grid`. The unmodified supplied comparer,
with live 1920x1009 reference and original tolerances, reports:

| Input | Original baseline | Continuation before (1c2f460) | Native-font result |
| --- | ---: | ---: | ---: |
| Dense/raw | 116 | 74 | **68** |
| Matched structure | 83 | 42 | **34** (12 inbox / 13 board / 9 task) |

Raw outputs: `evidence/look-font-before{,-parity}.txt` and
`evidence/look-pixel-grid{,-parity}.txt`. No filtering or tolerance change.
All six tab boxes (including active alias) now match the reference exactly:
positions **118/223/304/395/528** and widths **96/72/82/124/69**. Each footer hint
is **106px** wide; native first-hint x is **380/272/324** versus live
**380.625/273.171875/324.625**, all within the unchanged 1.5px gate. Thus **all
nine** previous same-label boxes close. The desktop composer is **154px**.

The 34 remaining matched differences are **24 collector-only rail keys** and
**ten synthetic/content-dependent boxes**: inbox author/title/kind and relative
date section, board title/count, first card title/id and second card height,
and task ticket ID. The corrected font exposes three previously accidental
width agreements (Synthetic Board versus QA Team Board, its following count,
and YESTERDAY versus LAST WEEK); these are reported, not hidden by changing
fixtures. No source-supported layout discrepancy remains among these boxes.
Style-field mismatch count remains **zero**.

`font-metrics.json` proves web/full/native **106/96/72** exactly and
`LAYOUT_WEB_FONT_METRICS_OK` in Ubuntu's log. Chromium keycap diagnostics explain
the fixed-pitch exception: source-stack CTRL/K widths are **28.8046875/7.201171875**,
whereas normal-hinting the embedded mono face gives **28/7**. Fractional keycap
advances are necessary for centering the complete hint row; no fixed label
width or compensating coordinate was introduced.

CI logs additionally prove **28 native text-system tests passed**, including
metric, combining-mark/cluster, bidi/fallback and the deterministic historical
wrap-hit control. Embedded-family/weight/no-hint and desktop-no-phone-actions
regressions pass on all three test platforms. Source-generation receipts retain
**891** mapped characters per weight, lose no existing character and match all
**232** web-subset advances/bearings. The original failing strict run
[40096584998](https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40096584998)
and the family-name regression failure in run 40099324788 are real negative
controls. Both original strict font assertions remain unchanged at 1.5px.

Acceptance F1–F4 and the superseded B3/B4 handoff are closed by these CI artifacts
and the final SHA-bound receipt. F011 is unchanged. No VPS build, test, app or
test-copy execution and no owner-laptop access occurred.

Final dependency review switches the vendored GPUI test dependency from
benchmark support to the existing `test-support` APIs. Cargo still resolves the
optional upstream benchmark feature graph for this workspace member, even when
the feature is not selected. Its 24 package records are pinned by copying the
exact Cargo.lock diff emitted by GitHub run 40103510354 (not by running Cargo on
the VPS). The new lock contract failed on that real drift while all 28 text
tests passed. Linux CI requires `git diff --exit-code -- Cargo.lock` after
workspace/backend tests, so green checks cannot silently use an uncommitted
regenerated lock.
