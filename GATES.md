# Gates: HTPR-7381 native interaction parity

Scope: isolated Windows behavior harness first, then comments, inbox/board/task/global shortcuts, command center and footer parity. No calendar/table implementation and no VPS builds/tests/test copies.

- [ ] G1: Behavior harness drives only an isolated offline laptop copy with real background window messages, verifies layout/SQLite effects, and prints per-case failures on the baseline.
  EVIDENCE: pending
- [ ] G2: Mouse comment send and Ctrl+M/type/Ctrl+Enter each enqueue the typed comment through ht_core Write; composer placement matches the web task detail.
  EVIDENCE: pending
- [ ] G3: Every in-scope shortcut is reconciled against SHORTCUTS.md with native status, source citations, and background-input behavior evidence for implemented shortcuts.
  EVIDENCE: pending
- [ ] G4: Ctrl+K modal matches web grouping/order/filter and keyboard behavior; every visible command executes a supported action; unsupported commands are explicitly hidden and listed.
  EVIDENCE: pending
- [ ] G5: Footer hints match the web per screen; CI runs and passes key-map and command-filter unit tests.
  EVIDENCE: pending
- [ ] G6: Branch keys is rebased on latest origin/layout-dump before every push, commits carry requested coauthor trailer, draft PR exists, final SHA CI is green, and final behavior round has every case PASS.
  EVIDENCE: pending

Evidence must cite real saved output, SHA and CI run. Manual gates require source review in addition to counters. No gate is passed by static presence alone. Commands are run explicitly after inspection, not by executing inherited ledgers.
