use crate::{Db, Result};
use anyhow::{Context, ensure};
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub struct Event {
    pub event: String,
    #[serde(default)]
    pub channel: Option<String>,
    #[serde(default)]
    pub data: Value,
}

impl Event {
    pub fn parse(frame: &str) -> Result<Self> {
        let mut event: Self = serde_json::from_str(frame)?;
        if let Value::String(data) = &event.data {
            event.data = serde_json::from_str(data)?;
        }
        Ok(event)
    }
}

impl Db {
    pub fn apply_event(&mut self, frame: &str) -> Result<()> {
        let event = Event::parse(frame)?;
        self.transaction(|db| {
            match event.event.as_str() {
                "board:changed" => {
                    db.request_refetch("board", channel_id(&event, "private-project-")?)?
                }
                "task:changed" => {
                    db.request_refetch("task", channel_id(&event, "private-task-")?)?
                }
                "comment:changed" => {
                    let task_id = channel_id(&event, "private-task-")?;
                    if let Some(comment) = event.data.get("comment") {
                        if comment["id"].as_i64().is_some()
                            && comment["text"].is_string()
                            && comment["createdAt"].is_string()
                        {
                            ensure!(
                                comment.get("taskId").is_none_or(|id| *id == task_id),
                                "comment channel mismatch"
                            );
                            db.upsert_comment(task_id, comment)?;
                        }
                    }
                    db.request_refetch("comments", task_id)?;
                    db.request_refetch("task", task_id)?;
                }
                "time:changed" => {
                    if event.channel.as_deref().is_some_and(|channel| channel.starts_with("private-time-task-")) {
                        db.request_refetch("time-task", channel_id(&event, "private-time-task-")?)?;
                    } else {
                        db.request_refetch("time-board", channel_id(&event, "private-time-project-")?)?;
                    }
                }
                "inbox:changed" => {
                    channel_id(&event, "private-user-")?;
                    db.request_refetch("inbox", 0)?;
                }
                _ => (),
            }
            Ok(())
        })
    }
}

fn channel_id(event: &Event, prefix: &str) -> Result<i64> {
    let id: i64 = event
        .channel
        .as_deref()
        .and_then(|c| c.strip_prefix(prefix))
        .context("event channel mismatch")?
        .parse()?;
    ensure!(id > 0, "invalid channel id");
    Ok(id)
}

#[cfg(feature = "realtime-ws")]
pub mod ws {
    use super::*;
    use crate::worker::WorkerHandle;
    use serde_json::json;
    use std::{
        net::{TcpStream, ToSocketAddrs},
        sync::mpsc,
        thread,
        time::Duration,
    };
    use tungstenite::{Message, WebSocket, stream::MaybeTlsStream};

    pub struct Subscriber {
        stop: mpsc::Sender<()>,
        thread: Option<thread::JoinHandle<()>>,
    }

    pub struct SessionAuth {
        pub endpoint: String,
        pub cookie: String,
    }

    impl SessionAuth {
        pub fn authorize(&self, socket_id: &str, channel: &str) -> Result<Value> {
            ensure!(
                url::Url::parse(&self.endpoint)?.scheme() == "https",
                "HTTPS required for channel authorization"
            );
            let agent = ureq::AgentBuilder::new()
                .timeout(Duration::from_secs(30))
                .redirects(0)
                .build();
            let response = agent
                .post(&self.endpoint)
                .set("Cookie", &self.cookie)
                .send_json(json!({"socket_id":socket_id,"channel_name":channel}))
                .map_err(|_| anyhow::anyhow!("realtime session authorization failed"))?;
            Ok(response
                .into_json()
                .map_err(|_| anyhow::anyhow!("invalid channel authorization response"))?)
        }
    }

    impl Subscriber {
        pub fn spawn(
            url: String,
            channels: Vec<String>,
            worker: WorkerHandle,
            mut authorize: impl FnMut(&str, &str) -> Result<Value> + Send + 'static,
        ) -> Result<Self> {
            let parsed = url::Url::parse(&url)?;
            ensure!(
                parsed.scheme() == "wss"
                    || (parsed.scheme() == "ws"
                        && matches!(parsed.host_str(), Some("localhost" | "127.0.0.1"))),
                "secure websocket required"
            );
            let (stop, receive) = mpsc::channel();
            let thread = thread::Builder::new()
                .name("ht-realtime".into())
                .spawn(move || {
                    let mut attempts = 0;
                    loop {
                        if receive.try_recv().is_ok() {
                            break;
                        }
                        let result = connect(&url, &channels, &mut authorize);
                        if let Ok(mut socket) = result {
                            if worker.wake().is_err() {
                                return;
                            }
                            attempts = 0;
                            loop {
                                if receive.try_recv().is_ok() {
                                    let _ = socket.close(None);
                                    return;
                                }
                                match socket.read() {
                                    Ok(Message::Text(frame)) => match Event::parse(&frame) {
                                        Ok(event) if event.event == "pusher:ping" => {
                                            let _ = socket.send(Message::Text(
                                                json!({"event":"pusher:pong","data":{}})
                                                    .to_string()
                                                    .into(),
                                            ));
                                        }
                                        Ok(event)
                                            if event.event == "pusher:error"
                                                || event.event == "pusher:subscription_error" =>
                                        {
                                            break;
                                        }
                                        Ok(event)
                                            if !event.event.starts_with("pusher:")
                                                && !event.event.starts_with("pusher_internal:") =>
                                        {
                                            if worker.event(frame.to_string()).is_err() {
                                                return;
                                            }
                                        }
                                        _ => (),
                                    },
                                    Ok(Message::Close(_)) => break,
                                    Ok(_) => {
                                        let _ = socket.flush();
                                    }
                                    Err(tungstenite::Error::Io(error))
                                        if matches!(
                                            error.kind(),
                                            std::io::ErrorKind::WouldBlock
                                                | std::io::ErrorKind::TimedOut
                                        ) =>
                                    {
                                        continue;
                                    }
                                    Err(_) => break,
                                }
                            }
                        }
                        if receive
                            .recv_timeout(Duration::from_secs(
                                crate::queue::backoff(attempts) as u64
                            ))
                            .is_ok()
                        {
                            break;
                        }
                        attempts = attempts.saturating_add(1);
                    }
                })?;
            Ok(Self {
                stop,
                thread: Some(thread),
            })
        }
    }

    impl Drop for Subscriber {
        fn drop(&mut self) {
            let _ = self.stop.send(());
            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
        }
    }

    fn connect(
        url: &str,
        channels: &[String],
        authorize: &mut impl FnMut(&str, &str) -> Result<Value>,
    ) -> Result<WebSocket<MaybeTlsStream<TcpStream>>> {
        let parsed = url::Url::parse(url)?;
        let host = parsed.host_str().context("missing websocket host")?;
        let port = parsed
            .port_or_known_default()
            .context("missing websocket port")?;
        let tcp = (host, port)
            .to_socket_addrs()?
            .find_map(|address| TcpStream::connect_timeout(&address, Duration::from_secs(10)).ok())
            .context("websocket connection failed")?;
        tcp.set_read_timeout(Some(Duration::from_secs(30)))?;
        tcp.set_write_timeout(Some(Duration::from_secs(30)))?;
        let (mut socket, _) = tungstenite::client_tls_with_config(url, tcp, None, None)?;
        match socket.get_mut() {
            MaybeTlsStream::Plain(tcp) => {
                tcp.set_read_timeout(Some(Duration::from_secs(1)))?;
                tcp.set_write_timeout(Some(Duration::from_secs(30)))?;
            }
            MaybeTlsStream::Rustls(tls) => {
                tls.sock.set_read_timeout(Some(Duration::from_secs(1)))?;
                tls.sock.set_write_timeout(Some(Duration::from_secs(30)))?;
            }
            _ => (),
        }
        let frame = socket.read()?.into_text()?;
        let event = Event::parse(&frame)?;
        ensure!(
            event.event == "pusher:connection_established",
            "missing Pusher handshake"
        );
        let socket_id = event.data["socket_id"]
            .as_str()
            .context("missing socket id")?;
        for channel in channels {
            let auth = authorize(socket_id, channel)?;
            ensure!(auth["auth"].is_string(), "missing channel signature");
            let mut data = auth;
            data["channel"] = json!(channel);
            socket.send(Message::Text(
                json!({"event":"pusher:subscribe","data":data})
                    .to_string()
                    .into(),
            ))?;
        }
        Ok(socket)
    }
}
