# HTPR-7381 acceptance ledger

Written before implementation. Builds, tests (including negative controls), lint,
and gate execution run only on public GitHub runners. Local work is read/edit/git
and remote evidence inspection. No laptop commands. Evidence is recorded below
only after observing the exact head's GitHub run.

- [x] G1: Vendor all upstream skills, docs, agents and MIT license unchanged;
  record upstream revision; expose every skill with resolving repo-local links.
  CHECK (GitHub): pstack integration gate checks file hashes, paths and license.
- [x] G2: Repo-local Claude/Codex entrypoints preserve existing knowledge, route
  every code/judgment role to hax Codex gpt-6.1-sol high, and keep Claude orchestration-only.
  CHECK (GitHub): integration gate checks model-role coverage and entrypoints.
- [x] G3: Desktop verification skill maps real features to existing tests, parity
  tables, layout and memory receipts; evidence is SHA-bound CI, never VPS/laptop execution.
  CHECK (GitHub): integration gate validates referenced paths and required surfaces;
  manual review compares the feature map with source and workflow commands.
- [x] G4: Existing CI/gates stay intact; new enforcement rejects real helper
  mistakes with negative controls, and all workflows for the pushed head pass.
  CHECK (GitHub): integration gate regression controls plus all existing CI jobs.
- [x] G5: Review repo and read-only orchestration, write short shipping guide and
  uncommitted ranked proposal at the requested scratchpad path.
  CHECK (manual): source-referenced review, guide, proposal; scoped git diff.
- [x] G6: Commit only allowed paths with requested trailer, fetch/rebase before
  every push, publish branch pstack and draft PR to main, report last SHA and CI URL.
  CHECK (manual + GitHub): git history, PR metadata, checks for final head.

## Evidence

Implementation proof: GitHub CI run
https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40139526593
at `b0114363559605c787f64784e071eea4a9b8ad3f`, attempt 1, completed success.
The final documentation-only evidence commit must pass the same full CI again;
its exact-head receipt is attached to draft PR #23, not inferred from this run.

- G1/G2: `PSTACK_INTEGRITY_OK files=184 skills=58 roles=17 features=4`.
  The manifest revision matches upstream checkout
  `9580c23696ac8d7abed0c166f2f9918800afb819`. GitHub job 121008108939
  passed all eight regression tests, including broken discovery and Claude
  fallback rejection. Artifact `pstack-contract` ID 12577014387.
- G3: Linux job 121008108832 printed
  `DESKTOP_INPUT_ARCHIVE_VERIFIED action=e persisted=Archive queue=inbox_archive cleanup=complete evidence=retained`.
  Artifact `desktop-input` ID 12578406363 contains READY then QUEUED 1,
  notification 321926, executable SHA-256
  `005d3e92a84ef21e7af844add0ccb6ba02107a4bc4ff0ac746268f69d63db4a5`,
  exited owned PID 6348, retained result.db and both screenshots. Visual review
  of the downloaded CI images confirms Important 5 to 4 and removal of the
  selected first row. Only inbox archive has this external native smoke proof.
- G4: macos-15, windows-arm64, windows-2025, ubuntu-24.04, Pstack contract and
  Windows memory receipt and budget all succeeded. The historical branch-only
  memory comparison is intentionally skipped, not an acceptance job. Twelve
  populated theme/screen layout logs have missing=[]; sparse cases retain their
  required explicit omissions. Font metric and lockfile steps passed.
  Job 121013927054 printed all demo/heavy cold/warm over-budget rejection
  controls and `HEAVY_MEMORY_RECEIPT_VERIFIED`. No existing step or budget was
  removed or relaxed. Bench remains a separate main/manual workflow and was
  not run for this PR.
- G5: source-grounded review is docs/pstack-review.md; shipping guide is
  docs/how-we-ship.md. The requested uncommitted scratchpad proposal exists as
  pstack-orchestration.md, SHA-256
  `93ca1d1483d7df51708b0acc7e04b5e8053a5a7cfc72e976139ea702c4bb4001`.
  No external scripts/prompts were edited or executed. Git diff against main
  contains only .claude/, AGENTS.md, docs/, gates/ and .github/ changes.
- G6: draft https://github.com/hypertask-ai/hypertask-desktop/pull/23 targets
  main from pstack. All task commits end in the requested co-author trailer.
  Fetch/rebase completed before publication. The read-only receipt printed
  `DESKTOP_CI_VERIFIED b0114363559605c787f64784e071eea4a9b8ad3f`
  and all job conclusions/artifact IDs/digests. Re-run it for the final SHA.

Implementation acceptance: 6 met, 0 unmet, 0 abandoned. Final-head publication
verification remains mandatory after this evidence-only commit. No VPS build,
test, linter, negative control or app launch ran. No laptop access occurred.

## Resumed execution plan (before new implementation)

Preserve the completed vendor/model commit 2f4847f and inherited uncommitted
CI/gate drafts. Finish in three bounded units: skill plus real CI drive,
repo/orchestration review, then exact-head GitHub evidence. No delegated agents.
All six original outcomes remain required; no gate is weakened. CHECK labels
above mean remote GitHub commands only, not permission to run local test copies.

Observable acceptance for the new smoke: presented READY before E, exactly one
Archive row with archivedAt, one pending inbox_archive mutation bound to that
row, nonblank changed before/after images, independent post-exit persistence,
owned PID stopped and temporary DB removed while proof files remain. The
Pstack contract job must accept the real checkout and reject the historical
stale-head/incomplete-jobs patterns plus broken discovery and Claude fallback.
