# Desktop pstack review

Source/diff self-review for HTPR-7381, 2026-10-25. This is an application of
poteto-mode, not a review by Lauren Tan. No independent reviewer was spawned.
Upstream is MIT, pinned in `.claude/pstack/UPSTREAM.md`; repo policy never edits
upstream bytes. Cursor-only tools and create-skill are unavailable here. The
bundled create-verification-skill procedure supplies the generated skill contract;
its end-to-end live proof runs on public GitHub, not this machine.

## What to keep

- `Cargo.toml` has a UI-free ht-core boundary, schema generator, GPUI desktop and
  two explicitly patched backends. `ht-core/src/queue.rs` owns optimistic writes,
  durable retry and authoritative acknowledgement. Do not duplicate that in UI.
- `.github/workflows/ci.yml` tests the release workspace on Linux, Windows and
  macOS, cross-builds Windows ARM64, checks lockfile drift and font advances,
  measures native layouts and freezes demo/heavy Windows memory budgets.
- `gates/layout-dump.py` validates measured geometry, required keys, actual
  fixture text, sparse cases and themes. Typography helpers preserve web font
  identity. This is stronger than screenshots alone but not full web pixel parity.
- `gates/windows-memory-check.py` and `gates/heavy-memory-check.py` verify nine
  independent launch samples, rendered receipts, owners, frozen budgets and
  rejection controls. Warm real-redraw evidence must not be replaced by a
  cold trimmed working set. Keep the heavy workload and resolution unchanged.
- `.github/workflows/bench.yml` independently runs macOS performance budgets on
  main pushes/manual dispatch. PR CI is not proof of that separate workflow.
- `docs/shell-parity.md`, `p2-parity.md`, `ai-parity.md`, `compose-comments.md`,
  `editor-formatting.md` and `data-fidelity.md` distinguish local implementation,
  mocked wire contracts, real authorization and full-row blockers. Preserve all
  route/auth citations and partial statuses. The older desktop README's four-write
  pilot is historical, not the current full feature inventory.

## Problems and smallest changes

1. **Partial/stale CI can look green.** `gates/ci.mjs` reads origin/main instead
   of the working head and requires only three platform jobs. The older Python
   receipts are tied to branches keys/aiside/mem2/mem3. Add one branch-neutral
   desktop receipt as the supported final proof, retain specialized historical
   checks without pretending their unit coverage is redundant. Require the exact
   SHA, completed successful workflow, all six acceptance jobs and live artifacts.
   Negative controls replay stale-head, missing-memory and failed-ARM64 states.
2. **No cold-session verification entrypoint.** Existing native verify.mjs and
   internal behavior scripts require tools/state not installed on every runner.
   Generate one local skill with a four-feature map, reusing existing gates.
   Add only the missing small external keyboard-to-SQLite proof on hosted Linux.
   It owns its display/database/PID, runs offline and retains evidence after
   cleanup. No new driver framework, scheduler or parallel app fleet.
3. **Pstack host defaults violate owner policy.** Upstream setup writes globally
   and the Claude adapter can inherit a non-Claude role onto Claude. Override in
   repo rules only, cover every setup role, and check discovery/hash/model/map
   invariants in a lightweight parallel CI job. Deliberately broken links and
   Claude fallback configurations must fail that same checker.
4. **Coverage is not exhaustive.** Existing GPUI tests drive real key/pointer
   paths, but public CI does not click every inventory row through the external
   OS window. The initial smoke certifies inbox E only. AI live flags, rich HTML
   sanitizer gaps and unexposed session routes remain explicit limitations.

## Read-only orchestration audit

Read the scratchpad h-*.md prompts and ship-branch.sh, install-real.sh,
qa-when-away.sh, plus the public worktree's internal/gates inventory, guarded
behavior/parity/memory scripts, laptop-idle.ps1 and acceptance ledgers. Nothing
there was executed or edited. Private content/credentials are not vendored.

The external merge script merges/pushes main without first verifying the branch
head or PR gates. The installer checks CI but kills all ht-desktop processes by
name and restarts the real app; it is not protected by the away-only QA guard.
when-away.sh --idle uploads files and creates a temp directory, so it is not a
pure read-only query. qa-when-away.sh regenerates a large stale prompt and waits
for idle; its historical 115-second lease failure consumed an available window.
The internal Windows ledger honestly records zero tested feature rows despite
successful guard/CI work. CI green and idle safety are not a native QA verdict.

The uncommitted `pstack-orchestration.md` proposal at the requested scratchpad
path ranks exact changes, expected benefit (not measured speedups), safety and
acceptance. Those scripts and prompts are outside this PR's ownership.

## Principles that changed the implementation

- Subtract before adding: one supported generic receipt replaces repeated
  branch selection in new instructions; no new orchestration framework.
- Prove it works: real hosted keyboard input, independent SQLite readback,
  screenshots, process cleanup and retained proof rather than a documentation-only skill.
- Encode lessons: wrong head, incomplete jobs, broken skill discovery and model
  fallback become CI rejection controls, not another prompt paragraph.
- Sequence verifiable units: pinned vendor/model commit first, runnable skill/
  gate integration next, exact-head CI evidence last. Nothing merges here.
- Never block on the human: use synthetic hosted state; do not wait for owner
  inactivity or broaden auth permissions to manufacture verification.
