# Visual round acceptance

Scope: Inbox, Board, Task at 1920x1009, Graphite. Main baseline is
8c6402b37c96ac6d843d424d856d510c3f7e7937, CI 40301021475.
Frozen references: ../hypertask-desktop-wt-public/internal/live-graphite-1920,
read-only. Web source authority: /home/valentin/projects/hypertask/src.

## Gates recorded before implementation

1. Retain the latest main Windows screenshot, exact-pixel/layout receipts and
   their run/SHA provenance. Inspect all three screens at their real dimensions.
2. Publish a complete element mismatch ledger before editing product code:
   positions, dimensions, colors, typography, spacing and absent/extra controls.
   Distinguish synthetic content, collector omissions and genuine layout defects.
   Cite component and token source lines for every proposed value.
3. Correct the largest source-supported defects first, using existing native
   components. Do not edit task-open, first-run, author, AI summary or draft-marker
   behavior. No new UI, changed reference, relaxed tolerance or removed assertion.
4. Add native layout/pixel regression assertions for each changed source value.
   Run every build/test/negative control only through public GitHub CI.
5. Compare final Windows CI screenshots/layouts against the same immutable web
   references. Publish the entire before/after ledger and counts, including all
   unresolved differences. Never represent layout-only proof as pixel identity.
6. Fetch/rebase main before each push. Open the requested draft PR. Require the
   exact final SHA's every required CI job and nonexpired artifacts through the
   supplied ci-wait-receipt.sh. Report test names, artifact evidence, SHA/PR/run.

## Execution constraints

No VPS build, test, lint, app or test-copy execution. No laptop transport.
Read-only image/JSON/source analysis is permitted. No secrets or private live
content published. Keep synthetic fixture data and existing memory budgets.

## Sequence

- Baseline and source-backed mismatch ledger.
- Shared native geometry fix plus regression, then CI verification.
- Remaining largest bounded correction if baseline supports it, then CI.
- Final frozen-reference comparison and exact-head receipt.

Unavailable upstream control-ui/deslop tools are replaced by source/diff
self-review and existing hosted native/layout/pixel gates, not fabricated runs.
Prove It Works selects actual downloaded CI artifacts. Sequence Verifiable
Units keeps each product correction paired with its regression and CI receipt.
