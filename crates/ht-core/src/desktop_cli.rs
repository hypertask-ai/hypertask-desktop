//! Web contracts explicitly enabled for human desktop CLI bearers by HTPR-7699.
use crate::{
    Db, Result, Transport,
    http::{Response, TransportError},
};
use anyhow::{Context, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub method: String,
    pub path: String,
    pub body: Value,
}
fn request(method: &str, path: String, body: Value) -> Request {
    Request {
        method: method.into(),
        path,
        body,
    }
}
pub fn defaults_request(project: i64) -> Request {
    request(
        "GET",
        format!("/api/tasks/createGlobally?projectId={project}&position=top"),
        Value::Null,
    )
}
pub fn mentions_request(project: i64, query: &str) -> Request {
    let query = if query.is_empty() { "all" } else { query };
    let params = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("param", query)
        .append_pair("projectId", &project.to_string())
        .finish();
    request(
        "GET",
        format!("/api/tasks/searchByParam?{params}"),
        Value::Null,
    )
}
pub fn saved_request(task: i64, project: i64, comment: i64, public: bool, remove: bool) -> Request {
    let mut body = json!({"taskId":task,"projectId":project,"commentId":comment.to_string(),"type":if public {"Public"}else{"Private"}});
    if remove {
        body["alwaysRemove"] = json!(true);
    }
    request("POST", "/api/savedContent/toggleSaved".into(), body)
}
pub fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
pub fn writer_request(project: &Value, text: &str, time_zone: &str) -> Request {
    let html = format!("<p>{}</p>", escape(text).replace('\n', "<br>"));
    let settings = &project["ai_custom_instructions"][0];
    let mut body = json!({
        "projectId":project["id"],"teamId":project.get("teamId").unwrap_or(&json!(-1)),
        "byokProviderFlags":[],
        "PROMPT":format!("Write a task title and description from this note in the board's style.\n\n{html}"),
        "userRetrievalTexts":[text],"customInstructions":settings["customInstruction"].as_str().unwrap_or(""),
        "sourceSelected":settings["source_selected"].as_str().unwrap_or("openai"),
        "aiMode":"AiTaskWriter","requestKind":"compose-task","timeZone":time_zone,
        "images64":[],"taskDescription":html
    });
    if let Some(model) = settings["model_selected"].as_str() {
        body["modelSelected"] = json!(model);
    }
    request("POST", "/api/ai/task-writer".into(), body)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Draft {
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub due_date: Option<String>,
    #[serde(default)]
    pub labels: Vec<String>,
    #[serde(default)]
    pub priority: Option<u8>,
    #[serde(default)]
    pub estimate: Option<u8>,
}

// Parse the same marker elements as the web, including entities and nested HTML.
fn marker(html: &mut String, id: &str) -> Option<String> {
    let mut doc = scraper::Html::parse_fragment(html);
    let selector = scraper::Selector::parse(&format!("[id=\"{id}\"]")).ok()?;
    let node = doc.select(&selector).next()?;
    let value = node.text().collect::<String>();
    let node_id = node.id();
    doc.tree.get_mut(node_id)?.detach();
    *html = doc.root_element().inner_html();
    Some(value.trim().to_owned())
}
fn plain(html: &str) -> String {
    scraper::Html::parse_fragment(html)
        .root_element()
        .text()
        .collect::<String>()
        .trim()
        .to_owned()
}
pub fn writer_drafts(body: &Value) -> Result<Vec<Draft>> {
    let drafts = if let Some(tasks) = body["tasks"].as_array() {
        ensure!(
            !tasks.is_empty() && tasks.len() <= 10,
            "writer returned no tickets"
        );
        tasks
            .iter()
            .map(|t| {
                Ok(Draft {
                    title: t["title"]
                        .as_str()
                        .context("missing writer title")?
                        .trim()
                        .into(),
                    description: t["description"]
                        .as_str()
                        .context("missing writer description")?
                        .into(),
                    priority: None,
                    estimate: None,
                    due_date: t["dueDate"].as_str().map(str::to_owned),
                    labels: t["labelIds"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(Value::as_str)
                        .map(str::to_owned)
                        .collect(),
                })
            })
            .collect::<Result<Vec<_>>>()?
    } else {
        let mut html = body.as_str().context("invalid writer response")?.to_owned();
        ensure!(
            !html
                .lines()
                .any(|l| l.starts_with("event: error") || l.starts_with("event: done")),
            "Task writer interrupted"
        );
        let title =
            marker(&mut html, "ai-generated-task-title").context("writer returned no title")?;
        let due_date = marker(&mut html, "ai-generated-task-due-date");
        let labels = marker(&mut html, "ai-generated-task-tags")
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect();
        let priority = marker(&mut html, "ai-generated-task-priority")
            .and_then(|v| v.parse::<u8>().ok())
            .filter(|v| *v <= 4);
        let estimate = marker(&mut html, "ai-generated-task-estimate")
            .and_then(|v| v.parse::<u8>().ok())
            .filter(|v| [0, 2, 3, 4, 5, 6].contains(v));
        for id in ["properties", "status", "assignees", "type", "custom-fields"] {
            marker(&mut html, &format!("ai-generated-task-{id}"));
        }
        vec![Draft {
            title,
            description: html,
            due_date,
            labels,
            priority,
            estimate,
        }]
    };
    ensure!(
        drafts
            .iter()
            .all(|d| !d.title.trim().is_empty() && !plain(&d.description).is_empty()),
        "writer returned an incomplete ticket"
    );
    Ok(drafts)
}

pub(crate) fn compose(
    db: &Db,
    transport: &mut impl Transport,
    payload: &Value,
    key: &str,
) -> std::result::Result<Response, TransportError> {
    let refused = |status| {
        Ok(Response {
            status,
            body: json!({"success":false}),
        })
    };
    if db.current_user_id() != payload["user_id"].as_i64() {
        return refused(403);
    }
    let project = &payload["project"];
    let cache_key = format!("compose-prepared:{key}");
    let prepared = db
        .kv_get(&cache_key)
        .ok()
        .flatten()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok());
    let prepared = if let Some(value) = prepared {
        value
    } else {
        let defaults = defaults_request(project["id"].as_i64().unwrap_or(0));
        let response = transport.get(&defaults.path)?;
        if !(200..300).contains(&response.status) {
            return Ok(response);
        }
        if response.body["sectionId"].as_i64().is_none_or(|id| id <= 0) {
            return refused(422);
        }
        let placement = response.body;
        let text = payload["text"].as_str().unwrap_or("");
        let req = writer_request(
            project,
            text,
            payload["time_zone"].as_str().unwrap_or("UTC"),
        );
        let response = transport.mutate(&req.method, &req.path, &req.body, key);
        let (drafts, failed) = match response {
            Ok(Response { status: 402, body }) if body["code"] == "PLAN_LIMIT_REACHED" => {
                return refused(402);
            }
            Ok(Response {
                status: 401 | 403, ..
            }) => return refused(403),
            Ok(Response { status, body }) if (200..300).contains(&status) => {
                match writer_drafts(&body) {
                    Ok(drafts) => (drafts, false),
                    Err(_) => (vec![], true),
                }
            }
            _ => (vec![], true),
        };
        let drafts = if drafts.is_empty() {
            vec![Draft {
                title: text.trim().into(),
                description: format!("<p>{}</p>", escape(text).replace('\n', "<br>")),
                due_date: None,
                labels: vec![],
                priority: None,
                estimate: None,
            }]
        } else {
            drafts
        };
        let value = json!({"placement":placement,"drafts":drafts,"writerFailed":failed});
        db.kv_set(&cache_key, &value.to_string())
            .map_err(|_| TransportError::InvalidResponse)?;
        value
    };
    let drafts: Vec<Draft> = serde_json::from_value(prepared["drafts"].clone())
        .map_err(|_| TransportError::InvalidResponse)?;
    let mut tasks = vec![];
    let mut failed_titles = vec![];
    for (index, draft) in drafts.iter().enumerate() {
        let receipt_key = format!("compose-created:{key}:{index}");
        if let Some(task) = db
            .kv_get(&receipt_key)
            .ok()
            .flatten()
            .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        {
            tasks.push(task);
            continue;
        }
        let mut body = json!({"project_id":project["id"],"section_id":prepared["placement"]["sectionId"],"title":draft.title,"description":draft.description});
        if let Some(priority) = draft.priority {
            body["priority"] = json!(priority);
        }
        if let Some(estimate) = draft.estimate {
            body["estimate"] = json!(estimate);
        }
        if let Some(date) = &draft.due_date {
            body["due_date"] = json!(date);
        }
        if !draft.labels.is_empty() {
            body["labels"] = json!(draft.labels);
        }
        // Stable per-draft key and frozen body allow a lost create acknowledgement to be replayed safely.
        let response = transport.post("/tasks/create", &body, &format!("{key}-{index}"))?;
        if response.status == 409 {
            return Err(TransportError::Timeout);
        }
        if !(200..300).contains(&response.status) {
            if crate::queue::permanent(response.status) && !tasks.is_empty() {
                failed_titles = drafts[index..]
                    .iter()
                    .map(|draft| draft.title.clone())
                    .collect();
                break;
            }
            return Ok(response);
        }
        let task = &response.body["task"];
        if response.body["success"] != true || task["id"].as_i64().is_none_or(|id| id <= 0) {
            return Err(TransportError::InvalidResponse);
        }
        db.kv_set(&receipt_key, &task.to_string())
            .map_err(|_| TransportError::InvalidResponse)?;
        db.upsert_task(task)
            .map_err(|_| TransportError::InvalidResponse)?;
        tasks.push(task.clone());
    }
    Ok(Response {
        status: 200,
        body: json!({"success":true,"tasks":tasks,"writerFailed":prepared["writerFailed"],"failedTitles":failed_titles}),
    })
}

pub fn mention_html(item: &Value) -> Option<String> {
    let kind = item["type"].as_str()?;
    let name = item["name"].as_str()?;
    let id = match &item["id"] {
        Value::String(id) if !id.is_empty() => id.clone(),
        Value::Number(id) if id.as_i64().is_some_and(|id| id > 0) => id.to_string(),
        _ => return None,
    };
    let (label, data_id, text, extra, href) = match kind {
        "name" | "agent" => (
            format!("{kind}-{id}"),
            name.to_owned(),
            name.to_owned(),
            String::new(),
            None,
        ),
        "task" => {
            let project = item["project_id"].as_i64()?;
            let index = item["index"].as_i64()?;
            if project <= 0 || index <= 0 {
                return None;
            }
            let text = format!("{} {name}", item["ticketNumber"].as_str().unwrap_or(""));
            (
                "task".into(),
                text.clone(),
                text,
                format!(" projectid=\"{project}\" uniqueindex=\"{index}\""),
                Some(format!(
                    "https://app.hypertask.ai/detail/project-{project}/{index}"
                )),
            )
        }
        "project" => (
            format!("project-{id}"),
            format!("{} {name}", item["identifier"].as_str().unwrap_or("")),
            format!("{} {name}", item["identifier"].as_str().unwrap_or("")),
            format!(" projectid=\"{}\"", item["id"].as_i64()?),
            None,
        ),
        "page" => {
            let path: String = url::form_urlencoded::byte_serialize(id.as_bytes()).collect();
            (
                "page".into(),
                id.clone(),
                name.to_owned(),
                format!(" text=\"{}\"", escape(name)),
                Some(format!(
                    "https://app.hypertask.ai/page/{}",
                    path.replace('+', "%20")
                )),
            )
        }
        _ => return None,
    };
    let tag = if href.is_some() { "a" } else { "span" };
    let href = href
        .map(|h| format!(" href=\"{}\"", escape(&h)))
        .unwrap_or_default();
    Some(format!(
        "<{tag}{href} data-type=\"mention\" data-id=\"{}\" data-label=\"{}\" data-mention-suggestion-char=\"@\"{extra}>{}</{tag}> ",
        escape(&data_id),
        escape(&label),
        escape(&text)
    ))
}
