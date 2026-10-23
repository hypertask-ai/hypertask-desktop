use ht_core::{
    Db, Http, Transport,
    auth::{
        CallbackError, ExchangeError, MemoryStore, TokenStore, clear_if_unauthorized,
        exchange_code, parse_callback, serve_callback, sign_in_url,
    },
};
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::atomic::Ordering,
    time::Duration,
};

fn mock(status: u16, body: &'static str) -> (String, std::thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let thread = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        let mut bytes = Vec::new();
        let mut buf = [0; 1024];
        loop {
            let size = socket.read(&mut buf).unwrap();
            bytes.extend_from_slice(&buf[..size]);
            if let Some(end) = bytes.windows(4).position(|s| s == b"\r\n\r\n") {
                let head = String::from_utf8_lossy(&bytes[..end]).to_lowercase();
                let len = head
                    .lines()
                    .find_map(|l| {
                        l.strip_prefix("content-length:")
                            .map(|n| n.trim().parse::<usize>().unwrap())
                    })
                    .unwrap_or(0);
                if bytes.len() >= end + 4 + len {
                    break;
                }
            }
        }
        write!(
            socket,
            "HTTP/1.1 {status} T\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .unwrap();
        String::from_utf8(bytes).unwrap()
    });
    (base, thread)
}

#[test]
fn callback_accepts_a_good_code() {
    assert_eq!(
        parse_callback("GET /callback?code=abc123 HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap(),
        "abc123"
    );
}

#[test]
fn callback_rejects_a_missing_or_empty_code() {
    for head in [
        "GET /callback HTTP/1.1\r\n\r\n",
        "GET /callback?code= HTTP/1.1\r\n\r\n",
        "GET /callback?other=1 HTTP/1.1\r\n\r\n",
    ] {
        assert_eq!(parse_callback(head), Err(CallbackError::MissingCode));
    }
}

#[test]
fn callback_ignores_other_paths() {
    for head in [
        "GET /favicon.ico HTTP/1.1\r\n\r\n",
        "GET /callbackx?code=a HTTP/1.1\r\n\r\n",
        "POST /callback?code=a HTTP/1.1\r\n\r\n",
    ] {
        assert_eq!(parse_callback(head), Err(CallbackError::WrongPath));
    }
}

#[test]
fn callback_rejects_junk() {
    for head in ["", "\u{0}\u{1}\u{2}", "GET", "hello world"] {
        assert!(parse_callback(head).is_err(), "{head:?}");
    }
}

#[test]
fn exchange_returns_the_token_and_posts_the_code() {
    let (base, thread) = mock(200, "{\"token\":\"tok-1\"}");
    assert_eq!(exchange_code(&base, "the-code").unwrap(), "tok-1");
    let request = thread.join().unwrap();
    assert!(request.starts_with("POST /api/cli/token-exchange "));
    assert!(request.ends_with("{\"code\":\"the-code\"}"));
}

#[test]
fn exchange_reports_a_rejected_code() {
    let (base, thread) = mock(401, "{\"error\":\"Invalid or expired code\"}");
    assert_eq!(exchange_code(&base, "x"), Err(ExchangeError::Rejected(401)));
    thread.join().unwrap();
}

#[test]
fn exchange_reports_malformed_json() {
    for body in ["not json", "{\"nope\":1}", "{\"token\":\"\"}"] {
        let (base, thread) = mock(200, body);
        assert_eq!(
            exchange_code(&base, "x"),
            Err(ExchangeError::Malformed),
            "{body}"
        );
        thread.join().unwrap();
    }
}

#[test]
fn store_memory_round_trips_and_clears() {
    let store = MemoryStore::default();
    assert_eq!(store.get(), None);
    store.set("t").unwrap();
    assert_eq!(store.get().as_deref(), Some("t"));
    store.clear();
    assert_eq!(store.get(), None);
}

#[test]
fn store_is_cleared_when_an_api_call_returns_401() {
    let (base, thread) = mock(401, "{\"error\":\"nope\"}");
    let store = MemoryStore::default();
    store.set("old").unwrap();
    let mut http = Http::with_base(&format!("{base}/api/mcp"), "old".into()).unwrap();
    let flag = http.unauthorized_flag();
    assert!(!clear_if_unauthorized(&flag, &store));
    assert_eq!(store.get().as_deref(), Some("old"));
    assert_eq!(http.get("/projects").unwrap().status, 401);
    thread.join().unwrap();
    assert!(flag.load(Ordering::SeqCst));
    assert!(clear_if_unauthorized(&flag, &store));
    assert_eq!(store.get(), None);
    assert!(!clear_if_unauthorized(&flag, &store));
}

#[test]
fn store_loopback_server_returns_the_code_and_ignores_other_paths() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let server = std::thread::spawn(move || serve_callback(&listener, Duration::from_secs(10)));
    let mut stray = TcpStream::connect(("127.0.0.1", port)).unwrap();
    stray.write_all(b"GET /favicon.ico HTTP/1.1\r\n\r\n").unwrap();
    let mut reply = String::new();
    stray.read_to_string(&mut reply).unwrap();
    assert!(reply.starts_with("HTTP/1.1 404"));
    let mut ok = TcpStream::connect(("127.0.0.1", port)).unwrap();
    ok.write_all(b"GET /callback?code=zz9 HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .unwrap();
    let mut reply = String::new();
    ok.read_to_string(&mut reply).unwrap();
    assert!(reply.contains("You can close this tab and return to Hypertask."));
    assert_eq!(server.join().unwrap().unwrap(), "zz9");
}

#[test]
fn store_loopback_server_times_out() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    assert!(serve_callback(&listener, Duration::from_millis(200)).is_err());
}

#[test]
fn store_remembers_the_last_board_and_the_first_project() {
    let db = Db::memory().unwrap();
    assert_eq!(db.kv_get("last_board").unwrap(), None);
    db.kv_set("last_board", "42").unwrap();
    db.kv_set("last_board", "43").unwrap();
    assert_eq!(db.kv_get("last_board").unwrap().as_deref(), Some("43"));
    assert_eq!(db.first_project_id().unwrap(), None);
}

#[test]
fn sign_in_url_carries_the_port() {
    assert_eq!(
        sign_in_url(4567),
        "https://app.hypertask.ai/cli-auth?port=4567"
    );
}
