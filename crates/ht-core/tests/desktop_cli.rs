use ht_core::{
    CommentWrite, Db, Drain, Transport, Write,
    desktop_cli::*,
    http::{Response, TransportError},
};
use serde_json::{Value, json};

fn db() -> Db {
    let mut db = Db::memory().unwrap();
    seed(&mut db);
    db
}
fn seed(db: &mut Db) {
    db.kv_set("current-user-id", "985").unwrap();
    db.apply_response(
        &json!({"success":true,"projects":[{"id":7283,"title":"QA","status":"Normal"}]}),
    )
    .unwrap();
    db.apply_task(&json!({"id":60253,"projectId":7283,"title":"QA","status":"Normal"}))
        .unwrap();
    db.apply_comments(60253,&json!({"success":true,"comments":[{"id":7,"creatorId":985,"text":"<p>QA</p>","createdAt":"2026-01-01T00:00:00Z"}]})).unwrap();
}
struct Server {
    calls: Vec<(String, Value, String)>,
    create_failure: bool,
    writer_failure: bool,
    saved_status: u16,
    split: bool,
    refuse_second: bool,
    other_user: bool,
    refusal: Option<&'static str>,
}
impl Default for Server {
    fn default() -> Self {
        Self {
            calls: vec![],
            create_failure: false,
            writer_failure: false,
            saved_status: 200,
            split: false,
            refuse_second: false,
            other_user: false,
            refusal: None,
        }
    }
}
impl Transport for Server {
    fn get(&mut self, path: &str) -> Result<Response, TransportError> {
        if path == "/user/context" {
            return Ok(Response {
                status: 200,
                body: json!({"success":true,"user":{"id":if self.other_user {4036}else{985}}}),
            });
        }
        assert_eq!(path, defaults_request(7283).path);
        if self.refusal == Some("defaults") {
            return Ok(Response { status: 401, body: json!({"message":"Unauthorized"}) });
        }
        Ok(Response {
            status: 200,
            body: json!({"sectionId":11,"section":"Todo","ranking":"a"}),
        })
    }
    fn post(&mut self, path: &str, body: &Value, key: &str) -> Result<Response, TransportError> {
        self.mutate("POST", path, body, key)
    }
    fn mutate(
        &mut self,
        method: &str,
        path: &str,
        body: &Value,
        key: &str,
    ) -> Result<Response, TransportError> {
        assert_eq!(method, "POST");
        self.calls.push((path.into(), body.clone(), key.into()));
        if path == "/api/ai/task-writer" && self.refusal.is_some() {
            return Ok(if self.refusal == Some("plan") {
                Response { status: 402, body: json!({"code":"PLAN_LIMIT_REACHED","error":"Plan limit reached"}) }
            } else {
                Response { status: 401, body: json!("event: error\ndata: {\"content\":\"Unauthorized\"}\n\nevent: done\ndata: {\"status\":\"error\"}\n\n") }
            });
        }
        match path {
            "/api/ai/task-writer" => Ok(Response {
                status: if self.writer_failure { 500 } else { 200 },
                body: if self.split {
                    json!({"tasks":[{"title":"First","description":"<p>One</p>"},{"title":"Second","description":"<p>Two</p>"}]})
                } else {
                    json!({"tasks":[{"title":"Written","description":"<p>Body</p>"}]})
                },
            }),
            "/tasks/create" => {
                assert_eq!(body["project_id"], 7283);
                assert_eq!(body["section_id"], 11);
                if self.create_failure {
                    self.create_failure = false;
                    return Err(TransportError::Timeout);
                }
                let index = key.rsplit('-').next().unwrap().parse::<i64>().unwrap();
                if self.refuse_second && index == 1 {
                    return Ok(Response {
                        status: 403,
                        body: json!({"success":false}),
                    });
                }
                Ok(Response {
                    status: 200,
                    body: json!({"success":true,"task":{"id":9+index,"title":body["title"],"description":body["description"],"projectId":7283,"status":"Normal"}}),
                })
            }
            "/api/savedContent/toggleSaved" => {
                if self.saved_status == 0 {
                    return Err(TransportError::Timeout);
                }
                Ok(Response {
                    status: self.saved_status,
                    body: if self.saved_status == 200 {
                        json!({"saved":{"id":"s","commentId":7,"type":body["type"],"userId":985}})
                    } else if self.saved_status == 401 {
                        json!({"message":"Unauthorized"})
                    } else {
                        json!({})
                    },
                })
            }
            _ => panic!("unexpected path {path}"),
        }
    }
}
#[test]
fn compose_writer_defaults_and_create_are_real_durable_queue_operations() {
    let mut db = db();
    let id = db
        .enqueue(
            Write::ComposeTask {
                project_id: 7283,
                text: "My note".into(),
                time_zone: "UTC".into(),
            },
            100,
        )
        .unwrap();
    let key = db.mutations().unwrap()[0].client_id.clone();
    let mut server = Server {
        create_failure: true,
        ..Default::default()
    };
    assert_eq!(
        db.drain_once(&mut server, 100).unwrap(),
        Drain::Retrying(id)
    );
    assert_eq!(db.drain_once(&mut server, 101).unwrap(), Drain::Applied(id));
    assert_eq!(
        server
            .calls
            .iter()
            .filter(|c| c.0 == "/api/ai/task-writer")
            .count(),
        1
    );
    let creates: Vec<_> = server
        .calls
        .iter()
        .filter(|c| c.0 == "/tasks/create")
        .collect();
    assert_eq!(creates.len(), 2);
    assert_eq!(creates[0], creates[1]);
    assert_eq!(db.task(9).unwrap().unwrap().title, "Written");
    let result: Value = serde_json::from_str(
        &db.kv_get(&format!("compose-result:{key}"))
            .unwrap()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(result["writerFailed"], false);
}
#[test]
fn unavailable_writer_falls_back_to_original_note_like_web_not_an_empty_task() {
    let mut db = db();
    db.enqueue(
        Write::ComposeTask {
            project_id: 7283,
            text: "é & note\nnext".into(),
            time_zone: "UTC".into(),
        },
        100,
    )
    .unwrap();
    let mut server = Server {
        writer_failure: true,
        ..Default::default()
    };
    assert!(matches!(
        db.drain_once(&mut server, 100).unwrap(),
        Drain::Applied(_)
    ));
    let create = &server
        .calls
        .iter()
        .find(|c| c.0 == "/tasks/create")
        .unwrap()
        .1;
    assert_eq!(create["title"], "é & note\nnext");
    assert_eq!(create["description"], "<p>é &amp; note<br>next</p>");
}
#[test]
fn writer_parses_streamed_html_split_json_unicode_entities_and_rejects_sse_errors() {
    let drafts=writer_drafts(&json!("<h1 id='ai-generated-task-title'>Fix &amp; é🙂</h1><span id='ai-generated-task-priority'>2</span><p>Body</p>")).unwrap();
    assert_eq!(drafts[0].title, "Fix & é🙂");
    assert_eq!(drafts[0].description, "<p>Body</p>");
    assert_eq!(writer_drafts(&json!({"tasks":[{"title":"one","description":"<p>a</p>"},{"title":"two","description":"<p>b</p>"}]})).unwrap().len(),2);
    for value in [
        json!("event: error\ndata: {\"content\":\"refused\"}\n\nevent: done\n"),
        json!({"tasks":[]}),
        json!({"tasks":[{"title":"","description":""}]}),
        json!("<p>no title</p>"),
    ] {
        assert!(writer_drafts(&value).is_err());
    }
}
#[test]
fn saved_comment_toggle_handles_200_and_201_and_never_retries_uncertain_writes() {
    for public in [false, true] {
        for status in [200, 201, 403, 0] {
            let mut db = db();
            let id = db
                .enqueue(
                    Write::CommentAction {
                        task_id: 60253,
                        comment_id: 7,
                        change: CommentWrite::Saved { public },
                    },
                    100,
                )
                .unwrap();
            let mut server = Server {
                saved_status: status,
                ..Default::default()
            };
            let result = db.drain_once(&mut server, 100).unwrap();
            assert_eq!(
                result,
                if status == 200 || status == 201 {
                    Drain::Applied(id)
                } else {
                    Drain::Failed(id)
                }
            );
            assert_eq!(
                server.calls[0].1,
                saved_request(60253, 7283, 7, public, false).body
            );
            assert_eq!(db.drain_once(&mut server, 1000).unwrap(), Drain::Empty);
            assert_eq!(server.calls.len(), 1);
        }
    }
}
#[test]
fn mention_contract_encodes_query_and_preserves_notification_ids_and_task_links() {
    assert!(mention_html(&json!({"type":"name","name":"Missing ID"})).is_none());
    assert_eq!(
        mentions_request(7283, "").path,
        "/api/tasks/searchByParam?param=all&projectId=7283"
    );
    assert!(
        mentions_request(7283, "A & é")
            .path
            .contains("A+%26+%C3%A9")
    );
    let person = mention_html(&json!({"type":"name","id":985,"name":"QA & é"})).unwrap();
    assert!(person.contains("data-label=\"name-985\""));
    assert!(person.contains("data-id=\"QA &amp; é\""));
    let task=mention_html(&json!({"type":"task","id":60253,"name":"QA","project_id":7283,"index":5,"ticketNumber":"QA-5"})).unwrap();
    assert!(task.contains("href=\"https://app.hypertask.ai/detail/project-7283/5\""));
    assert!(task.contains("projectid=\"7283\" uniqueindex=\"5\""));
    assert!(
        mention_html(&json!({"type":"agent","id":"abc","name":"Agent"}))
            .unwrap()
            .contains("agent-abc")
    );
    assert!(mention_html(&json!({"type":"peopleHeading","name":"People"})).is_none());
}
#[test]
fn rust_writes_production_contract_requests_without_credentials() {
    let project = json!({"id":7283});
    let comment = std::env::var("HT_QA_COMMENT_ID")
        .ok()
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(7);
    let requests = json!({
        "defaults":defaults_request(7283),
        "writer":writer_request(&project,"HTPR-7381 desktop contract probe: describe checking a status indicator. One task only; do not create any task.","UTC"),
        "mentions":mentions_request(7283,""),
        "star":saved_request(60253,7283,comment,false,false),"pin":saved_request(60253,7283,comment,true,false),
        "unstar":saved_request(60253,7283,comment,false,true),"unpin":saved_request(60253,7283,comment,true,true)
    });
    assert_eq!(requests["writer"]["body"]["requestKind"], "compose-task");
    let target = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target");
    std::fs::create_dir_all(&target).unwrap();
    std::fs::write(
        target.join("compose-live-requests.json"),
        serde_json::to_vec_pretty(&requests).unwrap(),
    )
    .unwrap();
}

#[test]
fn compose_frozen_payload_and_comment_toggle_safety_survive_restart() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let mut db = Db::open(file.path()).unwrap();
    seed(&mut db);
    let id = db
        .enqueue(
            Write::ComposeTask {
                project_id: 7283,
                text: "Note".into(),
                time_zone: "UTC".into(),
            },
            100,
        )
        .unwrap();
    let mut server = Server {
        create_failure: true,
        ..Default::default()
    };
    assert_eq!(
        db.drain_once(&mut server, 100).unwrap(),
        Drain::Retrying(id)
    );
    drop(db);
    let mut db = Db::open(file.path()).unwrap();
    assert_eq!(db.drain_once(&mut server, 101).unwrap(), Drain::Applied(id));
    assert_eq!(
        server
            .calls
            .iter()
            .filter(|c| c.0 == "/api/ai/task-writer")
            .count(),
        1
    );
    let id = db
        .enqueue(
            Write::CommentAction {
                task_id: 60253,
                comment_id: 7,
                change: CommentWrite::Saved { public: false },
            },
            102,
        )
        .unwrap();
    // Simulate process exit after claiming a toggle, before its acknowledgement was persisted.
    let conn = rusqlite::Connection::open(file.path()).unwrap();
    conn.execute(
        "UPDATE mutation_queue SET attempts=1,status='inflight',lease_until=103 WHERE id=?",
        [id],
    )
    .unwrap();
    drop(conn);
    drop(db);
    let mut db = Db::open(file.path()).unwrap();
    let before = server.calls.len();
    assert_eq!(db.drain_once(&mut server, 104).unwrap(), Drain::Failed(id));
    assert_eq!(server.calls.len(), before);
}

#[test]
fn split_writer_saves_each_ticket_and_reports_partial_failure_without_duplicate_creation() {
    for refuse_second in [false, true] {
        let mut db = db();
        let id = db
            .enqueue(
                Write::ComposeTask {
                    project_id: 7283,
                    text: "two tasks".into(),
                    time_zone: "UTC".into(),
                },
                100,
            )
            .unwrap();
        let key = db.mutations().unwrap()[0].client_id.clone();
        let mut server = Server {
            split: true,
            refuse_second,
            ..Default::default()
        };
        assert_eq!(db.drain_once(&mut server, 100).unwrap(), Drain::Applied(id));
        assert_eq!(
            server
                .calls
                .iter()
                .filter(|c| c.0 == "/tasks/create")
                .count(),
            2
        );
        let result: Value = serde_json::from_str(
            &db.kv_get(&format!("compose-result:{key}"))
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        assert_eq!(
            result["tasks"].as_array().unwrap().len(),
            if refuse_second { 1 } else { 2 }
        );
        assert_eq!(
            result["failedTitles"],
            if refuse_second {
                json!(["Second"])
            } else {
                json!([])
            }
        );
    }
}

#[test]
fn compose_and_saved_comment_queue_refuse_a_different_bearer_account() {
    for write in [
        Write::ComposeTask {
            project_id: 7283,
            text: "Do not retarget".into(),
            time_zone: "UTC".into(),
        },
        Write::CommentAction {
            task_id: 60253,
            comment_id: 7,
            change: CommentWrite::Saved { public: false },
        },
    ] {
        let mut db = db();
        let id = db.enqueue(write, 100).unwrap();
        let mut server = Server {
            other_user: true,
            ..Default::default()
        };
        assert_eq!(db.drain_once(&mut server, 100).unwrap(), Drain::Failed(id));
        assert!(server.calls.is_empty());
    }
}

#[test]
fn cli_flag_off_and_plan_refusals_keep_server_errors_without_creating_tasks() {
    for (refusal, expected) in [("defaults", "Unauthorized"), ("writer", "Unauthorized"), ("plan", "Plan limit reached")] {
        let mut db = db();
        let id = db.enqueue(Write::ComposeTask { project_id: 7283, text: "Keep this note".into(), time_zone: "UTC".into() }, 100).unwrap();
        let key = db.mutations().unwrap()[0].client_id.clone();
        let mut server = Server { refusal: Some(refusal), ..Default::default() };
        assert_eq!(db.drain_once(&mut server, 100).unwrap(), Drain::Failed(id));
        let result: Value = serde_json::from_str(&db.kv_get(&format!("compose-result:{key}")).unwrap().unwrap()).unwrap();
        assert_eq!(result["error"], expected);
        assert!(!server.calls.iter().any(|call| call.0 == "/tasks/create"));
        assert!(db.task(9).unwrap().is_none());
        assert_eq!(db.drain_once(&mut server, 1000).unwrap(), Drain::Empty);
    }
    for public in [false, true] {
        let mut db = db();
        let id = db.enqueue(Write::CommentAction { task_id: 60253, comment_id: 7, change: CommentWrite::Saved { public } }, 100).unwrap();
        let key = db.mutations().unwrap()[0].client_id.clone();
        let mut server = Server { saved_status: 401, ..Default::default() };
        assert_eq!(db.drain_once(&mut server, 100).unwrap(), Drain::Failed(id));
        let result: Value = serde_json::from_str(&db.kv_get(&format!("comment-saved-result:{key}")).unwrap().unwrap()).unwrap();
        assert_eq!(result["error"], "Unauthorized");
        assert_eq!(db.drain_once(&mut server, 1000).unwrap(), Drain::Empty);
        assert_eq!(server.calls.len(), 1);
        assert!(db.task_detail(60253).unwrap().unwrap().comments[0].extra.get("savedContent").is_none_or(|saved| saved.as_array().unwrap().is_empty()));
    }
}

#[test]
fn writer_body_matches_web_board_scope_without_copying_provider_secrets() {
    let body = writer_request(&json!({
        "id":7283,"teamId":42,"team":{"byokApiKeys":[{"enabled":true,"provider":"claude","apiKey":"never-send-this"},{"enabled":false,"provider":"openai"}]},
        "ai_custom_instructions":[{"customInstruction":"Board style","source_selected":"claude","model_selected":"model-from-board"}]
    }), "é & note\nnext", "Europe/London").body;
    assert_eq!(body, json!({
        "projectId":7283,"teamId":42,"byokProviderFlags":[{"enabled":true,"provider":"claude"},{"enabled":false,"provider":"openai"}],
        "PROMPT":"Write a task title and description from this note in the board's style.\n\n<p>é &amp; note<br>next</p>",
        "userRetrievalTexts":["é & note\nnext"],"customInstructions":"Board style","sourceSelected":"claude","modelSelected":"model-from-board",
        "aiMode":"AiTaskWriter","requestKind":"compose-task","timeZone":"Europe/London","images64":[],"taskDescription":"<p>é &amp; note<br>next</p>"
    }));
    assert_eq!(writer_request(&json!({"id":7283,"teamId":null}), "Note", "UTC").body["teamId"], -1);
    assert!(!body.to_string().contains("never-send-this"));
}
