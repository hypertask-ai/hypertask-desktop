# Visual round 2 acceptance, before implementation

Scope: inbox, board, task. Own rendering changes and native layout/pixel
regressions only. No keyboard shortcut code, AI sidebar changes, fixture
rewrites, reference edits, tolerance changes, or collector filtering.

Baseline: PR 50 head e1e2f67663cb0f9a79d90206969b498959690569,
CI https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40318389622.
Frozen reference: /home/valentin/projects/hypertask-desktop-wt-public/internal/live-graphite-1920
(read only). Authoritative values: /home/valentin/projects/hypertask/src.

## Acceptance checks

1. Retain baseline CI screenshots/layouts and their SHA; compare the same
   1920x1009 Windows screens against unchanged frozen references. Original
   measured counts: inbox 24, board 24, task 23. Combined inventory: 91.
2. Inspect largest discrepancies first. Distinguish content-dependent bounds
   and collector omissions from actual product defects. Do not remove real
   data, change column counts, or hard-code reference text to reduce counts.
3. Every fix has component AND token/cascade file:line citations from the web
   source, plus a named native geometry/pixel regression. Existing assertions
   remain intact. Source/diff self-review replaces unavailable control-ui/deslop.
4. Target at least 30 fewer original inventory records (91 to <=61). Report
   actual counts even if this gate cannot be reached honestly. No reopened
   AI sidebar records; preserve prior native sidebar assertions and pixels.
5. Sequence bounded rendering units with regressions and public GitHub CI
   evidence. Prove changed behavior, not only compilation. No local builds,
   tests, lint, app launches, or test copies. No laptop access.
6. Fetch/rebase origin/main before each push. Draft PR visual2 to main. Exact
   final SHA must pass every required CI job and artifact receipt through the
   supplied ci-wait-receipt.sh. Retain screenshot comparisons, test names,
   job conclusions, artifact IDs/digests, full SHA and run URL.

## Work sequence

- Baseline artifact inspection and source-backed defect selection.
- Largest genuine inbox/board/task rendering corrections and native tests.
- Hosted verification and immutable-reference comparisons per unit.
- Further bounded fixes if supported; final exact-head receipt and full ledger.

Prove It Works selects downloaded hosted artifacts over inferred success.
Sequence Verifiable Units pairs each correction with focused native proof.
All unmet checks remain explicit, never silently waived.
