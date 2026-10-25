# Presentation and memory

Inbox, Board and Task must retain their four themes, measured layout, font
advances and bounded Windows residency after real redraws.

## Sub-features

- `layout-themes` measures all screens/themes with populated and sparse data.
- `font-advances` matches the pinned web faces rather than substitute metrics.
- `memory-demo` measures three independent launches per screen.
- `memory-heavy` proves the heavy workload, ownership and frozen warm budgets.

## How to get to it (user POV)

- Open Inbox, Board and a task; switch among porcelain, graphite, amoled and dia.
- Resize/maximize the native window and return to it for a warm redraw.

## Driving it with GitHub CI

Preconditions: the full exact-head CI matrix, hosted Windows at the recorded
1920x1080 display / 1920x1009 client / 96 DPI contract, synthetic fixture data.

- Inspect Linux `Layout dump contract`, `Native web-font advance regression`,
  `Dependency lock contract` and `Web-source typography contract` steps.
- Inspect Windows demo/heavy receipt validation and the separate
  `Windows memory receipt and budget` acceptance job.
- Proof: `layout-dumps`, `web-source-fonts`, `windows-memory` and
  `windows-memory-heavy` artifacts remain available. Compare memory.json revision,
  executable hashes, nine samples, medians, warm render receipts and ownership
  breakdowns. Require all budget negative controls to reject over-limit data.
- Read `docs/layout-parity.md`, `docs/memory-ci.md` and `docs/memory-heavy.md`
  before interpreting differences. Inspect relevant screenshots with the receipts.

## Gotchas

- The layout gate validates measured native geometry, not complete web pixel parity.
- Cold trimmed working set alone is insufficient; warm genuine renders are gated.
- Do not drop fixture workload, resolution, sample count, ownership, or budgets.
- `.github/workflows/bench.yml` separately gates main-push macOS performance;
  a PR CI receipt is not proof that an unrun Bench workflow passed.
- Historical laptop receipts are context, never authorization to access it.
