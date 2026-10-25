---
name: verify-desktop
description: Verify the native Hypertask GPUI desktop through public GitHub CI, real keyboard input, SQLite effects, layout dumps and Windows memory receipts. Use before claiming a desktop change works or handing off a PR.
---

# Verify desktop

Read `features/README.md`, then the relevant feature recipe. This is the output
of create-verification-skill, adapted through the repo's pstack host rule.
No local builds, tests, lint, benchmarks, or test copies. No laptop transport.

## Launch

The launch host is the public repo's GitHub-hosted runner, not this terminal.
After committing, fetch and rebase origin/main, push the branch, and open/update
a draft PR. `.github/workflows/ci.yml` runs workspace release tests and builds
`ht-desktop` and `ht-core-cli`. Watch its exact head run:

```sh
env -u GH_TOKEN /usr/bin/gh run list -R hypertask-ai/hypertask-desktop --workflow ci.yml --commit "$(git rev-parse HEAD)" --json databaseId,headSha,status,conclusion,url
env -u GH_TOKEN /usr/bin/gh run watch RUN_ID -R hypertask-ai/hypertask-desktop --exit-status
```

Replace RUN_ID with the returned numeric ID, never an earlier run.
For a new branch there is no CI push trigger until the PR exists.

On the Linux runner, the `Native inbox keyboard archive` step executes:

```sh
xvfb-run -a -s '-screen 0 1440x900x24' python3 gates/desktop-input.py
```

That script imports `fixtures/demo` into a new temporary database, then starts
`target/release/ht-desktop --db <owned-temp>/local.db --offline --screen inbox
--board 15 --task 58006 --theme graphite --hold-after-ready`. `READY` means a
GPUI data frame was presented, not merely that a process started. It does not
need auth, a port, a browser profile, or the owner's data. Each invocation has
its own Xvfb display and database. Windows demo/heavy receipts similarly use
owned fixture state. Do not drive another invocation's display or process.

## Doctor

Before interpreting proof, inspect the run identity and job steps read-only:

```sh
env -u GH_TOKEN /usr/bin/gh run view RUN_ID -R hypertask-ai/hypertask-desktop --json headSha,status,conclusion,jobs,url
```

Require headSha to equal `git rev-parse HEAD`. The runner smoke checks a live
owned process, one visible `ai.hypertask.desktop` window on its isolated display,
and the presented-frame READY marker before input. If any check fails, inspect
its uploaded logs; never switch to the installed app or an old executable.

After the run finishes, require the full read-only receipt:

```sh
python3 gates/desktop-ci-receipt.py "$(git rev-parse HEAD)" --run RUN_ID
```

This command only queries GitHub. It never launches/builds/tests anything here.
Success requires the CI workflow, all six required jobs, and nonempty,
nonexpired proof artifacts. A skipped memory job is failure, not green.

## Drive

The runner sends a real X11 `e` key into the focused inbox window via xdotool.
It observes `QUEUED`, independently queries SQLite for exactly one newly archived
notification and one `inbox_archive` queue entry, captures before/after windows,
and reads the persisted state again after terminating its owned PID. No internal
setter, production request, cookie, or provider key is used.

Other feature recipes map to existing workspace GPUI input tests, layout
contracts and Windows receipts. These are different proof layers. A request mock
proves transport behavior, not live SaaS authorization. A layout dump proves
measured geometry, not every click. The initial live skill proof is inbox
archive only; no claim of complete native feature coverage follows from it.

## Evidence

The GitHub run retains `desktop-input` with action logs, stderr, before/after
PNGs, `result.db`, and a receipt containing revision, executable SHA-256, PID,
notification ID and queue kind. The cleanup check runs before the success marker
`DESKTOP_INPUT_ARCHIVE_VERIFIED`.

Also require `pstack-contract`, `layout-dumps`, `web-source-fonts`, both Windows
memory artifacts and all four platform binaries. Record run URL, full commit
SHA, run attempt, job conclusions and artifact IDs/digests from the receipt.
Download proof with `gh run download RUN_ID -R hypertask-ai/hypertask-desktop
-n desktop-input -D <new-evidence-directory>`; keep it out of commits.

Never mark an auth-blocked or partial parity row Done. Read the exact route and
auth boundary in `docs/*parity.md`, `docs/compose-comments.md`,
`docs/editor-formatting.md` and `docs/data-fidelity.md`. CI fixtures are synthetic;
no successful real-server round trip is implied.

## Cleanup

The smoke's finally block terminates only its Popen PID, waits, and kills that
same process only if needed. The temporary database directory is removed;
`result.db`, logs and images survive under `evidence/desktop-input`. Xvfb-run
owns display teardown. A failed drive still attempts process cleanup and CI
uploads available evidence with `always()`. Never kill by executable name.
Do not delete GitHub artifacts during cleanup.

## Helpers

- `gates/desktop-input.py` is invoked by Python in the Linux CI step above. It
  refuses execution outside GitHub-hosted Actions. Do not run it on the VPS.
- `gates/desktop-ci-receipt.py SHA [--run RUN_ID]` is the read-only GitHub doctor
  and final evidence command, safe on this host.
- `gates/pstack-check.py` and `gates/pstack-regressions.py` run only in the CI
  `Pstack contract` job. They validate discovery/model/map integrity and reject
  stale/partial evidence with negative controls.

For upkeep use `/maintain-verification-skill`. Check every mapped feature
against source and live CI evidence; report harness gaps/product blockers
separately. Keep vendor bytes unchanged and adapt policy outside the vendor.
