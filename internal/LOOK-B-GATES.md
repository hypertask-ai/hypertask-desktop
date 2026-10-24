# Gates: HTPR-7381 part B — Graphite layout parity

Scope: only layout parity; F011 POST createGlobally remains server-blocked under HTPR-7699 and is not changed. No builds, tests, test copies or app runs on this VPS; no owner laptop access. Runtime proof comes only from public GitHub CI. Local commands may inspect source and compare downloaded JSON evidence.

- [x] B1: CI validates and uploads real Graphite inbox/board/task dumps at 1920x1009 in addition to the existing resolution/theme contract, with downloaded artifacts bound to a run and SHA.
  EVIDENCE: CI 40094950702, SHA f9ee7e4fbe2b8acc4d546f1486fd56e0c5879902; layout-dumps artifact 12560028723, digest sha256:fa0fd7e208c23aa14a185c0f25f027eaf0bbb9151bdf7a66db7b2811e21fc5e6. Downloaded evidence/look-final; LAYOUT_DUMP_OK covers all screens/themes, Graphite 1920x1009, hold, exit and failed write. Final descendant receipt: https://github.com/hypertask-ai/hypertask-desktop/pull/16#issuecomment-6384475006
- [x] B2: The supplied comparer records baseline and final difference counts from live measurements and CI dumps; positions, sizes, fonts and colors remain compared, while synthetic text-dependent differences are explicitly reconciled.
  EVIDENCE: Unmodified public-worktree internal/gates/layout-compare.py at width 1920: dense baseline CI 40086213038 39/41/36=116 to 25/26/23=74; structural baseline CI 40087161031 32/27/24=83 to 19/12/11=42. Source style mismatches 29 to 0. Raw output evidence/look-{before,structural-before,final,final-parity}.txt; residuals and source citations docs/layout-parity.md. No output filtering or changed tolerance.
- [x] B3: All actionable look differences are fixed with web-source file:line citations and CI regression assertions; no UI absent from web is introduced.
  EVIDENCE: Owner authorized deeper native font work. Runtime CI 40102196576, SHA dde82c20b363a0bcdc0c0f5a08280072726567c4 closes all nine metric boxes, retains both strict font gates, removes the phone-only composer row, and records dense 68/matched 34 differences (only coverage/synthetic content remain). See internal/LOOK-FONT-GATES.md, docs/layout-parity.md and final receipt https://github.com/hypertask-ai/hypertask-desktop/pull/16#issuecomment-6385476101.
- [x] B4: Final pushed SHA has successful public GitHub CI and the requested draft PR; every push is preceded by fetch/rebase origin/main, and every new commit ends with the requested coauthor trailer.
  EVIDENCE: Runtime CI 40102196576: all five applicable jobs success, unchanged ram2-only baseline skipped. Every continuation push follows fetch/rebase origin/main with the required coauthor trailer; PR 16 remains draft/main/look. Final descendant is independently verified and SHA-bound in https://github.com/hypertask-ai/hypertask-desktop/pull/16#issuecomment-6385476101.



Outcome: four gates met after the authorized native-font continuation; earlier abandonment is superseded, not waived. Latest comparison is dense 116 to 68 and matched 83 to 34; exact final SHA/CI receipt linked above. Runtime proof comes only from GitHub.
