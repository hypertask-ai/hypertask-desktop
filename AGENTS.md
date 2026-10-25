# Desktop agent entrypoint

Scope is this repository only. Preserve the existing source, fixtures, gates,
parity ledgers, and auth blockers. Owner decision HTPR-7381, 2026-10-25.

Before nontrivial work, read:

1. `.claude/rules/pstack-models.md` for CODEX mode and hax transport.
2. `.claude/pstack/skills/poteto-mode/SKILL.md` and its selected playbook.
3. `.claude/pstack/docs/harness/claude-code.md` if using Claude Code, then
   `.claude/rules/pstack-host.md` for the repository's overriding host adapter.
4. `.claude/skills/verify-desktop/SKILL.md` and `docs/how-we-ship.md`.

Read skills by path, including their referenced files. `.claude/skills/<name>/`
links to the complete vendored skill directory. Codex/hax uses this same tree;
no global skill install or Cursor plugin is required.

## Existing knowledge

`README.md` maps the crates. `crates/ht-desktop/README.md` documents native input.
`docs/shell-parity.md`, `docs/ai-parity.md`, `docs/p2-parity.md`,
`docs/compose-comments.md`, `docs/editor-formatting.md`, `docs/data-fidelity.md`,
and `docs/layout-parity.md` retain the feature and blocker evidence.
`docs/memory-ci.md`, `docs/memory-heavy.md`, and `docs/memory-laptop.md` retain
memory policies and their historical evidence. `GATES.md` and `GATES-mem4.md`
are historical contracts, not permission to run their commands on this host.
Ignored `internal/` evidence is optional and must never be published wholesale.

## Rule and enforcement

| Rule | Enforcement or explicit limit |
| --- | --- |
| Build, test, lint, and launch only on public GitHub-hosted runners, never this VPS. | CI runs the commands. The input smoke gate refuses outside GitHub Actions. This does not sandbox arbitrary terminal commands. |
| No laptop access by code helpers, including idle queries and round scripts. | Verification skill has no laptop transport. Separate owner-authorized idle QA remains outside this workflow. |
| Verify the exact current head, every required job, and nonexpired artifacts. | `gates/desktop-ci-receipt.py`; CI negative controls reject stale SHA, failed ARM64, and missing memory proof. |
| Preserve the vendored MIT notice and complete skill links. | `gates/pstack-check.py` compares the upstream manifest and resolving paths in CI. |
| Never claim an auth-blocked feature works or a partial row is Done. | Existing parity blocker tables and feature-specific tests. Full inventory completion still requires judgment and native action evidence. |
| All code and judgment use hax Codex high; Claude only coordinates. | Repo model rule and CI role coverage check. Process selection is not a security boundary. |

Work in a dedicated worktree with explicit path ownership. Fetch and rebase
`origin/main` before each push. Never alter another helper's work. Write
acceptance checks before implementation. No local test copies, no `cargo`
build/test/check/clippy, and no credential or cookie copying. Existing developer
build examples are reference for GitHub runners, not a VPS execution path.
