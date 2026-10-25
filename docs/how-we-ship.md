# How we ship desktop

1. Read AGENTS.md, the repo-local model/host rules, poteto-mode and the relevant
   verify-desktop feature recipe. Keep the existing parity/auth ledgers.
   Claude coordinates; code and judgment run on hax Codex gpt-6.1-sol high.
2. Use one worktree/branch per bounded change and declare owned paths. Write
   acceptance checks first. Remove duplication before adding scaffolding;
   preserve every current budget, workload and assertion.
3. Commit small verifiable units. Add a focused regression for a real mistake;
   show its rejection control in GitHub CI. Never build, test, lint or launch
   a test copy on this VPS. Never access the owner's laptop from a helper.
4. Before each push, fetch origin and rebase origin/main. Push and open/update
   a draft PR. Feature branches get CI through their PR, not a push trigger.
5. Watch the exact SHA's CI. Fix failures, not thresholds. Inspect the native
   action/result proof, layout/font artifacts, workspace tests, ARM64 build,
   and both frozen Windows memory receipts. Run the read-only receipt command:
   `python3 gates/desktop-ci-receipt.py "$(git rev-parse HEAD)" --run RUN_ID`.
6. Report full SHA, run URL/result and remaining auth/native-coverage gaps.
   Green tests never promote a partial feature row to Done. Merge only when
   separately authorized; this task ends at a green draft PR. Main-push Bench
   remains an additional performance gate, not a substitute for PR CI.

For a new UI path, extend the existing GPUI input tests or the small native CI
smoke. Prove the action and visible/stored result, not a setter or final screen
alone. Preserve evidence through teardown. Use maintain-verification-skill after
feature-map drift; product bugs stay product bugs, not rewritten expectations.
Guarded idle QA is a separate owner-authorized workflow outside this repository
setup. It is neither a helper escape hatch nor a requirement to wait on a human.

Review and limitations: [pstack-review.md](pstack-review.md).
Acceptance and CI evidence: [pstack-acceptance.md](pstack-acceptance.md).
