# Gates: HTPR-7381 part B — Graphite layout parity

Scope: only layout parity; F011 POST createGlobally remains server-blocked under HTPR-7699 and is not changed. No builds, tests, test copies or app runs on this VPS; no owner laptop access. Runtime proof comes only from public GitHub CI. Local commands may inspect source and compare downloaded JSON evidence.

- [ ] B1: CI validates and uploads real Graphite inbox/board/task dumps at 1920x1009 in addition to the existing resolution/theme contract, with downloaded artifacts bound to a run and SHA.
  EVIDENCE: pending
- [ ] B2: The supplied comparer records baseline and final difference counts from live measurements and CI dumps; positions, sizes, fonts and colors remain compared, while synthetic text-dependent differences are explicitly reconciled.
  EVIDENCE: pending
- [ ] B3: All actionable look differences are fixed with web-source file:line citations and CI regression assertions; no UI absent from web is introduced.
  EVIDENCE: pending
- [ ] B4: Final pushed SHA has successful public GitHub CI and the requested draft PR; every push is preceded by fetch/rebase origin/main, and every new commit ends with the requested coauthor trailer.
  EVIDENCE: pending

All gates are manual evidence reviews because runtime checks must execute on GitHub, not via this local ledger. Record exact run URLs, SHA, artifact locations, comparison output and source citations before marking any gate met.
