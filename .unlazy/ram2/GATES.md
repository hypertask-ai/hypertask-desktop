# Gates: HTPR-7381 CI memory regression

OWNS: .github/workflows/ci.yml, gates/windows-memory.ps1, gates/windows-memory-check.py, gates/windows-memory-budget.json, gates/GATES-ram2.md, docs/memory-ci.md, vendor/gpui_windows/src/**, crates/ht-desktop/src/**

Scope: GitHub-hosted Windows-only measurement and regression fix, with no VPS builds/tests or Windows laptop access. Keep app behavior and visuals unchanged.

- [x] G1: Windows CI produces nine live release-process samples (three per inbox/board/task), eight seconds after maximizing; artifacts include working-set/private medians, actual display/client size, executable/database hashes and source revision.
  EVIDENCE: GitHub CI 40057951407, all six jobs success. windows-memory and both historical artifacts each contain nine raw samples, three medians, PNG/layout and resident maps. All use display 1920x1080, maximized client 1920x1009, DPI 96, eight seconds. Exact medians are recorded in docs/memory-ci.md; no local app execution.

- [x] G2: CI measures both 3d00223 (before PR 11) and 5016a9ebe48b03dbac2f2312c170878166586c59 (PR 11) with identical fixtures and measurement procedure, establishing the regression independently of laptop observations.
  EVIDENCE: GitHub CI 40057951407 measured full SHA 3d002231efdbc5ea4b823e5a1af430e912a168d9 at inbox/board/task WS 60.444672/69.853184/85.184512 MB and 5016a9ebe48b03dbac2f2312c170878166586c59 at 62.541824/69.959680/80.691200 MB. Same fixture JSON, board 15/task 58006 (50 comments), harness and window dimensions. Only +2.097152 MB inbox reproduces; the laptop's +14 MB does not. Importer changes account for the different seed DB hash; layout shows Important split 2 -> 5 rendered rows.

- [x] G3: CI measures the fixed release below 100,000,000 working-set bytes per screen; source analysis plus controlled before/after CI identifies the allocation(s) reduced, without trimming the working set or changing behavior/visuals.
  EVIDENCE: CI 40060353046 (087cc86c15420ea52b5daf58086f2948dd18c178) proves WS inbox 62.640, board 69.972, task 80.437 decimal MB, all below 100 MB. Same-run PR11 private MB 36.164/46.678/59.421 vs fixed 35.353/44.724/54.526. Sparse mono/image atlas sizing and per-layout deep document clones were reduced; ClearType packing retained. All three MEMORY_VISUAL_IDENTICAL markers, equal layout/DB and all platform tests pass. This does not establish a fix for the un-reproduced 119 MB laptop observation.

- [x] G4: The normal windows-2025 job enforces a frozen inbox threshold equal to its measured fixed baseline plus five percent, and GitHub CI proves a deliberately over-budget result is rejected.
  EVIDENCE: CI 40060353046 windows-2025 and visual-proof jobs print MEMORY_OVER_BUDGET_CONTROL_REJECTED after the normal receipt passed. Frozen budget 65,514,086 bytes equals floor(CI 40059480037 fixed inbox baseline 62,394,368 * 105 / 100); 65,514,087-byte control rejects through the same check_budget oracle. Configuration is committed and ordinary Windows CI passes --budget.

- [x] G5: All final PR CI jobs pass; Windows CI verifies unchanged baseline/fixed screen captures and layout, with any unrelated upstream visual differences clearly distinguished.
  CHECK: env -u GH_TOKEN /usr/bin/gh run view 40060353046 -R hypertask-ai/hypertask-desktop --json conclusion --jq 'if .conclusion == "success" then "FINAL_CI_GREEN" else error("Final CI is not green") end'
  EXPECT: FINAL_CI_GREEN
  EVIDENCE: automatic-evidence=v1; definition-sha256=4c4f1fa2bd0dfb5c6f67cb9fbcdaa411dbb035c227d5f4077dc0518299fbcfe5; exit=0; EXPECT=matched; output-sha256=eb59285325bfcd2d4269686c11bafaed1008ededa83b726484419bbb77ee6233; output-bytes=15; shell=/bin/sh; cwd=/home/valentin/projects/hypertask-desktop-wt-ram2; path=695691a31144/31 entries

- [x] G6: Draft PR targets main from ram2; every push follows fetch/rebase origin/main, commits carry the requested coauthor trailer, and the final report cites exact SHA-bound CI measurements and URL.
  EVIDENCE: Draft PR https://github.com/hypertask-ai/hypertask-desktop/pull/12 targets main from ram2, head 087cc86c15420ea52b5daf58086f2948dd18c178. Every successful push followed fetch origin/rebase origin/main; six commits all end with the exact requested coauthor trailer. Git status is clean. GitHub run 40060353046 headSha equals local/remote PR head; all seven jobs success. Final answer uses its per-screen measurements and explicitly states the laptop observation did not reproduce.
