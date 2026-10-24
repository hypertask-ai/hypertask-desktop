use crate::{Db, Result, Transport, db::ensure_success, http::Response};
use anyhow::{Context, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub(crate) fn validate_editor_html(html: &str, route: &str) -> Result<()> {
    // The bearer writers erase these web formats, even when content_type is HTML.
    // Refuse a lossy save instead of claiming an optimistic echo is persistence parity.
    let lower = html.to_ascii_lowercase();
    let highlight = lower.match_indices("<mark").any(|(at, _)| {
        lower
            .as_bytes()
            .get(at + 5)
            .is_some_and(|b| b.is_ascii_whitespace() || matches!(b, b'>' | b'/'))
    });
    ensure!(
        !highlight,
        "{route}: the MCP sanitizeRichHtml allowlist strips highlights; remove Highlight or save in the web editor"
    );
    let emoji = lower.contains("data-name=")
        && (lower.contains("data-type=\"emoji\"") || lower.contains("data-type='emoji'"));
    ensure!(
        !emoji,
        "{route}: MCP sanitizeRichHtml strips emoji data-name; named-emoji HTML persistence needs a server allowlist fix"
    );
    ensure!(
        ![
            "href=\"mailto:",
            "href='mailto:",
            "href=\"tel:",
            "href='tel:"
        ]
        .iter()
        .any(|prefix| lower.contains(prefix)),
        "{route}: MCP sanitizeRichHtml strips non-HTTP link hrefs; this link cannot be saved losslessly"
    );
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImproveCommand {
    #[serde(rename = "FixSpellingAndGrammar")]
    Grammar,
    #[serde(rename = "ImproveReadability")]
    Readability,
    Summarize,
    MakeShorter,
    #[serde(rename = "Translate:English")]
    English,
    #[serde(rename = "Translate:German")]
    German,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DraftType {
    Comment,
    Description,
}
impl DraftType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Comment => "comment",
            Self::Description => "description",
        }
    }
}
#[derive(Debug, Deserialize)]
pub struct EditorResult {
    pub task_id: i64,
    pub source: String,
    pub html: Option<String>,
    pub error: Option<String>,
}
impl Db {
    pub fn cancel_editor_result(&self, id: i64) -> Result<()> {
        self.kv_set(&format!("editor-result:{id}"), "{\"canceled\":true}")
    }
    pub fn clear_editor_result(&self, id: i64) -> Result<()> {
        self.conn.execute(
            "DELETE FROM kv WHERE key=?",
            [format!("editor-result:{id}")],
        )?;
        Ok(())
    }
    pub fn editor_result(&self, id: i64) -> Result<Option<EditorResult>> {
        self.kv_get(&format!("editor-result:{id}"))?
            .map(|v| serde_json::from_str(&v).map_err(Into::into))
            .transpose()
    }
    pub fn editor_drafts(&self, task: i64) -> Result<Vec<Value>> {
        let Some(user) = self.current_user_id() else {
            return Ok(vec![]);
        };
        self.kv_get(&format!("editor-drafts:{user}:{task}"))?
            .map(|v| serde_json::from_str(&v).map_err(Into::into))
            .transpose()
            .map(|v| v.unwrap_or_default())
    }
    pub(crate) fn cache_editor_drafts(&self, task: i64, drafts: &[Value]) -> Result<()> {
        ensure!(
            drafts.iter().all(|d| d["taskId"] == task),
            "draft response task mismatch"
        );
        if let Some(user) = self.current_user_id() {
            self.kv_set(
                &format!("editor-drafts:{user}:{task}"),
                &serde_json::to_string(drafts)?,
            )?;
        }
        Ok(())
    }
    pub(crate) fn optimistic_discard(&self, payload: &Value) -> Result<()> {
        let id = payload["task_id"].as_i64().context("missing draft task")?;
        let kind = payload["draft_type"]
            .as_str()
            .context("missing draft type")?;
        let mut task = self
            .raw("Task", &json!(id))?
            .context("draft task is not cached")?;
        task[if kind == "comment" {
            "hasCommentDraft"
        } else {
            "hasDescriptionDraft"
        }] = json!(false);
        if kind == "comment" {
            task["hasBoardDraft"] = json!(false);
        }
        self.upsert_task(&task)?;
        let drafts = self
            .editor_drafts(id)?
            .into_iter()
            .filter(|d| d["draftType"] != kind)
            .collect::<Vec<_>>();
        self.cache_editor_drafts(id, &drafts)
    }
    pub(crate) fn resolve_editor(
        &self,
        id: i64,
        kind: &str,
        payload: &Value,
        body: &Value,
    ) -> Result<()> {
        if kind == "editor_improve" {
            if self.kv_get(&format!("editor-result:{id}"))?.as_deref()
                == Some("{\"canceled\":true}")
            {
                return self.clear_editor_result(id);
            }
            let html = body["html"]
                .as_str()
                .filter(|v| !v.is_empty())
                .context("missing improved HTML")?;
            ensure!(
                body["command"] == payload["command"],
                "AI returned a different command"
            );
            self.kv_set(
                &format!("editor-result:{id}"),
                &json!({"task_id":payload["task_id"],"source":payload["text"],"html":html})
                    .to_string(),
            )?;
        } else {
            self.request_refetch(
                "drafts",
                payload["task_id"].as_i64().context("missing draft task")?,
            )?;
        }
        Ok(())
    }
    pub(crate) fn fail_editor(
        &self,
        id: i64,
        kind: &str,
        payload: &Value,
        error: &str,
    ) -> Result<()> {
        if kind == "editor_improve" {
            if self.kv_get(&format!("editor-result:{id}"))?.as_deref()
                == Some("{\"canceled\":true}")
            {
                return self.clear_editor_result(id);
            }
            self.kv_set(
                &format!("editor-result:{id}"),
                &json!({"task_id":payload["task_id"],"source":payload["text"],"error":error})
                    .to_string(),
            )?;
        } else if kind == "draft_discard" {
            let task_id = payload["task_id"].as_i64().context("missing draft task")?;
            let mut task = self
                .raw("Task", &json!(task_id))?
                .context("draft task is not cached")?;
            for field in ["hasCommentDraft", "hasDescriptionDraft", "hasBoardDraft"] {
                task[field] = payload["before"][field].clone();
            }
            self.upsert_task(&task)?;
            self.cache_editor_drafts(
                task_id,
                payload["drafts_before"]
                    .as_array()
                    .context("missing previous drafts")?,
            )?;
            self.reapply_pending()?;
        }
        Ok(())
    }
}

pub(crate) fn discard(
    transport: &mut impl Transport,
    payload: &Value,
    key: &str,
) -> std::result::Result<Response, crate::http::TransportError> {
    // Resolve own draft IDs at drain time: offline saves/discards can outlive cached IDs.
    let task = payload["task_id"].as_i64().unwrap();
    let response = transport.get(&format!("/drafts?task_id={task}"))?;
    if !(200..300).contains(&response.status) {
        return Ok(response);
    }
    if ensure_success(&response.body).is_err() {
        return Ok(Response {
            status: 502,
            body: json!({"success":false}),
        });
    }
    let Some(drafts) = response.body["drafts"].as_array() else {
        return Ok(Response {
            status: 502,
            body: json!({"success":false}),
        });
    };
    if !drafts
        .iter()
        .all(|d| d["taskId"] == task && d["id"].as_i64().is_some_and(|id| id > 0))
    {
        return Ok(Response {
            status: 502,
            body: json!({"success":false}),
        });
    }
    if drafts.iter().any(|d| {
        d["draftType"] == payload["draft_type"]
            && d.pointer("/createdBy/id") != Some(&payload["user_id"])
    }) {
        return Ok(Response {
            status: 403,
            body: json!({"success":false}),
        });
    }
    for draft in drafts
        .iter()
        .filter(|d| d["draftType"] == payload["draft_type"])
    {
        let id = draft["id"].as_i64().unwrap();
        let response = transport.mutate(
            "DELETE",
            &format!("/drafts/{id}"),
            &json!({}),
            &format!("{key}:{id}"),
        )?;
        if response.status != 404
            && (!(200..300).contains(&response.status) || ensure_success(&response.body).is_err())
        {
            return Ok(response);
        }
    }
    Ok(Response {
        status: 200,
        body: json!({"success":true}),
    })
}
