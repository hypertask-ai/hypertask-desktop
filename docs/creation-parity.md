# Creation defaults and remaining authorization boundary

Source root `/home/valentin/projects/hypertask/src`, read-only revision
`10377b20e8c338357fda306b025ef16699c9b4ab`. These are runtime rules, not
shortcut-help labels. No authenticated server request was made for this audit.

## Quick entry is not AI compose

| Step | Exact web rule | Source relative to source root |
| --- | --- | --- |
| Shortcut | Plain N or Shift+C; no Ctrl/Meta/Alt, repeat, composition, open commands, modal or input target | `hooks/Homepage/useHandleKeyDownOperations.ts:98-107` |
| Target | Focused column first, otherwise the selected task's filtered column, otherwise column zero | `hooks/Homepage/useHandleKeyDownOperations.ts:108-120` |
| Position | OPEN_QUICK_ENTRY selects bottom and opens inline entry | `hooks/Homepage/useSections.ts:108-115` |
| Item | Title from entry, empty description, temporary ID -1 | `hooks/Homepage/useSections.ts:118-132` |
| Active filters | First personal view; unsaved board_filters, then applied, then board default, then defaultFilterSettings. Nullish precedence preserves an explicitly empty filter | `utils/helperFunctions/Views/ViewsHelperFunctions.ts:203-210` |
| Filter value selection | First addedFilters entry of each type. Only absent match, ANY or ALL; searchPayload must be an array | `utils/helperFunctions/Views/NewTaskViewDefaults.ts:7-12` |
| Tags | Label IDs must be UUID strings | `utils/helperFunctions/Views/NewTaskViewDefaults.ts:5,15` |
| People | A uid property identifies a person; ID must be a positive safe integer | `utils/helperFunctions/Views/NewTaskViewDefaults.ts:16-19` |
| Agents | Without uid, ID must be UUID and revokedAt must be false/absent | `utils/helperFunctions/Views/NewTaskViewDefaults.ts:16-20` |
| Priority and size | First payload value matching a positive index in the authoritative constant catalog, not simply the first payload value | `utils/helperFunctions/Views/NewTaskViewDefaults.ts:21-26` |
| Rank | Generate from first or last item of the current section, depending on top/bottom | `hooks/MultiPages/useAddDeleteTaskInBoards.tsx:118-124` |
| Empty section rank | A0100 only when both neighboring ranks are undefined | `utils/generateRank.ts:12-16` |
| Bottom rank | Previous numeric rank plus 90, 15 decimal places, padded to 20, retaining prefix | `utils/generateRank.ts:28-33` |
| Top rank | Random fractional value below the next numeric rank, with the same precision/padding | `utils/generateRank.ts:2-9,19-24` |
| Nonempty defaults | Any tags, assignees, priority or size selects createGlobally; requires currentUser.id | `hooks/MultiPages/useAddDeleteTaskInBoards.tsx:127-148` |
| Priority-sort exception | Only top entry with existing cards defaults to priority index 1 when no view priority | `hooks/MultiPages/useAddDeleteTaskInBoards.tsx:137-140` |
| Empty defaults | POST /api/tasks/create with assignees [], current user, exact rank and index. Index is zero for Priority/top; otherwise section length | `hooks/MultiPages/useAddDeleteTaskInBoards.tsx:169-179` |
| Successful save | Without create-another, close entry and restore column focus. Failed save leaves entry intact | `hooks/Homepage/useSections.ts:134-141` |

## Full create defaults are different

`hooks/MultiPages/Tasks/useCreateTaskModalStates.ts:183-234` supplies the full
form. A cross-board column payload cannot inherit the current board's filters.
Duplicate title/description override prefills. Nonduplicate assignees prefer
explicit column values, then view defaults; duplicates start unassigned.
Priority/size prefer duplicate, then column, then nonduplicate view defaults.
Duplicate labels override view tags. Duplicates retain their section at top.
An explicit empty assignee array is an override, not missing data.

On board changes, already edited fields remain owned by the user. Unedited
assignees/tags retain nonempty values only on the same board. Unedited priority
and size reset to the new board's defaults. Status clears.
Source `hooks/MultiPages/Tasks/useCreateTaskModalStates.ts:465-511`.

## Why N and Shift+C remain blocked

This is not an unknown-defaults excuse. The rules above are known. The native
cache lacks the complete authoritative personal active view, unsaved filters,
filtered/sorted section item order and all assignable catalogs. Most importantly,
the exact two save routes still require web session authorization:

- POST `/api/tasks/createGlobally` loads `create-global.POST` via
  `pages/api/tasks/createGlobally.ts:113-115`.
- POST `/api/tasks/create` uses `taskWriteRoute` in
  `lib/api/task-writes/create.ts:20-24`.
- `lib/api/task-writes/route.ts:35-40,101-106` authenticates both with
  `getSessionUser`, without the desktop CLI endpoint adapter.
- GET defaults alone accepts the gated human CLI bearer via
  `pages/api/tasks/createGlobally.ts:30-32`. It returns section/rank, not view
  defaults (`:91-95`). Its fallback to another section (`:47-67`) cannot silently
  replace the focused quick-entry column.
- HTPR-7699 currently permits only task-writer, task-defaults, comment-star-pin
  and mention-search in `lib/auth/desktopCliTokenUser.ts:7,53-77`.
- MCP create passes section, scalar priority/size, labels and human assignees,
  but no caller-supplied rank or agent assignment into createTask
  (`lib/mcp/operations/tasks/create/operation.ts:250-268`). It cannot substitute
  for the source's exact ordered, mixed-assignee create request.

N, Shift+C, C and the new-task save chords therefore remain B3/B11 blocked
pending the server sign-in work under HTPR-7699 and authoritative active-view
hydration. No placeholder quick-entry UI, AI substitution, guessed default,
new transport or misleading successful save is introduced. Existing AI compose
retains its separately documented append-only limitation.
