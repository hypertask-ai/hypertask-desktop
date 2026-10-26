# Visual round 3 capture content

Separate visual sample; dense demo and heavy memory workloads stay unchanged.
Derived from the existing approved layout-reference fixture. Same synthetic
IDs, three columns (Todo, Doing, Done), two tasks, and zero comments.

Content aligns the read-only live-graphite-1920 reference:

- Board title QA Team Board; cards QABO-7 and QABO-5.
- Task 103 uses the frozen QABO-5 title, empty description, QA owner author,
  October 13 creation date, The Assignees assignment, and visible summary text.
- Two notifications populate Important 1, Agents 1, MyBoard 1, QA Team Board 1,
  All 2. Only the QABO-5 reminder belongs to the selected Important split.
- No extra board columns, card labels, comments, or Important rows are hidden.
  They do not exist in this separate synthetic sample.

The old and new executables both capture this exact same fixture in CI.
Existing dense memory captures remain available separately, without workload
or budget changes. Fixture-aligned reductions must not be described as product
layout fixes. AI preservation is measured within each identical-data pair,
never between different account content.
