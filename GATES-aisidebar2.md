# AI sidebar round two acceptance

Baseline 2a5474e6639a86e97a7b1fcfb3a7c61da5234d63, PR https://github.com/hypertask-ai/hypertask-desktop/pull/56.
Rows before Inbox 14, Board 11, Task 13, total 38.

## Checks fixed before implementation

1. Account for all 38 remaining screen/property rows, with decisions grounded in /home/valentin/projects/hypertask/src file:line. Native actions must perform the real web operation with existing data and bearer routes. Missing fields/routes remain open, not decorative controls.
   CHECK: inspect docs/ai-sidebar-round2.md against the cited web files.
   EXPECT: every remaining C/I row has a disposition; attachment, recorder, context, model and scope boundaries are explicit.
2. Close at least twelve rows, measured per screen using the round-one inventory. Every changed size/stroke/spacing/color has a native regression citing the web source. Content/account/renderer differences cannot be renamed or excluded to meet the target.
   CHECK: inspect exact-head GitHub workspace test output and Windows visual3 artifact against frozen web images/JSON.
   EXPECT: total remaining rows <=26; all claimed changes have actual source and native evidence.
3. Preserve main content left of x1500 exactly on Inbox, Board, Task.
   CHECK: compare exact-head windows-visual3 images to round-one artifact from run 40391866819.
   EXPECT: zero changed pixels in [0,0,1500,1009] for all three screens.
4. Preserve frozen references, all thresholds, capture fixture, workflows, keyboard/command-center and other helpers' data code.
   CHECK: git diff 2a5474e6639a86e97a7b1fcfb3a7c61da5234d63 -- gates .github crates/ht-desktop/src/input.rs crates/ht-desktop/src/command* crates/ht-core; sha256sum /home/valentin/projects/hypertask-desktop-wt-public/internal/live-graphite-1920/*.png
   EXPECT: protected diffs empty; frozen image hashes unchanged.
5. Push aisidebar2 after fetch/rebase origin/main. Keep the requested draft PR and exact-head hosted CI green, every required job and receipt included. No local build/test/lint/app execution or laptop transport.
   CHECK: supplied ci-wait-receipt.sh on final SHA, once, after CI completes; its current unbounded 3-second default watch must not run while pending.
   EXPECT: DESKTOP_CI_VERIFIED, full job list, exact SHA, artifact identities, no expired or stale evidence.

## Owned paths and sequence

Own ai_sidebar.rs, ai_message_tests.rs, sidebar-only SVG assets and their main.rs registrations, plus this gate and round-two evidence doc. No delegation. Data shapes remain Scope, DefaultContext, Chat; no new data layer or compatibility path.

1. Establish the baseline first, before any migration: a visual regression harness that screenshots the current component across its states, plus the target when matching two implementations. No baseline, no parity claim. A blocking prerequisite, not a follow-up.
2. Anti-shortcut clauses, stated and held: no harness modifications, no baseline tampering, no component restructuring to make a diff pass. If the baseline looks wrong, stop and ask, don't edit it.
3. Migrate one component at a time. Parallelize across worktrees, one owner per component (the separate-before-serializing-shared-state principle skill). Shared primitives migrate first as a blocking phase.
4. Verify each component against its baseline via image diff on the matching surface via the control skill. A nonzero diff is a fail. Investigate the pixel delta. /loop per component until the diff is zero.
5. Run Opening a PR per component or per safe batch.

Task-specific overrides: partial sidebar pixel differences remain inventoried rather than pretending identical content; the hard unchanged crop is main content. CI only; no parallel agents requested. Sequence-verifiable-units chooses audit, bounded sidebar changes, hosted native proof, exact-head capture measurement. Prove-it-works chooses actual artifact pixels, native pointer/request tests, and full CI receipt rather than source-only claims.

Throughput checkpoint: after source audit, identify whether twelve closures are possible without changing the frozen source projection or adding nonworking actions. Do not trade the constraints for the numerical target.
