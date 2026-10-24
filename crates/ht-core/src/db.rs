use crate::{Result, generated::MODEL_COLUMNS};
use anyhow::{Context, bail, ensure};
use rusqlite::{Connection, OptionalExtension, params, types::Value as SqlValue};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{path::Path, time::Duration};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskView {
    pub id: i64,
    pub title: String,
    pub project_id: i64,
    #[serde(default)]
    pub ticket_number: Option<String>,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub section: String,
    #[serde(default)]
    pub section_id: Option<i64>,
    #[serde(default)]
    pub board_title: String,
    pub status: crate::generated::Status,
    #[serde(default)]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub due_date: Option<String>,
    #[serde(default)]
    pub comment_count: i64,
    #[serde(default)]
    pub assignee_count: i64,
    #[serde(default)]
    pub label_count: i64,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommentView {
    pub id: i64,
    pub task_id: i64,
    pub text: String,
    #[serde(default)]
    pub comment_text: String,
    pub created_at: String,
    #[serde(default)]
    pub creator_id: Option<i64>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InboxItem {
    pub id: i64,
    pub task_id: Option<i64>,
    pub title: String,
    pub ticket_number: Option<String>,
    pub source: Option<String>,
    pub seen: bool,
    pub created_at: String,
    pub raw_json: Value,
}

#[derive(Debug, Serialize)]
pub struct BoardColumn {
    pub id: i64,
    pub title: String,
    pub ranking: String,
    pub cards: Vec<TaskView>,
}

#[derive(Debug, Serialize)]
pub struct TaskDetail {
    pub task: TaskView,
    pub comments: Vec<CommentView>,
}

pub struct Db {
    pub(crate) conn: Connection,
}

impl Db {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        Self::from_connection(Connection::open(path)?)
    }

    pub fn memory() -> Result<Self> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    fn from_connection(conn: Connection) -> Result<Self> {
        conn.busy_timeout(Duration::from_secs(5))?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        if cfg!(feature = "bounded-sync-cache") {
            // SQLite's soft page target excludes pager overhead; leave room for it.
            conn.execute_batch("PRAGMA cache_size=-256;")?;
        }
        conn.execute_batch(crate::generated::SQLITE_DDL)?;
        conn.execute_batch(
            "CREATE INDEX IF NOT EXISTS task_board ON Task(projectId, status, sectionId, updatedAt);
             CREATE INDEX IF NOT EXISTS comment_task ON Comment(taskId, createdAt);
             CREATE INDEX IF NOT EXISTS notification_inbox ON Notification(status, archivedAt, createdAt);
             CREATE TABLE IF NOT EXISTS sync_state(project_id INTEGER NOT NULL, status TEXT NOT NULL, cursor TEXT, PRIMARY KEY(project_id,status));
             CREATE TABLE IF NOT EXISTS mutation_queue(
               id INTEGER PRIMARY KEY AUTOINCREMENT, client_id TEXT NOT NULL UNIQUE,
               entity TEXT NOT NULL, kind TEXT NOT NULL, payload_json TEXT NOT NULL,
               created_at INTEGER NOT NULL, attempts INTEGER NOT NULL DEFAULT 0,
               last_error TEXT, status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending','inflight','failed')),
               next_attempt_at INTEGER NOT NULL, lease_until INTEGER);
             CREATE TABLE IF NOT EXISTS kv(key TEXT PRIMARY KEY, value TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS refetch_queue(target TEXT NOT NULL, entity_id INTEGER NOT NULL, generation INTEGER NOT NULL DEFAULT 1, PRIMARY KEY(target,entity_id));",
        )?;
        Ok(Self { conn })
    }

    pub(crate) fn transaction<T>(&mut self, f: impl FnOnce(&mut Self) -> Result<T>) -> Result<T> {
        self.conn.execute_batch("BEGIN IMMEDIATE")?;
        match f(self) {
            Ok(value) => {
                if let Err(error) = self.conn.execute_batch("COMMIT") {
                    let _ = self.conn.execute_batch("ROLLBACK");
                    return Err(error.into());
                }
                Ok(value)
            }
            Err(error) => {
                self.conn.execute_batch("ROLLBACK")?;
                Err(error)
            }
        }
    }

    pub(crate) fn raw(&self, model: &str, id: &Value) -> Result<Option<Value>> {
        ensure!(
            MODEL_COLUMNS.iter().any(|(name, _)| *name == model),
            "unknown model"
        );
        let text: Option<String> = self
            .conn
            .query_row(
                &format!("SELECT raw_json FROM \"{model}\" WHERE id=?"),
                [sql_value(id)],
                |r| r.get(0),
            )
            .optional()?;
        text.map(|s| serde_json::from_str(&s).map_err(Into::into))
            .transpose()
    }

    pub(crate) fn upsert(&self, model: &str, value: &Value) -> Result<()> {
        let (_, columns) = MODEL_COLUMNS
            .iter()
            .find(|(name, _)| *name == model)
            .context("unknown model")?;
        let id = value.get("id").context("missing entity id")?;
        ensure!(id.is_i64() || id.is_string(), "invalid entity id");
        let mut merged = self.raw(model, id)?.unwrap_or(json!({}));
        for (key, value) in value.as_object().context("entity must be an object")? {
            merged[key] = value.clone();
        }
        let mut names = Vec::new();
        let mut values = Vec::new();
        for &col in *columns {
            if let Some(value) = merged.get(col) {
                names.push(format!("\"{col}\""));
                values.push(sql_value(value));
            }
        }
        names.push("raw_json".into());
        values.push(SqlValue::Text(serde_json::to_string(&merged)?));
        let updates: Vec<_> = names
            .iter()
            .filter(|n| *n != "\"id\"")
            .map(|n| format!("{n}=excluded.{n}"))
            .collect();
        let sql = format!(
            "INSERT INTO \"{model}\" ({}) VALUES ({}) ON CONFLICT(id) DO UPDATE SET {}",
            names.join(","),
            vec!["?"; values.len()].join(","),
            updates.join(",")
        );
        self.conn
            .execute(&sql, rusqlite::params_from_iter(values))?;
        Ok(())
    }

    pub fn apply_task(&mut self, value: &Value) -> Result<()> {
        self.transaction(|db| {
            db.upsert_task(value)?;
            db.reapply_pending()
        })
    }

    pub(crate) fn upsert_task(&self, value: &Value) -> Result<()> {
        let mut task = value.clone();
        let _: TaskView = task_view(value.clone())?;
        if task.get("sectionId").is_none() && task.get("section").is_some() {
            task["sectionId"] = Value::Null;
        }
        if let Some(count) = task.get("commentCount").cloned() {
            task["totalComments"] = count;
        } else if let Some(count) = task.get("totalComments").cloned() {
            task["commentCount"] = count;
        }
        for (array, count) in [("assignees", "assigneeCount"), ("labels", "labelCount")] {
            if task.get(count).is_none() {
                if let Some(items) = task[array].as_array() {
                    task[count] = json!(items.len());
                }
            }
        }
        self.upsert("Task", &task)?;
        for label in value["labels"].as_array().into_iter().flatten() {
            let mut row = label.clone();
            row["value"] = label.get("name").cloned().unwrap_or(Value::Null);
            row["projectId"] = value["projectId"].clone();
            self.upsert("Label", &row)?;
        }
        Ok(())
    }

    pub(crate) fn upsert_comment(&self, task_id: i64, value: &Value) -> Result<()> {
        let mut comment = value.clone();
        comment["taskId"] = json!(task_id);
        let _: CommentView = serde_json::from_value(comment.clone())?;
        self.upsert("Comment", &comment)
    }

    pub fn apply_comments(&mut self, task_id: i64, response: &Value) -> Result<()> {
        ensure_success(response)?;
        self.transaction(|db| {
            for c in array(response, "comments")? {
                db.upsert_comment(task_id, c)?;
            }
            Ok(())
        })
    }

    pub fn apply_response(&mut self, response: &Value) -> Result<()> {
        ensure_success(response)?;
        self.transaction(|db| {
            let mut recognized = false;
            for (key, model) in [
                ("projects", "Project"),
                ("sections", "Section"),
                ("tasks", "Task"),
                ("user_notifications", "Notification"),
                ("agent_notifications", "Notification"),
            ] {
                if let Some(items) = response.get(key) {
                    recognized = true;
                    for value in items.as_array().context("expected array")? {
                        if model == "Task" {
                            db.upsert_task(value)?;
                        } else {
                            db.upsert(model, value)?;
                        }
                    }
                }
            }
            if let Some(task) = response.get("task") {
                recognized = true;
                db.upsert_task(task)?;
            }
            db.cache_inbox_structure(response)?;
            ensure!(recognized, "unrecognized response");
            db.reapply_pending()
        })
    }

    pub fn task(&self, id: i64) -> Result<Option<TaskView>> {
        self.raw("Task", &json!(id))?.map(task_view).transpose()
    }

    pub fn task_detail(&self, id: i64) -> Result<Option<TaskDetail>> {
        let Some(task) = self.task(id)? else {
            return Ok(None);
        };
        let comments = self
            .json_rows(
                "SELECT raw_json FROM Comment WHERE taskId=? ORDER BY createdAt DESC,id DESC",
                [id],
            )?
            .into_iter()
            .map(serde_json::from_value)
            .collect::<std::result::Result<_, _>>()?;
        Ok(Some(TaskDetail { task, comments }))
    }

    pub fn board(&self, project_id: i64) -> Result<Vec<BoardColumn>> {self.board_in_view(project_id,None)}
    pub fn board_in_view(&self,project_id:i64,builtin:Option<&str>)->Result<Vec<BoardColumn>>{
        let mut sections = self.json_rows("SELECT raw_json FROM Section WHERE projectId=? AND deleted=0 AND visibility=1 ORDER BY ranking,id", [project_id])?;
        let project=self.board_project(project_id).unwrap_or(Value::Null);
        let mut settings=crate::board_views::settings(&project);
        if builtin.is_some(){settings["board_filters"]=json!({"addedFilters":[],"matchFilters":"ANY"});}
        if let Some(columns)=settings["board_columns_view"].as_array().filter(|_|project.is_object()){
            sections.retain(|section|columns.iter().any(|column|column.get("id").or_else(||column.get("sectionId"))==section.get("id") && column["visibility"]==true));
        }
        let mut tasks = self.json_rows("SELECT raw_json FROM Task WHERE projectId=? AND status='Normal' ORDER BY ranking ASC,updatedAt DESC,id ASC", [project_id])?;
        let running=self.kv_get(&format!("web-running-timers:{}:{project_id}",self.current_user_id().unwrap_or(0)))?.and_then(|value|serde_json::from_str::<Vec<Value>>(&value).ok()).unwrap_or_default();
        if let Some(builtin)=builtin{tasks.retain(|task|crate::board_views::builtin_matches(builtin,task,&project,self.current_user_id(),chrono::Utc::now().timestamp()));}
        crate::board_views::filter_sort(&mut tasks,&settings,&project,&running);
        tasks.retain(|task|match settings["board_subtask_setting"].as_str(){Some("None"|"Parent")=>task["parentTask"].is_null(),Some("Card")=>task["parentTask"].is_null() || task["subTasks"].as_array().is_some_and(|subtasks|!subtasks.is_empty()),_=>true});
        let show_archived = settings["board_show_archived"].as_bool().unwrap_or(self.kv_get("showArchivedOnBoard")?.as_deref() == Some("true"));
        if show_archived {
            let mut archived = self.json_rows("SELECT raw_json FROM Task WHERE projectId=? AND status='Archive'", [project_id])?;
            if let Some(ids) = self.kv_get(&format!("web-archived-tasks:{}:{project_id}", self.current_user_id().unwrap_or(0)))?.and_then(|value| serde_json::from_str::<Vec<Value>>(&value).ok()) {
                archived.retain(|task| ids.contains(&task["id"]));
            }
            crate::board_views::sort_archived(&mut archived, &settings, &project);
            tasks.extend(archived);
        }
        let cards = tasks
            .into_iter()
            .map(task_view)
            .collect::<Result<Vec<_>>>()?;
        sections
            .into_iter()
            .filter(|section|(settings["board_empty_sections"]!="Hidden" && builtin!=Some("builtin:decisions")) || cards.iter().any(|task|task.section_id==section["id"].as_i64() || (task.section_id.is_none() && task.section==section["section_title"].as_str().unwrap_or(""))))
            .map(|s| {
                let id = integer(&s, "id")?;
                let title = s["section_title"]
                    .as_str()
                    .context("section title missing")?
                    .to_owned();
                Ok(BoardColumn {
                    id,
                    ranking: s["ranking"]
                        .as_str()
                        .context("section ranking missing")?
                        .to_owned(),
                    cards: cards
                        .iter()
                        .filter(|t| {
                            t.section_id == Some(id)
                                || (t.section_id.is_none() && t.section == title)
                        })
                        .cloned()
                        .collect(),
                    title,
                })
            })
            .collect()
    }

    pub fn inbox(&self) -> Result<Vec<InboxItem>> {
        self.json_rows("SELECT n.raw_json FROM Notification n LEFT JOIN Task t ON t.id=n.taskId WHERE n.status='Normal' AND n.archivedAt IS NULL AND (t.id IS NULL OR t.status='Normal') ORDER BY n.createdAt DESC,n.id DESC", [])?
            .into_iter().map(|n| {
                let actors = n["recentActors"].as_array();
                let source = if actors.is_some_and(|actors| actors.len() >= 2) {
                    Some(actors.unwrap().iter().filter_map(|actor| actor["displayName"].as_str())
                        .collect::<Vec<_>>().join(", "))
                } else {
                    ["/fromAgent/displayName", "/fromUser/displayName"].into_iter()
                        .find_map(|path| n.pointer(path).and_then(Value::as_str)).map(str::to_owned)
                };
                let task = n["taskId"].as_i64().map(|id| self.task(id)).transpose()?.flatten();
                let title = task.as_ref().map(|t| t.title.clone()).unwrap_or_else(|| n.pointer("/task/title").or_else(|| n.get("message")).and_then(Value::as_str).unwrap_or("").to_owned());
                let ticket_number = task.and_then(|t| t.ticket_number).or_else(|| n.pointer("/task/ticketNumber").and_then(Value::as_str).map(str::to_owned));
                Ok(InboxItem { id: integer(&n,"id")?, task_id: n["taskId"].as_i64(), title,
                    ticket_number, source,
                    seen: n["seen"].as_bool().unwrap_or(false), created_at: n["createdAt"].as_str().context("notification createdAt missing")?.to_owned(), raw_json: n })
            }).collect()
    }

    pub fn dump(&self) -> Result<Value> {
        let mut out = json!({});
        for &(model, _) in MODEL_COLUMNS {
            out[model] = json!(
                self.json_rows(&format!("SELECT raw_json FROM \"{model}\" ORDER BY id"), [])?
            );
        }
        out["mutation_queue"] = json!(self.mutations()?);
        Ok(out)
    }

    pub(crate) fn json_rows<const N: usize>(
        &self,
        sql: &str,
        params: [i64; N],
    ) -> Result<Vec<Value>> {
        let mut stmt = self.conn.prepare(sql)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(params), |r| {
            r.get::<_, String>(0)
        })?;
        rows.map(|r| Ok(serde_json::from_str(&r?)?)).collect()
    }

    pub fn kv_get(&self, key: &str) -> Result<Option<String>> {
        Ok(self
            .conn
            .query_row("SELECT value FROM kv WHERE key=?", [key], |r| r.get(0))
            .optional()?)
    }

    pub fn kv_set(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO kv(key,value) VALUES(?,?) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            [key, value],
        )?;
        Ok(())
    }

    pub fn first_project_id(&self) -> Result<Option<i64>> {
        Ok(self
            .conn
            .query_row("SELECT id FROM Project ORDER BY rowid LIMIT 1", [], |r| r.get(0))
            .optional()?)
    }

    pub fn cursor(&self, project_id: i64, status: &str) -> Result<Option<String>> {
        Ok(self
            .conn
            .query_row(
                "SELECT cursor FROM sync_state WHERE project_id=? AND status=?",
                params![project_id, status],
                |r| r.get::<_, Option<String>>(0),
            )
            .optional()?
            .flatten())
    }

    pub(crate) fn request_refetch(&self, target: &str, id: i64) -> Result<()> {
        self.conn.execute("INSERT INTO refetch_queue(target,entity_id) VALUES(?,?) ON CONFLICT(target,entity_id) DO UPDATE SET generation=generation+1", params![target,id])?;
        Ok(())
    }

    pub fn refetches(&self) -> Result<Vec<(String, i64, i64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT target,entity_id,generation FROM refetch_queue ORDER BY target,entity_id",
        )?;
        Ok(stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
            .collect::<std::result::Result<_, _>>()?)
    }
}

pub(crate) fn ensure_success(value: &Value) -> Result<()> {
    ensure!(value["success"] == true, "API response was not successful");
    Ok(())
}

pub(crate) fn array<'a>(value: &'a Value, key: &str) -> Result<&'a Vec<Value>> {
    value[key]
        .as_array()
        .with_context(|| format!("missing {key} array"))
}

pub(crate) fn integer(value: &Value, key: &str) -> Result<i64> {
    value[key]
        .as_i64()
        .with_context(|| format!("missing integer {key}"))
}

pub(crate) fn task_view(mut value: Value) -> Result<TaskView> {
    if value.get("commentCount").is_none() {
        value["commentCount"] = value.get("totalComments").cloned().unwrap_or(json!(0));
    }
    for (array, count) in [("assignees", "assigneeCount"), ("labels", "labelCount")] {
        if value.get(count).is_none() {
            value[count] = json!(value[array].as_array().map_or(0, Vec::len));
        }
    }
    let task: TaskView = serde_json::from_value(value)?;
    if task.id == 0 {
        bail!("task id must not be zero");
    }
    Ok(task)
}

fn sql_value(value: &Value) -> SqlValue {
    match value {
        Value::Null => SqlValue::Null,
        Value::Bool(v) => SqlValue::Integer(i64::from(*v)),
        Value::Number(n) if n.is_i64() => SqlValue::Integer(n.as_i64().unwrap()),
        Value::Number(n) => SqlValue::Real(n.as_f64().unwrap()),
        Value::String(s) => SqlValue::Text(s.clone()),
        other => SqlValue::Text(other.to_string()),
    }
}
