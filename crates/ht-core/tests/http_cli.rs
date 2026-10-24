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

#[test]
fn ranked_web_move_sends_desktop_bearer_and_route_denial_does_not_destroy_login() {
    use std::sync::atomic::Ordering;
    for status in [200,401,403] {
        let (url, thread, receive) = server(status, "{\"id\":1}");
        let mut http = Http::with_base(&url, "desktop-test-token".into()).unwrap();
        let body = json!({"taskId":1,"sectionId":11,"section_title":"Doing","ranking":"A0100","projectId":15});
        assert_eq!(http.mutate("PUT", "/api/tasks/moveTask", &body, "drop-key").unwrap().status, status);
        let request = receive.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(request.starts_with("PUT /api/tasks/moveTask HTTP/1.1"));
        assert!(request.to_lowercase().contains("authorization: bearer desktop-test-token\r\n"));
        assert_eq!(serde_json::from_str::<Value>(request.split_once("\r\n\r\n").unwrap().1).unwrap(),body);
        assert!(!http.unauthorized_flag().load(Ordering::SeqCst));
        thread.join().unwrap();
    }
}

#[test]
fn native_task_creation_routes_send_exact_web_json_and_bearer_on_accept_and_refusal() {
    for path in ["/api/tasks/create", "/api/tasks/createGlobally"] { for status in [200,401,403] {
        let (url, thread, receive) = server(status,"{\"newTask\":{\"id\":51}}");
        let mut http=Http::with_base(&url,"desktop-create-token".into()).unwrap();
        let body=json!({"projectId":15,"sectionId":10,"section_title":"Todo","title":"é & task","ranking":"A0100","userId":42,"assignees":[]});
        assert_eq!(http.post(path,&body,"create-key").unwrap().status,status);
        let request=receive.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(request.starts_with(&format!("POST {path} HTTP/1.1")));
        assert!(request.to_lowercase().contains("authorization: bearer desktop-create-token\r\n"));
        assert_eq!(serde_json::from_str::<Value>(request.split_once("\r\n\r\n").unwrap().1).unwrap(),body);
        thread.join().unwrap();
    }}
}

#[test]
fn sidebar_web_requests_use_desktop_bearer_on_accept_and_refusal(){
    for (method,path,body) in [("POST","/api/favorites/addEditFavorites",json!({"userSettingId":99,"index":0,"projectId":15})),("PUT","/api/favorites/addEditFavorites",json!({"userSettingId":99,"index":0,"projectId":-1})),("GET","/api/users/getById",Value::Null),("POST","/api/teams/getAllSidebar",json!({"userId":42})),("GET","/api/favorites/getFavorites",Value::Null),("PUT","/api/favorites/reorderFavorites",json!({"favorites":[{"projectId":15}]}))] {
        for status in [200,401,403]{
            let(url,thread,receive)=server(status,"[]");let mut http=Http::with_base(&url,"desktop-sidebar-token".into()).unwrap();
            let response=if method=="GET"{http.get(path)}else{http.mutate(method,path,&body,"favorites-key")}.unwrap();assert_eq!(response.status,status);
            let request=receive.recv_timeout(Duration::from_secs(5)).unwrap();assert!(request.starts_with(&format!("{method} {path} HTTP/1.1")));assert!(request.to_lowercase().contains("authorization: bearer desktop-sidebar-token\r\n"));
            if method!="GET"{assert_eq!(serde_json::from_str::<Value>(request.split_once("\r\n\r\n").unwrap().1).unwrap(),body);}
            thread.join().unwrap();
        }
    }
}

#[test]
fn board_view_web_routes_send_bearer_and_unmodified_body_on_accept_and_refusal(){
    for path in ["/api/projects/boardTasks","/api/projects/views/unsaved-view","/api/projects/views/switch-view","/api/projects/views/create-view","/api/projects/views/update-view","/api/projects/views/update-order","/api/projects/views/set-default-order","/api/projects/views/reset-order","/api/projects/views/reset-to-default"] {for status in [200,401,403]{
        let(url,thread,receive)=server(status,"{}");let mut http=Http::with_base(&url,"desktop-views-token".into()).unwrap();let body=json!({"projectId":15,"board_sorting_mode":"Priority"});
        assert_eq!(http.post(path,&body,"view-key").unwrap().status,status);
        let request=receive.recv_timeout(Duration::from_secs(5)).unwrap();assert!(request.starts_with(&format!("POST {path} HTTP/1.1")));assert!(request.to_lowercase().contains("authorization: bearer desktop-views-token\r\n"));assert_eq!(serde_json::from_str::<Value>(request.split_once("\r\n\r\n").unwrap().1).unwrap(),body);thread.join().unwrap();
    }}
}

#[test]
fn ai_lifecycle_web_requests_send_bearer_and_exact_body_for_accept_and_refusal(){
    use ht_core::AiSessionWrite;
    for change in [AiSessionWrite::Create{task:Some(7),project:Some(15)},AiSessionWrite::Rename{id:"chat/é &".into(),title:"  New title  ".into()},AiSessionWrite::Delete{id:"chat/é &".into()},AiSessionWrite::DeleteAll] {
        let(method,path,body)=change.request().unwrap();
        for status in [200,401,403]{
            let(url,thread,receive)=server(status,"{}");let mut http=Http::with_base(&url,"desktop-ai-token".into()).unwrap();
            assert_eq!(http.mutate(method,&path,&body,"session-key").unwrap().status,status);
            let wire=receive.recv_timeout(Duration::from_secs(5)).unwrap();assert!(wire.starts_with(&format!("{method} {path} HTTP/1.1")));assert!(wire.to_ascii_lowercase().contains("authorization: bearer desktop-ai-token\r\n"));assert!(!wire.to_ascii_lowercase().contains("cookie:"));if method=="DELETE"{assert_eq!(wire.split_once("\r\n\r\n").unwrap().1,"");}else{assert_eq!(serde_json::from_str::<Value>(wire.split_once("\r\n\r\n").unwrap().1).unwrap(),body);}thread.join().unwrap();
        }
    }
}

#[test]
fn ai_context_web_search_and_flags_send_bearer_and_no_cookie_on_accept_and_refusal(){
    for path in [ht_core::ai_context::mention_path("é & team",Some(15)),ht_core::ai_context::mention_path("",None),"/api/flags".into()] {
        for status in [200,401,403]{let(url,thread,receive)=server(status,"[]");let mut http=Http::with_base(&url,"desktop-context-token".into()).unwrap();assert_eq!(http.get(&path).unwrap().status,status);let wire=receive.recv_timeout(Duration::from_secs(5)).unwrap();assert!(wire.starts_with(&format!("GET {path} HTTP/1.1")));assert!(wire.to_ascii_lowercase().contains("authorization: bearer desktop-context-token\r\n"));assert!(!wire.to_ascii_lowercase().contains("cookie:"));thread.join().unwrap();}
    }
}

#[test]
fn configured_model_reads_and_team_preference_write_use_desktop_bearer_on_accept_and_refusal(){
    for(method,path,body)in [("GET","/api/teams/getTeam?teamId=team",Value::Null),("GET","/api/teams/aiFeatureModels?teamId=team",Value::Null),("GET","/api/teams/aiProviderSettings?teamId=team",Value::Null),("GET","/api/teams/byokKeys?teamId=team",Value::Null),("GET","/api/users/preferences",Value::Null),("POST","/api/teams/getAllSidebar",json!({"userId":42})),("POST","/api/users/preferences",json!({"aiModelPreferences":{"aiChat":"gpt-6-luna-high"},"aiModelPreferencesTeamId":"team"}))]{for status in [200,401,403]{
        let(url,thread,receive)=server(status,"{}");let mut http=Http::with_base(&url,"desktop-model-token".into()).unwrap();let response=if method=="GET"{http.get(path)}else{http.mutate(method,path,&body,"model-key")}.unwrap();assert_eq!(response.status,status);
        let wire=receive.recv_timeout(Duration::from_secs(5)).unwrap();assert!(wire.starts_with(&format!("{method} {path} HTTP/1.1")));assert!(wire.to_ascii_lowercase().contains("authorization: bearer desktop-model-token\r\n"));assert!(!wire.to_ascii_lowercase().contains("cookie:"));if method!="GET"{assert_eq!(serde_json::from_str::<Value>(wire.split_once("\r\n\r\n").unwrap().1).unwrap(),body);}thread.join().unwrap();
    }}
}

#[test]
fn web_view_rename_and_delete_use_exact_routes_bearer_and_empty_delete_body(){
    for route in ["rename-view","delete-view"]{for status in [200,401,403]{let change=ht_core::ViewWrite{project:15,route:route.into(),body:json!({"projectId":15,"viewId":"view /é","title":"Renamed"})};let(method,path,body)=change.request().unwrap();let(url,thread,receive)=server(status,"{}");let mut http=Http::with_base(&url,"desktop-view-token".into()).unwrap();assert_eq!(http.mutate(method,&path,&body,"view-key").unwrap().status,status);let wire=receive.recv_timeout(Duration::from_secs(5)).unwrap();assert!(wire.starts_with(&format!("{method} {path} HTTP/1.1")));assert!(wire.to_ascii_lowercase().contains("authorization: bearer desktop-view-token\r\n"));if method=="DELETE"{assert_eq!(wire.split_once("\r\n\r\n").unwrap().1,"");}else{assert_eq!(serde_json::from_str::<Value>(wire.split_once("\r\n\r\n").unwrap().1).unwrap(),body);}thread.join().unwrap();}}
}

#[test]
fn advanced_view_people_and_timer_reads_use_bearer_on_accept_and_refusals(){
    for path in ["/api/tasks/single?id=1", "/api/tasks/getArchivedTasksByProject?projectId=15", "/api/members/getAllForAssignees?projectId=15","/api/time/board-summary?board=15","/api/agents/all?projectId=15"]{for status in [200,401,403]{
        let(url,thread,receive)=server(status,"{}");let mut http=Http::with_base(&url,"desktop-view-token".into()).unwrap();assert_eq!(http.get(path).unwrap().status,status);
        let wire=receive.recv_timeout(Duration::from_secs(5)).unwrap();assert!(wire.starts_with(&format!("GET {path} HTTP/1.1")));assert!(wire.to_ascii_lowercase().contains("authorization: bearer desktop-view-token\r\n"));assert!(!wire.to_ascii_lowercase().contains("cookie:"));thread.join().unwrap();
    }}
}

#[test]
fn expanded_create_form_sends_desktop_bearer_and_exact_web_payload_for_accept_and_refusals(){
    let body=serde_json::json!({"userId":42,"projectId":15,"sectionId":10,"section_title":"Todo","projectIdentifier":"HT","fullScreenTask":true,"title":"Expanded","description":"<p>Rich <strong>body</strong></p>","assignees":[],"priority":{"priority_index":2},"dueDate":"2026-12-15T09:00:00Z"});
    for status in [200,401,403]{let(url,thread,receive)=server(status,"{}");let mut http=Http::with_base(&url,"desktop-create-token".into()).unwrap();assert_eq!(http.post("/api/tasks/createGlobally",&body,"create-key").unwrap().status,status);let wire=receive.recv_timeout(Duration::from_secs(5)).unwrap();assert!(wire.starts_with("POST /api/tasks/createGlobally HTTP/1.1"));assert!(wire.to_ascii_lowercase().contains("authorization: bearer desktop-create-token\r\n"));assert!(!wire.to_ascii_lowercase().contains("cookie:"));assert_eq!(serde_json::from_str::<Value>(wire.split_once("\r\n\r\n").unwrap().1).unwrap(),body);thread.join().unwrap();}
}

#[test]
fn native_model_settings_handoff_uses_same_web_routes_bearer_and_refusal_statuses() {
    for path in ["/api/members/getAllTeamMembers?teamId=team","/api/teams/getTeam?teamId=team","/api/teams/byokKeys?teamId=team","/api/teams/aiProviderSettings?teamId=team"] {
        for status in [200,401,403] {
            let (url, thread, receive)=server(status,"{}"); let mut http=Http::with_base(&url,"desktop-settings-token".into()).unwrap(); assert_eq!(http.get(path).unwrap().status,status);
            let wire=receive.recv_timeout(Duration::from_secs(5)).unwrap();assert!(wire.starts_with(&format!("GET {path} HTTP/1.1")));assert!(wire.to_ascii_lowercase().contains("authorization: bearer desktop-settings-token\r\n"));assert!(!wire.to_ascii_lowercase().contains("cookie:"));thread.join().unwrap();
        }
    }
    for (path,body) in [
        ("/api/teams/byokKeys",json!({"teamId":"team","provider":"custom","apiKey":"fixture-secret","baseUrl":"https://example.test/v1","modelId":"model","gdprCompliant":true})),
        ("/api/settings/byok-test",json!({"teamId":"team","provider":"claude"})),
        ("/api/stripe/ensure-customer",json!({"teamId":"team"})),
        ("/api/stripe/session/checkout",json!({"teamId":"team","priceId":"price","mode":"Normal"})),
        ("/api/stripe/cancelSubscription",json!({"stripe_customer_id":"cus_1"})),
    ] { for status in [200,401,403] {
        let (url,thread,receive)=server(status,"{}");let mut http=Http::with_base(&url,"desktop-settings-token".into()).unwrap();assert_eq!(http.post(path,&body,"settings-key").unwrap().status,status);
        let wire=receive.recv_timeout(Duration::from_secs(5)).unwrap();assert!(wire.starts_with(&format!("POST {path} HTTP/1.1")));assert!(wire.to_ascii_lowercase().contains("authorization: bearer desktop-settings-token\r\n"));assert!(!wire.to_ascii_lowercase().contains("cookie:"));assert_eq!(serde_json::from_str::<Value>(wire.split_once("\r\n\r\n").unwrap().1).unwrap(),body);thread.join().unwrap();
    }}
}

#[test]
fn smart_split_methods_send_the_desktop_bearer_to_exact_web_route_and_preserve_denial() {
    for (route, method, accepted) in [("smart-split-create", "POST", 201), ("smart-split-edit", "PATCH", 200), ("smart-split-delete", "DELETE", 200)] {
        for status in [accepted, 401, 403] {
            let body = if method == "DELETE" { json!({"projectId":15,"viewId":"smart"}) } else if method == "POST" { json!({"projectId":15,"name":"Daily","prompt":"My daily tasks"}) } else { json!({"projectId":15,"viewId":"smart","name":"Daily","prompt":"My daily tasks"}) };
            let change = ht_core::ViewWrite { project:15, route:route.into(), body:body.clone() };
            let (actual_method, path, request_body) = change.request().unwrap();
            let (url, thread, receive) = server(status, "{\"message\":\"Sign in to manage smart splits\"}");
            let mut http = Http::with_base(&url, "desktop-smart-split-token".into()).unwrap();
            let response = http.mutate(actual_method, &path, &request_body, "smart-split-client").unwrap();
            assert_eq!(response.status, status);
            assert_eq!(response.body["message"], "Sign in to manage smart splits");
            let request = receive.recv_timeout(Duration::from_secs(5)).unwrap();
            assert!(request.starts_with(&format!("{method} /api/projects/views/smart-split HTTP/1.1\r\n")));
            assert!(request.to_ascii_lowercase().contains("authorization: bearer desktop-smart-split-token\r\n"));
            assert_eq!(serde_json::from_str::<Value>(request.split_once("\r\n\r\n").unwrap().1).unwrap(), body);
            thread.join().unwrap();
        }
    }
}

#[test]
fn column_controls_use_web_paths_with_bearer_and_only_column_delete_accepts_empty_204() {
    for (route,path,body,accepted) in [
        ("column-create","/api/section/create",json!({"projectId":15,"title":"Review"}),200),
        ("column-update","/api/section/update",json!({"sectionId":10,"userId":42,"newSection":{"deleted":true}}),204),
        ("column-visibility","/api/sections/view-visibility",json!({"sectionId":10,"visible":true}),200),
        ("column-autoassign","/api/sections/auto-assign",json!({"sectionId":10,"autoAssignUserId":7,"autoAssignAgentId":null}),200),
    ] {
        for status in [accepted,401,403] {
            let mut queued=body.clone();queued["projectId"]=json!(15);
            let (method,actual_path,actual_body)=ht_core::ViewWrite{project:15,route:route.into(),body:queued}.request().unwrap();
            let (url,thread,receive)=server(status,if status==204{""}else{"{\"message\":\"Forbidden\"}"});
            let mut http=Http::with_base(&url,"desktop-column-token".into()).unwrap();
            let response=http.mutate(method,&actual_path,&actual_body,"column-client").unwrap();assert_eq!(response.status,status);
            let request=receive.recv_timeout(Duration::from_secs(5)).unwrap();assert!(request.starts_with(&format!("POST {path} HTTP/1.1\r\n")));
            assert!(request.to_ascii_lowercase().contains("authorization: bearer desktop-column-token\r\n"));
            assert_eq!(serde_json::from_str::<Value>(request.split_once("\r\n\r\n").unwrap().1).unwrap(),body);thread.join().unwrap();
        }
    }
    let (url,thread,receive)=server(204,"");
    let mut http=Http::with_base(&url,"desktop-column-token".into()).unwrap();
    assert_eq!(http.post("/api/sections/auto-assign",&json!({}),"negative-control").unwrap_err(),TransportError::InvalidResponse);
    receive.recv_timeout(Duration::from_secs(5)).unwrap();thread.join().unwrap();
}

#[test]
fn task_timer_routes_send_the_desktop_bearer_and_preserve_start_stop_setting_refusals() {
    for (change,path,body) in [(ht_core::TimeWrite::Start{task:1},"/api/time/start",json!({"taskId":1})),(ht_core::TimeWrite::Stop{task:1},"/api/time/stop",json!({"taskId":1})),(ht_core::TimeWrite::Board{project:15,enabled:false},"/api/projects/time-tracking",json!({"projectId":15,"enabled":false}))] {
        for status in [200,401,403] {
            let (actual_path,actual_body)=change.request().unwrap();let (url,thread,receive)=server(status,"{\"error\":\"Unauthorized\"}");let mut http=Http::with_base(&url,"desktop-time-token".into()).unwrap();
            let response=http.post(&actual_path,&actual_body,"time-client").unwrap();assert_eq!(response.status,status);assert_eq!(response.body["error"],"Unauthorized");
            let request=receive.recv_timeout(Duration::from_secs(5)).unwrap();assert!(request.starts_with(&format!("POST {path} HTTP/1.1\r\n")));assert!(request.to_ascii_lowercase().contains("authorization: bearer desktop-time-token\r\n"));assert_eq!(serde_json::from_str::<Value>(request.split_once("\r\n\r\n").unwrap().1).unwrap(),body);thread.join().unwrap();
        }
    }
}

#[test]
fn daily_task_action_routes_send_desktop_bearer_without_cookies_and_preserve_refusals() {
    for (change,path,body) in [
        (ht_core::TaskAction::StartDate(Some("2026-10-01T09:00:00+02:00".into())),"/api/tasks/setStartDate",json!({"taskId":1,"startDate":"2026-10-01T09:00:00+02:00"})),
        (ht_core::TaskAction::StartDate(None),"/api/tasks/setStartDate",json!({"taskId":1,"startDate":null})),
        (ht_core::TaskAction::WaitingOn(Some(7)),"/api/tasks/waiting-on",json!({"taskId":1,"userId":7})),
        (ht_core::TaskAction::WaitingOn(None),"/api/tasks/waiting-on",json!({"taskId":1,"userId":null})),
        (ht_core::TaskAction::Recurrence(Some("Biweekly".into())),"/api/tasks/setRecurrence",json!({"taskId":1,"recurrence":"Biweekly"})),
        (ht_core::TaskAction::Recurrence(None),"/api/tasks/setRecurrence",json!({"taskId":1,"recurrence":null})),
        (ht_core::TaskAction::AssignMe,"/api/assignees/assign",json!({"taskId":1,"userId":42,"intent":"assign"})),
    ] { for status in [200,401,403] {
        let (actual_path,actual_body)=change.request(1,42).unwrap();assert_eq!(actual_path,path);assert_eq!(actual_body,body);
        let (url,thread,receive)=server(status,"{\"message\":\"Unauthorized\"}");let mut http=Http::with_base(&url,"desktop-task-token".into()).unwrap();
        let response=http.post(actual_path,&actual_body,"task-action-client").unwrap();assert_eq!(response.status,status);assert_eq!(response.body["message"],"Unauthorized");
        let wire=receive.recv_timeout(Duration::from_secs(5)).unwrap();assert!(wire.starts_with(&format!("POST {path} HTTP/1.1\r\n")));assert!(wire.to_ascii_lowercase().contains("authorization: bearer desktop-task-token\r\n"));assert!(!wire.to_ascii_lowercase().contains("cookie:"));
        assert_eq!(serde_json::from_str::<Value>(wire.split_once("\r\n\r\n").unwrap().1).unwrap(),body);thread.join().unwrap();
    }}
}
