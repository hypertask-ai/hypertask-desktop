# Keyboard round 4 acceptance

Scope: inbox, board, task, AI sidebar only. Owned paths: keyboard ledger,
focused shortcut dispatch/actions and native keyboard tests. No general layout
changes. No VPS builds/tests/lints/copies or laptop access. GitHub CI only.

## Acceptance checks (before implementation)

- G1: Re-derive registry, every scoped hotkey/useHotkeys/keydown handler and
  footer hint from `/home/valentin/projects/hypertask/src`. Add omitted rows,
  preserve conflicting hints, cite file:line. Record source revision and scan.
- G2: Recheck every incomplete scoped row against main including PRs 45/48.
  Implement every now-supported action with the same key, scope, result and UI;
  blocked rows name exact missing route/field with current source citations.
- G3: Each newly supported shortcut has native GPUI input coverage asserting
  observable results, durable writes where applicable, and input/scope guards.
  CHECK (GitHub only): CI workspace release tests, native Linux/Windows input.
  EXPECT: new named tests and existing suite pass without weakened assertions.
- G4: Small focused commits with requested coauthor trailer; fetch and rebase
  origin/main before every push. Draft keys4 PR to main with requested title/body.
- G5: Exact final HEAD has every required CI job green and valid artifacts.
  CHECK (read-only): supplied ci-wait-receipt.sh SHA --run RUN_ID --repo-dir
  /home/valentin/projects/hypertask-desktop-wt-keys4 --output receipt-keys4.
  EXPECT: receipt.json validates current SHA, required jobs and proof artifacts.

## Approach and throughput

- Blocking first: acceptance, source inventory and main capability audit.
- Independent workstreams: n/a; no delegated helpers requested.
- Shared mutable state: only this worktree; avoid visual helper layout files.
- Smallest decomposition: one owner for dispatch/action/native-test contracts.

Model the Domain: reuse typed Action/Modal/BoardWrite and existing editor state.
Prove It Works: real native keystrokes plus durable effects in GitHub CI.
Sequence Verifiable Units: audit -> focused action/tests -> exact-head CI.
Serial self-review; unavailable external review plugins not claimed.

## Evidence

Pending source audit and exact-head GitHub receipt. CI is synthetic native proof,
not live-owner server acceptance. No blocked/partial row promoted by green CI.
