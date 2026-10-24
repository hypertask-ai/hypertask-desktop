# Gates: HTPR-7381 native font metrics continuation

Scope: close the blocking web typography contract and nine same-label metric boxes, improve other source-supported layout differences, and retain draft PR 16 targeting main. F011 untouched. No builds/tests/test copies/app execution on the VPS; no laptop access. Runtime evidence comes exclusively from public GitHub CI. Local commands only inspect source and downloaded CI evidence.

- [ ] F1: Final pushed SHA passes every applicable CI job, including the unchanged strict web-font assertions on ubuntu-24.04.
  EVIDENCE: pending; inspect GitHub run jobs, strict gate log and SHA-bound font-metrics.json.
- [ ] F2: The nine same-label metric boxes disappear from the unmodified live comparer on CI Graphite 1920x1009 dumps, with independently measured dense and matched before/after totals.
  EVIDENCE: pending; compare original baseline and final downloaded layout artifacts against internal/live-graphite-1920 in the public worktree.
- [ ] F3: Font/loading/layout changes use web-source values with file:line citations, preserve character coverage, add no unsupported UI, leave F011 untouched and never weaken or skip a gate.
  EVIDENCE: pending; source/diff review and CI regression output.
- [ ] F4: Final SHA is pushed to look, draft PR 16 still targets main, each push follows fetch/rebase origin/main, and session commits end with the requested coauthor trailer.
  EVIDENCE: pending; git history, push output and GitHub PR/run metadata.

All gates are manual reviews of GitHub runtime evidence and downloaded artifacts, not commands that execute tests on this VPS. Prior B3/B4 handoff is superseded by the owner's explicit deeper-font authorization; earlier evidence is retained as history.
