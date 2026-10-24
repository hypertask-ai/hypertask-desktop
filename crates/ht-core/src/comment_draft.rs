use crate::{
    Db, Result, Transport,
    http::{Response, TransportError},
};
use anyhow::{Context, ensure};
use serde_json::{Value, json};

fn key(user: i64, task: i64, local: bool) -> String {
    format!(
        "comment-draft:{}:{user}:{task}",
        if local { "local" } else { "server" }
    )
}

impl Db {
    pub fn comment_draft(&self, task: i64) -> Result<Option<String>> {
        let user = self.current_user_id().unwrap_or(0);
        Ok(self
            .kv_get(&key(user, task, true))?
            .or(self.kv_get(&key(user, task, false))?))
    }

    pub fn save_local_comment_draft(&self, task: i64, html: &str) -> Result<()> {
        self.kv_set(&key(self.current_user_id().unwrap_or(0), task, true), html)
    }

    pub(crate) fn cache_comment_drafts(&self, task: i64, drafts: &[Value]) -> Result<()> {
        let Some(user) = self.current_user_id() else {
            return Ok(());
        };
        let text = drafts
            .iter()
            .find(|d| d["draftType"] == "comment" && d["createdBy"]["id"] == user)
            .and_then(|d| d["text"].as_str())
            .unwrap_or("");
        self.kv_set(&key(user, task, false), text)
    }

    pub(crate) fn optimistic_comment_draft(&self, payload: &Value) -> Result<()> {
        let user = payload["user_id"].as_i64().context("missing draft owner")?;
        let task = payload["task_id"].as_i64().context("missing draft task")?;
        let mut value = self
            .raw("Task", &json!(task))?
            .context("draft task not cached")?;
        value["hasCommentDraft"] = json!(
            !self
                .kv_get(&key(user, task, true))?
                .unwrap_or_else(|| payload["text"].as_str().unwrap_or("").to_owned())
                .is_empty()
        );
        value["hasBoardDraft"] = value["hasCommentDraft"].clone();
        self.upsert("Task", &value)?;
        if payload["text"].is_null() {
            self.optimistic_discard(&json!({"task_id":task,"draft_type":"comment"}))?;
        }
        Ok(())
    }

    pub(crate) fn resolve_comment_draft(&self, payload: &Value, body: &Value) -> Result<()> {
        let user = payload["user_id"].as_i64().context("missing draft owner")?;
        let task = payload["task_id"].as_i64().context("missing draft task")?;
        let text = payload["text"].as_str().unwrap_or("");
        if !text.is_empty() {
            ensure!(
                body["draft"]["taskId"] == task
                    && body["draft"]["draftType"] == "comment"
                    && body["draft"]["createdBy"]["id"] == user
                    && body["draft"]["text"] == text,
                "draft acknowledgement mismatch"
            );
        }
        self.kv_set(&key(user, task, false), text)?;
        if self.kv_get(&key(user, task, true))?.as_deref() == Some(text) {
            self.conn
                .execute("DELETE FROM kv WHERE key=?", [key(user, task, true)])?;
        }
        Ok(())
    }

    pub(crate) fn clear_sent_comment_draft(&mut self, payload: &Value, now: i64) -> Result<()> {
        let Some(user) = payload["draft_user_id"].as_i64() else {
            return Ok(());
        };
        let task = payload["task_id"].as_i64().context("missing task")?;
        // A new draft typed while the comment was sending must not be deleted.
        if self.kv_get(&key(user, task, true))?.as_deref() != Some("") {
            return Ok(());
        }
        if self.editor_drafts(task)?.iter().any(|d| d["draftType"] == "comment") {
            self.enqueue_in_transaction(crate::Write::DiscardDraft {
                task_id: task,
                draft_type: crate::DraftType::Comment,
            }, now)?;
            return Ok(());
        }
        let body = json!({"task_id":task,"user_id":user,"text":null});
        self.conn.execute("INSERT INTO mutation_queue(client_id,entity,kind,payload_json,created_at,next_attempt_at) VALUES(?,?,'comment_draft',?,?,?)",
            rusqlite::params![uuid::Uuid::new_v4().to_string(),format!("draft:{user}:{task}"),body.to_string(),now,now])?;
        Ok(())
    }

    pub(crate) fn restore_failed_comment_draft(&self, payload: &Value) -> Result<()> {
        if let (Some(user), Some(task), Some(text)) = (
            payload["draft_user_id"].as_i64(),
            payload["task_id"].as_i64(),
            payload["draft_text"].as_str(),
        ) {
            if self.kv_get(&key(user, task, true))?.as_deref() == Some("") {
                self.kv_set(&key(user, task, true), text)?;
            }
        }
        Ok(())
    }
}

pub(crate) fn send_draft(
    transport: &mut impl Transport,
    payload: &Value,
    client: &str,
) -> std::result::Result<Response, TransportError> {
    if payload["text"].is_string() {
        return transport.post(
            "/drafts",
            &json!({"task_id":payload["task_id"],"draft_type":"comment","text":payload["text"]}),
            client,
        );
    }
    let task = payload["task_id"]
        .as_i64()
        .ok_or(TransportError::InvalidResponse)?;
    let response = transport.get(&format!("/drafts?task_id={task}"))?;
    if !(200..300).contains(&response.status) {
        return Ok(response);
    }
    let drafts = response.body["drafts"]
        .as_array()
        .ok_or(TransportError::InvalidResponse)?;
    if response.body["success"] != true || drafts.iter().any(|d| d["taskId"] != task) {
        return Err(TransportError::InvalidResponse);
    }
    for draft in drafts
        .iter()
        .filter(|d| d["draftType"] == "comment" && d["createdBy"]["id"] == payload["user_id"])
    {
        let id = draft["id"]
            .as_i64()
            .filter(|id| *id > 0)
            .ok_or(TransportError::InvalidResponse)?;
        let result = transport.mutate("DELETE", &format!("/drafts/{id}"), &json!({}), client)?;
        // Retry is idempotent if the preceding delete succeeded but its response was lost.
        if result.status != 404
            && (!(200..300).contains(&result.status) || result.body["success"] != true)
        {
            return Ok(result);
        }
    }
    Ok(Response {
        status: 200,
        body: json!({"success":true}),
    })
}
