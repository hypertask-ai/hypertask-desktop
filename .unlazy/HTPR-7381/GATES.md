# Gates: HTPR-7381

OWNS: crates/ht-desktop/src/**, .github/workflows/ci.yml, gates/layout-dump.py

Scope: Add --dump-layout with measured GPUI bounds, contract JSON, screen key coverage, tests, and a pushed commit with green GitHub CI. No builds or tests run on this VPS; Rust tests and builds run in GitHub CI.

- [x] G1: CLI accepts a required dump path and composes with exit-after-ready; CLI unit assertions pass in GitHub CI
  CHECK: env -u GH_TOKEN /usr/bin/gh run view 40006168001 -R hypertask-ai/hypertask-desktop --log | rg 'test tests::cli_validates_all_contract_flags \.\.\. ok'
  EXPECT: cli_validates_all_contract_flags ... ok
  EVIDENCE: automatic-evidence=v1; definition-sha256=e65cd5c54186a1f598acf7d2865eeaff975a1e6bd532e2785c1bb874a42fef69; exit=0; EXPECT=matched; output-sha256=5bc2c117c244a5bcde08e2071b2096a7090de648b0d00aca543ef4a1a3b3fd63; output-bytes=389; shell=/bin/sh; cwd=/home/valentin/projects/hypertask-desktop-wt-layout; path=695691a31144/31 entries

- [x] G2: JSON matches the shared layout contract; schema unit assertions pass in GitHub CI
  CHECK: env -u GH_TOKEN /usr/bin/gh run view 40006168001 -R hypertask-ai/hypertask-desktop --log | rg 'test layout::tests::layout_json_matches_shared_shape \.\.\. ok'
  EXPECT: layout_json_matches_shared_shape ... ok
  EVIDENCE: automatic-evidence=v1; definition-sha256=518b406a330e3cd47c609b0cab36ac0bc992d04adc3e27555417377a3e554767; exit=0; EXPECT=matched; output-sha256=cca60676b5cb7eb4981b7fd23421578c837d2b2d96f7a93f7c05c6cefc51aa73; output-bytes=413; shell=/bin/sh; cwd=/home/valentin/projects/hypertask-desktop-wt-layout; path=695691a31144/31 entries

- [x] G3: Every native contract element is instrumented with GPUI bounds and theme styles; absent keys are reported per screen
  CHECK: env -u GH_TOKEN /usr/bin/gh run view 40006168001 -R hypertask-ai/hypertask-desktop --log | rg 'LAYOUT_DUMP_OK: all screens, all themes, hold, exit and write failure'
  EXPECT: LAYOUT_DUMP_OK
  EVIDENCE: automatic-evidence=v1; definition-sha256=3b654f3e6236511889375afd494c7aec4a71f9e3b1717634620aba02deadc856; exit=0; EXPECT=matched; output-sha256=3309f53527290a6abcfb691f613fc51ffffc855a7b763d93b3030bf1842471b1; output-bytes=133; shell=/bin/sh; cwd=/home/valentin/projects/hypertask-desktop-wt-layout; path=695691a31144/31 entries

- [x] G4: Dump occurs after presentation, before optional exit, and normal mode continues running
  CHECK: env -u GH_TOKEN /usr/bin/gh run view 40006168001 -R hypertask-ai/hypertask-desktop --log | rg 'LAYOUT_DUMP_OK: all screens, all themes, hold, exit and write failure'
  EXPECT: LAYOUT_DUMP_OK
  EVIDENCE: automatic-evidence=v1; definition-sha256=3b654f3e6236511889375afd494c7aec4a71f9e3b1717634620aba02deadc856; exit=0; EXPECT=matched; output-sha256=3309f53527290a6abcfb691f613fc51ffffc855a7b763d93b3030bf1842471b1; output-bytes=133; shell=/bin/sh; cwd=/home/valentin/projects/hypertask-desktop-wt-layout; path=695691a31144/31 entries

- [x] G5: Rust formatting passes for changed Rust files (workspace has pre-existing formatting differences)
  CHECK: rustfmt --edition 2024 --config skip_children=true --check crates/ht-desktop/src/layout.rs crates/ht-desktop/src/main.rs crates/ht-desktop/src/ui.rs
  EXPECT: /^$/
  EVIDENCE: automatic-evidence=v1; definition-sha256=0ed2252125b6c07334866383ce8550dee207997127d7ac53abad7eefbf2cbee5; exit=0; EXPECT=matched; output-sha256=e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855; output-bytes=0; shell=/bin/sh; cwd=/home/valentin/projects/hypertask-desktop-wt-layout; path=695691a31144/31 entries

- [x] G6: Pushed layout-dump commit has requested coauthor trailer and all GitHub CI jobs are green
  CHECK: test "$(git rev-parse HEAD)" = 544d67ae4a04349015c88e7482616b28af19d5fa && test "$(git ls-remote origin refs/heads/layout-dump | cut -f1)" = 544d67ae4a04349015c88e7482616b28af19d5fa && env -u GH_TOKEN /usr/bin/gh run view 40006168001 -R hypertask-ai/hypertask-desktop --json conclusion,headSha,jobs --jq 'select(.headSha == "544d67ae4a04349015c88e7482616b28af19d5fa" and .conclusion == "success" and (.jobs | length) == 4 and all(.jobs[]; .conclusion == "success")) | "CI_ALL_GREEN"' && git log -1 --format=%B | rg -F 'Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>'
  EXPECT: CI_ALL_GREEN
  EVIDENCE: automatic-evidence=v1; definition-sha256=ba4cced451184346982ec3712ec8b60597c12a683e769eb1e641a4d673348143; exit=0; EXPECT=matched; output-sha256=253bfa3244be4401fdf6569994e97c5e691208feb76e9ee6effb1ba4866728a3; output-bytes=82; shell=/bin/sh; cwd=/home/valentin/projects/hypertask-desktop-wt-layout; path=695691a31144/31 entries
