# Round 12 acceptance and evidence

## Acceptance recorded before implementation

1. Add an additive Graphite source-projected browser measurement for AI sidebar welcome, suggestions, populated user/assistant messages, composer, and the `?` shortcut sheet. Cite every projected value to the read-only web source under `/home/valentin/projects/hypertask/src`. Do not invent UI or change existing references, tolerances, or budgets.
2. Collect native layouts through GitHub CI, using the real renderers and matched content. Retain complete per-element geometry/style differences and baseline counts in the `layout-dumps` artifact before product corrections.
3. Fix every source-proven difference, largest visible area first. Run the identical additive measure after fixes; preserve before and after CI artifacts and exact revision identities. Include missing-control and perturbed-box negative controls.
4. Preserve existing inbox, board, task, typography, keyboard, and memory checks. No VPS builds, tests, lint, app launches, test copies, or laptop commands. Only public GitHub CI supplies runtime proof.
5. Fetch/rebase main before each push. Deliver draft branch `look12` with exact-head required jobs green, downloaded before/after counts, last SHA, PR URL, CI URL and every job result.

## Work plan and shape

Use the existing layout-key JSON records (`key`, box, typography, colors) and Playwright source-projection approach. Add a separate panel comparison; existing frozen screens remain untouched. First deliver measurement-only baseline, then source-backed corrections, then exact-head evidence. Serial source/diff self-review replaces unavailable cursor-team-kit tooling; no independent review claimed.

Principle-prove-it-works requires hosted renderer measurements, not inferred parity. Principle-sequence-verifiable-units requires the baseline artifact before correction. Throughput checkpoint: keep collection separate from corrections; inspect complete counts before choosing edits.

## Baseline collector review

Before trusting a count, source review found GPUI's ordinary test context uses `NoopTextSystem`. The measurement fixtures now explicitly use the production Linux platform text system. This prevents fake glyph widths from masquerading as product defects. Added shortcut child keys use distinct prefixes so the existing row-search behavior contract remains unchanged. The composer projection includes source selector padding (4px vertical), not just its text height. These are collector corrections before baseline acceptance, not product styling changes.

The source cascade audit also found Bootstrap's important `pt-4`, `py-4`, `px-4`, `gap-4`, `mb-3` and bare `rounded` utilities. The web entrypoint imports them into the earlier layer (`src/styles/tailwind-entry.css:7-10`), so they win at 24px, 16px and 6px as documented in the projection authority. The token-only first projection is rejected before accepting a baseline. No native product correction has occurred.

## Accepted hosted baseline

Run https://github.com/hypertask-ai/hypertask-desktop/actions/runs/40267668089 at `8301e60f6924bc7543a243cdc605ff7ecb431798` completed the Linux job successfully. Its downloaded `layout-dumps/compare-panels.txt` reports welcome **41**, messages **46**, shortcuts **129**, total **216**. The projection hash is `7eb35baa37eb49601920706daf90fa149c119be2e0371b2b1645c3d1361baa98`. Coverage is 17 welcome + 16 message + 39 shortcut records, 72 total. Counts are raw box/style-property records, not 216 independently visible bugs; inherited SVG-parent text styles also contribute. Full precision records remain in `panel-comparison.json`.

The earlier token-only projection on run `40264773202` reported 208; it is retained as rejected collector evidence, not used for the before/after claim. Its native data is already production-font-backed. The accepted source projection adds the imported-cascade authority before any native styling correction.

## Source-backed corrections

- Shortcut sheet uses its own source background, search typography/padding and heading spacing rather than Settings presentation. Rows regain source 60/40 columns, label line height and individually styled monospace keycaps. Settings retains its existing presentation and search matching. Source `src/components/sidebars/keyboardShortcuts.tsx:21-30,128-179`, cascade import `src/styles/tailwind-entry.css:7-10`, Graphite `src/styles/tailwindThemes/graphite.css:25`.
- AI panel restores source outer/inner borders, header text/title/control grouping, 6px welcome chips, native paragraph bottom margins, full-width assistant bubbles, delivered-action spacing and source composer/input/control heights. Source `src/components/AI_CHAT/AI_Chat_Sidebar.tsx:237-273`, `ChatHeader.tsx:383-450`, `WelcomeScreen.tsx:351-364`, `MessageItem.tsx:46-95`, `AI_Tiptap_Container.tsx:261,298-335`; `src/styles/messageItem.scss:55-74`, `src/styles/tiptap.module.scss:45-53,200-204`.
- TextInput defaults remain 24px. Only AI composer (21px) and the shortcut sheet search (21.74px) request source-specific line height. Rich comment rendering keeps its existing spacing; AI explicitly requests the web message paragraph spacing. No response, remote write, menu or extra product surface is added.

Owned code is the five focused renderer/input files, additive measurement fixtures/gate and its CI step. Source-projection HTML, records, tolerances and prior screen references remain unchanged during product correction. Serial self-review checked input routing, Settings isolation and capture-key prefixes. No unavailable independent-review or screenshot-equivalence claim.
