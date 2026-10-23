# Gates: HTPR-7381 native interaction parity

Scope: isolated Windows behavior harness first, then comments, inbox/board/task/global shortcuts, command center and footer parity. No calendar/table implementation and no VPS builds/tests/test copies. All local Python gates below inspect saved laptop SQLite/layout evidence or GitHub logs; they do not run the app or its test suites.

- [x] G1: Behavior harness uses only offline htbehavior copies, background HWND input, measured elements and SQLite effects, and produces real per-case baseline failures.
  EVIDENCE: /tmp/htbehavior-baseline-complete/ci.txt binds d63d69227c2fb14096767ba6e8abd719dad4e116 to successful CI 40017821690. results.txt: BEHAVIOR 5/9 PASS; 4 FAIL. Failures: inbox-j-k, inbox-click-then-j, inbox-archive-task, command-open-close. Harness implementation was committed before interaction fixes (dedd2fa). Baseline included preliminary Send wiring from the interrupted session; it is NOT an untouched pre-fix release. Its mouse injector initially allowed offscreen targets, so baseline comment PASS is not visible-target proof. Subsequent runner requires in-client target centres, actual scrolling, exact modifier acknowledgment and fresh snapshots. Upload/run paths are only C:\Users\valen\AppData\Local\Temp\htbehavior; the source parity.db is read-only in htmeasure. No installed Desktop/AppData database was opened.

- [ ] G2: All 101 real-input cases pass on the final SHA, including mouse comment send, keyboard comment send, inbox flow, navigation, modal/filter actions, clipboard effects, per-screen footers and composer-after-comments geometry.
  CHECK: python3 gates/behavior-verify.py gates/behavior-cases.json /tmp/htbehavior-final
  EXPECT: BEHAVIOR 101/101 PASS; 0 FAIL
  EVIDENCE: pending

- [ ] G3: Every requested in-scope shortcut is reconciled against SHORTCUTS.md and actually implemented, with no silently reduced scope.
  EVIDENCE: UNMET. Source inventory currently records 72 done, 20 partial, 157 missing out of 249 direct context/action rows. Missing capabilities and reasons are explicit in internal/SHORTCUTS.md; hiding an unsupported command is not implementation of its shortcut. This gate must not be marked successful just because the 101 implemented-action cases pass.

- [ ] G4: Command center uses the web's supported command declaration order/grouping and token/typo filter, keyboard/mouse execution and dismissal; unsupported commands are hidden and explicitly listed, not inert entries.
  EVIDENCE: Source reviewed against AllCommands.ts, taskCommands.ts, navigationCommands.ts, useHTC.tsx, CommandList.tsx and CommandGroup.tsx. 22 supported definitions; 273 hidden definitions listed in internal/SHORTCUTS.md. Native frecency/frequently-used scoring and dynamic cached-board entries are now implemented, with source-derived keycaps and no desktop row icons; runtime/visual proof remains pending. Runtime command cases pending.

- [ ] G5: Final SHA has all four green CI jobs, and real GitHub logs prove key-map, consumed-Shift, command-filter and typed-write/catalog unit tests passed.
  CHECK: python3 gates/ci-receipt.py $(git rev-parse HEAD)
  EXPECT: CI_GREEN_UNIT_PROOF
  EVIDENCE: pending

- [ ] G6: Every push is rebased on origin/layout-dump, commits carry the requested coauthor trailer, draft PR exists, and final behavior/CI receipts agree with the final pushed SHA.
  EVIDENCE: Draft PR https://github.com/hypertask-ai/hypertask-desktop/pull/2 exists with the requested title/body/base/head. Every push in this resumed session ran git fetch origin && git rebase origin/layout-dump first; no upstream conflicts so far. Commits use the exact requested coauthor trailer. Final SHA-bound receipts pending.
