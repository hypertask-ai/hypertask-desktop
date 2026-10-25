# Reference-content layout fixture

This separate fixture is approved under HTPR-7381. It leaves `fixtures/demo` and
`gates/fixtures/layout-graphite` unchanged. The three-column board, two cards,
notification badge, ticket strings, titles and QA author match the frozen live
1920x1009 measurement. IDs remain synthetic. The import contains no credentials,
comments or server requests. It is not evidence of SaaS authorization.

The detail task is the board's second card and the one notification targets it,
matching the live badge condition. CI dates that notification to the previous
Thursday, so the section stays LAST WEEK rather than depending on when CI runs.
All five tab counters retain the previous sparse fixture's cardinality.

`../layout-reference-live` contains only the original geometry/style fields,
screen identity and missing keys from the read-only live measurement. Selectors,
URLs, text and browser metadata are omitted. No compared field is changed.
The original reference is
`hypertask-desktop-wt-public/internal/live-graphite-1920/`.
Original file SHA-256 values are:

- inbox.json `daabd2c106f7b9158b10989143c31e71ec0dc2ad5d35b64875f169893da1cb2d`.
- board.json `b86b685de28390c61833bb17d175d65037ee96b749a6bd63f4994d6ea8b057c9`.
- task.json `9caf7f5426d17c5ac228601231ec14f683b2307577006dfc799635ebc6aef270`.

`gates/layout-compare.py` is a byte-identical copy of the supplied internal
comparer, SHA-256
`cde549a870edbcc146a7e40fc38b70a10496ffaad09edc3deda75b27454df06c`.
It compares every original geometry/style field with its original tolerances,
including all native-only rail keys. CI retains unfiltered output for dense,
sparse/parity and reference-content dumps. A lower reference-content count is
not a product improvement. Only before/after runs with the same fixture prove
product geometry improvements.

`regressions/ticket-space-before.json` retains the three failing native inbox records from
baseline CI 40145936859, before the trailing-space correction. Its run, revision,
artifact ID and digest identify the original `layout-dumps` artifact. CI must
reject each old record with the same box assertion used for the current dump.
The browser probe independently retains the web source's trailing space and
requires its collapsed width and gap to agree with native. Measurement records
stay in the subdirectory because the fixture importer applies every top-level
JSON file as an API response.
