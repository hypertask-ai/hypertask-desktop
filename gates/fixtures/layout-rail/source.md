# Rail source projection

Authority is `/home/valentin/projects/hypertask/src` at revision
`10377b20e8c338357fda306b025ef16699c9b4ab`. Paths below are relative to that
source directory. No native measurements supply expected values.

| Projection | Web authority |
| --- | --- |
| Tailwind reset, inherited 16px/400/1.5, zero button padding, transparent button background, default border color | `styles/globals.scss:10` imports `styles/tailwind-entry.css:7-11,37-48`; frozen first-five records independently validate these computed fields |
| Item flex centering, 34px height, 36px width, 5px wrapper radius, hover background | `components/PageComponents/Kanban/HeaderComponents/AppShellRail.tsx:65-87` |
| Fixed origin, 100svh, 48px width, border-right 1.3px, vertical padding 8px | `components/PageComponents/Kanban/HeaderComponents/AppShellRail.tsx:151-154` |
| Toggle 20px height, bottom margin 4px, 16px SVG | `components/PageComponents/Kanban/HeaderComponents/AppShellRail.tsx:156-166` |
| Boards group and rounded child | `components/PageComponents/Kanban/HeaderComponents/AppShellRail.tsx:175-189` |
| Navigation `mt-3`, gap 4px; Bootstrap's important `mt-3` wins at 1rem | `components/PageComponents/Kanban/HeaderComponents/AppShellRail.tsx:191`; `styles/tailwind-entry.css:7-8` imports Bootstrap in the earlier cascade layer, which wins for important utilities |
| Inbox/Board selected foreground and background | `components/PageComponents/Kanban/HeaderComponents/AppShellRail.tsx:195-199,219-221` |
| Rounded Table, Calendar, AI and Search children | `components/PageComponents/Kanban/HeaderComponents/AppShellRail.tsx:234,243,256,266,276` |
| Optional timers excluded, matching frozen non-time-tracking fixture | `components/PageComponents/Kanban/HeaderComponents/AppShellRail.tsx:286-300` |
| Bottom auto margin, 4px gaps, bottom padding 4px | `components/PageComponents/Kanban/HeaderComponents/AppShellRail.tsx:303` |
| Rounded Agents child | `components/PageComponents/Kanban/HeaderComponents/AppShellRail.tsx:308` |
| Invite, News, Feedback and Settings children have no radius class | `components/PageComponents/Kanban/HeaderComponents/AppShellRail.tsx:321,333-335,347,356` |
| Unread announcement and guest states excluded | `components/PageComponents/Kanban/HeaderComponents/AppShellRail.tsx:334,362` |
| Control glyph 18px | `components/PageComponents/Kanban/HeaderComponents/AppShellRail.tsx:313,324,339,351,357` |
| Background #1a1d21, hover #3a3d45, foreground #ffffff, muted #8e9093, border #3a3f47 | `styles/tailwindThemes/graphite.css:14,18,53-55,145-147` |

The projection retains layout-affecting DOM groups and control styles. Empty
SVG boxes stand in for glyph paths. Tooltips are absent at idle. There is no
font-dependent text measurement in these fixed control boxes. This is a hosted
browser measurement of a source projection, not an authenticated app capture.

`layout-collect-rail.mjs` measures all visible buttons/links with the original
collector's computed-field semantics. The old collector's `slice(0, 5)` cap is
absent. The original frozen JSON and comparer remain untouched. CI validates
every previously captured rail field before appending the eight new records
per screen to `reference-complete-web`. Removing the bottom group must fail
coverage; shifting one native box by 4px must increase the unchanged comparer
count by one. Raw measurements, extended references and comparison logs upload
inside `layout-dumps`.
