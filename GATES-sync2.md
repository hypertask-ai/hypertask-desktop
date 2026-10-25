# Gates: HTPR-7381 repeated installed sync failure

OWNS: crates/ht-core/**, .github/workflows/ci.yml, docs/sync2-evidence.md, GATES-sync2.md

Scope: Diagnose the installed application's repeated sync failure using owner-authorized read-only laptop observations, fix the evidenced root cause, and deliver a green draft PR without VPS builds/tests or laptop writes.

- [ ] G1: Screenshot and secret-safe installed sync-log output identify the exact failing request, status and causal boundary.
  EVIDENCE: pending

- [ ] G2: A focused regression reproduces the observed failure before the fix on GitHub CI and passes with the fix; errors and existing sync/auth behavior remain covered.
  EVIDENCE: pending

- [ ] G3: The minimal fix addresses the evidenced cause rather than hiding the banner; self-review checks auth, data and retry implications.
  EVIDENCE: pending

- [ ] G4: The exact pushed SHA has all required CI jobs and native/layout/font, ARM64 and frozen Windows memory proof artifacts verified by the read-only receipt.
  EVIDENCE: pending

- [ ] G5: Branch sync2 is rebased on current main, committed with the requested co-author, pushed, and linked to a draft PR to main; no VPS build/test/lint or laptop mutation occurred and no secrets were published.
  EVIDENCE: pending

Manual gates bind external runtime observations and exact remote CI evidence; no inherited test or lint commands are authorized on this VPS.

Plan: reproduce read-only; trace request and history; add CI regression; fix; review; push draft; verify exact-SHA CI and artifacts. Subagents and unavailable Cursor tools are skipped; this Codex session performs serial self-review. Throughput checkpoint: laptop log diagnosis is the critical path, then CI runtime; keep the patch focused and use CI red/green proof.
