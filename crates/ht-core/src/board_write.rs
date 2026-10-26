use crate::{Db, Result, TaskPatch, Write};
use anyhow::{Context, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BoardWrite {
    MoveToInbox { task: i64, project: i64 },
    WaitingOn { task: i64, user: Option<i64> },
    ToggleTimer { task: i64 },
    StarTask { task: i64, project: i64, active: bool },
    ArchiveBoard {
        project: i64,
    },
    AssignAgent {
        task: i64,
        agent: String,
        active: bool,
    },
    AssignToMe {
        task: i64,
        user: i64,
    },
    AddColumn {
        project: i64,
        title: String,
    },
    RenameColumn {
        project: i64,
        section: i64,
        title: String,
    },
    DeleteColumn {
        project: i64,
        section: i64,
    },
    RenameBoard {
        project: i64,
        title: String,
    },
    CreateTaskLabel {
        project: i64,
        task: i64,
        name: String,
    },
    MoveTask {
        task: i64,
        project: i64,
        section: i64,
    },
}
impl BoardWrite {
    pub(crate) fn request(&self) -> Result<(&'static str, String, Value)> {
        let title = |value: &str, max| {
            ensure!(
                !value.trim().is_empty() && value.chars().count() <= max,
                "name is empty or too long"
            );
            Ok::<(), anyhow::Error>(())
        };
        let positive = |id| {
            ensure!(id > 0, "invalid board, column or task id");
            Ok::<(), anyhow::Error>(())
        };
        let request = match self {
            Self::WaitingOn { task, user } => {
                positive(*task)?;
                if let Some(user) = user { positive(*user)?; }
                ("POST", "/tasks/waiting-on".into(), json!({"taskId":task,"userId":user}))
            }
            Self::ToggleTimer { task } => {
                positive(*task)?;
                ("POST", "/time/status".into(), json!({"task":task.to_string()}))
            }
            Self::MoveToInbox { task, project } => {
                positive(*task)?;
                positive(*project)?;
                ("POST", "/tasks/card-actions".into(), json!({"task_id":task,"project_id":project,"action":"inbox","active":true}))
            }
            Self::StarTask { task, project, active } => {
                positive(*task)?;
                positive(*project)?;
                ("POST", "/tasks/card-actions".into(), json!({"task_id":task,"project_id":project,"action":"star","active":active}))
            }
            Self::ArchiveBoard { project } => {
                positive(*project)?;
                (
                    "POST",
                    "/projects/archive".into(),
                    json!({"project_id":project,"status":"Archive"}),
                )
            }
            Self::AssignToMe { task, user } => {
                positive(*task)?;
                positive(*user)?;
                (
                    "POST",
                    "/assignees/assign".into(),
                    json!({"task_id":task,"user_id":user,"intent":"assign"}),
                )
            }
            Self::AssignAgent {
                task,
                agent,
                active,
            } => {
                positive(*task)?;
                ensure!(
                    !agent.is_empty()
                        && agent
                            .bytes()
                            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_')),
                    "invalid agent id"
                );
                (
                    "POST",
                    "/assignees/assign".into(),
                    json!({"task_id":task,"agent_id":agent,"intent":if *active {"assign"} else {"unassign"}}),
                )
            }
            Self::AddColumn {
                project,
                title: name,
            } => {
                positive(*project)?;
                title(name, 200)?;
                (
                    "POST",
                    format!("/projects/{project}/sections"),
                    json!({"title":name.trim()}),
                )
            }
            Self::RenameColumn {
                project,
                section,
                title: name,
            } => {
                positive(*project)?;
                positive(*section)?;
                title(name, 200)?;
                (
                    "PATCH",
                    format!("/projects/{project}/sections/{section}"),
                    json!({"title":name.trim()}),
                )
            }
            Self::DeleteColumn { project, section } => {
                positive(*project)?;
                positive(*section)?;
                (
                    "DELETE",
                    format!("/projects/{project}/sections/{section}"),
                    Value::Null,
                )
            }
            Self::RenameBoard {
                project,
                title: name,
            } => {
                positive(*project)?;
                title(name, 200)?;
                (
                    "PATCH",
                    format!("/projects/{project}"),
                    json!({"title":name.trim()}),
                )
            }
            Self::CreateTaskLabel {
                project,
                task,
                name,
            } => {
                positive(*project)?;
                positive(*task)?;
                title(name, 100)?;
                (
                    "POST",
                    format!("/projects/{project}/labels"),
                    json!({"name":name.trim()}),
                )
            }
            Self::MoveTask {
                task,
                project,
                section,
            } => {
                positive(*task)?;
                positive(*project)?;
                positive(*section)?;
                (
                    "POST",
                    "/tasks/move".into(),
                    json!({"task_id":task,"target_project_id":project,"target_section_id":section}),
                )
            }
        };
        Ok(request)
    }
    pub(crate) fn resolve(&self, db: &mut Db, body: &Value, now: i64) -> Result<()> {
        match self {
            Self::WaitingOn { task, user } => {
                let row = &body["task"];
                ensure!(row["id"] == *task && row.get("waitingOnUserId").is_some() && row["waitingOnUserId"] == json!(user), "missing blocked by person acknowledgement");
                let mut current = db.raw("Task", &json!(task))?.context("task is not cached")?;
                for field in ["waitingOnUserId", "waitingOnSetById", "waitingOnSetAt"] {
                    current[field] = row.get(field).context("incomplete blocked by person acknowledgement")?.clone();
                }
                db.upsert_task(&current)?;
            }
            Self::ToggleTimer { task } => {
                ensure!(body["timer_running"].is_boolean() || body["timer_disabled"] == true, "missing timer acknowledgement");
                db.kv_set(&format!("timer-state:{task}"), &body.to_string())?;
            }
            Self::MoveToInbox { .. } => {
                ensure!(body["action"] == "inbox" && body["active"] == true, "missing move to inbox acknowledgement");
                db.request_refetch("inbox", 0)?;
            }
            Self::StarTask { task, active, .. } => {
                ensure!(body["action"] == "star" && body["active"] == *active, "missing task star acknowledgement");
                let saved = body["saved_content"].as_array().context("missing saved task state")?;
                ensure!(!saved.is_empty() == *active && saved.iter().all(|row| row["type"] == "Private" && row["id"].as_str().is_some_and(|id| !id.is_empty())), "invalid saved task state");
                let mut row = db.raw("Task", &json!(task))?.context("task is not cached")?;
                row["savedContent"] = json!(saved);
                db.upsert_task(&row)?;
            }
            Self::AssignAgent { task, .. } | Self::AssignToMe { task, .. } => {
                let rows = body["assignees"]
                    .as_array()
                    .context("missing assignment acknowledgement")?;
                match self {
                    Self::AssignAgent { agent, active, .. } => ensure!(
                        rows.iter().any(|row| row["agent"]["id"] == *agent) == *active,
                        "agent assignment not acknowledged"
                    ),
                    Self::AssignToMe { user, .. } => ensure!(
                        rows.iter().any(|row| row["userId"] == *user && row["agent"].is_null()),
                        "self assignment not acknowledged"
                    ),
                    _ => unreachable!(),
                }
                let mut task_row = db
                    .raw("Task", &json!(task))?
                    .context("task is not cached")?;
                let project = task_row["projectId"]
                    .as_i64()
                    .context("missing task board")?;
                let members = db.project_members(project)?;
                let selected = rows
                    .iter()
                    .map(|row| {
                        let id = row["userId"].as_i64().context("missing assignment user")?;
                        let mut member = members
                            .iter()
                            .find(|member| member["id"] == id)
                            .cloned()
                            .unwrap_or_else(|| json!({"id":id}));
                        if row["agent"].is_object() {
                            member["agent"] = row["agent"].clone();
                            member["displayName"] = row["agent"]["displayName"].clone();
                        }
                        Ok(member)
                    })
                    .collect::<Result<Vec<Value>>>()?;
                task_row["assignees"] = json!(selected);
                task_row["assigneeCount"] = json!(selected.len());
                db.upsert_task(&task_row)?;
                db.request_refetch("task", *task)?;
            }
            Self::AddColumn { project, .. } | Self::RenameColumn { project, .. } => {
                let section = body
                    .get("section")
                    .context("missing section acknowledgement")?;
                ensure!(
                    section["projectId"] == *project
                        && section["id"].as_i64().is_some_and(|id| id > 0),
                    "section acknowledgement belongs to another board"
                );
                if let Self::RenameColumn { section: id, .. } = self {
                    ensure!(section["id"] == *id, "wrong column acknowledgement");
                }
                db.upsert("Section", section)?;
                db.request_refetch("board", *project)?;
            }
            Self::DeleteColumn { project, section } => {
                let mut column = db
                    .raw("Section", &json!(section))?
                    .context("column is not cached")?;
                column["deleted"] = json!(true);
                db.upsert("Section", &column)?;
                db.request_refetch("board", *project)?;
            }
            Self::RenameBoard { project, .. } | Self::ArchiveBoard { project } => {
                let value = body
                    .get("project")
                    .context("missing board acknowledgement")?;
                ensure!(value["id"] == *project, "wrong board acknowledgement");
                if matches!(self, Self::ArchiveBoard { .. }) {
                    ensure!(
                        value["status"] == "Archive",
                        "missing archive acknowledgement"
                    );
                }
                db.upsert("Project", value)?;
            }
            Self::CreateTaskLabel { project, task, .. } => {
                let mut label = body
                    .get("label")
                    .context("missing label acknowledgement")?
                    .clone();
                let id = label["id"]
                    .as_str()
                    .filter(|id| !id.is_empty())
                    .context("missing label id")?
                    .to_owned();
                label["projectId"] = json!(project);
                label["value"] = label["name"].clone();
                db.upsert("Label", &label)?;
                let task_row = db.task(*task)?.context("task is not cached")?;
                ensure!(
                    task_row.project_id == *project,
                    "task moved while creating label"
                );
                let mut ids: Vec<_> = task_row
                    .extra
                    .get("labels")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(|l| l["id"].as_str().map(str::to_owned))
                    .collect();
                if !ids.contains(&id) {
                    ids.push(id);
                }
                db.enqueue_in_transaction(
                    Write::TaskPatch {
                        task_id: *task,
                        patch: TaskPatch::Labels(ids),
                    },
                    now,
                )?;
            }
            Self::MoveTask {
                task,
                project,
                section,
            } => {
                let value = body
                    .get("task")
                    .context("missing moved task acknowledgement")?;
                ensure!(
                    value["id"] == *task
                        && value["projectId"] == *project
                        && value["sectionId"] == *section,
                    "wrong move acknowledgement"
                );
                db.upsert_task(value)?;
                db.request_refetch("task", *task)?;
                db.request_refetch("board", *project)?;
            }
        }
        Ok(())
    }
}
impl Db {
    pub fn take_board_result(&mut self, client_id: &str) -> Result<Option<Value>> {
        self.transaction(|db| {
            let key = format!("board-result:{client_id}");
            let value = db
                .kv_get(&key)?
                .map(|value| serde_json::from_str(&value))
                .transpose()?;
            db.conn.execute("DELETE FROM kv WHERE key=?", [&key])?;
            Ok(value)
        })
    }
}

// Time routes have no idempotency contract. Never automatically replay a toggle.
pub(crate) fn toggle_timer(transport: &mut impl crate::Transport, task: i64, client_id: &str) -> std::result::Result<crate::http::Response, crate::http::TransportError> {
    use crate::http::{Response, TransportError};
    let body = json!({"task":task.to_string()});
    let status = transport.post("/time/status", &body, client_id)?;
    if !(200..300).contains(&status.status) || status.body["success"] != true { return Ok(status); }
    let summary = &status.body["summary"];
    let enabled = summary["enabled"].as_bool().ok_or(TransportError::InvalidResponse)?;
    let running = summary.get("runningEntry").ok_or(TransportError::InvalidResponse)?;
    if !running.is_null() && !running.is_object() { return Err(TransportError::InvalidResponse); }
    if !enabled && running.is_null() { return Ok(Response { status:200, body:json!({"success":true,"timer_disabled":true}) }); }
    let start = running.is_null();
    let mut response = transport.post(if start { "/time/start" } else { "/time/stop" }, &body, client_id)?;
    if (200..300).contains(&response.status) && response.body["success"] == true {
        let entry = &response.body["entry"];
        if start && (entry["taskId"] != task || !entry["endedAt"].is_null()) { return Err(TransportError::InvalidResponse); }
        if !start && !entry.is_null() && (entry["taskId"] != task || !entry["endedAt"].is_string()) { return Err(TransportError::InvalidResponse); }
        response.body["timer_running"] = json!(start);
        response.body["timer_enabled"] = json!(enabled);
    }
    Ok(response)
}
