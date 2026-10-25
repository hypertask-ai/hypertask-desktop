# Gates: HTPR-7381 repeated installed sync failure

OWNS: crates/ht-core/**, .github/workflows/ci.yml, docs/sync2-evidence.md, GATES-sync2.md

Scope: Diagnose the installed application's repeated sync failure using owner-authorized read-only laptop observations, fix the evidenced root cause, and deliver a green draft PR without VPS builds/tests or laptop writes.

- [x] G1: Screenshot and secret-safe installed sync-log output identify the exact failing request, status and causal boundary.
  EVIDENCE: Screenshot read from https://screencast2.com/plC4o.png; laptop Get-Content returned 2026-10-25T09:58:14.169697500+00:00 event=failed operation=inbox code=invalid-data attempts=330. Separate same-user VPS HTTP GET /api/mcp/inbox/list returned 200, success=true, 505 rows, 13 synthetic string-ID rows and 23 noninteger split references; source rejects those at as_i64. See docs/sync2-evidence.md. The installed log itself does not retain an HTTP status.

- [ ] G2: A focused regression reproduces the observed failure before the fix on GitHub CI and passes with the fix; errors and existing sync/auth behavior remain covered.
  EVIDENCE: Red control at 87a5a8eb48a6398cdd7a18682425ee559c947ca1, run 40183084063 Ubuntu job 121138067251: HTTP 200 synthetic inbox must sync: inbox split index is invalid; 20 passed, 1 failed, exit 101. Fixed-head green proof pending.

- [x] G3: The minimal fix addresses the evidenced cause rather than hiding the banner; self-review checks auth, data and retry implications.
  EVIDENCE: Serial source/diff self-review completed; four production files normalize explicit synthetic IDs across SQLite, split and snapshot identity and reject unsupported notification writes. No HTTP/auth/worker/banner policy changed. CI regression coverage includes durable compact/expanded splits, malformed-ID rollback, positive-ID existing actions and HTTP 503-to-200 worker recovery. See docs/sync2-evidence.md.

- [ ] G4: The exact pushed SHA has all required CI jobs and native/layout/font, ARM64 and frozen Windows memory proof artifacts verified by the read-only receipt.
  EVIDENCE: pending

- [ ] G5: Branch sync2 is rebased on current main, committed with the requested co-author, pushed, and linked to a draft PR to main; no VPS build/test/lint or laptop mutation occurred and no secrets were published.
  EVIDENCE: pending

Manual gates bind external runtime observations and exact remote CI evidence; no inherited test or lint commands are authorized on this VPS.

Plan: reproduce read-only; trace request and history; add CI regression; fix; review; push draft; verify exact-SHA CI and artifacts. Subagents and unavailable Cursor tools are skipped; this Codex session performs serial self-review. Throughput checkpoint: laptop log diagnosis is the critical path, then CI runtime; keep the patch focused and use CI red/green proof.
