# Visual parity round 9

Acceptance precedes implementation in `GATES-look9.md`. All executable proof
runs on public GitHub runners. No VPS build/test/lint/app copy and no laptop
access. The original fixtures, first-five live records, comparator and
1.5px/0.5 style tolerances remain unchanged.

## Original baseline

SHA `72aeb6fbef22e125472c1e3427955a948c6ace00`,
[CI 40150377661](https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40150377661),
layout artifact `12582144767`, digest
`sha256:89069a374784221d8546cd72e05793cfd5dd4729ce0ea6cb3f610cc4b0fc70bc`.

Requested main `badac6363bc3ee23bd96f0fd1f9bfa6287167f61`,
[CI 40240987694](https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40240987694),
layout artifact `12617078583`, digest
`sha256:33c4eae98f5ea898a5a9f072ddf36a17acd53a972c0463ead04ac347f123a893`.
Both downloaded `reference-comparison.json` files record the same counts.

| Original comparison | Both baselines |
| --- | ---: |
| Dense | 68 |
| Sparse | 34 |
| Reference content | 24 |
| Reference shared-box differences | 0 |

## Collector correction

The original collector limits visible rail controls with `slice(0, 5)`.
`gates/layout-collect-rail.mjs` collects all visible buttons and links with the
same computed-field semantics. The hosted probe measures 13 controls per
screen, then appends the previously missing eight records to a separate
`reference-complete-web` directory. It never overwrites the frozen reference.

The source projection comes only from `/home/valentin/projects/hypertask/src`
at `10377b20e8c338357fda306b025ef16699c9b4ab`. The rail component is unchanged
from the earlier reference source revision. Every rule has a file:line citation
in `gates/fixtures/layout-rail/source.md`. This is a browser-rendered source
projection, not a new authenticated app capture. Original inbox, board and
task records outside the newly covered rail controls remain the frozen live
measurements. Glyph paths, timers, unread news and guest states are not claimed.

Before appending anything, CI checks all first-five computed rail fields against
the frozen browser output. This rejected an incomplete projection at
`904bced2478c34b0b9ef47b816775696b57a9071`,
[CI 40241403285](https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40241403285).
The source imports Bootstrap through `styles/tailwind-entry.css:7-8`; its
important `mt-3` wins at 1rem instead of Tailwind's 0.75rem. Restoring that
imported rule reproduces the already-measured rail positions. No native
position changes.

## Measured real differences

Collector-only checkpoint `0ccb8638e4656bf5bd86ccf3e522ac3d483b3dc9`,
[CI 40242172983](https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40242172983),
layout artifact `12618045263`, digest
`sha256:cc7c649dbf4df8c9a593f6ab454a4b895b8cf3351837c02d04ee641b3918db52`.
Downloaded `rail-collector-comparison.json` measures all 24 new records.

| Expanded comparison | Before native correction | Required final result |
| --- | ---: | ---: |
| Dense | 56 | 44 |
| Sparse | 22 | 10 |
| Reference content | 12 | 0 |

The 12 actual style differences are `rail.icon.9` through `rail.icon.12` on
all three screens. Each button has web radius 0 and native radius 5. Their
source wrappers are rounded 5px; their buttons are not. Authority is
`components/PageComponents/Kanban/HeaderComponents/AppShellRail.tsx:65-87,321,333-335,347,356`.
The earlier supplemental audit incorrectly assumed every button was rounded.

The correction uses existing native wrappers for rounded hover backgrounds.
The four inner controls retain square transparent boxes, matching the source.
No controls, dimensions, positions or labels change. Native pointer tests still
require white foreground, the painted hover background and muted expanded
labels; they additionally verify the parent/child radius split and identical
bounds. `layout-rail.py` checks the corrected source values and the wrapper's
actual background rather than assigning wrapper styling to the child.

## Hosted acceptance

`layout-dumps` retains raw `rail-collected-*.json`, extended web references,
`compare-complete-*.txt`, and `rail-collector-comparison.json`. Final acceptance
requires zero completed reference mismatches, with the same source projection
digest as the measured checkpoint:
`f1131b782ebc3ebc55ebe81cfaed0d5b4d13f2a7fc67c99deb22c36e8a5d9568`.
Original comparisons must remain 68/34/24. Coverage improvement is reported
separately from the 12 source-backed style corrections.

Controls reject a hidden bottom group on each screen, a 4px native box shift,
and all 12 real pre-fix native records retained in `radius-before.json` with
their CI identity. All use the unchanged comparator. The current reference
must pass the same comparison that rejects those old records.

Final handoff requires the exact-head supplied CI receipt and all six required
jobs. Final SHA, CI URL, downloaded after-counts and artifact identity belong
in the draft PR evidence comment and the handoff, after those jobs finish.
