# Keyboard parity round 3 acceptance

Scope: inbox, board, task, AI sidebar, keyboard only. No N/Shift+C work
(HTPR-7699). Own keyboard parity ledger, focused shortcut implementation and
native keyboard tests. No VPS builds/tests/lints, laptop access, or live writes.

## Checks written before implementation

- G1: Recheck every blocked/partial/unavailable in-scope keyboard row, including
  runtime duplicates. Record exact web handler, route/controller, required
  fields and whether the existing bearer client can authorize it. No guessed
  routes or vague B-number-only reasons. Source root:
  `/home/valentin/projects/hypertask/src`.
- G2: Implement every action supported by today's authenticated client without
  broadening its authorization. Each added action has a GPUI keystroke test
  asserting the observable result and, for writes, durable queue payload.
  Unsupported paths perform no substitute action. Native UI follows web only.
- G3: CI workspace tests and native input checks pass on the final HEAD.
  CHECK (GitHub only): `.github/workflows/ci.yml` workspace/native input steps.
  EXPECT: all required jobs successful, exact HEAD and valid proof artifacts.
- G4: Fetch/rebase main before each push. Draft PR keys3 -> main, requested
  title/body, coauthor trailer on each work commit. No unrelated edits.
- G5: Final read-only receipt validates current HEAD, every required job and
  nonexpired artifacts. Record exact SHA, PR/run URLs, every job conclusion.
  CHECK: `python3 gates/desktop-ci-receipt.py HEAD_SHA --run RUN_ID`
  EXPECT: verified receipt for final HEAD; never infer SaaS acceptance from CI.

## Evidence

Source audit: docs/keyboard-parity.md, U1-U3 and R1-R21.
Final exact-SHA CI evidence: draft PR final verification comment and
external scratchpad/receipt-keys3/receipt.json; valid only after receipt success.

### Grounding and design

Native input routes focus/modal guards into typed Action, then the durable queue.
BoardWrite owns authenticated board/task operations and authoritative replies.
Serial source/diff review only; no further helpers requested. Unbundled review
plugins are unavailable; no invocation claimed.

Two shapes considered: direct network writes versus existing typed BoardWrite.
Chose BoardWrite for account fencing, durable payloads and common error handling.
Time routes have no idempotency contract: each toggle gets one effect attempt;
ambiguous/crashed attempts fail closed instead of silently reversing a timer.
Model the Domain: typed StarTask/WaitingOn/ToggleTimer, not arbitrary API writes.
Prove It Works: native keystrokes plus observed requests/cache, not key enums.
Sequence Verifiable Units: audit -> focused write/input unit -> exact-head CI.

Throughput checkpoint: acceptance first; dedicated keys3 worktree; one owner
because dispatch/queue share identity; unrelated helpers' paths left untouched.

Source findings: MCP task card-actions and waiting-on now exist; time/status
returns enabled/runningEntry. Previous star/person/timer reasons were stale.
MCP saved views still omit the active transient view/rendered tab order.

### CI correction

Initial run: https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40259947343
at `5013a073f385111eeb2fe364e7b2e84325d175a2`.
Three native jobs rejected a missing JSON fixture brace at
`keyboard_parity_tests.rs:396`. Corrected fixture syntax; no assertion weakened.
This was compiler rejection, not behavioral red proof.

### Audit self-review

Corrected stale ranking premise: Shift+J/K swaps two ranks through PUT
`/api/tasks/single`, not drag/drop `/api/tasks/moveTask`. Both are session-only.
Distinguished actual runtime keys from Shift+S/2xTab/Ctrl+Y help-only claims.
History preference is local; missing interactive feed fields remain the blocker.
Time-log MCP report restricts ordinary members; web entries expose all task users.
Native options/compose/column-focus subsets remain explicitly partial.

Run https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40260330688
rejected test calls to nonexistent `drain_one`; corrected them to the existing
public `Db::drain_once`. Compiler evidence only, no weakened runtime assertions.


### Behavioral rejection and root fix

Run https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40260601962
at `2296094a823f27f303f8e28b1b7ca2bfbf85af53`: each native desktop suite
reported 214 passed, 3 failed. Timer aliases, disabled-running stop, account
fencing and uncertain-effect no-replay checks passed.

Two star fixtures supplied partial TaskView input; supplied required fields.
WaitingOn revealed a real acknowledgement bug: web returns only identity and
waitingOn fields, not a full TaskView. Merge those acknowledged fields into the
cached task before validation; never assume title/status in that response.
No shared task validation weakened. Native key and persisted-field assertions
remain unchanged. Failed receipt retained, never counted as green proof.
