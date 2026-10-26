# First-run acceptance checks

All runtime checks execute only on public GitHub-hosted CI. No VPS build/test/lint/launch; no laptop access.

1. Trace inbox, unsynced board, task, author/assignee/tag/status/project references, comments/drafts, AI context, command center, search and keyboard navigation. Record native and authoritative web source citations, including paths already correct.
2. Empty database + mocked server: inbox/board fetch before showing an empty state; show the web loading state while outstanding. Verify populated rendered content and durable cache.
3. Empty database + mocked server: task and related metadata/comments/drafts load, failures remain retryable, and late navigation never replaces the current destination.
4. Empty database + mocked server: uncached search/command destinations and AI task/board context retain correct IDs and fetched labels.
5. Native first-sign-in walkthrough: empty DB -> inbox -> first row -> its board -> card. Assert loading then real content, never the offline fallback.
6. Exact pushed HEAD: every required GitHub CI job succeeds; retain named test output and read-only CI receipt, artifact IDs and URLs.

Plan: source audit and failing native regressions; focused fetch/loading fixes; self-review; exact-head public CI; correct failures and retain receipt. Source/diff review substitutes for unavailable cursor-team-kit deslop/control tools, never runtime evidence. Prove-it-works selects rendered content plus persisted DB assertions; sequence-verifiable-units keeps audit, regression and fix commits separate. No delegated work.
