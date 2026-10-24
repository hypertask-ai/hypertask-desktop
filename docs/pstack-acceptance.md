# HTPR-7381 acceptance ledger

Written before implementation. Builds, tests (including negative controls), lint,
and gate execution run only on public GitHub runners. Local work is read/edit/git
and remote evidence inspection. No laptop commands. Evidence is recorded below
only after observing the exact head's GitHub run.

- [ ] G1: Vendor all upstream skills, docs, agents and MIT license unchanged;
  record upstream revision; expose every skill with resolving repo-local links.
  CHECK (GitHub): pstack integration gate checks file hashes, paths and license.
- [ ] G2: Repo-local Claude/Codex entrypoints preserve existing knowledge, route
  every code/judgment role to hax Codex gpt-6.1-sol high, and keep Claude orchestration-only.
  CHECK (GitHub): integration gate checks model-role coverage and entrypoints.
- [ ] G3: Desktop verification skill maps real features to existing tests, parity
  tables, layout and memory receipts; evidence is SHA-bound CI, never VPS/laptop execution.
  CHECK (GitHub): integration gate validates referenced paths and required surfaces;
  manual review compares the feature map with source and workflow commands.
- [ ] G4: Existing CI/gates stay intact; new enforcement rejects real helper
  mistakes with negative controls, and all workflows for the pushed head pass.
  CHECK (GitHub): integration gate regression controls plus all existing CI jobs.
- [ ] G5: Review repo and read-only orchestration, write short shipping guide and
  uncommitted ranked proposal at the requested scratchpad path.
  CHECK (manual): source-referenced review, guide, proposal; scoped git diff.
- [ ] G6: Commit only allowed paths with requested trailer, fetch/rebase before
  every push, publish branch pstack and draft PR to main, report last SHA and CI URL.
  CHECK (manual + GitHub): git history, PR metadata, checks for final head.

## Evidence

Pending. No gate is complete on documentation or a local test run alone.
