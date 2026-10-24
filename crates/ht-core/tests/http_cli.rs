use ht_core::{Db, Http, Transport, http::TransportError};
use serde_json::{Value, json};
use std::{
    io::{Read, Write},
    net::TcpListener,
    process::Command,
    sync::mpsc,
    time::Duration,
};

fn server(
    status: u16,
    body: &str,
) -> (String, std::thread::JoinHandle<()>, mpsc::Receiver<String>) {
    server_type(status, body, "application/json")
}

fn server_type(status: u16, body: &str, content_type: &str) -> (String, std::thread::JoinHandle<()>, mpsc::Receiver<String>) {
    let content_type=content_type.to_owned();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/api/mcp", listener.local_addr().unwrap());
    let body = body.to_owned();
    let (send, receive) = mpsc::channel();
    let thread = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut bytes = Vec::new();
        let mut buf = [0; 1024];
        loop {
            let size = socket.read(&mut buf).unwrap();
            assert!(size > 0);
            bytes.extend_from_slice(&buf[..size]);
            if let Some(end) = bytes.windows(4).position(|s| s == b"\r\n\r\n") {
                let header = String::from_utf8_lossy(&bytes[..end]);
                let len = header
                    .lines()
                    .find_map(|line| {
                        line.to_lowercase()
                            .strip_prefix("content-length:")
                            .map(|n| n.trim().parse::<usize>().unwrap())
                    })
                    .unwrap_or(0);
                if bytes.len() >= end + 4 + len {
                    break;
                }
            }
        }
        send.send(String::from_utf8(bytes).unwrap()).unwrap();
        write!(socket,"HTTP/1.1 {status} Test\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
    });
    (url, thread, receive)
}

#[test]
fn http_sends_bearer_correlation_idempotency_and_exact_json() {
    let (url, thread, receive) = server(201, "{\"success\":true,\"comment\":{\"id\":1}}");
    let mut http = Http::with_base(&url, "test-not-a-real-token".into()).unwrap();
    let response = http
        .post(
            "/comments",
            &json!({"task_id":1,"text":"quotes \" & Unicode ü"}),
            "client-uuid",
        )
        .unwrap();
    assert_eq!(response.status, 201);
    let request = receive.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(request.starts_with("POST /api/mcp/comments HTTP/1.1"));
    let headers = request.to_ascii_lowercase();
    assert!(headers.contains("authorization: bearer test-not-a-real-token\r\n"));
    assert!(headers.contains("x-correlation-id: client-uuid\r\n"));
    assert!(headers.contains("idempotency-key: client-uuid\r\n"));
    assert_eq!(
        serde_json::from_str::<Value>(request.split_once("\r\n\r\n").unwrap().1).unwrap()["text"],
        "quotes \" & Unicode ü"
    );
    thread.join().unwrap();
}

#[test]
fn http_preserves_error_status_and_rejects_invalid_success_and_insecure_hosts() {
    for (status, body) in [(429, "busy"), (500, "error"), (200, "invalid JSON")] {
        let (url, thread, receive) = server(status, body);
        let mut http = Http::with_base(&url, "test-token".into()).unwrap();
        let response = http.get("/tasks?project_id=15");
        if status == 200 {
            assert_eq!(response.unwrap_err(), TransportError::InvalidResponse);
        } else {
            assert_eq!(response.unwrap().status, status);
        }
        receive.recv_timeout(Duration::from_secs(5)).unwrap();
        thread.join().unwrap();
    }
    assert!(Http::with_base("http://example.com", "test".into()).is_err());
    assert!(Http::with_base("https://user@example.com", "test".into()).is_err());
    assert!(Http::new("token\nheader".into()).is_err());
}

#[test]
fn cli_offline_write_and_dump_use_only_sqlite_and_do_not_print_token() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let mut db = Db::open(file.path()).unwrap();
    db.apply_task(&json!({"id":1,"title":"initial","projectId":15,"status":"Normal"}))
        .unwrap();
    drop(db);
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_ht-core-cli"))
            .arg("--db")
            .arg(file.path())
            .args(args)
            .env_remove("HT_TOKEN")
            .output()
            .unwrap()
    };
    let write = run(&["write", "task_title", "1", "offline title"]);
    assert!(
        write.status.success(),
        "{}",
        String::from_utf8_lossy(&write.stderr)
    );
    assert_eq!(String::from_utf8(write.stdout).unwrap(), "QUEUED 1\n");
    let dump = Command::new(env!("CARGO_BIN_EXE_ht-core-cli"))
        .arg("--db")
        .arg(file.path())
        .arg("dump")
        .env("HT_TOKEN", "must-not-be-printed")
        .output()
        .unwrap();
    assert!(dump.status.success());
    assert!(!String::from_utf8_lossy(&dump.stdout).contains("must-not-be-printed"));
    assert!(!String::from_utf8_lossy(&dump.stderr).contains("must-not-be-printed"));
    let value: Value = serde_json::from_slice(&dump.stdout).unwrap();
    assert_eq!(value["Task"][0]["title"], "offline title");
    assert_eq!(value["mutation_queue"].as_array().unwrap().len(), 1);
    let sync = run(&["sync", "--board", "15"]);
    assert!(!sync.status.success());
    assert!(String::from_utf8_lossy(&sync.stderr).contains("HT_TOKEN is required"));
}

#[test]
fn worker_drains_durable_mutations_on_a_background_thread() {
    struct Server {
        send: mpsc::Sender<String>,
    }
    impl Transport for Server {
        fn get(&mut self, _: &str) -> std::result::Result<ht_core::http::Response, TransportError> {
            Err(TransportError::Offline)
        }
        fn post(
            &mut self,
            _: &str,
            _: &Value,
            _: &str,
        ) -> std::result::Result<ht_core::http::Response, TransportError> {
            self.send
                .send(std::thread::current().name().unwrap().to_owned())
                .unwrap();
            Ok(ht_core::http::Response {
                status: 200,
                body: json!({"success":true,"task":{"id":1,"title":"server","projectId":15,"status":"Normal"}}),
            })
        }
    }
    let file = tempfile::NamedTempFile::new().unwrap();
    let mut db = Db::open(file.path()).unwrap();
    db.apply_task(&json!({"id":1,"title":"initial","projectId":15,"status":"Normal"}))
        .unwrap();
    db.enqueue(
        ht_core::Write::TaskTitle {
            task_id: 1,
            title: "queued".into(),
        },
        1,
    )
    .unwrap();
    let (send, receive) = mpsc::channel();
    let worker = ht_core::Worker::start(file.path(), Server { send }).unwrap();
    worker.handle().wake().unwrap();
    assert_eq!(
        receive.recv_timeout(Duration::from_secs(5)).unwrap(),
        "ht-sync"
    );
    drop(worker);
    assert!(db.mutations().unwrap().is_empty());
    assert_eq!(db.task(1).unwrap().unwrap().title, "server");
}

#[test]
fn http_comment_patch_delete_keep_bearer_and_idempotency_on_the_wire() {
    for method in ["PATCH", "DELETE"] {
        let (url, thread, receive) = server(200, "{\"success\":true}");
        let mut http = Http::with_base(&url, "test-not-a-real-token".into()).unwrap();
        let body = if method == "PATCH" {
            json!({"text":"<p>Edit<br>é🙂</p>","content_type":"html"})
        } else {
            json!({})
        };
        http.mutate(method, "/comments/7", &body, "stable-comment-id")
            .unwrap();
        let request = receive.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(request.starts_with(&format!("{method} /api/mcp/comments/7 HTTP/1.1")));
        let headers = request.to_ascii_lowercase();
        assert!(headers.contains("authorization: bearer test-not-a-real-token\r\n"));
        assert!(headers.contains("idempotency-key: stable-comment-id\r\n"));
        assert!(headers.contains("x-correlation-id: stable-comment-id\r\n"));
        assert_eq!(
            serde_json::from_str::<Value>(request.split_once("\r\n\r\n").unwrap().1).unwrap(),
            body
        );
        thread.join().unwrap();
    }
}

#[test]
fn desktop_cli_web_routes_use_same_origin_bearer_without_mcp_prefix_or_cookies() {
    use ht_core::desktop_cli::*;
    for request in [defaults_request(7283),mentions_request(7283,"é & QA"),saved_request(60253,7283,7,false,false),writer_request(&json!({"id":7283}),"Note","UTC")] {
        let writer=request.path=="/api/ai/task-writer";
        let (url,thread,receive)=server_type(200,if writer {"<h1 id=\"ai-generated-task-title\">Title</h1><p>Body</p>"}else{"{}"},if writer {"text/event-stream"}else{"application/json"});
        let mut http=Http::with_base(&url,"test-human-token".into()).unwrap();
        let response=if request.method=="GET" {http.get(&request.path)}else{http.post(&request.path,&request.body,"client-id")}.unwrap();
        if writer {assert!(response.body.as_str().unwrap().contains("ai-generated-task-title"));}
        let wire=receive.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(wire.starts_with(&format!("{} {} HTTP/1.1",request.method,request.path)));
        assert!(wire.to_ascii_lowercase().contains("authorization: bearer test-human-token\r\n"));
        assert!(!wire.to_ascii_lowercase().contains("cookie:"));
        if request.method=="POST" {assert_eq!(serde_json::from_str::<Value>(wire.split_once("\r\n\r\n").unwrap().1).unwrap(),request.body);}
        thread.join().unwrap();
    }
}
