use ht_core::{Db, Http, Worker, worker::SyncStatus};
use serde_json::json;
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU16, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

struct Server {
    base: String,
    status: Arc<AtomicU16>,
    requests: mpsc::Receiver<String>,
    stop: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}

impl Server {
    fn new(status: u16) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        listener.set_nonblocking(true).unwrap();
        let status = Arc::new(AtomicU16::new(status));
        let stop = Arc::new(AtomicBool::new(false));
        let (send, requests) = mpsc::channel();
        let server_status = status.clone();
        let server_stop = stop.clone();
        let thread = thread::spawn(move || {
            while !server_stop.load(Ordering::Acquire) {
                let (mut stream, _) = match listener.accept() {
                    Ok(pair) => pair,
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                        continue;
                    }
                    Err(e) => panic!("{e}"),
                };
                // macOS accepted sockets inherit the listener's nonblocking mode.
                stream.set_nonblocking(false).unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut request = Vec::new();
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
                let length = headers
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().unwrap())
                    })
                    .unwrap_or(0);
                while request.len() < header_end + length {
                    let count = stream.read(&mut buffer).unwrap();
                    assert!(count > 0);
                    request.extend_from_slice(&buffer[..count]);
                }
                let path = headers.split_whitespace().nth(1).unwrap().to_owned();
                let status = if matches!(path.as_str(), "/inbox/list" | "/tasks/update") {
                    server_status.load(Ordering::Acquire)
                } else {
                    200
                };
                let body = if status != 200 {
                    json!({"success":false,"error":"fixture-secret-server-body"})
                } else if path == "/inbox/list" {
                    json!({"success":true,"user_notifications":[]})
                } else if path == "/tasks?task_id=1" {
                    json!({"success":true,"tasks":[{"id":1,"title":"server","projectId":15,"status":"Normal"}]})
                } else if path == "/tasks/update" {
                    json!({"success":true,"task":{"id":1,"title":"server","projectId":15,"status":"Normal"}})
                } else if path.ends_with("/members") {
                    json!({"success":true,"projectId":15,"members":[]})
                } else if path.ends_with("/labels") {
                    json!({"success":true,"projectId":15,"labels":[]})
                } else { panic!("unexpected path {path}"); }.to_string();
                write!(stream, "HTTP/1.1 {status} Fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
                let _ = send.send(path);
            }
        });
        Self {
            base,
            status,
            requests,
            stop,
            thread: Some(thread),
        }
    }

    fn worker(&self, path: &std::path::Path) -> Worker {
        Worker::start(
            path,
            Http::with_base(&self.base, "fixture-secret-token".into()).unwrap(),
        )
        .unwrap()
    }

    fn request(&self, expected: &str) {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let path = self
                .requests
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                .unwrap();
            if path == expected {
                return;
            }
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        self.thread.take().unwrap().join().unwrap();
    }
}

fn wait_status(worker: &Worker, expected: SyncStatus) {
    let deadline = Instant::now() + Duration::from_secs(15);
    while worker.status() != expected {
        assert!(
            Instant::now() < deadline,
            "expected {expected:?}, got {:?}",
            worker.status()
        );
        thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn transient_http_error_retries_automatically_without_showing_banner() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("local.db");
    let server = Server::new(503);
    let worker = server.worker(&db);
    worker.handle().sync_inbox().unwrap();
    wait_status(&worker, SyncStatus::Retrying);
    assert_eq!(worker.status().message(), None);
    server.request("/inbox/list");
    server.status.store(200, Ordering::Release);
    // No wake or new user command: the failed pull must retry itself.
    server.request("/inbox/list");
    wait_status(&worker, SyncStatus::Healthy);
    assert_eq!(worker.status().message(), None);
    drop(worker);
    let log = std::fs::read_to_string(db.with_extension("sync.log")).unwrap();
    assert!(log.contains("operation=inbox code=http-503 attempts=1"));
    assert!(log.contains("event=recovered operation=inbox"));
    assert!(!log.contains("fixture-secret"));
}

#[test]
fn persistent_failure_shows_banner_and_reconnect_only_clears_after_verified_refetch() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("local.db");
    let server = Server::new(503);
    let worker = server.worker(&db);
    worker.handle().sync_inbox().unwrap();
    wait_status(&worker, SyncStatus::Unavailable);
    assert_eq!(
        worker.status().message(),
        Some("Sync unavailable — edits are saved locally")
    );
    // A successful unrelated pull and local event must not erase the failure.
    worker.handle().sync_catalog(15).unwrap();
    worker
        .handle()
        .event(json!({"event":"ignored","data":{}}).to_string())
        .unwrap();
    server.request("/projects/15/labels");
    assert_eq!(worker.status(), SyncStatus::Unavailable);
    while server.requests.try_recv().is_ok() {}
    worker.handle().wake().unwrap();
    server.request("/inbox/list");
    assert_eq!(worker.status(), SyncStatus::Unavailable);
    server.status.store(200, Ordering::Release);
    worker.handle().wake().unwrap();
    wait_status(&worker, SyncStatus::Healthy);
    assert_eq!(worker.status().message(), None);
    drop(worker);
    assert!(Db::open(&db).unwrap().inbox().unwrap().is_empty());
    let log = std::fs::read_to_string(db.with_extension("sync.log")).unwrap();
    assert!(log.contains("event=wake"));
    assert!(log.contains("event=recovered operation=inbox"));
    assert!(!log.contains("fixture-secret"));
}

#[test]
fn unauthorized_and_rate_limited_pulls_are_classified_without_logging_secrets() {
    for status in [401, 429] {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("local.db");
        let server = Server::new(status);
        let worker = server.worker(&db);
        worker.handle().sync_inbox().unwrap();
        wait_status(&worker, SyncStatus::Retrying);
        drop(worker);
        let log = std::fs::read_to_string(db.with_extension("sync.log")).unwrap();
        assert!(log.contains(&format!("code=http-{status}")));
        assert!(!log.contains("fixture-secret"));
    }
}

#[test]
fn mutation_retries_do_not_treat_idle_queue_ticks_as_recovery() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("local.db");
    let mut local = Db::open(&db).unwrap();
    local
        .apply_task(&json!({"id":1,"title":"initial","projectId":15,"status":"Normal"}))
        .unwrap();
    local
        .enqueue(
            ht_core::Write::TaskTitle {
                task_id: 1,
                title: "queued".into(),
            },
            chrono::Utc::now().timestamp(),
        )
        .unwrap();
    let server = Server::new(503);
    let worker = server.worker(&db);
    worker.handle().wake().unwrap();
    wait_status(&worker, SyncStatus::Unavailable);
    assert!(worker.status().message().is_some());
    // Successful reads cannot hide the still-pending edit.
    worker.handle().sync_catalog(15).unwrap();
    server.request("/projects/15/labels");
    assert_eq!(worker.status(), SyncStatus::Unavailable);
    assert_eq!(local.mutations().unwrap().len(), 1);
    server.status.store(200, Ordering::Release);
    // The queue's own backoff, without another wake, must recover.
    wait_status(&worker, SyncStatus::Healthy);
    assert_eq!(worker.status().message(), None);
    drop(worker);
    assert!(local.mutations().unwrap().is_empty());
    assert_eq!(local.task(1).unwrap().unwrap().title, "server");
    let log = std::fs::read_to_string(db.with_extension("sync.log")).unwrap();
    assert!(log.contains("operation=mutations code=http-503"));
    assert!(log.contains("event=recovered operation=mutations"));
    assert!(!log.contains("fixture-secret"));
}

#[test]
fn healthy_reconnect_refetches_the_tracked_inbox() {
    let dir = tempfile::tempdir().unwrap();
    let server = Server::new(200);
    let worker = server.worker(&dir.path().join("local.db"));
    worker.handle().sync_inbox().unwrap();
    server.request("/inbox/list");
    worker.handle().wake().unwrap();
    server.request("/inbox/list");
    assert_eq!(worker.status().message(), None);
}

#[test]
fn malformed_realtime_frame_is_logged_once_and_reconciled_without_false_outage() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("local.db");
    let server = Server::new(200);
    let worker = server.worker(&db);
    worker.handle().sync_inbox().unwrap();
    server.request("/inbox/list");
    worker
        .handle()
        .event("fixture-secret-malformed-frame".into())
        .unwrap();
    // Recover the missed event via a fresh snapshot, not by replaying bad JSON.
    server.request("/inbox/list");
    assert_eq!(worker.status(), SyncStatus::Healthy);
    assert_eq!(worker.status().message(), None);
    drop(worker);
    let log = std::fs::read_to_string(db.with_extension("sync.log")).unwrap();
    assert_eq!(
        log.matches("event=failed operation=event code=invalid-data attempts=1")
            .count(),
        1
    );
    assert!(!log.contains("fixture-secret"));
}
