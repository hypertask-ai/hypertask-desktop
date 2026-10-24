# Desktop host adapter

This adapter overrides the vendored Cursor and Claude Code defaults for this
repository. Never edit `~/.claude`, `~/.cursor`, other repositories, or install
hooks globally. Vendored files remain byte-identical to the pinned upstream.

- The model rule is `.claude/rules/pstack-models.md`, never a global path.
- Claude coordinates. Code, prose, exploration, review, and judgment go through
  hax with the explicit Codex model and high effort in that rule. A Codex session
  may do its own work. No implicit Claude fallback or model diversity claims.
- `Task` means an authorized `hax -p` worker, not a Claude Agent. Include the
  absolute poteto-mode path, exact base SHA, owned paths, expected evidence, and
  VPS/laptop prohibitions. Launch in background and collect with `task_wait`.
  With no user permission to delegate, do the review serially and label it
  self-review. Never claim an independent reviewer ran when none did.
- Use terminal read/edit/git and GitHub APIs on the VPS. Every build, test,
  negative control, lint, benchmark, and app launch runs on GitHub only. A TDD
  red/green cycle therefore uses CI commits, not a local execution exception.
- Follow `.claude/skills/verify-desktop/SKILL.md`. Do not invoke `lh`, uploaders,
  `*-round.sh`, installation scripts, or idle guards. Guarded idle QA is a
  separate explicitly authorized owner workflow, never a prerequisite for CI.
- Exposed commands are `/poteto-mode`, `/setup-pstack`, and the other unprefixed
  project skills, not the upstream plugin's `/pstack:*` names.
- Skill creation writes under `.claude/skills/`. Its local validation and live
  proof run on GitHub. Use the vendored generator and feature-map contract.
- Cursor built-ins, Bugbot, and unbundled `cursor-team-kit` skills are not
  installed. State an unavailable step once, then use source/diff review and
  the existing native CI checks. Do not invent a successful tool invocation.

On a repeated correction, prefer structure, then types, then CI/tests, then
prose. Keep upstream updates separate from repo policy changes. Read
`docs/pstack-review.md` for the concrete review behind this adaptation.
