# HTPR-7381 continuation contracts

Runtime proof is GitHub CI only. The uncommitted `FEATURES.md` and
`.unlazy/prewire/7381-gates.md` retain the current candidate and exact evidence.
A cancelled run, an ARM64-only compile or a prior-SHA green run is not proof of
this continuation. No calendar, table or time-report surface was added.

## Implemented portions

| Web behavior | Native entry | Authenticated write contract |
| --- | --- | --- |
| Add / edit / delete smart split | Ctrl+K Add smart split; Manage views detects paired and unambiguous legacy smart tags | POST/PATCH/DELETE `/api/projects/views/smart-split`, web `projectId`, optional `viewId`, trimmed `name` and `prompt`; raw prompt UTF-16 limit 1,000 |
| Manage columns / rename / delete commands | Web column manager, edit panel and confirmed delete | POST `/api/section/update`, `userId`, `sectionId`, `newSection`; deletion alone accepts empty HTTP 204 |
| Add column | Web Add Column form | POST `/api/section/create`, `projectId`, `title`; replace provisional ID only from acknowledged section and project view |
| Drag column order | Manager drag handle | Section ranking update followed by queued `/api/projects/views/unsaved-view` with reordered settings |
| Visibility in all views | Edit column Show/Hide in all views | POST `/api/sections/view-visibility`, `sectionId`, `visible`; canonical section visibility is not changed |
| Auto-assign | Edit column picker or board-only Ctrl+K column chooser | POST `/api/sections/auto-assign`, `sectionId`, mutually exclusive nullable `autoAssignUserId` / `autoAssignAgentId` |
| Task timer | Task Time Start/Stop, W, task-only Ctrl+K | GET `/api/time/task?taskId=…`, POST `/api/time/start` or `/api/time/stop`, `{taskId}` |
| Board tracking | Board/task Ctrl+K, dynamic on/off label | POST `/api/projects/time-tracking`, `{projectId, enabled}`; disabling also clears `showTimeTotals` and refusal restores both |

Every write is queued, verifies `/user/context`, sends the desktop bearer to
the web path through `Http`, applies optimistic state and rolls back refusals.
Smart-split and column creates and timer writes never replay an uncertain
acknowledgement. Error messages retain server errors or the web's action-specific
fallbacks; no client-only success toast substitutes for an acknowledgement.
Column control intents serialize behind dirty-name saves. Rollback preserves
unrelated task fields and later acknowledged moves.

CI cases are in `crates/ht-core/tests/{smart_splits,board_columns,task_time,http_cli}.rs`
and native tests alongside `board_views.rs` / `task_time.rs`. They cover success,
401/403, exact routes/bodies, actual bearer headers, restart, provisional IDs,
form text, context gating and refusal rollback. These are CI definitions, not a
claim that they passed before the final receipt is recorded.

## Open families, not silently counted as implemented

Web declarations are in `src/components/Modals/commands/HTC/AllCommands.ts`,
`navigationCommands.ts`, and `taskCommands.ts` in the web repository. Static source
inspection found 179, 52 and 66 unique literal keys respectively (per file; not a
sum of distinct global commands). Visibility/flag/context branches and dynamic
commands mean these counts are not an execution-coverage denominator.

- Task: start date, recurrence, templates, relations/dependencies/duplicate-of,
  blocked-by-person, assign-to-self and complete follow/unfollow flows.
- Board/team: remaining administration, labels/templates/custom fields,
  staleness/auto-archive preferences and feature-specific settings/eligibility.
- Inbox/navigation/preferences: remaining actual destination effects and settings,
  not just searchable command names; maintain web context and flag gating.
- Attachment/writer/routing: staged upload/finalization, empty-ticket/writer flags,
  typed-board routing and source-comment/parent flows.
- Dates/time: full Sugar parsing, task start dates, complete task time-log CRUD,
  realtime time events and board timer-filter refresh. Active task timer display
  ticks every second and refreshes its summary every thirty seconds; this is not
  advertised as web realtime parity. No unwired Log time button was added.

Calendar/table/report/tutorial/agent-side conversation surfaces are not native
completion requirements for this bounded continuation. Existing AI sidebar
behavior and all previous mocked cases remain regression requirements.
