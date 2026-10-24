# Gates: HTPR-7381 Graphite native parity

OWNS: crates/ht-desktop/src/ui.rs, gates/layout-dump.py

Scope: Source-backed native layout parity at 1920x1009, with remote-only build/test and laptop captures in the isolated htmeasure folder.

- [x] G1: Board columns fill available width, header is 56 high, and card layout matches web padding and heights.
  CHECK: node .unlazy/HTPR-7381/verify-parity.mjs board
  EXPECT: PARITY_ACCEPTED board
  CWD: .
  EVIDENCE: automatic-evidence=v1; definition-sha256=43d3da7d859cbf2b6d3349a710bda965cd321e36d35808c3c9fdbf3097a1898c; exit=0; EXPECT=matched; output-sha256=80ec8fb130dc3630fb6edc76f07932cce805721ade757d484ce2b31921e24a06; output-bytes=290; shell=/bin/sh; cwd=/home/valentin/projects/hypertask-desktop-wt-layout; path=695691a31144/31 entries

- [x] G2: All three screens have an empty 420-wide Graphite AI panel with web background and border.
  CHECK: node .unlazy/HTPR-7381/verify-parity.mjs panel && python3 .unlazy/HTPR-7381/painted-borders.py panel
  EXPECT: BORDER_PIXELS_MATCH panel
  CWD: .
  EVIDENCE: automatic-evidence=v1; definition-sha256=4c3dc431c1a586f41e27d980dcbd714decafeeb027a256a85ec0bbbe8d075421; exit=0; EXPECT=matched; output-sha256=a987af6c2e4c65f5e415f22306a2287501b8f0e5df012f3ad188264c602ea4d2; output-bytes=386; shell=/bin/sh; cwd=/home/valentin/projects/hypertask-desktop-wt-layout; path=695691a31144/31 entries

- [x] G3: Task header, description, composer and meta list match the web centered layout, borders and radius.
  CHECK: node .unlazy/HTPR-7381/verify-parity.mjs task && python3 .unlazy/HTPR-7381/painted-borders.py task
  EXPECT: BORDER_PIXELS_MATCH task
  CWD: .
  EVIDENCE: automatic-evidence=v1; definition-sha256=48dfb284ad9d23eae3656256bafcd2d8123248fd1cae0cb1443ff26da00cdd9d; exit=0; EXPECT=matched; output-sha256=890bbbe28d7e0cbc500be4f9b79df23e0831c9c6ae5a404b833b9734a67c2ba5; output-bytes=619; shell=/bin/sh; cwd=/home/valentin/projects/hypertask-desktop-wt-layout; path=695691a31144/31 entries

- [x] G4: Inbox tab boxes, typography, row padding, and section header match web.
  CHECK: node .unlazy/HTPR-7381/verify-parity.mjs inbox
  EXPECT: PARITY_ACCEPTED inbox
  CWD: .
  EVIDENCE: automatic-evidence=v1; definition-sha256=9ab2d9f80041c771c80ba04b474fb1319c705f8fe61f9c6b7859a9b166f190e9; exit=0; EXPECT=matched; output-sha256=09d8cfa28cebb9a71991b7d7999ada852629ecf99ab208544c6db0e11cddff7c; output-bytes=497; shell=/bin/sh; cwd=/home/valentin/projects/hypertask-desktop-wt-layout; path=695691a31144/31 entries

- [x] G5: Board title/count and rail active color match web; moved dump keys measure the corresponding elements, not their icons or outer containers.
  CHECK: node .unlazy/HTPR-7381/verify-parity.mjs small
  EXPECT: PARITY_ACCEPTED small
  CWD: .
  EVIDENCE: automatic-evidence=v1; definition-sha256=af8c4d6b7d2df103cc4dc45f2f3df669fbd9d479bfbb8bd0fde56b9211431fe2; exit=0; EXPECT=matched; output-sha256=195490a25793349627f7299e2a60b8957d80589390ef12c5ea954ae02eb2bb7d; output-bytes=734; shell=/bin/sh; cwd=/home/valentin/projects/hypertask-desktop-wt-layout; path=695691a31144/31 entries

- [x] G6: Final pushed commit passes GitHub CI, cites changed values' web sources and ends in the requested co-author trailer; parity TOTAL plateaus or remaining differences are explained noise.
  CHECK: node .unlazy/HTPR-7381/verify-final.mjs
  EXPECT: FINAL_PARITY_VERIFIED
  CWD: .
  EVIDENCE: automatic-evidence=v1; definition-sha256=bf912190754a9a0e70a18815c461a38790bed89130dd77127c8df134ffea55d4; exit=0; EXPECT=matched; output-sha256=7bc77652b2697b2bd9dee25a1cb236195cb5dbf23cd273fed889c8f0234c8d14; output-bytes=179; shell=/bin/sh; cwd=/home/valentin/projects/hypertask-desktop-wt-layout; path=695691a31144/31 entries

- [x] G7: No local builds/app test runs or real owner app modifications; application execution uses the remote htmeasure folder only.
  EVIDENCE: Session command audit: no cargo build/test/check/clippy or app execution on VPS; rustfmt only for formatting and local JSON/PNG acceptance analysis. Inspected parity-round.sh and measure.ps1 launch only C:\Users\valen\AppData\Local\Temp\htmeasure\ht-layout.exe; debug PID 16848 was stopped explicitly. No command addressed the real OneDrive app. Upload gate fixes: last nonblank hash line, unique temporary chunk directories. Final round-3/ci.txt reports all four remote jobs success; all three captures report client=1920x1009 json=True.

Residual accounting: round-2 has 24 extra rail keys absent from the sparse web sample, 3 footer.text whole-bar versus item comparisons, 2 inherited add-button text styles, and 11 intrinsic text-width/flow boxes (2–7px). Structural geometry/style gates do not claim identical intrinsic text advances; G6 requires the requested plateau. The pixel border oracles failed against round-2 real captures (67,67,67 vs 58,63,71 panel; 54,59,68 vs 51,55,62 description), proving these oracles detect the incorrect painted values.

Final review: round-3 PNGs inspected for inbox/board/task; painted borders match live samples exactly. Plateau 55 -> 40 -> 40. Beyond measured boxes, the offline task creator and summary differ from the live snapshot (Hypertask / Not enough content for summary); cached content handling from the evolving base is not claimed as fixed by this layout work. CI 40013304276 failed because main removed the original panel-width declaration in the synthetic merge; bfd88ea relocated it, and CI 40013975218 passed all four jobs.
