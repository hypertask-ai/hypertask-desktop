use crate::{Db, Result};
use anyhow::{Context, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum CommentWrite {
    Edit { text: String },
    Delete,
    Saved { public: bool },
    Reaction { emoji: String, active: bool },
}

impl CommentWrite {
    pub(crate) fn validate(&self) -> Result<()> {
        match self {
            Self::Edit { text } => {
                ensure!(!text.trim().is_empty(), "comment must not be empty");
                ensure!(
                    text.encode_utf16().count() <= 5000,
                    "comment must be 5000 characters or less"
                );
            }
            Self::Reaction { emoji, .. } => {
                ensure!(
                    !emoji.is_empty()
                        && emoji.chars().count() <= 8
                        && emoji.trim() == emoji
                        && !emoji
                            .chars()
                            .any(|c| c.is_ascii_alphanumeric() || c.is_control()),
                    "invalid emoji"
                );
            }
            Self::Delete | Self::Saved { .. } => (),
        }
        Ok(())
    }

    pub(crate) fn request(&self, comment_id: i64) -> (&'static str, String, Value) {
        let path = format!("/comments/{comment_id}");
        match self {
            Self::Edit { text } => ("PATCH", path, json!({"text":text,"content_type":"html"})),
            Self::Delete => ("DELETE", path, json!({})),
            Self::Saved { .. } => unreachable!("saved comments require task and project scope"),
            Self::Reaction { emoji, active } => (
                "POST",
                format!("{path}/reactions"),
                json!({"emoji":emoji,"active":active}),
            ),
        }
    }
}

impl Db {
    pub fn current_user_id(&self) -> Option<i64> {
        self.kv_get("current-user-id").ok().flatten()?.parse().ok()
    }

    pub(crate) fn optimistic_comment_action(&self, payload: &Value) -> Result<()> {
        let action: CommentWrite = serde_json::from_value(payload["change"].clone())?;
        let id = payload["comment_id"]
            .as_i64()
            .context("missing comment id")?;
        let task_id = payload["task_id"].as_i64().context("missing task id")?;
        let mut comment = self
            .raw("Comment", &json!(id))?
            .unwrap_or_else(|| payload["before"].clone());
        match action {
            CommentWrite::Edit { text } => {
                comment["text"] = json!(text);
                comment["commentText"] = comment["text"].clone();
                self.upsert_comment(task_id, &comment)?;
            }
            CommentWrite::Saved { .. } => (),
            CommentWrite::Delete => {
                self.conn.execute(
                    "DELETE FROM Comment WHERE id=? AND taskId=?",
                    rusqlite::params![id, task_id],
                )?;
            }
            CommentWrite::Reaction { emoji, active } => {
                let user_id = payload["user_id"].clone();
                let mut reactions = comment["reactions"].as_array().cloned().unwrap_or_default();
                reactions.retain(|r| {
                    !(r["emoji"] == emoji && r["userId"] == user_id && r["isAgent"] != true)
                });
                if active {
                    reactions.push(json!({"id":format!("pending:{id}:{emoji}"),"emoji":emoji,"userId":user_id,"pending":true}));
                }
                comment["reactions"] = json!(reactions);
                self.upsert_comment(task_id, &comment)?;
            }
        }
        Ok(())
    }

    pub(crate) fn resolve_comment_action(&self, payload: &Value, body: &Value) -> Result<()> {
        let action: CommentWrite = serde_json::from_value(payload["change"].clone())?;
        let task_id = payload["task_id"].as_i64().context("missing task id")?;
        let id = &payload["comment_id"];
        match action {
            CommentWrite::Saved { public } => {
                ensure!(body["active"].is_boolean(), "missing saved state");
                let mut comment = self.raw("Comment", id)?.unwrap_or_else(||payload["before"].clone());
                let mut saved = comment["savedContent"].as_array().cloned().unwrap_or_default();
                let kind = if public {"Public"} else {"Private"};
                saved.retain(|s| !(s["type"] == kind && (public || s["userId"] == payload["user_id"])));
                if body["active"] == true { saved.push(body["saved"].clone()); }
                comment["savedContent"] = json!(saved);
                self.upsert_comment(task_id, &comment)?;
            }
            CommentWrite::Delete => {
                self.conn
                    .execute("DELETE FROM Comment WHERE id=?", [id.as_i64().unwrap()])?;
            }
            CommentWrite::Edit { .. } => {
                let server = body.get("comment").context("missing comment")?;
                ensure!(
                    server["id"] == *id && server["text"].is_string(),
                    "invalid updated comment"
                );
                let mut comment = self
                    .raw("Comment", id)?
                    .unwrap_or_else(|| payload["before"].clone());
                for (key, value) in server.as_object().context("invalid comment")? {
                    comment[key] = value.clone();
                }
                comment["commentText"] = comment["text"].clone();
                self.upsert_comment(task_id, &comment)?;
            }
            CommentWrite::Reaction { active, .. } => {
                ensure!(
                    body["active"] == active && body["reactions"].is_array(),
                    "invalid reaction response"
                );
                let mut comment = self
                    .raw("Comment", id)?
                    .unwrap_or_else(|| payload["before"].clone());
                comment["reactions"] = body["reactions"].clone();
                self.upsert_comment(task_id, &comment)?;
            }
        }
        self.request_refetch("comments", task_id)
    }
}
