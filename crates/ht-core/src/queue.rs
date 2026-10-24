use crate::{Db, Result, Transport, db::ensure_success, http::Response};
use anyhow::{Context, bail, ensure};
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Write {
    Preferences { updates: Value },
    AiSession { change: crate::AiSessionWrite },
    View { change: crate::ViewWrite },
    Favorites { project_ids: Vec<i64> },
    FavoriteSlot { index: u8, project_id: Option<i64> },
    CreateTask { project_id: i64, section_id: i64, title: String, top: bool },
    CreateTaskForm { project_id:i64, section_id:i64, title:String, fields:Value },
    ComposeTask { project_id: i64, text: String, time_zone: String },
    Shell { request: crate::ShellWrite },
    Board { change: crate::BoardWrite },
    EditorImprove { task_id:i64, command:crate::ImproveCommand, text:String },
    DiscardDraft { task_id:i64, draft_type:crate::DraftType },
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
    CommentDraft {
        task_id: i64,
        text: Option<String>,
    },
    CommentAction {
        task_id: i64,
        comment_id: i64,
        change: crate::CommentWrite,
    },
    TaskPatch {
        task_id: i64,
        patch: crate::TaskPatch,
    },
    InboxSeen { notification_id: i64 },
    InboxRemind { notification_ids: Vec<i64>, remind_at: String },
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
        self.transaction(|db|db.enqueue_in_transaction(write,now))
    }
    pub(crate) fn enqueue_in_transaction(&mut self,write:Write,now:i64)->Result<i64> {
        let db=self;
        let (entity, kind, payload) = match &write {
            Write::Preferences { updates } => {
                crate::preferences::validate(updates)?;
                let user=db.current_user_id().context("Unauthorized")?;
                ensure!(!db.mutations()?.iter().any(|row|row.status!="failed" && row.kind=="web_preferences"),"Wait for the previous preference update");
                ("preferences".into(),"web_preferences",json!({"user_id":user,"updates":updates,"before":db.web_preferences()?}))
            }
            Write::AiSession { change } => {
                change.request()?;
                let user=db.current_user_id().context("Sign in before changing chats")?;
                ensure!(!db.mutations()?.iter().any(|row|row.status!="failed" && row.kind=="ai_session"),"Wait for the previous chat update");
                ("ai-session".into(),"ai_session",json!({"user_id":user,"change":change,"before":db.ai_sessions()?,"attempted":false}))
            }
            Write::View { change } => {
                change.validate()?;
                ensure!(!db.mutations()?.iter().any(|row|row.status!="failed" && row.kind=="web_view" && row.payload_json["change"]["project"]==change.project), "wait for the previous view update to finish");
                let user=db.current_user_id().context("Unauthorized")?;
                if change.smart_split() {
                    let project = db.board_project(change.project)?;
                    if change.route == "smart-split-create" {
                        ensure!(project["project_view"]["default_view"].is_object(), "This board does not have a default view");
                    } else {
                        let views = project["project_view"]["allViews"].as_array().context("Smart split details are still loading")?;
                        let labels = project["desktop_smart_labels"].as_array().context("Smart split details are still loading")?;
                        let view = views.iter().find(|view| view["id"] == change.body["viewId"]).context("Smart split not found")?;
                        ensure!(crate::smart_splits::label(view, labels, views).is_some(), "This smart tag is linked to multiple views and cannot be edited safely");
                        ensure!(change.route != "smart-split-delete" || project["project_view"]["default_view_id"] != change.body["viewId"], "Choose another default view before deleting this smart split");
                    }
                }
                (format!("view:{}",change.project),"web_view",json!({"user_id":user,"change":change,"before":db.board_project(change.project)?,"attempted":false}))
            }
            Write::FavoriteSlot { index, project_id } => {
                ensure!(!db.mutations()?.iter().any(|row|row.status!="failed" && matches!(row.kind.as_str(),"web_favorites"|"web_favorite_slot")), "wait for the previous favorite update to finish");
                ("favorites".into(),"web_favorite_slot",db.prepare_favorite_slot(*index,*project_id)?)
            }
            Write::Favorites { project_ids } => {
                ensure!(!db.mutations()?.iter().any(|row|row.status!="failed" && matches!(row.kind.as_str(),"web_favorites"|"web_favorite_slot")), "wait for the previous favorite update to finish");
                ("favorites".into(),"web_favorites",db.prepare_favorites(project_ids)?)
            },
            Write::CreateTask { project_id, section_id, title, top } => {
                let mut payload=db.prepare_create(*project_id,*section_id,title,*top)?;
                payload["attempted"]=json!(false);
                (format!("create:{project_id}"), "web_create", payload)
            },
            Write::CreateTaskForm {project_id,section_id,title,fields}=>{
                let payload=db.prepare_create_form(*project_id,*section_id,title,fields)?;
                (format!("create:{project_id}"),"web_create",payload)
            }
            Write::ComposeTask { project_id, text, time_zone } => {
                ensure!(*project_id > 0 && !text.trim().is_empty(), "choose a board and describe the task");
                let user = db.current_user_id().context("sign in before composing a task")?;
                let project = db.raw("Project", &json!(project_id))?.context("board is unavailable")?;
                (format!("compose:{project_id}"), "compose_task", json!({"project":project,"text":text,"time_zone":time_zone,"user_id":user}))
            }
            Write::Board { change } => {
                change.request()?;
                let user = db.current_user_id().context("sync your account before changing a board")?;
                if let crate::BoardWrite::CreateTaskLabel { task, project, .. } = change {
                    ensure!(db.task(*task)?.is_some_and(|task| task.project_id == *project), "task is not cached in this board");
                }
                if let crate::BoardWrite::PositionTask { task, project, section, .. } = change {
                    ensure!(!db.mutations()?.iter().any(|row|row.status!="failed" && row.payload_json["change"]["PositionTask"]["task"]==*task), "wait for the previous move to finish");
                    ensure!(db.task(*task)?.is_some_and(|t| t.project_id == *project), "task must belong to the board");
                    let column = db.raw("Section", &json!(section))?.context("column is not cached")?;
                    ensure!(column["projectId"] == *project && column["deleted"] == false && column["visibility"] == true, "column is not visible in this board");
                }
                let before = if let crate::BoardWrite::AssignAgent { task, .. } | crate::BoardWrite::PositionTask { task, .. } = change { db.raw("Task",&json!(task))?.context("task is not cached")? } else { Value::Null };
                ("board".into(), "board_write", json!({"user_id":user,"change":change,"before":before}))
            }
            Write::Shell { request } => {
                request.validate()?;
                ("shell".into(), "shell_write", serde_json::to_value(request)?)
            }
            Write::EditorImprove {task_id,command,text}=>{
                let task=db.task(*task_id)?.context("task is not cached")?;
                ensure!(!text.trim().is_empty(),"editor content must not be empty");
                (format!("editor:{task_id}"),"editor_improve",json!({"task_id":task_id,"project_id":task.project_id,"text":text,"command":command}))
            }
            Write::DiscardDraft {task_id,draft_type}=>{
                let task=db.raw("Task",&json!(task_id))?.context("task is not cached")?;
                let user=db.current_user_id().context("sync your account before discarding drafts")?;
                (format!("draft:{task_id}:{}",draft_type.as_str()),"draft_discard",json!({"task_id":task_id,"draft_type":draft_type,"user_id":user,"before":task,"drafts_before":db.editor_drafts(*task_id)?}))
            }
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
                crate::editor_queue::validate_editor_html(text,"POST /api/mcp/comments")?;
                ensure!(!text.trim().is_empty(), "comment must not be empty");
                ensure!(text.encode_utf16().count() <= 5000, "comment must be 5000 characters or less");
                ensure!(db.task(*task_id)?.is_some(), "task is not cached");
                let draft = db.comment_draft(*task_id)?.filter(|s| !s.is_empty());
                let user = db.current_user_id();
                let payload = json!({"task_id":task_id,"text":text,"draft_text":draft,"draft_user_id":user.filter(|_| draft.is_some())});
                if draft.is_some() { db.save_local_comment_draft(*task_id, "")?; }
                (format!("task:{task_id}"), "task_comment", payload)
            }
            Write::CommentDraft { task_id, text } => {
                ensure!(db.task(*task_id)?.is_some(), "draft task is not cached");
                let user = db.current_user_id().context("sync your account before saving drafts")?;
                if let Some(text) = text { ensure!(!text.trim().is_empty(), "draft must not be empty"); }
                db.save_local_comment_draft(*task_id, text.as_deref().unwrap_or(""))?;
                let entity = format!("draft:{user}:{task_id}");
                db.conn.execute("DELETE FROM mutation_queue WHERE entity=? AND status='pending' AND attempts=0", [&entity])?;
                (entity, "comment_draft", json!({"task_id":task_id,"user_id":user,"draft_type":"comment","text":text}))
            }
            Write::CommentAction { task_id, comment_id, change } => {
                change.validate()?;
                if let crate::CommentWrite::Edit{text}=change {crate::editor_queue::validate_editor_html(text,"PATCH /api/mcp/comments/[comment_id]")?;}
                ensure!(*comment_id > 0, "pending comments cannot be changed");
                let before = db.raw("Comment", &json!(comment_id))?.context("comment is not cached")?;
                ensure!(before["taskId"] == *task_id && before["activity"].is_null(), "not a comment in this task");
                let user_id = db.current_user_id().context("sync your account before changing comments")?;
                if matches!(change, crate::CommentWrite::Edit { .. } | crate::CommentWrite::Delete) {
                    ensure!(before["creatorId"] == user_id, "you can only change your own comments");
                }
                (format!("comment:{comment_id}"), "comment_action", json!({"task_id":task_id,"comment_id":comment_id,"change":change,"before":before,"user_id":user_id}))
            }
            Write::TaskPatch { task_id, patch } => {
                if let crate::TaskPatch::Description(html)=patch {crate::editor_queue::validate_editor_html(html,"POST /api/mcp/tasks/update")?;}
                ensure!(db.task(*task_id)?.is_some(), "task is not cached");
                let mut fields = patch.fields()?;
                fields["task_id"] = json!(task_id);
                (format!("task:{task_id}"), "task_patch", fields)
            }
            Write::InboxSeen { notification_id } => {
                let before = db.raw("Notification", &json!(notification_id))?.context("notification is not cached")?;
                (format!("notification:{notification_id}"), "inbox_seen", json!({"notification_ids":[notification_id],"before":[before]}))
            }
            Write::InboxRemind { notification_ids, remind_at } => {
                ensure!(!notification_ids.is_empty() && notification_ids.len() <= 100, "choose between 1 and 100 notifications");
                let time = chrono::DateTime::parse_from_rfc3339(remind_at).context("reminder requires an ISO timestamp")?.timestamp();
                ensure!(time > now && time - now <= 365 * 86400, "reminder must be in the future within one year");
                let mut ids = notification_ids.clone();
                ids.sort_unstable();
                ids.dedup();
                let before = ids.iter().map(|id| {
                    let n = db.raw("Notification", &json!(id))?.context("notification is not cached")?;
                    ensure!(*id > 0 && n["status"] == "Normal" && n["taskId"].as_i64().is_some() && n["projectId"].as_i64().is_some(), "Inbox item is unavailable for a reminder");
                    Ok(n)
                }).collect::<Result<Vec<_>>>()?;
                let tasks: Vec<_> = before.iter().filter_map(|n|n["taskId"].as_i64()).collect();
                let before: Vec<_> = db.inbox()?.into_iter().filter(|n|n.task_id.is_some_and(|id|tasks.contains(&id))).map(|n|n.raw_json).collect();
                (format!("inbox-reminder:{}", ids[0]), "inbox_remind", json!({"notification_ids":ids,"remind_at":remind_at,"before":before}))
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
    }

    fn apply_optimistic(&self, id: i64, kind: &str, payload: &Value, now: i64) -> Result<()> {
        match kind {
            "shell_write" | "compose_task" => (),
            "ai_session" => self.optimistic_ai_session(payload)?,
            "web_preferences" => self.optimistic_preferences(payload)?,
            "web_view" => {
                if self.current_user_id()!=payload["user_id"].as_i64(){return Ok(());}
                let change:crate::ViewWrite=serde_json::from_value(payload["change"].clone())?;
                let mut project=self.board_project(change.project)?;change.optimistic(&mut project);self.upsert("Project",&project)?;
            }
            "web_favorites" | "web_favorite_slot" => {
                let user=payload["user_id"].as_i64().context("missing account")?;
                if self.current_user_id()==Some(user) {self.kv_set(&crate::board_sidebar::cache_key(user,"favorites"),&payload["favorites"].to_string())?;}
            }
            "web_create" => {
                let body = &payload["body"];
                let labels=body["tags"].as_array().cloned().unwrap_or_default();
                let assignees:Vec<_>=body["assignees"].as_array().into_iter().flatten().map(|person|if person.get("uid").is_some(){json!({"userId":person["id"],"user":person})}else{json!({"userId":person["userId"],"agentId":person["id"],"agent":person})}).collect();
                self.upsert_task(&json!({"id":-id,"projectId":body["projectId"],"sectionId":body["sectionId"],"section":body.get("section").or_else(||body.get("section_title")),"title":body["title"],"ranking":body["ranking"],"status":"Normal","pending":true,"createdBy":null,"priority":body["priority"],"estimate":body["estimate"],"labels":labels,"assignees":assignees,"description":body["description"].as_str().unwrap_or(""),"description_":{"content":body["description"]},"dueDate":body["dueDate"],"startDate":body["startDate"]}))?;
            }
            "board_write" => {
                if let crate::BoardWrite::PositionTask { task, section, title, ranking, .. } = serde_json::from_value(payload["change"].clone())? {
                    let mut value = self.raw("Task", &json!(task))?.context("task is not cached")?;
                    value["sectionId"] = json!(section); value["section"] = json!(title); value["ranking"] = json!(ranking);
                    self.upsert_task(&value)?;
                }
                if let crate::BoardWrite::AssignAgent { task, agent, active } = serde_json::from_value(payload["change"].clone())? {
                    let mut task_row = self.raw("Task",&json!(task))?.context("task is not cached")?;
                    let project = task_row["projectId"].as_i64().context("missing task board")?;
                    let members = self.project_members(project)?;
                    let member = members.iter().find(|member|member["id"] == agent).context("agent is not cached in this board")?;
                    let mut selected = task_row["assignees"].as_array().cloned().unwrap_or_default();
                    selected.retain(|row|row["agent"]["id"] != agent);
                    if active { selected.push(json!({"id":member["owner"]["id"],"displayName":member["displayName"],"agent":{"id":agent,"displayName":member["displayName"]}})); }
                    task_row["assigneeCount"] = json!(selected.len()); task_row["assignees"] = json!(selected); self.upsert_task(&task_row)?;
                }
            },
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
                    let mut selected = ids
                        .iter()
                        .map(|id| {
                            members
                                .iter()
                                .find(|m| &m["id"] == id)
                                .cloned()
                                .context("member is not cached in this board")
                        })
                        .collect::<Result<Vec<_>>>()?;
                    selected.extend(task["assignees"].as_array().into_iter().flatten().filter(|row|row["agent"].is_object()).cloned());
                    task["assigneeCount"] = json!(selected.len());
                    task["assignees"] = json!(selected);
                }
                self.upsert_task(&task)?;
            }
            "editor_improve"=>(),
            "draft_discard"=>self.optimistic_discard(payload)?,
            "comment_draft" => self.optimistic_comment_draft(payload)?,
            "comment_action" => self.optimistic_comment_action(payload)?,
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
            "inbox_seen" => {
                for id in payload["notification_ids"].as_array().context("missing notification ids")? {
                    let mut n = self.raw("Notification", id)?.context("notification is not cached")?;
                    n["seen"] = json!(true);
                    n["unreadCount"] = json!(0);
                    self.upsert("Notification", &n)?;
                }
            }
            "inbox_archive" | "inbox_remind" => {
                let ids = if kind == "inbox_remind" {
                    json!(payload["before"].as_array().context("missing inbox rollback")?.iter().map(|n|n["id"].clone()).collect::<Vec<_>>())
                } else { payload["notification_ids"].clone() };
                for notification_id in ids
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
            "editor_improve" => ("/ai/improve",row.payload_json.clone()),
            "draft_discard" => ("",Value::Null),
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
                json!({"task_id":row.payload_json["task_id"],"text":row.payload_json["text"],"kind":"qa","content_type":"html"}),
            ),
            "inbox_archive" => ("/inbox/archive", row.payload_json.clone()),
            "inbox_seen" => ("/inbox/seen", json!({"notification_ids":row.payload_json["notification_ids"]})),
            "inbox_remind" => ("/inbox/archive", json!({"notification_ids":row.payload_json["notification_ids"],"remind_at":row.payload_json["remind_at"]})),
            "web_preferences" | "ai_session" | "web_view" | "web_favorites" | "web_favorite_slot" | "web_create" | "compose_task" | "comment_action" | "comment_draft" | "shell_write" | "board_write" => ("", Value::Null),
            _ => bail!("unknown queued kind"),
        };
        let response = if row.kind == "web_preferences" {
            self.send_preferences(transport,&row.payload_json,&row.client_id)
        } else if row.kind == "ai_session" {
            self.send_ai_session(transport,&row)
        } else if row.kind == "web_view" {
            let change:crate::ViewWrite=serde_json::from_value(row.payload_json["change"].clone())?;
            change.validate()?;
            if matches!(change.route.as_str(),"create-view"|"smart-split-create") && (row.payload_json["attempted"]==true || (row.payload_json.get("attempted").is_none() && row.attempts>1)) {Ok(Response{status:409,body:json!({"message":"View creation result uncertain; refresh before trying again"})})}else{match transport.get("/user/context") {
                Ok(identity) if identity.status==200 && identity.body["success"]==true && self.current_user_id()==row.payload_json["user_id"].as_i64() && identity.body["user"]["id"]==row.payload_json["user_id"] => {
                    if matches!(change.route.as_str(),"create-view"|"smart-split-create") {
                        let mut payload=row.payload_json.clone();payload["attempted"]=json!(true);
                        self.conn.execute("UPDATE mutation_queue SET payload_json=? WHERE id=?",params![payload.to_string(),row.id])?;
                    }
                    {
                        let(method,path,body)=change.request()?;
                        if method=="POST"{transport.post(&path,&body,&row.client_id)}else{transport.mutate(method,&path,&body,&row.client_id)}
                    }
                },
                Err(error) => Err(error),
                Ok(identity) if !(200..300).contains(&identity.status) => Ok(identity),
                _ => Ok(Response{status:403,body:json!({"message":"Forbidden"})}),
            }}.map(|r|if (200..300).contains(&r.status) && r.body["success"] != false {Response{status:r.status,body:json!({"success":true,"view":r.body})}}else{r})
        } else if row.kind == "web_favorite_slot" {
            match transport.get("/user/context") {
                Ok(identity) if identity.status==200 && identity.body["success"]==true && self.current_user_id()==row.payload_json["user_id"].as_i64() && identity.body["user"]["id"]==row.payload_json["user_id"] => transport.mutate(row.payload_json["method"].as_str().context("Missing favorite method")?,"/api/favorites/addEditFavorites",&row.payload_json["body"],&row.client_id),
                Err(error) => Err(error),
                Ok(identity) if !(200..300).contains(&identity.status) => Ok(identity),
                _ => Ok(Response{status:403,body:json!({"message":"Forbidden"})}),
            }.map(|r|if r.status==200 && (r.body["count"].is_number() || r.body["projectId"]==row.payload_json["body"]["projectId"]) {Response{status:200,body:json!({"success":true})}}else{r})
        } else if row.kind == "web_favorites" {
            match transport.get("/user/context") {
                Ok(identity) if identity.status==200 && identity.body["success"]==true && self.current_user_id()==row.payload_json["user_id"].as_i64() && identity.body["user"]["id"]==row.payload_json["user_id"] => transport.mutate("PUT","/api/favorites/reorderFavorites",&row.payload_json["body"],&row.client_id),
                Err(error) => Err(error),
                Ok(identity) if !(200..300).contains(&identity.status) => Ok(identity),
                _ => Ok(Response{status:403,body:json!({"message":"Forbidden"})}),
            }.map(|r|if r.status==200 && r.body["ok"]==true {Response{status:200,body:json!({"success":true})}}else{r})
        } else if row.kind == "web_create" {
            // Web creation has no idempotency contract: a lost acknowledgement must not create twice.
            if row.payload_json["attempted"]==true || (row.payload_json.get("attempted").is_none() && row.attempts>1) { Ok(Response {status:409,body:json!({"message":"Creation result uncertain; refresh before trying again"})}) }
            else {
                match transport.get("/user/context") {
                    Ok(identity) if identity.status == 200 && identity.body["success"] == true && self.current_user_id() == row.payload_json["user_id"].as_i64() && identity.body["user"]["id"] == row.payload_json["user_id"] => {
                        let path = if row.payload_json["global"] == true {"/api/tasks/createGlobally"}else{"/api/tasks/create"};
                        let mut payload=row.payload_json.clone();payload["attempted"]=json!(true);
                        self.conn.execute("UPDATE mutation_queue SET payload_json=? WHERE id=?",params![payload.to_string(),row.id])?;
                        match transport.post(path, &row.payload_json["body"], &row.client_id) {
                            Ok(response) if response.status < 500 => Ok(response),
                            _ => Ok(Response {status:409,body:json!({"message":"Creation result uncertain; refresh before trying again"})}),
                        }
                    },
                    Err(error) => Err(error),
                    Ok(identity) if !(200..300).contains(&identity.status) => Ok(identity),
                    _ => Ok(Response {status:403,body:json!({"message":"Forbidden"})}),
                }
            }
        } else if row.kind == "compose_task" {
            crate::desktop_cli::compose(self, transport, &row.payload_json, &row.client_id)
        } else if row.kind == "board_write" {
            let change: crate::BoardWrite = serde_json::from_value(row.payload_json["change"].clone())?;
            let (method, path, body) = change.request()?;
            match transport.get("/user/context") {
                Ok(response) if (200..300).contains(&response.status) && response.body["success"] == true => {
                    if response.body["user"]["id"] == row.payload_json["user_id"] {
                        transport.mutate(method, &path, &body, &row.client_id)
                    } else { Ok(Response { status:403, body:json!({"success":false,"error":"Board write belongs to another account"}) }) }
                }
                response => response,
            }
        } else if row.kind == "shell_write" {
            let request: crate::ShellWrite = serde_json::from_value(row.payload_json.clone())?;
            request.validate()?;
            let (method, path, body) = request.request();
            match transport.get("/user/context") {
                Ok(response) if (200..300).contains(&response.status) && response.body["success"] == true => {
                    if response.body["user"]["id"] == request.user_id && (!path.starts_with("/api/") || self.current_user_id() == Some(request.user_id)) {
                        if path == "/api/stripe/session/checkout" {
                            if row.payload_json["attempted"] == true { Ok(Response { status:409, body:json!({"message":"Checkout result uncertain; try again."}) }) }
                            else {
                                let mut payload = row.payload_json.clone(); payload["attempted"] = json!(true);
                                self.conn.execute("UPDATE mutation_queue SET payload_json=? WHERE id=?", params![payload.to_string(), row.id])?;
                                transport.mutate(method, path, body, &row.client_id)
                            }
                        } else { transport.mutate(method, path, body, &row.client_id) }
                    } else {
                        Ok(Response { status:403, body:json!({"success":false,"error":"Settings write belongs to another account"}) })
                    }
                }
                response => response,
            }
        } else if row.kind == "draft_discard" {
            crate::editor_queue::discard(transport,&row.payload_json,&row.client_id)
        } else if row.kind == "comment_draft" {
            if self.current_user_id() != row.payload_json["user_id"].as_i64() {
                Ok(Response {
                    status: 403,
                    body: json!({"success":false,"error":"Draft belongs to another account"}),
                })
            } else {
                crate::comment_draft::send_draft(transport, &row.payload_json, &row.client_id)
            }
        } else if row.kind == "comment_action" {
            let change: crate::CommentWrite =
                serde_json::from_value(row.payload_json["change"].clone())?;
            if let crate::CommentWrite::Saved { public } = change {
                // toggleSaved has no idempotency support: never replay an uncertain toggle.
                if row.attempts > 1 { Ok(Response { status: 409, body: json!({"success":false}) }) }
                else if self.current_user_id() != row.payload_json["user_id"].as_i64() {
                    Ok(Response { status:403, body:json!({"success":false}) })
                } else {
                    let request = crate::desktop_cli::saved_request(
                        row.payload_json["task_id"].as_i64().unwrap(),
                        row.payload_json["before"]["projectId"].as_i64().or_else(|| self.task(row.payload_json["task_id"].as_i64().unwrap()).ok().flatten().map(|t|t.project_id)).unwrap_or(0),
                        row.payload_json["comment_id"].as_i64().unwrap(), public, false);
                    let verified = match transport.get("/user/context") {
                        Ok(response) if (200..300).contains(&response.status) && response.body["success"] == true && response.body["user"]["id"] == row.payload_json["user_id"] => true,
                        _ => false,
                    };
                    let response = if verified {transport.mutate(&request.method, &request.path, &request.body, &row.client_id)}
                        else {Ok(Response {status:403,body:json!({"success":false})})};
                    match response {
                        Ok(Response {status:200, body}) if body["saved"]["commentId"] == row.payload_json["comment_id"] => Ok(Response {status:200,body:json!({"success":true,"active":true,"saved":body["saved"]})}),
                        Ok(Response {status:201, body}) if body.as_object().is_some_and(|v|v.is_empty()) => Ok(Response {status:201,body:json!({"success":true,"active":false})}),
                        Ok(Response {status, ..}) if (400..500).contains(&status) => Ok(Response {status,body:json!({"success":false})}),
                        _ => Ok(Response {status:409,body:json!({"success":false,"error":"Saved-comment result uncertain; refresh before trying again"})}),
                    }
                }
            } else {
                let (method, path, body) = change.request(row.payload_json["comment_id"].as_i64().context("missing comment id")?);
                transport.mutate(method, &path, &body, &row.client_id)
            }
        } else {
            transport.post(path, &body, &row.client_id)
        };
        let response = if row.kind == "web_create" {
            response.map(|r| if (200..300).contains(&r.status) {
                let task = if row.payload_json["global"] == true {r.body["newTask"].clone()}else{r.body};
                Response { status:r.status,body:json!({"success":true,"task":task}) }
            } else {r})
        } else if row.kind == "shell_write" && row.payload_json["path"].as_str().is_some_and(|path| path.starts_with("/api/")) {
            response.map(|mut response| {
                let valid = match row.payload_json["path"].as_str() {
                    Some("/api/teams/byokKeys") => response.body["ok"] == true,
                    Some("/api/settings/byok-test") => matches!(response.body["result"].as_str(), Some("works" | "invalid" | "no_credit" | "error")) && response.body["elapsedMs"].is_number() && response.body["model"].is_string(),
                    Some("/api/stripe/ensure-customer") => response.body["stripe_customer_id"].is_string(),
                    Some("/api/stripe/session/checkout") => response.body["url"].as_str().is_some_and(|url| url::Url::parse(url).is_ok_and(|url| url.scheme() == "https")),
                    Some("/api/stripe/cancelSubscription") => response.body["message"] == "Success",
                    _ => false,
                };
                if (200..300).contains(&response.status) { let success = valid && response.body["success"] != false; if response.body.is_object() { response.body["success"] = json!(success); } }
                response
            })
        } else if row.kind == "board_write" && row.payload_json["change"]["PositionTask"].is_object() {
            response.map(|response| if (200..300).contains(&response.status) {
                Response { status: response.status, body: json!({"success":true,"task":response.body}) }
            } else { response })
        } else { response };
        let response_status = response.as_ref().ok().map(|response| response.status);
        let result = match response {
            Ok(Response { status, body }) if (200..300).contains(&status) => {
                match ensure_success(&body)
                    .and_then(|()| self.resolve_server_authoritative(&row, &body, now))
                {
                    Ok(()) => return Ok(Drain::Applied(row.id)),
                    Err(_) => (false, "invalid success response".to_owned()),
                }
            }
            Ok(Response { status, body }) => (permanent(status), if row.kind=="web_view" {body["message"].as_str().unwrap_or("Could not save the view").to_owned()} else if row.kind=="shell_write" && row.payload_json["path"].as_str().is_some_and(|path| path.starts_with("/api/")) {body["message"].as_str().or(body["error"].as_str()).map(str::to_owned).unwrap_or_else(|| format!("HTTP {status}"))} else {format!("HTTP {status}")}),
            Err(error) => (false, error.to_string()),
        };
        self.transaction(|db| {
            let current: Option<String> = db.conn.query_row("SELECT client_id FROM mutation_queue WHERE id=? AND attempts=?", params![row.id,row.attempts], |r| r.get(0)).optional()?;
            if current.is_none() { return Ok(()); }
            db.conn.execute("UPDATE mutation_queue SET status=?,last_error=?,next_attempt_at=?,lease_until=NULL WHERE id=? AND attempts=?",
                params![if result.0 { "failed" } else { "pending" },result.1,now+backoff(row.attempts-1),row.id,row.attempts])?;
            if result.0 {
                if row.kind == "web_preferences" {
                    db.rollback_preferences(&row.payload_json)?;
                    db.kv_set(&format!("preference-result:{}",row.client_id),&json!({"error":result.1}).to_string())?;
                }
                if row.kind == "ai_session" {
                    db.rollback_ai_session(&row.payload_json)?;
                    db.kv_set(&format!("ai-session-result:{}",row.client_id),&json!({"error":result.1}).to_string())?;
                }
                if row.kind == "web_view" {
                    if db.current_user_id()==row.payload_json["user_id"].as_i64(){
                        let before=&row.payload_json["before"];
                        let mut current=db.board_project(before["id"].as_i64().context("missing board")?)?;
                        current["project_view"]=before["project_view"].clone();
                        if row.payload_json["change"]["route"].as_str().is_some_and(|route| route.starts_with("smart-split-")) { current["desktop_smart_labels"] = before["desktop_smart_labels"].clone(); }
                        db.upsert("Project",&current)?;db.reapply_pending()?;
                    }
                    db.kv_set(&format!("view-result:{}",row.client_id),&json!({"error":result.1}).to_string())?;
                }
                if matches!(row.kind.as_str(), "web_favorites" | "web_favorite_slot") {
                    let user=row.payload_json["user_id"].as_i64().context("missing account")?;
                    db.kv_set(&crate::board_sidebar::cache_key(user,"favorites"),&row.payload_json["before"].to_string())?;
                    db.reapply_pending()?;
                }
                if row.kind == "web_create" {
                    db.conn.execute("DELETE FROM Task WHERE id=?", [-row.id])?;
                    db.kv_set(&format!("create-result:{}",row.client_id), &json!({"error":result.1}).to_string())?;
                }
                if row.kind == "compose_task" { db.kv_set(&format!("compose-result:{}",row.client_id), &json!({"error":result.1}).to_string())?; }
                if row.kind == "board_write" {
                    if !row.payload_json["before"].is_null() {
                        if let Some(task)=row.payload_json["change"]["PositionTask"]["task"].as_i64(){
                            let mut current=db.raw("Task",&json!(task))?.context("task is not cached")?;
                            for field in ["sectionId","ranking"]{current[field]=row.payload_json["before"][field].clone();}
                            current["section"]=json!(row.payload_json["before"]["section"].as_str().unwrap_or(""));
                            db.upsert_task(&current)?;
                        }else{db.upsert_task(&row.payload_json["before"])?;}
                        db.reapply_pending()?;
                    }
                    db.kv_set(&format!("board-result:{}", row.client_id), &json!({"error":result.1}).to_string())?;
                }
                if row.kind == "shell_write" {
                    if row.payload_json["body"]["apiKey"].is_string() {
                        let mut payload = row.payload_json.clone(); payload["body"].as_object_mut().unwrap().remove("apiKey");
                        db.conn.execute("UPDATE mutation_queue SET payload_json=? WHERE id=?", params![payload.to_string(), row.id])?;
                    }
                    db.kv_set(&format!("shell-result:{}", row.client_id), &json!({"error":result.1,"status":response_status}).to_string())?;
                }
                db.fail_editor(row.id,&row.kind,&row.payload_json,&result.1)?;
                if matches!(row.kind.as_str(), "inbox_seen" | "inbox_remind") {
                    for before in row.payload_json["before"].as_array().context("missing inbox rollback")? {
                        let mut current = db.raw("Notification", &before["id"])?.context("notification is not cached")?;
                        // Upserts merge missing fields: explicitly clear optimistic fields absent in the snapshot.
                        let fields = if row.kind == "inbox_seen" { ["seen", "unreadCount"] } else { ["status", "archivedAt"] };
                        for field in fields { current[field] = before[field].clone(); }
                        db.upsert("Notification", &current)?;
                    }
                    db.reapply_pending()?;
                }
                if row.kind == "comment_action" {
                    if row.payload_json["change"]["action"]=="saved" {db.kv_set(&format!("comment-saved-result:{}",row.client_id),&json!({"error":result.1}).to_string())?;}
                    db.upsert_comment(row.payload_json["task_id"].as_i64().context("missing task id")?, &row.payload_json["before"])?;
                    db.request_refetch("comments", row.payload_json["task_id"].as_i64().unwrap())?;
                    db.reapply_pending()?;
                }
                if row.kind == "task_comment" {
                    db.restore_failed_comment_draft(&row.payload_json)?;
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

    fn resolve_server_authoritative(&mut self, row: &Mutation, body: &Value, now:i64) -> Result<()> {
        self.transaction(|db| {
            let active: bool = db.conn.query_row("SELECT EXISTS(SELECT 1 FROM mutation_queue WHERE id=? AND attempts=? AND status='inflight')", params![row.id,row.attempts], |r| r.get(0))?;
            if !active { return Ok(()); }
            match row.kind.as_str() {
                "web_preferences" => {
                    db.cache_web_preferences(row.payload_json["user_id"].as_i64().context("Missing preference account")?,&body["settings"])?;
                    db.kv_set(&format!("preference-result:{}",row.client_id),&body.to_string())?;
                }
                "ai_session" => {
                    db.resolve_ai_session(&row.payload_json,body)?;
                    db.kv_set(&format!("ai-session-result:{}",row.client_id),&body.to_string())?;
                }
                "web_view" => {
                    let change:crate::ViewWrite=serde_json::from_value(row.payload_json["change"].clone())?;
                    let view=&body["view"];
                    if change.smart_split() {
                        let mut project = db.board_project(change.project)?;
                        change.resolve_smart_split(&mut project, view)?;
                        db.upsert("Project", &project)?;
                        db.request_refetch("board", change.project)?;
                    }else if change.route=="update-view" && change.body["updateMode"]=="personal-empty-sections"{
                        ensure!(view["viewId"]==change.body["viewId"] && matches!(view["board_empty_sections"].as_str(),Some("Show"|"Hidden")),"Wrong empty-column acknowledgement");
                        let mut project=db.board_project(change.project)?;let mut authoritative=change.clone();authoritative.body["view_settings"]["board_empty_sections"]=view["board_empty_sections"].clone();authoritative.optimistic(&mut project);db.upsert("Project",&project)?;
                    }else if matches!(change.route.as_str(),"update-order"|"reset-order"|"set-default-order") {
                        let mut project=db.board_project(change.project)?;
                        match change.route.as_str(){
                            "set-default-order"=>{ensure!(view["defaultViewOrder"].is_array(),"Missing default order acknowledgement");project["project_view"]["default_view_order"]=view["defaultViewOrder"].clone();}
                            _=>{ensure!(view.get("viewOrder").is_some(),"Missing view order acknowledgement");project["project_view"]["user_project_views"][0]["view_order"]=view["viewOrder"].clone();}
                        }
                        db.upsert("Project",&project)?;
                    } else {
                        let view=if matches!(change.route.as_str(),"create-view"|"rename-view"){view.get("project_view_updated").unwrap_or(view)}else{view};
                        ensure!(view["projectId"]==change.project && view["id"].is_string(),"Wrong view acknowledgement");
                        let mut project=db.board_project(change.project)?;project["project_view"]=view.clone();db.upsert("Project",&project)?;
                    }
                    db.kv_set(&format!("view-result:{}",row.client_id),&body.to_string())?;
                }
                "web_favorites" | "web_favorite_slot" => (),
                "web_create" => {
                    let task = &body["task"];
                    ensure!(task["id"].as_i64().is_some_and(|id|id>0) && task["projectId"] == row.payload_json["body"]["projectId"], "invalid created task");
                    db.upsert_task(task)?;
                    db.conn.execute("DELETE FROM Task WHERE id=?",[-row.id])?;
                    db.kv_set(&format!("create-result:{}",row.client_id), &body.to_string())?;
                }
                "compose_task" => {
                    let tasks = body["tasks"].as_array().context("missing composed tasks")?;
                    ensure!(!tasks.is_empty(), "no tasks created");
                    for task in tasks { db.upsert_task(task)?; }
                    db.kv_set(&format!("compose-result:{}",row.client_id), &body.to_string())?;
                }
                "board_write" => {
                    let change: crate::BoardWrite = serde_json::from_value(row.payload_json["change"].clone())?;
                    change.resolve(db, body, now)?;
                    db.kv_set(&format!("board-result:{}", row.client_id), &json!({"value":body}).to_string())?;
                }
                "shell_write" => {
                    // Preserve the acknowledgement until the initiating form consumes it.
                    db.kv_set(&format!("shell-result:{}", row.client_id), &json!({"value":body}).to_string())?;
                    if row.payload_json["path"] == "/projects/archive" {
                        let project = body.get("project").context("archive response missing project")?;
                        ensure!(project["id"] == row.payload_json["body"]["project_id"], "archive returned another board");
                        db.upsert("Project", project)?;
                    }
                }
                "editor_improve"|"draft_discard"=>db.resolve_editor(row.id,&row.kind,&row.payload_json,body)?,
                "comment_draft" => db.resolve_comment_draft(&row.payload_json, body)?,
                "comment_action" => {
                    db.resolve_comment_action(&row.payload_json, body)?;
                    if row.payload_json["change"]["action"]=="saved" {db.kv_set(&format!("comment-saved-result:{}",row.client_id),&body.to_string())?;}
                }
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
                    db.clear_sent_comment_draft(&row.payload_json, row.created_at)?;
                    db.request_refetch("task", row.payload_json["task_id"].as_i64().unwrap())?;
                }
                "inbox_seen" => {
                    ensure!(body["seen_count"].as_u64().is_some_and(|n| n <= 1), "seen response missing count");
                    db.request_refetch("inbox", 0)?;
                }
                "inbox_remind" => {
                    let count = row.payload_json["notification_ids"].as_array().context("missing notification ids")?.len() as u64;
                    ensure!(body["archived_count"].as_u64() == Some(count) && body["reminder_count"].as_u64().is_some_and(|n| n > 0 && n <= count), "reminder response missing counts");
                    db.request_refetch("inbox", 0)?;
                }
                "inbox_archive" => {
                    ensure!(body["archived_count"].as_u64().is_some(), "archive response missing count");
                    db.request_refetch("inbox", 0)?;
                }
                _ => bail!("unknown mutation kind"),
            }
            if let Some(task_id)=row.payload_json["task_id"].as_i64() {
                let kind=if row.kind=="task_comment" && db.comment_draft(task_id)?.is_none_or(|html| html.is_empty()){Some(crate::DraftType::Comment)}else if row.kind=="task_patch"&&row.payload_json["description"].is_string(){Some(crate::DraftType::Description)}else{None};
                if let Some(draft_type)=kind {
                    if db.editor_drafts(task_id)?.iter().any(|d|d["draftType"]==draft_type.as_str()) {
                        db.enqueue_in_transaction(Write::DiscardDraft{task_id,draft_type},now)?;
                    }
                }
            }
            db.conn.execute("DELETE FROM mutation_queue WHERE id=? AND attempts=?", params![row.id,row.attempts])?;
            db.reapply_pending()?;
            Ok(())
        })
    }
}
