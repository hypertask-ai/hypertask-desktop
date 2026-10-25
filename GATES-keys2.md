# Keyboard parity acceptance

## Plan and ownership

1. `how` over the affected subsystem. Read web registry, footer hints, live handlers, native resolver and GPUI dispatch.
2. `architect` for parallel design exploration. Serial self-review here; no extra helpers requested. Reuse typed `Action` and existing native controls. Reject a second shortcut registry.
3. Write the throughput checkpoint as four todo items.
   - Blocking first steps. Inventory and acceptance before code.
   - Independent workstreams. Other branches own command center, CI, presentation and wiring. This branch owns shortcut dispatch, focused input tests and this ledger.
   - Shared mutable state. Dedicated `keys2` worktree; fetch/rebase before every push.
   - Smallest safe decomposition. One owner; input routing shares screen/focus state.
4. Delegate code-writing to a subagent. Skip; this is the assigned Codex helper, without further delegation.
5. Verify on the matching surface. GitHub-hosted GPUI tests and existing native input receipts only.
6. Rebase into small, ordered commits. Acceptance/inventory, failing behavior tests, fixes, exact-head evidence.
7. If the design is contested, `interrogate` before shipping. Skip; existing typed dispatch remains authoritative.
8. Run Opening a PR. Draft only; no merge or release.

## Acceptance checks

- K1. `docs/keyboard-parity.md` inventories every applicable web registry entry, footer hint and additional inbox/board/task handler shortcut. Every web value cites `/home/valentin/projects/hypertask/src` with file and line. Distinguish advertised keys from actual handlers and feature-flag variants.
- K2. Every missing shortcut supported by existing desktop data/control paths dispatches the real action. No invented task creation defaults, rankings, favorites, server search, share links, timer state, history feeds or unrelated pages. Preserve existing auth blockers.
- K3. GPUI input tests send actual keystrokes through the desktop view. A/M/P/S/T/D commit the expected target and payload; archive, column focus, search, board switching and editor guards assert observable results, not resolver enums alone.
- K4. Added shortcut tests fail on the pre-fix implementation in GitHub CI, then pass after the fix. Each alias receives input coverage. No VPS builds, tests, lint, test copies or laptop access.
- K5. Final SHA matches the pushed branch and draft PR. Every required CI job passes, including ARM64 and Windows memory. Read-only `gates/desktop-ci-receipt.py` confirms nonexpired artifacts. Record exact run/job/test evidence below.

## Data shape and source findings

`Desktop::key_down` routes modal/editor handlers, shell navigation and typed `shortcuts::Action`. `run_shortcut` executes existing controls and durable queue writes. Reuse that shape rather than duplicating key behavior in shell state.

Quick add N/Shift+C is not equivalent to Ctrl+J compose. Web quick entry derives active-view defaults and top/bottom ranking. Native lacks those authoritative values and their bearer mutation. Preserve B3 in `docs/p2-parity.md`; do not silently create elsewhere or relabel compose as quick add.

Principle choices: Model the Domain keeps typed actions. Sequence Verifiable Units puts rejection before fixes. Prove It Works requires input/result assertions and exact-head CI receipts.

## Evidence

Pending GitHub CI. No local execution permitted.
