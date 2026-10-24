# HTPR-7381 shell parity ledger

Full-sign-in client follow-up: see [prewire-parity.md](prewire-parity.md) for
newly wired portions and native gaps; historical auth blockers below are not
claims that server approval alone completes these rows.

The shell is in `crates/ht-desktop/src/shell*.rs`; `ui.rs` only integrates it.
Executable proof is GitHub CI, including GPUI mouse/key tests and the existing
xvfb layout gate. No builds, tests, app launches or Windows-laptop access were
used on the VPS. The draft PR must not be treated as whole-app parity.

## Implemented paths

- 48px/130px rail, persisted expansion, chevron/`[`, content/board width updates.
- Boards flyout, keyboard/mouse selection, search, Escape/outside dismissal,
  remembered boards; Recent ordering. Team/favorite metadata is blocked below.
- Inbox/Board/Table entry points; Table's read-only ticket/title shell is present,
  not table-view operations. Search is a full-page cached search with board
  context, not the web's server search engine.
- AI toggle icon, `5`, `]`, Ctrl+Shift+/, consumed-Shift Ctrl+?, and reopen arrow.
- Rail tooltips, active states and the inbox unseen indicator.
- Agents uses the web's existing `Unauthorized` state, not a fabricated register
  built from MCP's incompatible identity summaries.
- Invite supports comma-separated email/Enter through the bearer board-members
  endpoint. Cookie-only member/pending/share-link entries are not fabricated.
- Feedback opens the web form and its existing submission-error state; it never
  files a user message through the unrelated MCP agent-diagnostics endpoint.
- Settings: profile name/Enter/blur, appearance/follow-system, persisted Calendar week-start/weekends, board instructions,
  board-member names, file downloads, user/board Markdown skills including
  preview/select/import and delete confirmation, selected-board webhook
  list/create/secret-copy/test/delete, owner board-archive confirmation, MCP
  client instructions, CLI/API copy controls, shortcut registry/filter, Help
  links and persisted Quick Tips. Settings close preserves the workspace route.
- Async responses are matched to request identity/section/board. Writes are
  single-flight. Account changes clear shell caches and one-time secrets.

The source-derived desktop shortcut registry contains the unflagged desktop
entries, with separate macOS key labels (including literal CTRL where the web
uses CTRL on macOS). Feature-flagged Material/composer variants are not assumed
on when the app cannot read that flag for a human bearer.

## Authentication and response-contract blockers

Paths below are web paths relative to `hypertask/src`. The auth helper itself is
part of the evidence: a route under `/api/mcp` is **not** automatically usable by
a personal MCP bearer. Cookie sessions are not copied, forged or exchanged.

| Ref | Exact route(s) and auth boundary | Consequence |
| --- | --- | --- |
| B1 | `GET/POST /api/users/preferences`, `app/api/users/preferences/route.ts:60-75,117-132`: `loadCurrentUser` -> `getSessionUser` (`lib/auth/currentUser.ts:17`), otherwise `getCurrentUserFromCookies`. Neither accepts MCP bearer. | Task-page/inbox/notifications/default-model/dictation/announcement preferences cannot be read or saved. Local fake preferences would not be durable web parity. |
| B2 | `GET /api/teams/getAllSidebar`, `pages/api/teams/getAllSidebar.ts:8`; `GET /api/teams/getTeam`, `pages/api/teams/getTeam.ts:11`: `verifySession(req.cookies[SESSION_COOKIE])`. `GET /api/members/getAll`, `pages/api/members/getAll.ts:22`: `getSessionUser`. | Team grouping/pickers, team membership/roles/seats and team-scoped settings lack authenticated data. MCP `/user/context` supplies team names, but MCP project listings omit `teamId`; guessing a board/team association is unsafe. |
| B3 | `GET/POST /api/users/announcements/getUserAnnouncements`, `pages/api/users/announcements/getUserAnnouncements.ts:14`: `verifySession`. Mute is B1. | News/read-state/announcement-detail features are blocked. The web has no explicit announcement-unavailable message, so native does not invent one or show fake updates. |
| B4 | `GET /api/agents/owned`, `app/api/agents/owned/route.ts:54-64`: `getSessionUser`; `/api/agents`, `/api/agents/all`, `/api/members/addAgent` use the web session routes. | Register/runtime/team/model/provider/unread data and add-existing picker team classification are unavailable. `/api/mcp/agents` only exposes `OwnedAgent` (`lib/mcp/agents/ownedAgents.ts:176-185`), not the register model. Native shows the existing register auth error (`app/agents/AgentsRegister.tsx:421-441,569-570`). |
| B5 | `POST /api/feedback`, `app/api/feedback/route.ts:120`: `getSessionUser`; uploads use `/api/tasks/uploadUrl`, `/api/tasks/uploadFinalize`, `/api/tasks/n8nUpload` (`lib/storage/uploadViaApi.ts:130,170,210`). | User-feedback delivery/attachments require a cookie session. The error text comes from `components/Modals/Feedback/FeedbackForm.tsx:72-75`. `/api/mcp/feedback` calls `fileAiFeedback` with a different diagnostic schema and is not a substitute. |
| B6 | `GET /api/calendar/read-model`, `app/api/calendar/read-model/route.ts:20`: `loadCurrentUser(..., true)` -> `getSessionUser`; Google Calendar connection/OAuth routes also require a session. | Calendar destination/account connection cannot be hydrated by the app bearer. Local Calendar week-start/weekends are implemented independently (`components/Modals/Settings/CalendarSection.tsx:320-364`, `models/Calendar/model.ts:73-76`, `store/index.ts:696-700`). Only the destination/read-model and Google connection/sync are blocked; no calendar view implementation is claimed. |
| B7 | `GET /api/settings/billing`, `app/api/settings/billing/route.ts:29`; `POST /api/stripe/billing-portal`, `app/api/stripe/billing-portal/route.ts:10`: `getServerCookieUser`. `/api/stripe/ensure-customer`, `/api/stripe/cancelSubscription` are cookie-authenticated Stripe flows. | Billing/plans/payment/invoice/subscription settings are blocked, not replaced with guessed prices or external checkout UI. |
| B8 | `/api/settings/ai-usage`, `/api/settings/ai-usage/personal`, `/api/settings/ai-usage/personal/all`, `/api/settings/team-usage`: `getServerCookieUser`; `/api/teams/aiProviderSettings` (`pages/api/teams/aiProviderSettings.ts:63`), `/api/teams/aiFeatureModels` (`pages/api/teams/aiFeatureModels.ts:125`), `/api/teams/byokKeys` (`pages/api/teams/byokKeys.ts:58`): `getSessionUser`; `/api/settings/byok-test` (`app/api/settings/byok-test/route.ts:35`): `getServerCookieUser`. | Team/personal allowance, rankings, BYOK and team model policies cannot be read/written with the app bearer. |
| B9 | `/api/projects/update`: `taskWriteHandler`/`taskWriteRoute` -> `getSessionUser` (`lib/api/task-writes/route.ts:33,90`); `/api/notifications/project-mute` (`app/api/notifications/project-mute/route.ts:19`), `/api/projects/time-tracking` (`app/api/projects/time-tracking/route.ts:12`), `/api/sections/auto-assign` (`app/api/sections/auto-assign/route.ts:10`): `getSessionUser`. `/api/projects/planning`, `app/api/projects/planning/route.ts:88,106`: `getSessionUser`; `/api/projects/staleness`, `app/api/projects/staleness/route.ts:7`: `getSessionUser`; `/api/projects/auto-archive` likewise. | Prefix, board mute/time tracking, planning, milestones/health/staleness/auto-archive controls are blocked. MCP project PATCH only renames (`lib/mcp/operations/projects/[projectId]/operation.ts:31-43`), so it cannot save those fields. The conditional Timers rail entry also lacks the web's `timeTrackingEnabled` board metadata. |
| B10 | `/api/ai/project/memory`, `app/api/ai/project/memory/route.ts:89`: `getCurrentUserFromCookies`; RAG file deletion `/api/ai/custom-instructions/delete-file-by-source` (`lib/constants/APIRouteConstants.ts:78-79` with `/api` base in `utils/axiosClient.ts:51`): `getCurrentUserFromCookies` (`app/api/ai/custom-instructions/delete-file-by-source/route.ts:21`); upload routes use web-session storage auth. | Learned memory cannot be listed/toggled/deleted. Instruction GET includes attachments, so downloads are available; upload/delete are not. |
| B11 | `/api/users/developer-access`, `app/api/users/developer-access/route.ts:23`; `/api/users/api-keys`: `getApiKeyOwnerFromCookies`. `/api/mcp/admin/keys`, `app/api/mcp/admin/keys/route.ts:51,120,315`: `validateManagementOrSessionAuth`. | REST developer keys and management keys require cookie or management-key credentials. A personal MCP bearer cannot create elevated credentials. |
| B12 | `GET/POST /api/mcp/token`, `app/api/mcp/token/route.ts:18-38,46,109`: `getCurrentUserFromCookies`/`verifySession`; connected-client actions and `/api/connections/revoke-all` use web sessions. | Current web token generation/renewal/client list/revoke-all are blocked. DELETE `/api/mcp/token` can revoke the presented bearer, but that is **not** the web's revoke-all action and is not silently substituted. Setup commands can use the explicitly selected app bearer without exposing it in layout dumps. |
| B13 | `/api/settings/webhooks`, `app/api/settings/webhooks/route.ts:39`: `getSessionUser`, plus trusted-origin validation for writes. | Bearer `/api/mcp/webhooks` supports selected-board list/create/delete/test, but not the web's team-wide endpoint list, active toggle, workspace-secret rotation or delivery history/retry. Those controls are not fabricated. |
| B14 | `/api/slack/installation`: `getSessionUser`/`getServerCookieUser` (`app/api/slack/installation/route.ts:44,101`); `/api/figma/connection`, `/api/figma/disconnect`, `/api/figma/oauth/start`: `getFigmaAuthenticatedUser` -> `getSessionUser` (`app/api/figma/_lib.ts:25`); Better Auth passkey/account endpoints under `/api/auth/[...all]` require browser authentication/WebAuthn. | Slack, Figma, passkeys, multi-account cookie switching and session-wide signout are blocked. |
| B15 | `/api/users/delete-account` uses `getSessionUser`; avatar upload requires B5 storage routes. `PATCH /api/mcp/user/profile` accepts display name/photo URL but `/user/context` does not return the current photo. | Display-name read/save is implemented; avatar upload/current-photo and delete-account are not represented by fake controls. |
| B16 | `GET /api/flags` requires a web session. `/api/mcp/flags`, `app/api/mcp/flags/route.ts:64-69`, requires an owner agent credential, not a personal MCP bearer. | Feature-flagged Material, agent role screens, add-existing picker, board deletion and other gated variants cannot be assumed enabled. |
| B17 | Web favorite reads/reordering use `/api/favorites/getFavorites`: `verifySession` (`pages/api/favorites/getFavorites.ts:10`); `/api/favorites/addEditFavorites`, `/api/favorites/reorderFavorites`: `getSessionUser` (`pages/api/favorites/addEditFavorites.ts:20`, `pages/api/favorites/reorderFavorites.ts:12`). | Favorites membership/ranking cannot be invented as a separate local-only favorite list. |
| B18 | Invite's `/api/members/getAll?projectId=...&teamMembers=true&teamId=...` uses B2; join-link reads/resets use `/api/invite/generatePublicInvite` GET/POST: `getSessionUser` (`pages/api/invite/generatePublicInvite.ts`). | Email invites are bearer-backed; member removal, pending/resend, team-member suggestions and join/read-only links are blocked. |

## Important partial rows (not Done)

- F421/F487: local week-start/weekends work and persist; Google connection/sync is B6.
- F305/F440: name works; current avatar/upload/removal section parity is incomplete (B15).
- F321/F463: owner Archive with confirmation works; archived-team listing/restore,
  flagged delete and leave are not complete (B2/B9/B16). Do not mark lifecycle Done.
- F323/F324: MCP member names are readable, but the MCP response drops member
  roles/photo/revocation metadata. Do not label all members Admin/Member by guess.
- F329/F479/F480: MCP connection instructions work; web token/client management is B12.
- F327/F470: instruction attachments are readable/downloadable; mutation is B10.
- F395/F411/F412 are distinct from F413/F414/F415: flyout and selection work;
  team/favorite grouping/dragging are B2/B17. Recent is local like the web.
- F401: shell Search now opens a page and preserves board context. Results are
  from the authenticated local cache; complete web search/chips/server search is
  not claimed by this PR.
- F404/F446/F473: email invitations work; all other member/agent management
  affordances require B2/B4/B18 or additional native implementation.
- F428/F475/F476: selected-board webhook operations are implemented; the rest is B13.
- F432/F435/F436: supported tabs/board picker preserve board selection, but
  authenticated team scoping is B2. No team id is guessed.
- F643: Follow system/four existing themes work. Material remains flag-blocked (B16).

Guest-only/mobile-only surfaces are not added to a bearer-only desktop session.
AI conversation internals (resize/pin/floating mode/model/context/attachment/
message actions, F606-F623 and F644-F655) are not completed by shell toggle work;
existing implementations/statuses are retained, not relabeled as shell parity.

The uncommitted `internal/FEATURES.md` records only rows with final SHA-bound CI
proof as Done. A partial row remains partial even when its implemented subset
has passing tests. This ledger is deliberately not a claim that the entire
HTPR-7381 inventory is finished.

## shell2 reconciliation before implementation

Reconciled the supplied 674-row ignored inventory against origin/main `cf763ef`,
this ledger, `docs/ai-parity.md`, `docs/compose-comments.md`, and the supplied
ignored `internal/SHELL_STATUS.md` (which is not present on origin/main).
Initial counts, in done/partial/missing/blocked order: P1 **60/32/74/176**;
all priorities **106/56/336/176**. Blocked is a primary inventory status now.
A partial implementation is not proof of its full row; existing receipts and
original audit totals are historical, not shell2 executable evidence.

Additional exact boundaries found while reconciling the remaining P1 rows:

| Ref | Exact route(s) and auth boundary | Consequence |
| --- | --- | --- |
| B19 | `POST /api/ai/audio-transcript`, `app/api/ai/audio-transcript/route.ts:98-101`: `getSessionUser(request.headers)`. Both ordinary and improved dictation use this route (`improve` field). | F117/F118 cannot transcribe with the app's MCP bearer; no local provider credentials or fake microphone result. |
| B20 | `POST /api/tasks/uploadUrl`, `POST /api/tasks/uploadFinalize`, `pages/api/tasks/uploadUrl.ts:1-5`, `uploadFinalize.ts:1-5`: `taskWriteHandler` -> `getSessionUser`, `lib/api/task-writes/route.ts:89`. Fallback `POST /api/tasks/n8nUpload`, `pages/api/tasks/n8nUpload.ts:102-108`: `getSessionUser`. | F125's task-editor attachment upload contract is session-only; no pretend upload or substituted agent attachment contract. |
| B21 | `POST /api/tours/update`, `pages/api/tours/update.ts:15-18`: `verifySession(req.cookies[SESSION_COOKIE])`; `lib/tours/hooks/useTour.tsx:130-133` and `useTourStatus.ts:67-74` persist start/completion/skip here. Onboarding/welcome requires board/team context from B2 (`components/generalCommandActions.ts:101-108`, `commandDispatcher.ts:610-614`). | F336/F339/F340/F341 remain blocked, including durable tutorial status and real task-writer prerequisites; no fake completed local tour. |
| B22 | `POST /api/search/document`, `pages/api/search/document.ts:30-37`: `getSessionUser`. Web search chips/project scopes feed this route (`hooks/Search/useSearch.ts:151,310`). | F401 retains contextual cached search, but full search is blocked. MCP task search is not the mixed web document/search-chip response contract. |
| B23 | Legacy shell's team/favorite read contracts are B2/B17 (`components/sidebars/leftSidebar.tsx:28-37,91-92` and `components/sidebars/RightSidebar/index.tsx`); `commandDispatcher.ts:490-493` switches the entire shell, not rail expansion. | F226 cannot reproduce the old authenticated shell with absent board/team/favorite metadata. Do not mislabel `[` expansion as old/new design parity. |

F033 remains partial with B2/B17 team/favorite gaps; F010 remains partial with
unavailable command contracts (B1–B23 and AI A1–A9), not a full command-set claim.
F435's current web picker (`components/Modals/Settings/SettingsBoardPicker.tsx:16-24,73-111`)
has no text search field: its unmet contract is authenticated team filtering and
synchronization (B2), not a missing native search input. No invented input added.
F438 hard reset includes browser-account-wide logout, FCM device removal and
`POST /api/auth/force-signout` (`hooks/MultiPages/HTC/useSignout.ts:36-83`,
`app/api/auth/force-signout/route.ts:9-10`: cookie-token `revokeSessionToken`).
Clearing queued native edits alone would not implement that contract.

## shell2 implemented entry points and queue discipline

- P1 F197/F210 palette entry points open the existing Feedback/Invite forms;
  unavailable feedback delivery and full membership administration remain B5/B18.
- F252/F304 open Calendar/Learn without changing their values. The web command
  payload selects a section, not a control focus target (`AllCommands.ts:630-635,994-999`).
- F334 opens Board settings; F335 opens the command's exact help URL
  `https://hypertask.ai/help` (not the different Learn-resource URL);
  F337 toggles/persists Quick Tips. F343 is the web's CLI **modal**, with
  install/sign-in copy, docs, Close/Escape/outside dismissal, not a redirect
  mislabeled as a modal. Its MCP link opens the already-supported instructions;
  token/client management is still B12.
- F434 now supports `--screen settings` direct entry. In-session settings never
  change the underlying workspace/search/table/task context; direct close uses
  the remembered cached board or Inbox/home when absent/invalid.
- Supported section commands and keyboard-shortcut help reuse the existing
  settings/shortcut components. No cookie-only per-control commands are shown.
- All settings/invite writes now enter `Db::enqueue(Write::Shell)` rather than
  starting an HTTP write thread. `ShellWrite` only accepts the already-supported
  personal-bearer MCP method/path pairs. Queue draining checks `/user/context`
  against the frozen account id before any mutation, preserving idempotency,
  retries, restart durability and permanent errors. Successful acknowledgements
  are consumed once by the original form; no server success is inferred from
  enqueue. One-time webhook receipts are removed when consumed, never logged.
  Existing GPUI form fixtures now drain the actual queue before showing success.

Bearer route evidence for the allowlist (relative to web `src`):
`lib/mcp/operations/user/profile/operation.ts:40-47`,
`lib/mcp/operations/skills/operation.ts:71-87`,
`lib/mcp/operations/skills/[skill_id]/operation.ts:89-94`,
`lib/mcp/operations/skills/import/operation.ts:24-28`,
`lib/mcp/operations/projects/[projectId]/members/operation.ts:109-120`,
`lib/mcp/operations/projects/[projectId]/instructions/operation.ts:63-70`,
`lib/mcp/operations/projects/archive/operation.ts:20-23,88`,
`lib/mcp/operations/webhooks/operation.ts:150-154,293-294`;
these use `validateMcpRouteAuth` (aliased as `validateMcpAuth`), not web cookies.

| Ref | Exact route(s) and auth boundary | Consequence |
| --- | --- | --- |
| B24 | `POST /api/comments/deleteCommentById`, `pages/api/comments/deleteCommentById.ts:13-18,28-36`: `getSessionUser`; bearer `DELETE /api/mcp/comments/[id]`, `lib/mcp/operations/comments/[comment_id]/operation.ts:460-474`, has stricter owner/HyperAI permission. | F112 retains real queued own-comment delete; broader web author permissions are blocked. |
| B25 | `POST /api/savedContent/toggleSaved`, `pages/api/savedContent/toggleSaved.ts:14-18`: `getSessionUser`. | F115/F116 comment star/pin have no equivalent human-bearer mutation; no local fake pin/star. |
| B26 | `POST /api/ai/task-writer`, `app/api/ai/task-writer/route.ts:41-44`: `getAiRequestUser` -> `hasOAuthAccessTokenClaims` (`app/api/ai/_lib/requestUser.ts:28`); creation defaults `GET /api/tasks/createGlobally` use `taskWriteHandler` -> `getSessionUser` (`lib/api/task-writes/route.ts:89`). | F001/F002/F003/F011 web compose remains blocked; the different MCP JSON-draft writer is not substituted. |

The final inventory retains full-row blockers as the primary `blocked` status,
with the implemented subset described alongside it. This includes F010/F033,
F112, F401/F403/F404, F432/F435, F463/F470/F475/F479/F482/F487/F643;
relabeling them does not remove the existing working client behavior.
Nine newly implemented P1 rows await shell2 CI before inventory promotion:
F197, F210, F252, F304, F334, F335, F337, F343, F434.

## p2 audit: remaining P1 inventory reconciliation

The ignored inventory still labels these full-row blockers Partial/Missing:
F240 (B15 profile photo upload/current-photo), F241 (B14 cookie-account switching),
F305 (B15), F321 (B2/B9/B16), F323 (B2 role/photo contract), F327 (B10),
F329/F331 (B12), F414 (B2), F416 (B2/B14 and blocked board/team creation),
F421 (B6), F428 (B13). These are not new implementation candidates.
Profile PATCH accepting a URL is not the web upload/remove/current-photo flow.
The exact switch-account routes are Better Auth
`/api/auth/multi-session/list-device-sessions` and
`/api/auth/multi-session/set-active`, using the **multiSession** browser-session
plugin (`src/lib/auth/accounts.ts:17-35`, `src/lib/auth/betterAuthClient.ts`),
served by `/api/auth/[...all]`; the MCP bearer is not a session token.

F649's supported-command token/typo ranking and limit already exist in
`command_center.rs`; the web renders an empty list, not an invented "No commands"
widget (`src/components/Modals/commands/HTC/CommandGroup.tsx:20-40`). F650's focused
comment commands and persisted frecency also exist on main. Unsupported auth-bound
commands remain excluded, not inert results. These two rows describe supported
command behavior, not completion of every command in the corpus.
