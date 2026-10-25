# pstack model configuration. Repository-local owner override, HTPR-7381.
# budget: medium (high)
feature, refactoring: gpt-6.1-sol high
bug-fix: gpt-6.1-sol high
perf-issue: gpt-6.1-sol high
hillclimb: gpt-6.1-sol high
judgment and prose: gpt-6.1-sol high
hardest tasks: gpt-6.1-sol high
how explorer: gpt-6.1-sol high
how explainer: gpt-6.1-sol high
why investigators: gpt-6.1-sol high
why synthesizer: gpt-6.1-sol high
reflect tooling: gpt-6.1-sol high
reflect judgment, divergent, synthesizer: gpt-6.1-sol high
arena runners: gpt-6.1-sol high, gpt-6.1-sol high
arena cross-judge pool: gpt-6.1-sol high, gpt-6.1-sol high
swarm workers: gpt-6.1-sol high
architect runners: gpt-6.1-sol high, gpt-6.1-sol high
interrogate reviewers: gpt-6.1-sol high, gpt-6.1-sol high

## CODEX mode transport

Every role above runs through `hax --provider=codex --model=gpt-6.1-sol --effort=high`.
Claude may orchestrate only. Never map a non-Claude slug to an inherited Claude
Agent, lower effort, or fall back to another family. This overrides the upstream
Claude Code adapter's non-Claude fallback and the mode's effort adjustments.
If hax cannot start, report the transport blocker, not a substituted review.
Same-model panel members use fresh sessions and distinct lenses. Do not claim
multi-model agreement. Delegate only when the user authorizes helpers.

`setup-pstack` updates this repo-local file only. Never write global rules,
install plugins globally, or modify the vendored upstream to change policy.
