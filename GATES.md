# Gates: HTPR-7381 Windows working set

OWNS: vendor/gpui_windows/**, crates/ht-desktop/src/main.rs, Cargo.toml, Cargo.lock, gates/ram-*, GATES.md, evidence/ram/**

Scope: Reduce maximized Graphite working set below 100 MB on the owner's laptop on inbox, board and task, with unchanged visuals and behavior; only GitHub builds/tests, isolated htram copies, no installed app or APPDATA access.

- [ ] G1: Laptop runner records three fresh-process samples and medians for every screen, exact artifact SHA and maximized 1920x1080/100% environment in isolated htram using a copy of parity.db.
  EVIDENCE: pending

- [ ] G2: Baseline and every retained code change have real laptop before/after working-set and private-byte output; non-improving changes are dropped.
  EVIDENCE: pending

- [ ] G3: Final median working set is below 100,000,000 bytes for inbox, board and task on the laptop.
  EVIDENCE: pending

- [ ] G4: Final laptop captures and interaction checks show unchanged visuals and behavior, including resizing and recovery of renderer resources.
  EVIDENCE: pending

- [ ] G5: Branch ram is rebased on origin/main before every push; signed-off attribution commits and a draft PR exist, and final SHA passes all GitHub CI jobs.
  EVIDENCE: pending

- [ ] G6: No builds, test suites or test copies execute on this VPS; no installed Windows app or APPDATA data is touched.
  EVIDENCE: pending
