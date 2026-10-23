use anyhow::{Result, bail};
use serde_json::Value;
use std::{
    io::{ErrorKind, Read, Write},
    net::TcpListener,
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

pub const SITE_URL: &str = "https://app.hypertask.ai";
pub const CALLBACK_TIMEOUT: Duration = Duration::from_secs(300);

const CLOSE_TAB_PAGE: &str = "<!doctype html><meta charset=\"utf-8\"><title>Hypertask</title><p style=\"font-family:sans-serif;margin:3em\">You can close this tab and return to Hypertask.</p>";

pub fn sign_in_url(port: u16) -> String {
    format!("{SITE_URL}/cli-auth?port={port}")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallbackError {
    Malformed,
    WrongPath,
    MissingCode,
}

pub fn parse_callback(head: &str) -> std::result::Result<String, CallbackError> {
    let line = head.lines().next().unwrap_or("");
    let mut parts = line.split_ascii_whitespace();
    let (Some(method), Some(target)) = (parts.next(), parts.next()) else {
        return Err(CallbackError::Malformed);
    };
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    if method != "GET" || path != "/callback" {
        return Err(CallbackError::WrongPath);
    }
    url::form_urlencoded::parse(query.as_bytes())
        .find(|(key, value)| key == "code" && !value.is_empty())
        .map(|(_, value)| value.into_owned())
        .ok_or(CallbackError::MissingCode)
}

fn reply(stream: &mut impl Write, status: &str, body: &str) {
    let _ = write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
}

pub fn serve_callback(listener: &TcpListener, timeout: Duration) -> Result<String> {
    listener.set_nonblocking(true)?;
    let deadline = Instant::now() + timeout;
    loop {
        match listener.accept() {
            Ok((mut stream, _)) => {
                stream.set_nonblocking(false)?;
                stream.set_read_timeout(Some(Duration::from_secs(1)))?;
                let mut bytes = Vec::new();
                let mut buf = [0; 1024];
                while bytes.len() < 8192 && !bytes.windows(4).any(|w| w == b"\r\n\r\n") {
                    match stream.read(&mut buf) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => bytes.extend_from_slice(&buf[..n]),
                    }
                }
                match parse_callback(&String::from_utf8_lossy(&bytes)) {
                    Ok(code) => {
                        reply(&mut stream, "200 OK", CLOSE_TAB_PAGE);
                        return Ok(code);
                    }
                    Err(CallbackError::WrongPath) => reply(&mut stream, "404 Not Found", ""),
                    Err(_) => reply(&mut stream, "400 Bad Request", ""),
                }
            }
            Err(error) if error.kind() == ErrorKind::WouldBlock => {
                if Instant::now() >= deadline {
                    bail!("sign-in timed out");
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(error) => return Err(error.into()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExchangeError {
    Rejected(u16),
    Malformed,
    Network,
}

impl std::fmt::Display for ExchangeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Rejected(_) => write!(f, "Hypertask rejected the sign-in code"),
            Self::Malformed => write!(f, "Hypertask sent an unexpected reply"),
            Self::Network => write!(f, "Could not reach Hypertask"),
        }
    }
}
impl std::error::Error for ExchangeError {}

pub fn exchange_code(base: &str, code: &str) -> std::result::Result<String, ExchangeError> {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(30))
        .redirects(0)
        .build();
    let response = agent
        .post(&format!(
            "{}/api/cli/token-exchange",
            base.trim_end_matches('/')
        ))
        .send_json(serde_json::json!({ "code": code }));
    match response {
        Ok(r) => r
            .into_json::<Value>()
            .ok()
            .and_then(|v| v["token"].as_str().map(str::to_owned))
            .filter(|t| !t.is_empty() && !t.contains(['\r', '\n']))
            .ok_or(ExchangeError::Malformed),
        Err(ureq::Error::Status(status, _)) => Err(ExchangeError::Rejected(status)),
        Err(ureq::Error::Transport(_)) => Err(ExchangeError::Network),
    }
}

pub trait TokenStore: Send + Sync {
    fn get(&self) -> Option<String>;
    fn set(&self, secret: &str) -> Result<()>;
    fn clear(&self);
}

#[derive(Default)]
pub struct MemoryStore(Mutex<Option<String>>);

impl TokenStore for MemoryStore {
    fn get(&self) -> Option<String> {
        self.0.lock().ok()?.clone()
    }
    fn set(&self, secret: &str) -> Result<()> {
        *self
            .0
            .lock()
            .map_err(|_| anyhow::anyhow!("credential store poisoned"))? = Some(secret.to_owned());
        Ok(())
    }
    fn clear(&self) {
        if let Ok(mut slot) = self.0.lock() {
            *slot = None;
        }
    }
}

pub fn clear_if_unauthorized(flag: &AtomicBool, store: &dyn TokenStore) -> bool {
    let hit = flag.swap(false, Ordering::SeqCst);
    if hit {
        store.clear();
    }
    hit
}
