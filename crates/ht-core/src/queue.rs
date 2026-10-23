use crate::{Db, Result, Transport, db::ensure_success, http::Response};
use anyhow::{Context, bail, ensure};
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Write {
    TaskTitle {
        task_id: i64,
        title: String,
    },
    TaskSection {
        task_id: i64,
        section_id: i64,
    },
    TaskComment {
        task_id: i64,
        text: String,
    },
    TaskPatch {
        task_id: i64,
        patch: crate::TaskPatch,
    },
    InboxArchive {
        notification_id: i64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mutation {
    pub id: i64,
    pub client_id: String,
    pub entity: String,
    pub kind: String,
    pub payload_json: Value,
    pub created_at: i64,
    pub attempts: u32,
    pub last_error: Option<String>,
    pub status: String,
    pub next_attempt_at: i64,
    pub lease_until: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Drain {
    Empty,
    Waiting(i64),
    Applied(i64),
    Retrying(i64),
    Failed(i64),
}

pub fn backoff(attempts: u32) -> i64 {
    [1, 2, 4, 8, 30, 60][(attempts as usize).min(5)]
}
pub fn permanent(status: u16) -> bool {
    (400..500).contains(&status) && !matches!(status, 408 | 429)
}

impl Db {
    pub fn enqueue(&mut self, write: Write, now: i64) -> Result<i64> {
        self.transaction(|db| {
            let (entity, kind, payload) = match &write {
                Write::TaskTitle { task_id, title } => {
                    ensure!(!title.trim().is_empty(), "title must not be empty");
                    ensure!(db.task(*task_id)?.is_some(), "task is not cached");
                    (format!("task:{task_id}"), "task_title", json!({"task_id":task_id,"title":title}))
                }
                Write::TaskSection { task_id, section_id } => {
                    let task = db.task(*task_id)?.context("task is not cached")?;
                    let section = db.raw("Section", &json!(section_id))?.context("section is not cached")?;
                    ensure!(section["projectId"] == task.project_id && section["deleted"] == false && section["visibility"] == true, "section must be visible and in the same board");
                    (format!("task:{task_id}"), "task_section", json!({"task_id":task_id,"sectionId":section_id,"section":section["section_title"]}))
                }
                Write::TaskComment { task_id, text } => {
                    ensure!(!text.trim().is_empty(), "comment must not be empty");
                    ensure!(db.task(*task_id)?.is_some(), "task is not cached");
                    (format!("task:{task_id}"), "task_comment", json!({"task_id":task_id,"text":text}))
                }
                Write::TaskPatch { task_id, patch } => {
                    ensure!(db.task(*task_id)?.is_some(), "task is not cached");
                    let mut fields = patch.fields()?;
                    fields["task_id"] = json!(task_id);
                    (format!("task:{task_id}"), "task_patch", fields)
                }
                Write::InboxArchive { notification_id } => {
                    ensure!(db.raw("Notification", &json!(notification_id))?.is_some(), "notification is not cached");
                    (format!("notification:{notification_id}"), "inbox_archive", json!({"notification_ids":[notification_id]}))
                }
            };
            let client_id = uuid::Uuid::new_v4().to_string();
            db.conn.execute("INSERT INTO mutation_queue(client_id,entity,kind,payload_json,created_at,next_attempt_at) VALUES(?,?,?,?,?,?)",
                params![client_id,entity,kind,payload.to_string(),now,now])?;
            let id = db.conn.last_insert_rowid();
            db.apply_optimistic(id, kind, &payload, now)?;
            Ok(id)
        })
    }

    fn apply_optimistic(&self, id: i64, kind: &str, payload: &Value, now: i64) -> Result<()> {
        match kind {
            "task_title" | "task_section" => {
                let mut task = self
                    .raw("Task", &payload["task_id"])?
                    .context("task is not cached")?;
                if kind == "task_title" {
                    task["title"] = payload["title"].clone();
                } else {
                    task["sectionId"] = payload["sectionId"].clone();
                    task["section"] = payload["section"].clone();
                }
                self.upsert_task(&task)?;
            }
            "task_patch" => {
                let mut task = self
                    .raw("Task", &payload["task_id"])?
                    .context("task is not cached")?;
                crate::task_patch::optimistic(&mut task, payload);
                let project = task["projectId"].as_i64().context("missing project id")?;
                if let Some(ids) = payload["labels"].as_array() {
                    let labels = self.project_labels(project)?;
                    let selected = ids
                        .iter()
                        .map(|id| {
                            labels
                                .iter()
                                .find(|l| &l["id"] == id)
                                .cloned()
                                .context("label is not cached in this board")
                        })
                        .collect::<Result<Vec<_>>>()?;
                    task["labelCount"] = json!(selected.len());
                    task["labels"] = json!(selected);
                }
                if let Some(ids) = payload["assignee"].as_array() {
                    let members = self.project_members(project)?;
                    let selected = ids
                        .iter()
                        .map(|id| {
                            members
                                .iter()
                                .find(|m| &m["id"] == id)
                                .cloned()
                                .context("member is not cached in this board")
                        })
                        .collect::<Result<Vec<_>>>()?;
                    task["assigneeCount"] = json!(selected.len());
                    task["assignees"] = json!(selected);
                }
                self.upsert_task(&task)?;
            }
            "task_comment" => {
                let timestamp = chrono::DateTime::from_timestamp(now, 0)
                    .context("invalid timestamp")?
                    .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
                let comment = json!({"id":-id,"text":payload["text"],"commentText":payload["text"],"createdAt":timestamp,"pending":true,"clientId":self.client_id(id)?});
                self.upsert_comment(
                    payload["task_id"].as_i64().context("missing task_id")?,
                    &comment,
                )?;
            }
            "inbox_archive" => {
                for notification_id in payload["notification_ids"]
                    .as_array()
                    .context("missing notification ids")?
                {
                    let mut n = self
                        .raw("Notification", notification_id)?
                        .context("notification is not cached")?;
                    n["status"] = json!("Archive");
                    n["archivedAt"] = json!(
                        chrono::DateTime::from_timestamp(now, 0)
                            .context("invalid timestamp")?
                            .to_rfc3339()
                    );
                    self.upsert("Notification", &n)?;
                }
            }
            _ => bail!("unknown mutation kind"),
        }
        Ok(())
    }

    fn client_id(&self, id: i64) -> Result<String> {
        Ok(self.conn.query_row(
            "SELECT client_id FROM mutation_queue WHERE id=?",
            [id],
            |r| r.get(0),
        )?)
    }

    pub(crate) fn reapply_pending(&self) -> Result<()> {
        for row in self
            .mutations()?
            .into_iter()
            .filter(|m| m.status != "failed")
        {
            self.apply_optimistic(row.id, &row.kind, &row.payload_json, row.created_at)?;
        }
        Ok(())
    }

    pub fn mutations(&self) -> Result<Vec<Mutation>> {
        let mut stmt = self.conn.prepare("SELECT id,client_id,entity,kind,payload_json,created_at,attempts,last_error,status,next_attempt_at,lease_until FROM mutation_queue ORDER BY id")?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, i64>(5)?,
                r.get::<_, u32>(6)?,
                r.get::<_, Option<String>>(7)?,
                r.get::<_, String>(8)?,
                r.get::<_, i64>(9)?,
                r.get::<_, Option<i64>>(10)?,
            ))
        })?;
        rows.map(|r| {
            let (
                id,
                client_id,
                entity,
                kind,
                payload,
                created_at,
                attempts,
                last_error,
                status,
                next_attempt_at,
                lease_until,
            ) = r?;
            Ok(Mutation {
                id,
                client_id,
                entity,
                kind,
                payload_json: serde_json::from_str(&payload)?,
                created_at,
                attempts,
                last_error,
                status,
                next_attempt_at,
                lease_until,
            })
        })
        .collect()
    }

    pub fn drain_once(&mut self, transport: &mut impl Transport, now: i64) -> Result<Drain> {
        let claim = self.transaction(|db| {
            let head = db.mutations()?.into_iter().find(|m| m.status != "failed");
            let Some(mut row) = head else { return Ok(None); };
            let ready = if row.status == "inflight" { row.lease_until.unwrap_or(0) } else { row.next_attempt_at };
            if ready > now { return Ok(Some((row, false, ready))); }
            db.conn.execute("UPDATE mutation_queue SET status='inflight', attempts=attempts+1, lease_until=? WHERE id=?", params![now+120,row.id])?;
            row.attempts += 1;
            Ok(Some((row, true, 0)))
        })?;
        let Some((row, claimed, ready)) = claim else {
            return Ok(Drain::Empty);
        };
        if !claimed {
            return Ok(Drain::Waiting(ready));
        }
        let (path, body) = match row.kind.as_str() {
            "task_patch" => ("/tasks/update", row.payload_json.clone()),
            "task_title" => (
                "/tasks/update",
                json!({"task_id":row.payload_json["task_id"],"title":row.payload_json["title"]}),
            ),
            "task_section" => (
                "/tasks/update",
                json!({"task_id":row.payload_json["task_id"],"sectionId":row.payload_json["sectionId"]}),
            ),
            "task_comment" => (
                "/comments",
                json!({"task_id":row.payload_json["task_id"],"text":row.payload_json["text"],"kind":"qa"}),
            ),
            "inbox_archive" => ("/inbox/archive", row.payload_json.clone()),
            _ => bail!("unknown queued kind"),
        };
        let response = transport.post(path, &body, &row.client_id);
        let result = match response {
            Ok(Response { status, body }) if (200..300).contains(&status) => {
                match ensure_success(&body)
                    .and_then(|()| self.resolve_server_authoritative(&row, &body))
                {
                    Ok(()) => return Ok(Drain::Applied(row.id)),
                    Err(_) => (false, "invalid success response".to_owned()),
                }
            }
            Ok(Response { status, .. }) => (permanent(status), format!("HTTP {status}")),
            Err(error) => (false, error.to_string()),
        };
        self.transaction(|db| {
            let current: Option<String> = db.conn.query_row("SELECT client_id FROM mutation_queue WHERE id=? AND attempts=?", params![row.id,row.attempts], |r| r.get(0)).optional()?;
            if current.is_none() { return Ok(()); }
            db.conn.execute("UPDATE mutation_queue SET status=?,last_error=?,next_attempt_at=?,lease_until=NULL WHERE id=? AND attempts=?",
                params![if result.0 { "failed" } else { "pending" },result.1,now+backoff(row.attempts-1),row.id,row.attempts])?;
            if result.0 {
                if row.kind == "task_comment" {
                    db.conn.execute("DELETE FROM Comment WHERE id=?", [-row.id])?;
                    db.request_refetch("comments", row.payload_json["task_id"].as_i64().context("missing task id")?)?;
                }
                if let Some(task_id) = row.payload_json["task_id"].as_i64() { db.request_refetch("task", task_id)?; }
                else { db.request_refetch("inbox", 0)?; }
            }
            Ok(())
        })?;
        Ok(if result.0 {
            Drain::Failed(row.id)
        } else {
            Drain::Retrying(row.id)
        })
    }

    fn resolve_server_authoritative(&mut self, row: &Mutation, body: &Value) -> Result<()> {
        self.transaction(|db| {
            let active: bool = db.conn.query_row("SELECT EXISTS(SELECT 1 FROM mutation_queue WHERE id=? AND attempts=? AND status='inflight')", params![row.id,row.attempts], |r| r.get(0))?;
            if !active { return Ok(()); }
            match row.kind.as_str() {
                "task_title" | "task_section" | "task_patch" => {
                    let task = body.get("task").or_else(|| body["tasks"].as_array().and_then(|v| v.iter().find(|t| t["id"] == row.payload_json["task_id"]))).context("mutation response missing task")?;
                    ensure!(task["id"] == row.payload_json["task_id"], "mutation returned another task");
                    db.upsert_task(task)?;
                    db.request_refetch("task", row.payload_json["task_id"].as_i64().context("missing task id")?)?;
                }
                "task_comment" => {
                    let comment = body.get("comment").context("mutation response missing comment")?;
                    ensure!(comment["id"].as_i64().is_some_and(|id| id>0), "invalid comment id");
                    db.upsert_comment(row.payload_json["task_id"].as_i64().context("missing task id")?, comment)?;
                    db.conn.execute("DELETE FROM Comment WHERE id=?", [-row.id])?;
                    db.request_refetch("task", row.payload_json["task_id"].as_i64().unwrap())?;
                }
                "inbox_archive" => {
                    ensure!(body["archived_count"].as_u64().is_some(), "archive response missing count");
                    db.request_refetch("inbox", 0)?;
                }
                _ => bail!("unknown mutation kind"),
            }
            db.conn.execute("DELETE FROM mutation_queue WHERE id=? AND attempts=?", params![row.id,row.attempts])?;
            db.reapply_pending()?;
            Ok(())
        })
    }
}
