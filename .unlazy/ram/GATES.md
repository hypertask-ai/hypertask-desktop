# Gates: HTPR-7381 Windows working set

OWNS: vendor/gpui_windows/**, crates/ht-desktop/src/main.rs, Cargo.toml, Cargo.lock, gates/ram-*, GATES.md, evidence/ram/**

Scope: Reduce maximized Graphite working set below 100 MB on the owner's laptop on inbox, board and task, with unchanged visuals and behavior; only GitHub builds/tests, isolated htram copies, no installed app or APPDATA access.

- [x] G1: Laptop runner records three fresh-process samples and medians for every screen, exact artifact SHA and maximized 1920x1080/100% environment in isolated htram using a copy of parity.db.
  CHECK: python3 gates/ram-evidence.py evidence/ram/baseline
  EXPECT: RAM_EVIDENCE_VERIFIED
  EVIDENCE: automatic-evidence=v1; definition-sha256=93b695697af394b89382305cf43ebfb9686d8cf0c78943701bd82463b2da3558; exit=0; EXPECT=matched; output-sha256=30e3c546e6822cfa33f63ad6b69001215402f71c3b7778e6a00c3347cdbd0b0f; output-bytes=162; shell=/bin/sh; cwd=/home/valentin/projects/hypertask-desktop-wt-ram; path=695691a31144/31 entries

- [x] G2: Baseline and every retained code change have real laptop before/after working-set and private-byte output; non-improving changes are dropped.
  CHECK: python3 gates/ram-evidence.py evidence/ram/baseline evidence/ram/fonts
  EXPECT: RAM_EVIDENCE_VERIFIED
  EVIDENCE: automatic-evidence=v1; definition-sha256=8304b07f95c7365758d85809d5024905ee60a4a902ee32c0f6d62542837558cf; exit=0; EXPECT=matched; output-sha256=4a8f111e1d50b9144070ac021a2cba9cdd780efa86b18d61cb0bf497bdde30dd; output-bytes=293; shell=/bin/sh; cwd=/home/valentin/projects/hypertask-desktop-wt-ram; path=695691a31144/31 entries

- [x] G3: Final median working set is below 100,000,000 bytes for inbox, board and task on the laptop.
  CHECK: python3 gates/ram-evidence.py evidence/ram/final --target
  EXPECT: RAM_EVIDENCE_VERIFIED
  EVIDENCE: automatic-evidence=v1; definition-sha256=770d8450ac561292328c8f6dacfdecb628f56e9a96a441274f23ffc47ff3e49d; exit=0; EXPECT=matched; output-sha256=47236bdcc0b35466b09fb587875e0d00d94b7cd879c85f2ea285ca87f706e070; output-bytes=153; shell=/bin/sh; cwd=/home/valentin/projects/hypertask-desktop-wt-ram; path=695691a31144/31 entries

- [x] G4: Final laptop captures and interaction checks show unchanged visuals and behavior, including resizing and recovery of renderer resources.
  CHECK: python3 gates/ram-evidence.py evidence/ram/baseline evidence/ram/final --pixels
  EXPECT: RAM_EVIDENCE_VERIFIED
  EVIDENCE: automatic-evidence=v1; definition-sha256=0d0bca64f97ca856d7fafddaa876ceb3860bb21621d6c6baa43c2799ed7a3930; exit=0; EXPECT=matched; output-sha256=676d410e824bc098507fa5c0dc1b99ded9af3b4464239894e3bc60f977f58197; output-bytes=546; shell=/bin/sh; cwd=/home/valentin/projects/hypertask-desktop-wt-ram; path=695691a31144/31 entries

- [x] G5: Branch ram is rebased on origin/main before every push; signed-off attribution commits and a draft PR exist, and final SHA passes all GitHub CI jobs.
  EVIDENCE: All six ram commits end with the requested Co-Authored-By footer; fetch/rebase immediately preceded every push. Draft PR #5 head is 1b073500b6ce4e2a31634bab2bffe48259f7b77b, base main. Push-time main was e5ce3d0d0ff75affe060dc4e3813d87c2ecadfdc; shared origin/main advanced only at 00:08 +0200, after the final 23:58 push (reflog proof retained). CI 40030576460: windows-arm64, ubuntu-24.04, macos-15, windows-2025 all success. Final Windows log proves the no-copy/lifetime test passed among 20 backend tests, zero failures. Exact PR, Git and CI output retained in evidence/ram/final.

- [x] G6: No builds, test suites or test copies execute on this VPS; no installed Windows app or APPDATA data is touched.
  EVIDENCE: Reviewed command history and gates/ram-round.sh plus ram-measure.ps1: VPS commands only downloaded/decoded artifacts and analyzed evidence; Cargo commands were metadata and offline lock resolution, never build/test/check/clippy. Every launched executable was htram/ht-ram.exe with an explicit unique htram DB and --offline; only the parity fixture was copied from htmeasure. No installed Desktop app or APPDATA/Hypertask path was accessed or changed.
