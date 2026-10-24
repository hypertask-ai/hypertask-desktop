use crate::Result;
use anyhow::{Context, ensure};
use serde_json::Value;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

pub const BASE_URL: &str = "https://app.hypertask.ai/api/mcp";

#[derive(Debug, Clone)]
pub struct Response {
    pub status: u16,
    pub body: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportError {
    Offline,
    Timeout,
    InvalidResponse,
}

impl std::fmt::Display for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for TransportError {}

pub trait Transport: Send + 'static {
    fn mutate(
        &mut self,
        method: &str,
        path: &str,
        body: &Value,
        client_id: &str,
    ) -> std::result::Result<Response, TransportError> {
        if method == "POST" {
            self.post(path, body, client_id)
        } else {
            Err(TransportError::InvalidResponse)
        }
    }
    fn get(&mut self, path: &str) -> std::result::Result<Response, TransportError>;
    fn post(
        &mut self,
        path: &str,
        body: &Value,
        client_id: &str,
    ) -> std::result::Result<Response, TransportError>;
}

pub struct Http {
    agent: ureq::Agent,
    base: String,
    token: String,
    unauthorized: Arc<AtomicBool>,
}

impl Http {
    pub fn new(token: String) -> Result<Self> {
        Self::with_base(BASE_URL, token)
    }

    pub fn with_base(base: &str, token: String) -> Result<Self> {
        let url = url::Url::parse(base)?;
        ensure!(
            url.scheme() == "https"
                || (url.scheme() == "http"
                    && matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "::1"))),
            "HTTPS required"
        );
        ensure!(
            url.username().is_empty()
                && url.password().is_none()
                && url.query().is_none()
                && url.fragment().is_none(),
            "invalid API base"
        );
        ensure!(
            !token.is_empty() && !token.contains(['\r', '\n']),
            "invalid bearer token"
        );
        let agent = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(30))
            .redirects(0)
            .build();
        Ok(Self {
            agent,
            base: base.trim_end_matches('/').to_owned(),
            token,
            unauthorized: Arc::new(AtomicBool::new(false)),
        })
    }

    pub fn json_request(&self, method: &str, path: &str, body: Option<&Value>) -> Result<Value> {
        ensure!(
            matches!(method, "GET" | "POST" | "PUT" | "PATCH" | "DELETE"),
            "unsupported HTTP method"
        );
        let response = self.request(method, path, body, None)?;
        if !(200..300).contains(&response.status)
            || response.body.get("success") == Some(&Value::Bool(false))
        {
            anyhow::bail!(
                "{}",
                response
                    .body
                    .get("message")
                    .or_else(|| response.body.get("error"))
                    .and_then(Value::as_str)
                    .unwrap_or("Request failed")
            );
        }
        Ok(response.body)
    }

    pub fn unauthorized_flag(&self) -> Arc<AtomicBool> {
        self.unauthorized.clone()
    }

    fn request(
        &self,
        method: &str,
        path: &str,
        body: Option<&Value>,
        client_id: Option<&str>,
    ) -> std::result::Result<Response, TransportError> {
        if !path.starts_with('/') || path.starts_with("//") {
            return Err(TransportError::InvalidResponse);
        }
        let endpoint = if path.starts_with("/api/") {
            let mut url = url::Url::parse(&self.base).map_err(|_| TransportError::InvalidResponse)?;
            let (path, query) = path.split_once('?').map_or((path, None), |(p, q)| (p, Some(q)));
            url.set_path(path);
            url.set_query(query);
            url.to_string()
        } else { format!("{}{path}", self.base) };
        let mut request = self
            .agent
            .request(method, &endpoint)
            .set("Authorization", &format!("Bearer {}", self.token))
            .set("Accept", "application/json");
        if path == "/api/ai/task-writer" { request = request.timeout(Duration::from_secs(310)); }
        if path=="/ai/improve" {request=request.timeout(Duration::from_secs(70));}
        if let Some(client_id) = client_id {
            request = request
                .set("X-Correlation-ID", client_id)
                .set("Idempotency-Key", client_id);
        }
        let response = match body {
            Some(v) => request.send_json(v),
            None => request.call(),
        };
        match response {
            Ok(r) | Err(ureq::Error::Status(_, r)) => {
                let status = r.status();
                if status == 401 && !path.starts_with("/api/") {
                    self.unauthorized.store(true, Ordering::SeqCst);
                }
                let writer_text = path == "/api/ai/task-writer" && !r.header("Content-Type").unwrap_or("").contains("application/json");
                let body = if writer_text { r.into_string().map(Value::String).unwrap_or(Value::Null) }
                    else { r.into_json().unwrap_or(Value::Null) };
                if (200..300).contains(&status) && body.is_null() {
                    return Err(TransportError::InvalidResponse);
                }
                Ok(Response { status, body })
            }
            Err(ureq::Error::Transport(error)) => {
                use std::error::Error;
                let mut source = error.source();
                while let Some(s) = source {
                    if let Some(io) = s.downcast_ref::<std::io::Error>() {
                        if matches!(
                            io.kind(),
                            std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                        ) {
                            return Err(TransportError::Timeout);
                        }
                    }
                    source = s.source();
                }
                Err(TransportError::Offline)
            }
        }
    }
}

impl Transport for Http {
    fn mutate(
        &mut self,
        method: &str,
        path: &str,
        body: &Value,
        client_id: &str,
    ) -> std::result::Result<Response, TransportError> {
        self.request(method, path, if body.is_null() {None}else{Some(body)}, Some(client_id))
    }
    fn get(&mut self, path: &str) -> std::result::Result<Response, TransportError> {
        self.request("GET", path, None, None)
    }
    fn post(
        &mut self,
        path: &str,
        body: &Value,
        client_id: &str,
    ) -> std::result::Result<Response, TransportError> {
        self.request("POST", path, Some(body), Some(client_id))
    }
}

pub(crate) fn successful(response: Response) -> Result<Value> {
    ensure!(
        (200..300).contains(&response.status),
        "HTTP {}",
        response.status
    );
    ensure!(
        response.body.get("success").context("missing success")? == true,
        "API response was not successful"
    );
    Ok(response.body)
}

#[cfg(test)]
mod shell_tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::TcpListener,
        thread,
    };

    #[test]
    fn shell_json_methods_send_bearer_and_preserve_server_errors() {
        for method in ["GET", "POST", "PUT", "PATCH", "DELETE"] {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap();
            let expected = method.to_owned();
            let server = thread::spawn(move || {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut request = vec![];
                let mut buffer = [0; 4096];
                loop {
                    let count = stream.read(&mut buffer).unwrap();
                    assert!(count > 0);
                    request.extend_from_slice(&buffer[..count]);
                    if request.windows(4).any(|w| w == b"\r\n\r\n") {
                        break;
                    }
                }
                let header_end = request.windows(4).position(|w| w == b"\r\n\r\n").unwrap() + 4;
                let headers = String::from_utf8(request[..header_end].to_vec()).unwrap();
                let content_length = headers
                    .lines()
                    .find_map(|line| {
                        let (key, value) = line.split_once(':')?;
                        key.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().unwrap())
                    })
                    .unwrap_or(0);
                while request.len() < header_end + content_length {
                    let count = stream.read(&mut buffer).unwrap();
                    assert!(count > 0);
                    request.extend_from_slice(&buffer[..count]);
                }
                if expected != "GET" {
                    let body: Value = serde_json::from_slice(&request[header_end..]).unwrap();
                    assert_eq!(body["displayName"], "Saved");
                }
                let request = String::from_utf8(request).unwrap();
                assert!(
                    request.starts_with(&format!("{expected} /user/profile HTTP/1.1\r\n")),
                    "{request}"
                );
                assert!(
                    request
                        .to_lowercase()
                        .contains("authorization: bearer fixture-secret\r\n")
                );
                let body = r#"{"success":true,"user":{"displayName":"Saved"}}"#;
                write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
            });
            let http =
                Http::with_base(&format!("http://{address}"), "fixture-secret".into()).unwrap();
            let result = http
                .json_request(
                    method,
                    "/user/profile",
                    (method != "GET")
                        .then(|| serde_json::json!({"displayName": "Saved"}))
                        .as_ref(),
                )
                .unwrap();
            assert_eq!(result["user"]["displayName"], "Saved");
            server.join().unwrap();
        }
    }
}
