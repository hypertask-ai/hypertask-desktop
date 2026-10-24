# Gates: HTPR-7381 CI memory regression

OWNS: .github/workflows/ci.yml, gates/windows-memory.ps1, gates/windows-memory-budget.json, gates/GATES-ram2.md, docs/memory-ci.md, vendor/gpui_windows/src/**, crates/ht-desktop/src/**

Scope: GitHub-hosted Windows-only measurement and regression fix, with no VPS builds/tests or Windows laptop access. Keep app behavior and visuals unchanged.

- [ ] G1: Windows CI produces nine live release-process samples (three per inbox/board/task), eight seconds after maximizing; artifacts include working-set/private medians, actual display/client size, executable/database hashes and source revision.
  EVIDENCE: pending

- [ ] G2: CI measures both 3d00223 (before PR 11) and 5016a9ebe48b03dbac2f2312c170878166586c59 (PR 11) with identical fixtures and measurement procedure, establishing the regression independently of laptop observations.
  EVIDENCE: pending

- [ ] G3: CI measures the fixed release below 100,000,000 working-set bytes per screen; source analysis plus controlled before/after CI identifies the allocation(s) reduced, without trimming the working set or changing behavior/visuals.
  EVIDENCE: pending

- [ ] G4: The normal windows-2025 job enforces a frozen inbox threshold equal to its measured fixed baseline plus five percent, and GitHub CI proves a deliberately over-budget result is rejected.
  EVIDENCE: pending

- [ ] G5: All final PR CI jobs pass; Windows CI verifies unchanged baseline/fixed screen captures and layout, with any unrelated upstream visual differences clearly distinguished.
  EVIDENCE: pending

- [ ] G6: Draft PR targets main from ram2; every push follows fetch/rebase origin/main, commits carry the requested coauthor trailer, and the final report cites exact SHA-bound CI measurements and URL.
  EVIDENCE: pending
