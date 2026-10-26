# AI sidebar acceptance, written before implementation

Base main 2ceb3255911725f8e780ede25c2ec4f415b0a269.
Read-only baseline CI https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40369851241, artifact windows-visual3, gates/fixtures/visual3.
Read-only web references /home/valentin/projects/hypertask-desktop-wt-public/internal/live-graphite-1920.

1. Inventory every observed AI sidebar defect class on Inbox, Board, Task within x1500..1919. Record web/native bounds, color, font, spacing, icons and presence, retaining content and raster residuals. Never change frozen references, thresholds, or capture data.
2. Close at least 15 screen/property rows against the same baseline. Source every changed value from /home/valentin/projects/hypertask/src with file:line references. Add native layout tests for each changed value. Preserve unavailable-action blockers; no fake controls.
3. Compare exact-head hosted Windows captures against baseline. Require zero changed pixels left of x1500 on each screen. No inbox/board/task layout, keyboard or command-center edits.
4. Native regression tests and every required exact-head CI job succeed. Retain CI run, SHA, individual job results, artifact identities and supplied receipt output. No local builds, tests, lint, app launches or laptop access. One-off historical artifact comparisons stay outside permanent CI.

Owned implementation paths: crates/ht-desktop/src/ai_sidebar.rs and crates/ht-desktop/src/ai_message_tests.rs. Evidence docs may be added. Other helpers own main-content layout.

Sequence: frozen inventory, native sidebar correction, exact-head CI capture verification, evidence. Throughput checkpoint after first correction: use measured rows and failing CI output to choose remaining work, not cosmetic guesses. Self-review only; unavailable cursor-team-kit tools are replaced by source/diff review and native CI evidence.

Principle prove-it-works chooses hosted native captures, not compile-only proof. Principle sequence-verifiable-units chooses inventory before correction and CI before further correction.

Asset ownership extension: main.rs changes only sidebar asset registration; new ai-scope-board.svg and ai-scope-task.svg are sidebar-only. Existing shared icons remain untouched. Frozen capture fixture, source projection, thresholds and references remain byte-identical.
