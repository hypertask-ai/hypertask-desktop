use crate::{Db, Result, TaskPatch, Write};
use anyhow::{Context, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BoardWrite {
    PositionTask { task: i64, project: i64, section: i64, title: String, ranking: String },
    ArchiveBoard {
        project: i64,
    },
    AssignAgent {
        task: i64,
        agent: String,
        active: bool,
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
            Self::PositionTask { task, project, section, title: name, ranking } => {
                positive(*task)?; positive(*project)?; positive(*section)?;
                title(name, 200)?;
                ensure!(!ranking.is_empty() && ranking.len() <= 128, "invalid task ranking");
                ("PUT", "/api/tasks/moveTask".into(), json!({"section_title":name,"sectionId":section,"taskId":task,"ranking":ranking,"projectId":project}))
            }
            Self::ArchiveBoard { project } => {
                positive(*project)?;
                (
                    "POST",
                    "/projects/archive".into(),
                    json!({"project_id":project,"status":"Archive"}),
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
            Self::PositionTask { task, project, section, ranking, .. } => {
                let value = body.get("task").context("missing moved task acknowledgement")?;
                ensure!(value["id"] == *task && value["projectId"] == *project && value["sectionId"] == *section && value["ranking"] == *ranking, "wrong move acknowledgement");
                db.upsert_task(value)?;
                db.request_refetch("board", *project)?;
            }
            Self::AssignAgent {
                task,
                agent,
                active,
            } => {
                let rows = body["assignees"]
                    .as_array()
                    .context("missing assignment acknowledgement")?;
                ensure!(
                    rows.iter().any(|row| row["agent"]["id"] == *agent) == *active,
                    "agent assignment not acknowledged"
                );
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
