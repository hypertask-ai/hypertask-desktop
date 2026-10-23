//! Ordinary AI chat uses named SSE events, not OpenAI deltas. Credentials stay server-side.
use crate::{BoardColumn, Db, Result, TaskView};
use anyhow::{Context, bail, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Read},
    time::Duration,
};
use uuid::Uuid;

#[derive(Clone, Copy, Debug)]
pub struct ModelOption {
    pub id: &'static str,
    pub label: &'static str,
    pub model: &'static str,
    pub provider: &'static str,
}

// A deliberately small picker until team availability/preferences have a bearer API.
pub const MODELS: &[ModelOption] = &[
    ModelOption {
        id: "gemini-3.5-flash-lite",
        label: "Gemini Flash Lite",
        model: "google/gemini-3.5-flash-lite",
        provider: "gateway",
    },
    ModelOption {
        id: "gpt-6-luna",
        label: "GPT 6 Luna · Standard",
        model: "gpt-6-luna",
        provider: "openai",
    },
    ModelOption {
        id: "gpt-6-luna-light",
        label: "GPT 6 Luna · Light",
        model: "gpt-6-luna",
        provider: "openai",
    },
    ModelOption {
        id: "gpt-6-luna-high",
        label: "GPT 6 Luna · High",
        model: "gpt-6-luna",
        provider: "openai",
    },
    ModelOption {
        id: "claude-opus-5-5-thinking",
        label: "Opus 5.5 · Thinking",
        model: "claude-opus-5.5",
        provider: "claude",
    },
    ModelOption {
        id: "claude-haiku-5-5",
        label: "Haiku 5.5",
        model: "claude-haiku-5.5",
        provider: "claude",
    },
];

pub fn session_id() -> String {
    Uuid::new_v4().to_string()
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct TurnIds {
    pub session_id: String,
    pub assistant_message_id: String,
    pub stream_id: String,
}

impl TurnIds {
    pub fn new(session_id: String) -> Self {
        Self {
            session_id,
            assistant_message_id: self::session_id(),
            stream_id: self::session_id(),
        }
    }
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Clone, Debug, Default, Serialize, PartialEq)]
pub struct DefaultContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<i64>,
    pub surface: String,
    pub surface_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub view_name: Option<String>,
}

pub fn request(
    message: &str,
    ids: &TurnIds,
    user_id: &str,
    model: ModelOption,
    context: &DefaultContext,
    team_id: Option<&str>,
    history: &[Message],
) -> Value {
    let mut body = json!({
        "message": message, "aiFeature": "aiChat",
        "session_id": ids.session_id, "user_message_id": user_id,
        "assistant_message_id": ids.assistant_message_id, "stream_id": ids.stream_id,
        "modelOptionId": model.id, "model": model.model, "provider": model.provider,
        "context_list": [], "default_context": context, "user_context": {},
        "chat_history": history, "attachments": [], "images64": [], "pdfs64": [],
        "docx64": [], "byokProviderFlags": [],
    });
    if let Some(team) = team_id {
        body["teamId"] = json!(team);
    }
    body
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Done {
    pub status: String,
    #[serde(default)]
    pub cancelled: bool,
    #[serde(default)]
    pub user_message_persisted: bool,
    #[serde(default)]
    pub assistant_persisted: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    Status(String),
    Content(String),
    Title(String),
    Error(String),
    Done(Done),
}

const MAX_FRAME: usize = 1024 * 1024;

/// Byte buffering keeps split UTF-8 intact. Dispatch happens only at a blank line.
#[derive(Default)]
pub struct SseParser {
    line: Vec<u8>,
    name: String,
    data: String,
    done: bool,
}

impl SseParser {
    pub fn push(&mut self, bytes: &[u8]) -> Result<Vec<Event>> {
        let mut events = Vec::new();
        for &byte in bytes {
            if self.done {
                break;
            }
            if byte != b'\n' {
                self.line.push(byte);
                ensure!(
                    self.line.len() + self.data.len() <= MAX_FRAME,
                    "AI stream frame is too large"
                );
                continue;
            }
            let line = std::mem::take(&mut self.line);
            let line = std::str::from_utf8(&line)
                .context("Invalid UTF-8 in AI stream")?
                .trim_end_matches('\r');
            if line.is_empty() {
                if !self.data.is_empty() {
                    let data: Value =
                        serde_json::from_str(&self.data).context("Invalid AI stream event JSON")?;
                    let content = || data["content"].as_str().unwrap_or_default().to_owned();
                    let event = match self.name.as_str() {
                        "status" | "thinking" => Some(Event::Status(content())),
                        "content" => Some(Event::Content(content())),
                        "title" => Some(Event::Title(content())),
                        "error" => Some(Event::Error(content())),
                        "done" | "complete" => {
                            let done: Done = serde_json::from_value(data)?;
                            ensure!(
                                matches!(done.status.as_str(), "complete" | "error"),
                                "Invalid AI completion status"
                            );
                            self.done = true;
                            Some(Event::Done(done))
                        }
                        _ => None, // agent metadata and future events do not contribute reply text.
                    };
                    if let Some(event) = event {
                        events.push(event);
                    }
                }
                self.name.clear();
                self.data.clear();
            } else if let Some(name) = line.strip_prefix("event:") {
                self.name = name.trim_start_matches(' ').to_owned();
            } else if let Some(data) = line.strip_prefix("data:") {
                if !self.data.is_empty() {
                    self.data.push('\n');
                }
                self.data.push_str(data.strip_prefix(' ').unwrap_or(data));
            }
        }
        Ok(events)
    }

    pub fn finish(&self) -> Result<()> {
        ensure!(
            self.done,
            "AI reply interrupted before done; partial text is preserved"
        );
        Ok(())
    }
}

#[derive(Clone)]
pub struct Client {
    agent: ureq::Agent,
    base: String,
    token: String,
}

impl Client {
    pub fn new(token: String) -> Result<Self> {
        Self::with_base(crate::auth::SITE_URL, token)
    }

    pub fn with_base(base: &str, token: String) -> Result<Self> {
        // Reuse the existing credential/origin validation, including localhost-only HTTP.
        crate::Http::with_base(base, token.clone())?;
        Ok(Self {
            agent: ureq::AgentBuilder::new()
                .redirects(0)
                .timeout_connect(Duration::from_secs(15))
                .timeout_read(Duration::from_secs(310))
                .timeout_write(Duration::from_secs(30))
                .build(),
            base: base.trim_end_matches('/').to_owned(),
            token,
        })
    }

    fn post(&self, path: &str, body: &Value, accept: &str) -> Result<ureq::Response> {
        match self
            .agent
            .post(&format!("{}{path}", self.base))
            .set("Authorization", &format!("Bearer {}", self.token))
            .set("Content-Type", "application/json")
            .set("Accept", accept)
            .send_json(body)
        {
            Ok(response) => Ok(response),
            Err(ureq::Error::Status(status, response)) => {
                let mut body = String::new();
                response
                    .into_reader()
                    .take(64 * 1024)
                    .read_to_string(&mut body)?;
                bail!("{}", refusal(status, &body));
            }
            Err(error) => Err(anyhow::anyhow!("AI request failed: {error}")),
        }
    }

    /// Runs on a worker thread; the callback can apply bounded channel backpressure.
    pub fn stream(&self, body: &Value, mut emit: impl FnMut(Event) -> bool) -> Result<()> {
        let response = self.post("/api/ai/chat/stream", body, "text/event-stream")?;
        ensure!(
            response.header("Content-Type").is_some_and(|v| v
                .split(';')
                .next()
                .is_some_and(|v| v.trim() == "text/event-stream")),
            "Server did not return an AI event stream"
        );
        let mut reader = BufReader::new(response.into_reader());
        let mut parser = SseParser::default();
        loop {
            let bytes = reader
                .fill_buf()
                .context("AI stream disconnected; partial text is preserved")?;
            if bytes.is_empty() {
                return parser.finish();
            }
            let len = bytes.len();
            for event in parser.push(bytes)? {
                if !emit(event) {
                    return Ok(());
                }
            }
            reader.consume(len);
            if parser.done {
                return Ok(());
            }
        }
    }

    pub fn cancel(&self, ids: &TurnIds) -> Result<()> {
        let response = self.post(
            "/api/ai/chat/cancel",
            &serde_json::to_value(ids)?,
            "application/json",
        )?;
        let data: Value = response.into_json()?;
        ensure!(
            data["success"] == true,
            "AI cancellation was not acknowledged"
        );
        Ok(())
    }
}

fn refusal(status: u16, body: &str) -> String {
    let data: Value = serde_json::from_str(body).unwrap_or(Value::Null);
    let message = data
        .get("message")
        .and_then(Value::as_str)
        .or_else(|| data.get("error").and_then(Value::as_str))
        .or_else(|| data.pointer("/error/message").and_then(Value::as_str))
        .unwrap_or(body)
        .trim();
    format!(
        "HTTP {status}: {}",
        if message.is_empty() {
            "AI request refused"
        } else {
            message
        }
    )
}

pub fn project_metadata(db: &Db, id: i64) -> (String, Option<String>) {
    let project = db
        .raw("Project", &json!(id))
        .ok()
        .flatten()
        .unwrap_or(Value::Null);
    let title = project["title"]
        .as_str()
        .or_else(|| project["name"].as_str())
        .unwrap_or("Current board")
        .to_owned();
    let team = project.get("teamId").filter(|v| !v.is_null()).map(|v| {
        v.as_str()
            .map(str::to_owned)
            .unwrap_or_else(|| v.to_string())
    });
    (title, team)
}

/// Web WelcomeScreen's local pool, kept deterministic so rerenders do not shuffle buttons.
pub fn suggestions(
    task: Option<&TaskView>,
    board: Option<(&str, &[BoardColumn])>,
    now: i64,
) -> Vec<String> {
    if let Some(task) = task {
        let reference = task.ticket_number.as_deref().unwrap_or("this ticket");
        return vec![
            format!("Summarize {reference}"),
            format!("Write my next reply on {reference}"),
            format!("What's blocking {reference}?"),
            format!("What are the next steps for {reference}?"),
        ];
    }
    if let Some((title, columns)) = board.filter(|(_, columns)| !columns.is_empty()) {
        let mut pool = vec!["What's important in my inbox?".into()];
        let busiest = columns
            .iter()
            .filter(|c| !c.cards.is_empty())
            .max_by_key(|c| c.cards.len());
        if let Some(column) = busiest {
            pool.push(format!(
                "Prioritize the {} tickets in {}",
                column.cards.len(),
                column.title
            ));
        }
        let unassigned = columns
            .iter()
            .flat_map(|c| &c.cards)
            .filter(|t| t.assignee_count == 0)
            .count();
        let overdue = columns
            .iter()
            .flat_map(|c| &c.cards)
            .filter(|t| {
                t.due_date
                    .as_deref()
                    .and_then(|d| chrono::DateTime::parse_from_rfc3339(d).ok())
                    .is_some_and(|d| d.timestamp() < now)
            })
            .count();
        if unassigned > 0 {
            pool.push(format!("Who should take the {unassigned} unassigned?"));
        }
        if overdue > 0 {
            pool.push(format!("Triage the {overdue} overdue tickets"));
        }
        pool.push(format!("What changed on {title} this week?"));
        if let Some(column) = columns
            .iter()
            .find(|c| !c.cards.is_empty() && busiest.is_none_or(|b| b.id != c.id))
            .or(busiest)
        {
            pool.push(format!("Summarize the {} column", column.title));
        }
        pool.extend([
            "What's been stuck the longest?".into(),
            "Top 3 focus this week?".into(),
        ]);
        pool.truncate(4);
        return pool;
    }
    [
        "What should I work on next?",
        "Summarize the last 3 days",
        "What's unassigned?",
        "What's blocking release?",
    ]
    .map(str::to_owned)
    .to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "event: status\ndata: {\"content\":\"Working…\"}\n\nevent: content\ndata: {\"content\":\"The blocking task is \"}\n\nevent: content\ndata: {\"content\":\"HTPR-123.\"}\n\nevent: title\ndata: {\"content\":\"Release blockers\"}\n\nevent: done\ndata: {\"status\":\"complete\",\"user_message_persisted\":true,\"assistant_persisted\":true}\n\n";

    #[test]
    fn request_matches_web_contract_and_cancel_identity() {
        let ids = TurnIds::new(session_id());
        let user = session_id();
        for id in [
            &ids.session_id,
            &ids.assistant_message_id,
            &ids.stream_id,
            &user,
        ] {
            assert_eq!(Uuid::parse_str(id).unwrap().get_version_num(), 4);
        }
        assert_ne!(ids.assistant_message_id, ids.stream_id);
        let context = DefaultContext {
            project_id: Some(123),
            task_id: Some(456),
            surface: "task_detail".into(),
            surface_path: "/detail/project-123/1".into(),
            view_name: Some("Board".into()),
        };
        let history = vec![Message {
            role: "human".into(),
            content: "Earlier message".into(),
        }];
        let body = request(
            "What is blocking this task?",
            &ids,
            &user,
            MODELS[1],
            &context,
            Some("42"),
            &history,
        );
        assert_eq!(
            body,
            json!({"message":"What is blocking this task?","aiFeature":"aiChat","teamId":"42",
            "session_id":ids.session_id,"user_message_id":user,"assistant_message_id":ids.assistant_message_id,"stream_id":ids.stream_id,
            "modelOptionId":"gpt-6-luna","model":"gpt-6-luna","provider":"openai","context_list":[],
            "default_context":{"project_id":123,"task_id":456,"surface":"task_detail","surface_path":"/detail/project-123/1","view_name":"Board"},
            "user_context":{},"chat_history":[{"role":"human","content":"Earlier message"}],"attachments":[],"images64":[],"pdfs64":[],"docx64":[],"byokProviderFlags":[]})
        );
        assert_eq!(
            serde_json::to_value(&ids).unwrap(),
            json!({"session_id":ids.session_id,"assistant_message_id":ids.assistant_message_id,"stream_id":ids.stream_id})
        );
        let all = request(
            "hello",
            &ids,
            &user,
            MODELS[0],
            &DefaultContext::default(),
            None,
            &[],
        );
        assert!(all.get("teamId").is_none());
        assert!(all["default_context"].get("project_id").is_none());
    }

    #[test]
    fn recorded_spec_frames_survive_every_chunk_boundary_and_crlf() {
        for sample in [SAMPLE.to_owned(), SAMPLE.replace('\n', "\r\n")] {
            for size in 1..=sample.len() {
                let mut parser = SseParser::default();
                let mut events = vec![];
                for bytes in sample.as_bytes().chunks(size) {
                    events.extend(parser.push(bytes).unwrap());
                }
                parser.finish().unwrap();
                assert_eq!(events.len(), 5);
                assert_eq!(events[0], Event::Status("Working…".into()));
                assert_eq!(events[1], Event::Content("The blocking task is ".into()));
                assert_eq!(events[2], Event::Content("HTPR-123.".into()));
                assert_eq!(events[3], Event::Title("Release blockers".into()));
                assert!(
                    matches!(&events[4], Event::Done(d) if d.status == "complete" && d.user_message_persisted && d.assistant_persisted)
                );
            }
        }
    }

    #[test]
    fn errors_cancellation_and_incomplete_streams_are_not_successful_replies() {
        let mut parser = SseParser::default();
        parser
            .push(b"event: content\ndata: {\"content\":\"partial\"}\n\n")
            .unwrap();
        assert!(parser.finish().is_err());
        let events = parser.push(b"event: error\ndata: {\"content\":\"Forbidden\"}\n\nevent: done\ndata: {\"status\":\"error\"}\n\n").unwrap();
        assert_eq!(events[0], Event::Error("Forbidden".into()));
        assert!(matches!(&events[1], Event::Done(d) if d.status == "error"));
        let mut cancelled = SseParser::default();
        assert!(
            matches!(&cancelled.push(b"event: done\ndata: {\"status\":\"complete\",\"cancelled\":true}\n\n").unwrap()[0], Event::Done(d) if d.cancelled)
        );
        assert!(
            SseParser::default()
                .push(b"event: content\ndata: nope\n\n")
                .is_err()
        );
        assert!(
            SseParser::default()
                .push(&vec![b'x'; MAX_FRAME + 1])
                .is_err()
        );
        assert_eq!(
            refusal(401, r#"{"error":"Unauthorized"}"#),
            "HTTP 401: Unauthorized"
        );
        assert_eq!(
            refusal(403, r#"{"message":"Model unavailable"}"#),
            "HTTP 403: Model unavailable"
        );
    }

    #[test]
    fn suggestions_use_task_reference_and_real_board_counts() {
        let task: TaskView = serde_json::from_value(json!({"id":456,"title":"Task","projectId":123,"ticketNumber":"HTPR-123","status":"Normal","dueDate":"2020-01-01T00:00:00Z"})).unwrap();
        assert_eq!(
            suggestions(Some(&task), None, 0)[2],
            "What's blocking HTPR-123?"
        );
        let columns = vec![BoardColumn {
            id: 1,
            title: "Todo".into(),
            ranking: "a".into(),
            cards: vec![task],
        }];
        assert_eq!(
            suggestions(None, Some(("Board", &columns)), 1_800_000_000),
            vec![
                "What's important in my inbox?",
                "Prioritize the 1 tickets in Todo",
                "Who should take the 1 unassigned?",
                "Triage the 1 overdue tickets"
            ]
        );
        assert_eq!(suggestions(None, None, 0).len(), 4);
        assert_eq!(
            suggestions(None, Some(("Empty", &[])), 0),
            suggestions(None, None, 0)
        );
    }
}
