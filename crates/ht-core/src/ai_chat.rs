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

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct Session {
    pub id: String,
    pub title: Option<String>,
    pub messages: Vec<Message>,
}

// The legacy endpoint returns every transcript. Parse directly from the reader on a worker,
// not ureq::into_json (10 MiB limit) or a second, unbounded in-memory JSON tree.
const MAX_HISTORY_BYTES: u64 = 256 * 1024 * 1024;

fn parse_sessions(reader: impl Read) -> Result<Vec<Session>> {
    #[derive(Deserialize)]
    struct History {
        success: bool,
        sessions: Vec<Session>,
    }
    let data: History = serde_json::from_reader(reader.take(MAX_HISTORY_BYTES))
        .context("Could not load AI history (invalid or larger than 256 MiB)")?;
    ensure!(data.success, "Server refused AI history");
    Ok(data.sessions)
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
            Err(ureq::Error::Status(409, response)) if path == "/api/ai/chat/cancel" => {
                Ok(response)
            }
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

    /// Fetch only when history opens, off the render thread.
    pub fn sessions(&self) -> Result<Vec<Session>> {
        let response = match self
            .agent
            .get(&format!("{}/api/ai-chat/all-sessions", self.base))
            .set("Authorization", &format!("Bearer {}", self.token))
            .set("Accept", "application/json")
            .call()
        {
            Ok(response) => response,
            Err(ureq::Error::Status(status, response)) => {
                let mut body = String::new();
                response
                    .into_reader()
                    .take(64 * 1024)
                    .read_to_string(&mut body)?;
                bail!("{}", refusal(status, &body));
            }
            Err(error) => bail!("AI history request failed: {error}"),
        };
        parse_sessions(response.into_reader())
    }

    pub fn cancel(&self, ids: &TurnIds) -> Result<()> {
        let response = self.post(
            "/api/ai/chat/cancel",
            &serde_json::to_value(ids)?,
            "application/json",
        )?;
        if response.status() == 409 {
            return Ok(()); // The stream is already idle, not a failed Stop.
        }
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
    seed: u64,
) -> Vec<String> {
    let mut pool = if let Some(task) = task {
        let reference = task.ticket_number.as_deref().unwrap_or("this ticket");
        vec![
            format!("Summarize {reference}"),
            format!("Write my next reply on {reference}"),
            format!("What's blocking {reference}?"),
            format!("What are the next steps for {reference}?"),
            format!("Draft a status update for {reference}"),
            format!("Suggest sub-tasks for {reference}"),
        ]
    } else if let Some((title, columns)) = board.filter(|(_, columns)| !columns.is_empty()) {
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
            "Any duplicate tickets here?".into(),
        ]);
        pool
    } else {
    [
        "What should I work on next?",
        "Summarize the last 3 days",
        "What's unassigned?",
        "What's blocking release?",
        "Draft a task for the next sprint",
        "What's overdue?",
        "Who's overloaded?",
        "What's been stuck over a week?",
        "Top 3 focus this week?",
    ]
    .map(str::to_owned)
    .to_vec()
    };
    pool.sort_by_cached_key(|text| text.bytes().fold(seed, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
    }));
    pool.truncate(4);
    pool
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "event: status\ndata: {\"content\":\"Working…\"}\n\nevent: content\ndata: {\"content\":\"The blocking task is \"}\n\nevent: content\ndata: {\"content\":\"HTPR-123.\"}\n\nevent: title\ndata: {\"content\":\"Release blockers\"}\n\nevent: done\ndata: {\"status\":\"complete\",\"user_message_persisted\":true,\"assistant_persisted\":true}\n\n";

    fn server(
        status: &str,
        content_type: &str,
        response_body: &str,
    ) -> (String, std::thread::JoinHandle<(String, Value)>) {
        use std::{io::Write, net::TcpListener};
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response_body}",
            response_body.len()
        );
        let handle = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(10)))
                .unwrap();
            let mut reader = BufReader::new(socket.try_clone().unwrap());
            let mut headers = String::new();
            let mut length = 0;
            loop {
                let mut line = String::new();
                assert!(reader.read_line(&mut line).unwrap() > 0);
                headers.push_str(&line);
                if line == "\r\n" {
                    break;
                }
                if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                    length = value.trim().parse::<usize>().unwrap();
                }
            }
            let mut body = vec![0; length];
            reader.read_exact(&mut body).unwrap();
            socket.write_all(response.as_bytes()).unwrap();
            let body = if body.is_empty() {
                Value::Null
            } else {
                serde_json::from_slice(&body).unwrap()
            };
            (headers, body)
        });
        (base, handle)
    }

    #[test]
    fn http_stream_and_stop_use_real_endpoints_headers_and_ids() {
        let ids = TurnIds::new(session_id());
        let body = request(
            "hello",
            &ids,
            &session_id(),
            MODELS[0],
            &DefaultContext::default(),
            None,
            &[],
        );
        let (base, server) = server("200 OK", "text/event-stream; charset=utf-8", SAMPLE);
        let client = Client::with_base(&base, "desktop-mcp-token".into()).unwrap();
        let mut events = vec![];
        client
            .stream(&body, |event| {
                events.push(event);
                true
            })
            .unwrap();
        assert_eq!(events.len(), 5);
        let (headers, received) = server.join().unwrap();
        assert!(headers.starts_with("POST /api/ai/chat/stream HTTP/1.1\r\n"));
        let headers = headers.to_ascii_lowercase();
        assert!(headers.contains("authorization: bearer desktop-mcp-token\r\n"));
        assert!(headers.contains("accept: text/event-stream\r\n"));
        assert!(headers.contains("content-type: application/json\r\n"));
        assert!(!headers.contains("x-hypertask-sidebar-project"));
        assert!(!headers.contains("cookie:"));
        assert_eq!(received, body);

        let (base, server) = self::server(
            "200 OK",
            "application/json",
            r#"{"success":true,"status":"cancelling"}"#,
        );
        Client::with_base(&base, "desktop-mcp-token".into())
            .unwrap()
            .cancel(&ids)
            .unwrap();
        let (headers, body) = server.join().unwrap();
        assert!(headers.starts_with("POST /api/ai/chat/cancel HTTP/1.1\r\n"));
        assert_eq!(body, serde_json::to_value(&ids).unwrap());
    }

    #[test]
    fn http_refusals_and_eof_are_visible_errors_not_fake_replies() {
        for (status, content_type, response, expected) in [
            (
                "401 Unauthorized",
                "application/json",
                r#"{"error":"Desktop bearer not supported"}"#,
                "HTTP 401: Desktop bearer not supported",
            ),
            (
                "403 Forbidden",
                "application/json",
                r#"{"message":"Model unavailable"}"#,
                "HTTP 403: Model unavailable",
            ),
            (
                "200 OK",
                "text/event-stream",
                "event: content\ndata: {\"content\":\"partial\"}\n\n",
                "interrupted before done",
            ),
            (
                "200 OK",
                "text/html",
                "login",
                "did not return an AI event stream",
            ),
        ] {
            let (base, server) = server(status, content_type, response);
            let mut content = String::new();
            let error = Client::with_base(&base, "desktop-mcp-token".into())
                .unwrap()
                .stream(&json!({"message":"hello"}), |event| {
                    if let Event::Content(delta) = event {
                        content.push_str(&delta);
                    }
                    true
                })
                .unwrap_err();
            assert!(error.to_string().contains(expected), "{error}");
            if response.contains("partial") {
                assert_eq!(content, "partial");
            }
            server.join().unwrap();
        }
        assert!(Client::with_base("http://example.com", "token".into()).is_err());
        assert!(Client::new("bad\r\ntoken".into()).is_err());
    }

    #[test]
    fn idle_cancel_is_success_but_other_refusals_preserve_server_message() {
        for (status, body, ok) in [
            ("409 Conflict", r#"{"error":"Stream already idle"}"#, true),
            ("403 Forbidden", r#"{"message":"Access denied"}"#, false),
        ] {
            let (base, server) = server(status, "application/json", body);
            let result = Client::with_base(&base, "token".into())
                .unwrap()
                .cancel(&TurnIds::new(session_id()));
            if ok {
                result.unwrap();
            } else {
                assert!(result.unwrap_err().to_string().contains("Access denied"));
            }
            server.join().unwrap();
        }
    }

    #[test]
    fn history_uses_bearer_only_get_and_preserves_server_refusals() {
        let (base, server) = server(
            "200 OK",
            "application/json",
            r#"{"success":true,"sessions":[{"id":"saved","title":"Saved","messages":[{"role":"human","content":"Hi"}]}]}"#,
        );
        let sessions = Client::with_base(&base, "human-token".into())
            .unwrap()
            .sessions()
            .unwrap();
        assert_eq!(sessions[0].messages[0].content, "Hi");
        let (headers, _) = server.join().unwrap();
        assert!(headers.starts_with("GET /api/ai-chat/all-sessions HTTP/1.1\r\n"));
        let headers = headers.to_ascii_lowercase();
        assert!(headers.contains("authorization: bearer human-token\r\n"));
        assert!(!headers.contains("x-hypertask-sidebar-project"));
        assert!(!headers.contains("cookie:"));
        let (base, server) = self::server(
            "403 Forbidden",
            "application/json",
            r#"{"message":"History denied"}"#,
        );
        let error = Client::with_base(&base, "token".into())
            .unwrap()
            .sessions()
            .unwrap_err();
        assert_eq!(error.to_string(), "HTTP 403: History denied");
        server.join().unwrap();
    }

    #[test]
    fn history_accepts_all_1517_transcripts_beyond_ureq_json_limit() {
        let content = "é".repeat(4096);
        let sessions: Vec<_> = (0..1517)
            .map(|i| {
                json!({
                    "id": format!("session-{i}"), "title": null,
                    "messages": [{"role":"assistant", "content":content}],
                    "ignoredAttachments": [1,2,3]
                })
            })
            .collect();
        let bytes = serde_json::to_vec(&json!({"success":true,"sessions":sessions})).unwrap();
        assert!(bytes.len() > 10 * 1024 * 1024);
        let parsed = parse_sessions(bytes.as_slice()).unwrap();
        assert_eq!(parsed.len(), 1517);
        assert_eq!(parsed[1516].messages[0].content, content);
        assert!(parse_sessions(&b"{\"success\":true,\"sessions\":["[..]).is_err());
        assert!(parse_sessions(&b"{\"success\":false,\"sessions\":[]}"[..]).is_err());
    }

    #[test]
    fn write_live_request_fixture() {
        let ids = TurnIds {
            session_id: "73810000-0000-4000-8000-000000000001".into(),
            assistant_message_id: "73810000-0000-4000-8000-000000000002".into(),
            stream_id: "73810000-0000-4000-8000-000000000003".into(),
        };
        let body = request(
            "Reply with exactly: Native AI sidebar is live.",
            &ids,
            "73810000-0000-4000-8000-000000000004",
            MODELS[0],
            &DefaultContext {
                surface: "inbox".into(),
                surface_path: "/inbox".into(),
                ..Default::default()
            },
            None,
            &[],
        );
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/ai-live-request.json");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, serde_json::to_vec_pretty(&body).unwrap()).unwrap();
    }

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
    fn suggestions_use_full_web_pools_without_duplicates_or_invented_counts() {
        let task: TaskView = serde_json::from_value(json!({"id":456,"title":"Task","projectId":123,"ticketNumber":"HTPR-123","status":"Normal","dueDate":"2020-01-01T00:00:00Z"})).unwrap();
        let ticket_pool = ["Summarize HTPR-123", "Write my next reply on HTPR-123", "What's blocking HTPR-123?", "What are the next steps for HTPR-123?", "Draft a status update for HTPR-123", "Suggest sub-tasks for HTPR-123"];
        let result = suggestions(Some(&task), None, 0, 7381);
        assert_eq!(result.len(), 4);
        assert!(result.iter().all(|s| ticket_pool.contains(&s.as_str())));
        let columns = vec![BoardColumn { id: 1, title: "Todo".into(), ranking: "a".into(), cards: vec![task] }];
        let board_pool = ["What's important in my inbox?", "Prioritize the 1 tickets in Todo", "Who should take the 1 unassigned?", "Triage the 1 overdue tickets", "What changed on Board this week?", "Summarize the Todo column", "What's been stuck the longest?", "Top 3 focus this week?", "Any duplicate tickets here?"];
        let result = suggestions(None, Some(("Board", &columns)), 1_800_000_000, 7381);
        assert_eq!(result.len(), 4);
        assert!(result.iter().all(|s| board_pool.contains(&s.as_str())));
        let mut unique = result.clone();
        unique.sort(); unique.dedup();
        assert_eq!(unique.len(), 4);
        assert_eq!(suggestions(None, None, 0, 7381), suggestions(None, None, 1, 7381));
        let general_pool = ["What should I work on next?", "Summarize the last 3 days", "What's unassigned?", "What's blocking release?", "Draft a task for the next sprint", "What's overdue?", "Who's overloaded?", "What's been stuck over a week?", "Top 3 focus this week?"];
        for result in [suggestions(None, None, 0, 7381), suggestions(None, Some(("Empty", &[])), 0, 7381)] {
            assert_eq!(result.len(), 4);
            assert!(result.iter().all(|s| general_pool.contains(&s.as_str())));
        }
        let result = suggestions(None, Some(("Board", &columns)), 0, 7381);
        assert!(!result.iter().any(|s| s.contains("overdue")));
    }
}
