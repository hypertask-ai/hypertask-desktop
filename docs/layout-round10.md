# Round 10 layout audit

## Source-backed task spacing

The task header without a summary needs more bottom space. Desktop always uses 8px. Web selects `pb-3` without a summary and `pb-[8px]` with one at `/home/valentin/projects/hypertask/src/components/PageComponents/TaskDetail/TopRow/BaseTitleContainer.tsx:35-47`.

The web imports Bootstrap into the first cascade layer at `src/styles/tailwind-entry.css:7-8`, through `src/styles/globals.scss:10`. Bootstrap's important `pb-3` wins over the ordinary Tailwind utility. It resolves to 1rem, or 16px, rather than Tailwind's nominal 12px. Round 9 already preserves the same imported-utility precedence for the rail's `mt-3`.

The correction changes only summary-free headers from 8px to 16px bottom padding. It leaves headers with summaries unchanged. Source revision is `003a872014c35eeba1ab21047cdfdf70160e654e`. A native regression measures both short and wrapping titles. The expected height uses the source's `pt-9`, 37px title minimum, and imported `pb-3`. The title minimum comes from `src/components/PageComponents/TaskDetail/TopRow/TaskTitle.tsx:210`.

## What the difference totals measure

The unchanged comparison includes content-dependent text boxes and native-only data rows. It compares the same frozen web task against three different native data sets. A raw difference is not automatically a product defect. No fixture, web measurement, comparer tolerance, or existing assertion changes in round 10.

Baseline CI is https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40243346848 at `303d27bb9c62ea3b6c8dd521ea136f5cab1db2fa`. Artifact `layout-dumps`, ID `12617763359`, has digest `sha256:a082c1c613ea25ddfefa1aed2fca216609f2d237e1659d7d6dc89bd07e2a7758`.

| Data set | Inbox | Board | Task | Total |
| --- | ---: | ---: | ---: | ---: |
| Dense | 13 | 17 | 14 | 44 |
| Sparse | 4 | 5 | 1 | 10 |
| Reference content | 0 | 0 | 0 | 0 |

These are the unfiltered `compare-complete-dense.txt`, `compare-complete-parity.txt`, and `compare-complete-reference.txt` outputs. The older incomplete comparisons still include the 24 rail records absent from the original frozen measurement. They are not the totals above.

## Every reported group

All web paths below are relative to `/home/valentin/projects/hypertask`, at the source revision above.

| Screen and records | Dense differences | Sparse differences | Source and observed cause |
| --- | ---: | ---: | --- |
| Inbox author, ticket, title, kind | 4 | 4 | `src/components/notifications/NotificationRow.tsx:62-78,265-276` renders the notification's actual author and content. `src/components/Common/TaskRowComponents/TaskTitle.tsx:185-208` renders its ticket and title with the source gap. Dense has Morgan Example and Alex Example, the DB3-422 ticket and a long task title. Sparse has Synthetic User, QA-103 and Synthetic reminder task. Reference content has the web strings and zero differences. Fixed widths would truncate valid content or move following text incorrectly. |
| Inbox tabs 1 through 4 | 4 | 0 | `src/components/notifications/inboxSplit/SplitTitle.tsx:32-54` sizes each label and counter from actual data. Dense has Mentions, Updates, Agents and Demo Board labels, unlike the frozen five-tab reference. |
| Inbox extra tab and rows | 5 | 0 | Dense includes tab 5 and rows 1 through 4. The reference has one notification and five tabs. Removing the extra native data would hide valid content. The row rendering authority is `src/components/notifications/NotificationRow.tsx:49-59`; split label authority is `src/components/notifications/inboxSplit/SplitTitle.tsx:32-54`. |
| Board columns 0 through 2 and add control | 4 | 0 | `src/components/PageComponents/Kanban/KanbanHomepageComponents/Homepage.tsx:922-925` uses 97% width, 16px gaps, and 280px to 535px tracks with the rail. Dense has thirteen sections, so tracks reach the 280px minimum. The three-column sparse and reference fixtures both measure 459px tracks. The add control follows its column's right edge. |
| Board cards 0 and 1 | 2 | 1 | `src/components/PageComponents/Kanban/KanbanTaskComponents/KanbanTaskCard.tsx:162-203` renders data-dependent top rows, titles and tags. `TaskTopRow.tsx:93-132` renders notification and comment controls conditionally. Dense card 0 includes the retained layout-coverage label, inbox badge, comment count and a wrapped title. Sparse card 1 lacks the reference card's inbox badge. Reference content matches both full card boxes. |
| Board title, count, first ticket and title | 4 | 4 | `KanbanTaskCard.tsx:169,181,266-272` and `TaskTopRow.tsx:83` render actual task text. `src/components/PageComponents/Kanban/HeaderComponents/TitleKanbanHeader.tsx:26-30` renders the project title. Dense has Demo Board and 83 tasks. Sparse has Synthetic Board, QA-101 and Synthetic card one. Reference has QA Team Board, QABO-7 and the captured title. Count position follows project-title width. |
| Board extra column, cards, badge, label and metadata | 7 | 0 | Dense-only column 3, cards 2 through 4, and card 0 badge, label and metadata follow actual sections and task data. Authorities are `Homepage.tsx:922-934`, `KanbanTaskCard.tsx:162-203` and `TaskTopRow.tsx:93-132`. They remain visible and counted. |
| Task header, title, description, composer and metadata rows 0 through 5 | 10 | 0 | `src/components/PageComponents/TaskDetail/TopRow/TaskTitle.tsx:72-77,168-174,210` allows content-sized titles. `CommentAndDescription/DescriptionContainer/InnerHtmlDescription.tsx:33-51` selects actual description or placeholder. `CommentAndDescription/index.tsx:123-154,208-220,273-280` lays out actual thread content before the composer. Dense has a multiline title, populated description, 50 loaded comments and no summary, unlike the empty-description reference task. The no-summary header padding is the genuine source-backed correction above. The remaining content differences do not justify fixed heights or moving the composer over comments. |
| Task ID | 1 | 1 | `src/components/PageComponents/TaskDetail/TaskInfoColumn/TaskInfo.tsx:288-307` renders the actual ticket. Dense, sparse and reference use different strings. Reference content matches the captured ticket width. |
| Task comment 0, author and body | 3 | 0 | `src/components/PageComponents/TaskDetail/CommentAndDescription/index.tsx:208-220` renders actual comments. Dense has comments; the reference task does not. Removing them would lose content. |
| Total | 44 | 10 | No differences discarded. |

`KanbanTaskCard.tsx`, `TaskTopRow.tsx` and `Homepage.tsx` in abbreviated cells retain the full directory named in their previous cells. All comparisons and source-specific inferences remain separate from proof of authenticated web actions.

## Verification

Hosted red proof is https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40246073718 at `6e48862abcaecc242389f025f7c1ec3ded841c23`. Ubuntu, macOS and Windows all reject the same native regression. Each reports 203 native tests passed and one failed. The summary-free short-title header measures 81px instead of the source-backed 89px.

The exact-head final CI receipt and downloaded artifact counts belong to https://github.com/hypertask-ai/hypertask-desktop/pull/35. No local build, test, lint, app launch, or laptop access occurs. The existing CI layout comparisons remain unchanged. The final report retains dense, sparse and reference totals, even if the genuine spacing correction does not reduce the content-dependent totals.
