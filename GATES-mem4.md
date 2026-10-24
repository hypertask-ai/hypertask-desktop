# Gates: HTPR-7381 real laptop memory gap

Scope: Read-only installed laptop memory attribution, targeted renderer change, CI-only proof and draft PR; no local builds/tests or laptop mutation.

- [ ] G1: Installed laptop resident pages are attributed to private/shared and image/mapped/private owners, with GPU counters and display/DPI context recorded without mutating the app
  EVIDENCE: pending

- [ ] G2: Before/after CI receipts identify heavy per-screen memory and renderer resources and compare unchanged pixels/layout against main
  EVIDENCE: pending

- [ ] G3: The app-controlled allocation change is justified by observed ownership, preserves rendering, and avoids unrelated UI/prewire changes
  EVIDENCE: pending

- [ ] G4: Branch mem4 is rebased on main, committed with required trailer, pushed, and has a draft PR with every GitHub CI job successful at the last SHA
  EVIDENCE: pending

- [ ] G5: Final evidence distinguishes measured CI improvements, laptop observations, estimates and unavailable attribution; laptop after-install memory is not claimed
  EVIDENCE: pending
