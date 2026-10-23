use crate::{
    Db, Result, Transport,
    db::{array, ensure_success, integer},
    http::successful,
};
use anyhow::{Context, bail, ensure};
use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::params;
use serde_json::json;

pub struct Sync<T> {
    pub db: Db,
    pub transport: T,
}

impl<T: Transport> Sync<T> {
    pub fn new(db: Db, transport: T) -> Self {
        Self { db, transport }
    }

    pub fn projects(&mut self) -> Result<()> {
        let mut offset = 0;
        loop {
            let response = successful(
                self.transport
                    .get(&format!("/projects?limit=100&offset={offset}"))?,
            )?;
            let count = array(&response, "projects")?.len();
            self.db.apply_response(&response)?;
            offset += count;
            if offset >= response["total"].as_u64().unwrap_or(offset as u64) as usize {
                break;
            }
            ensure!(count > 0, "projects pagination made no progress");
        }
        Ok(())
    }

    pub fn sections(&mut self, project_id: i64) -> Result<()> {
        let response = successful(self.transport.get(&format!(
            "/projects/{project_id}/sections?include_hidden=false"
        ))?)?;
        ensure!(
            response["projectId"] == project_id,
            "sections returned another board"
        );
        self.db.transaction(|db| {
            for old in db.json_rows(
                "SELECT raw_json FROM Section WHERE projectId=?",
                [project_id],
            )? {
                let mut old = old;
                old["deleted"] = json!(true);
                db.upsert("Section", &old)?;
            }
            for section in array(&response, "sections")? {
                db.upsert("Section", section)?;
            }
            Ok(())
        })
    }

    pub fn board(&mut self, project_id: i64) -> Result<()> {
        ensure!(project_id > 0, "invalid board id");
        self.sections(project_id)?;
        self.delta(project_id)
    }

    pub fn delta(&mut self, project_id: i64) -> Result<()> {
        let initialized: bool = self.db.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM sync_state WHERE project_id=? AND status='Normal')",
            [project_id],
            |r| r.get(0),
        )?;
        let statuses: &[&str] = if initialized {
            &["Normal", "Archive", "Deleted"]
        } else {
            &["Normal"]
        };
        for &status in statuses {
            let cursor = self.db.cursor(project_id, status)?;
            let since = cursor.as_deref().map(cursor_minus_second).transpose()?;
            let mut offset = 0;
            let mut tasks = Vec::new();
            let mut max_seen = cursor.as_deref().map(parse_time).transpose()?;
            loop {
                let mut params = url::form_urlencoded::Serializer::new(String::new());
                params
                    .append_pair("project_id", &project_id.to_string())
                    .append_pair("status", status)
                    .append_pair("sort_by", "updatedAt")
                    .append_pair("sort_order", "asc")
                    .append_pair("limit", "100")
                    .append_pair("offset", &offset.to_string());
                if let Some(since) = &since {
                    params.append_pair("updated_since", since);
                }
                let response =
                    successful(self.transport.get(&format!("/tasks?{}", params.finish()))?)?;
                let page = array(&response, "tasks")?;
                for task in page {
                    ensure!(
                        integer(task, "projectId")? == project_id && task["status"] == status,
                        "delta returned another board or status"
                    );
                    if let Some(updated) = task["updatedAt"].as_str() {
                        let updated = parse_time(updated)?;
                        max_seen = Some(max_seen.map_or(updated, |old| old.max(updated)));
                    }
                }
                let total = response["total"].as_u64().context("delta missing total")? as usize;
                offset += page.len();
                tasks.extend(page.iter().cloned());
                if offset >= total {
                    break;
                }
                ensure!(!page.is_empty(), "delta pagination made no progress");
            }
            self.db.transaction(|db| {
                for task in &tasks { db.upsert_task(task)?; }
                db.conn.execute("INSERT INTO sync_state(project_id,status,cursor) VALUES(?,?,?) ON CONFLICT(project_id,status) DO UPDATE SET cursor=excluded.cursor",
                    params![project_id,status,max_seen.map(|t| t.to_rfc3339_opts(SecondsFormat::Millis,true))])?;
                db.reapply_pending()?;
                Ok(())
            })?;
        }
        Ok(())
    }

    pub fn task(&mut self, task_id: i64) -> Result<()> {
        let response = successful(self.transport.get(&format!("/tasks?task_id={task_id}"))?)?;
        let task = response
            .get("task")
            .or_else(|| {
                response["tasks"]
                    .as_array()
                    .and_then(|a| a.iter().find(|t| t["id"] == task_id))
            })
            .context("task response missing requested task")?;
        ensure!(task["id"] == task_id, "task response id mismatch");
        self.db.transaction(|db| {
            db.upsert_task(task)?;
            db.reapply_pending()
        })
    }

    pub fn comments(&mut self, task_id: i64) -> Result<()> {
        let mut offset = 0;
        let mut comments = Vec::new();
        loop {
            let response = successful(self.transport.get(&format!(
                "/comments?task_id={task_id}&limit=50&offset={offset}&sort_order=desc"
            ))?)?;
            let page = array(&response, "comments")?;
            offset += page.len();
            comments.extend(page.iter().cloned());
            if offset
                >= response["total"]
                    .as_u64()
                    .context("comments missing total")? as usize
            {
                break;
            }
            ensure!(!page.is_empty(), "comments pagination made no progress");
        }
        self.db.transaction(|db| {
            db.conn
                .execute("DELETE FROM Comment WHERE taskId=? AND id>0", [task_id])?;
            for comment in &comments {
                db.upsert_comment(task_id, comment)?;
            }
            db.reapply_pending()
        })
    }

    pub fn inbox(&mut self) -> Result<()> {
        let response = successful(self.transport.get("/inbox/list")?)?;
        ensure_success(&response)?;
        self.db.transaction(|db| {
            let mut incoming = array(&response, "user_notifications")?.clone();
            if let Some(agents) = response.get("agent_notifications") {
                incoming.extend(
                    agents
                        .as_array()
                        .context("agent_notifications must be an array")?
                        .iter()
                        .cloned(),
                );
            }
            let ids: std::collections::HashSet<_> = incoming
                .iter()
                .map(|n| integer(n, "id"))
                .collect::<Result<_>>()?;
            for mut old in db.json_rows(
                "SELECT raw_json FROM Notification WHERE status='Normal'",
                [],
            )? {
                if !ids.contains(&integer(&old, "id")?) {
                    old["status"] = json!("Archive");
                    db.upsert("Notification", &old)?;
                }
            }
            for item in &incoming {
                db.upsert("Notification", item)?;
            }
            db.reapply_pending()
        })
    }

    pub fn refetch_pending(&mut self) -> Result<()> {
        for (target, id, generation) in self.db.refetches()? {
            match target.as_str() {
                "board" => self.board(id)?,
                "task" => self.task(id)?,
                "comments" => self.comments(id)?,
                "inbox" => self.inbox()?,
                _ => bail!("unknown refetch target"),
            }
            self.db.conn.execute(
                "DELETE FROM refetch_queue WHERE target=? AND entity_id=? AND generation=?",
                params![target, id, generation],
            )?;
        }
        Ok(())
    }
}

fn parse_time(cursor: &str) -> Result<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339(cursor)?.with_timezone(&Utc))
}

pub fn cursor_minus_second(cursor: &str) -> Result<String> {
    Ok(parse_time(cursor)?
        .checked_sub_signed(chrono::Duration::seconds(1))
        .context("cursor underflow")?
        .to_rfc3339_opts(SecondsFormat::Millis, true))
}
