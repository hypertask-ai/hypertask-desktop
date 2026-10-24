# Gates: HTPR-7381 real laptop memory gap

Scope: Read-only installed laptop memory attribution, targeted startup residency change, CI-only proof and draft PR; no local builds/tests or laptop mutation.

- [x] G1: Installed laptop resident pages are attributed to private/shared and image/mapped/private owners, with GPU counters and display/DPI context recorded without mutating the app
  EVIDENCE: 2026-10-24T20:21:04Z lh read-only PID 23744 scan: 114724864 resident bytes = 71200768 private + 43524096 shareable; image 46055424; GPU shared/committed 37220352; maximized 1920x1009 DPI 96. docs/memory-laptop-before.json retains the real output; docs/memory-laptop.md distinguishes unavailable per-resource laptop attribution and non-atomic counters. Repeat failed after the owner process disappeared; no restart or mutation was attempted.

- [x] G2: Before/after CI receipts identify heavy per-screen memory and renderer resources and compare unchanged pixels/layout against the same-source untrimmed control (main runtime policy)
  EVIDENCE: GitHub run 40133081908 at cb9c4939c064ab6972e7bc39bd352d0de02a7e6b succeeded. Heavy 8 s WS 75632640/96649216/98021376 -> 2125824/2347008/2400256; real-redraw warm 73789440/110309376/96702464 -> 36622336/67637248/56127488 bytes. All three demo and heavy MEMORY_VISUAL_IDENTICAL plus MEM4_WARM_VISUAL_IDENTICAL markers; exact renderer/font payload equality. Raw before/after maps and counters are in that run's windows-memory artifacts.

- [x] G3: The app-controlled residency change is justified by observed ownership, preserves rendering, and avoids unrelated UI/prewire changes
  EVIDENCE: Source review: only main.rs two-line Windows hook plus memory.rs one-shot 5 s EmptyWorkingSet policy and control feature; no ui.rs/prewire or UI changes. Same-source CI control isolates the policy. Run 40133081908 proves all six jobs and real redraws, not the rejected-token probe from run 40129719605; docs disclose the soft/hard-fault tradeoff and no allocation reduction.

- [x] G4: Branch mem4 is rebased on main, committed with required trailer, pushed, and has a draft PR with every GitHub CI job successful at the last SHA
  EVIDENCE: PR https://github.com/hypertask-ai/hypertask-desktop/pull/21 is draft on branch mem4. git fetch/rebase origin/main precedes each push. cb9c493 source proof run 40133081908 passed windows-2025, ubuntu-24.04, macos-15, windows-arm64, memory-untrimmed-startup, Windows memory receipt and budget. All commits have the requested co-author trailer. The final evidence/warm-budget head is verified again before the final response and linked in the PR.

- [x] G5: Final evidence distinguishes measured CI improvements, laptop observations, estimates and unavailable attribution; laptop after-install memory is not claimed
  EVIDENCE: docs/memory-laptop.md records separate initial/warm residency, private commit (not reduced), private/shared/image maps, unchanged nominal GPU/font/swapchain payload, GPU counter variability and exact attribution limits. No after-install laptop number or sustained active-account improvement is claimed. Only GitHub ran app builds/tests; VPS scripts analyzed downloaded evidence and laptop commands queried the existing process read-only.
