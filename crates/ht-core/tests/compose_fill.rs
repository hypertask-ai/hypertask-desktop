use ht_core::{
    Db, Drain, Transport, Write,
    desktop_cli::empty_compose_target,
    http::{Response, TransportError},
};
use serde_json::{Value, json};

fn seed(db: &mut Db) {
    db.kv_set("current-user-id", "42").unwrap();
    db.apply_response(&json!({"success":true,"projects":[{"id":15,"title":"Board"}]}))
        .unwrap();
    db.apply_task(&json!({"id":1,"projectId":15,"sectionId":10,"title":"New task","description":"<p></p>","description_":{"content":"<p></p>"},"status":"Normal","priority":{"priority_index":4}})).unwrap();
}
struct Server {
    flags: bool,
    target: Value,
    read_status: u16,
    writer_status: u16,
    save_status: u16,
    split: bool,
    wrong_ack: bool,
    calls: Vec<(String, Value)>,
}
impl Default for Server {
    fn default() -> Self {
        Self {
            flags: true,
            target: json!({"id":1,"projectId":15,"title":"New task","description":"<p></p>","description_":{"content":"<p></p>"},"status":"Normal"}),
            read_status: 200,
            writer_status: 200,
            save_status: 200,
            split: false,
            wrong_ack: false,
            calls: vec![],
        }
    }
}
impl Transport for Server {
    fn get(&mut self, path: &str) -> Result<Response, TransportError> {
        self.calls.push((path.into(), Value::Null));
        Ok(match path {
            "/user/context" => Response {
                status: 200,
                body: json!({"success":true,"user":{"id":42}}),
            },
            "/api/flags" => Response {
                status: 200,
                body: json!({"flags":{"htpr-6929-compose-task-writer":self.flags,"htpr-6937-new-task-window":self.flags}}),
            },
            "/api/tasks/single?id=1" => Response {
                status: self.read_status,
                body: self.target.clone(),
            },
            "/api/tasks/createGlobally?projectId=15&position=top" => Response {
                status: 200,
                body: json!({"sectionId":10,"section":"Todo","ranking":"a"}),
            },
            _ => panic!("unexpected read {path}"),
        })
    }
    fn post(&mut self, path: &str, body: &Value, key: &str) -> Result<Response, TransportError> {
        assert!(!key.is_empty());
        self.calls.push((path.into(), body.clone()));
        Ok(match path {
            "/api/ai/task-writer" => Response {
                status: self.writer_status,
                body: json!({"tasks":if self.split {vec![json!({"title":"First","description":"<p>One</p>"}),json!({"title":"Second","description":"<p>Two</p>"})]}else{vec![json!({"title":"Written","description":"<p>Body</p>"})]}}),
            },
            "/api/tasks/createGlobally" => Response {
                status: self.save_status,
                body: json!({"newTask":{"id":if body["existingTaskId"] == 1 && !self.wrong_ack {1}else{9},"projectId":15,"title":body["title"],"description":body["description"],"status":"Normal"}}),
            },
            _ => panic!("unexpected write {path}"),
        })
    }
}
fn queue(db: &mut Db) -> (i64, String) {
    let id = db
        .enqueue(
            Write::ComposeTaskInTask {
                project_id: 15,
                task_id: 1,
                text: "Original note".into(),
                time_zone: "Europe/Berlin".into(),
            },
            100,
        )
        .unwrap();
    let key = db
        .mutations()
        .unwrap()
        .into_iter()
        .find(|row| row.id == id)
        .unwrap()
        .client_id;
    (id, key)
}
#[test]
fn empty_compose_target_matches_web_placeholders_rich_content_and_media_guards() {
    for title in ["", " New task ", "ENTER TASK TITLE HERE"] {
        for html in ["", "<p><br></p>", "<p>&nbsp;&#160;</p>"] {
            assert!(empty_compose_target(
                &json!({"title":title,"description":html})
            ));
        }
    }
    for html in [
        "<p>Real content</p>",
        "<img src='x'>",
        "<video></video>",
        "<audio></audio>",
        "<iframe></iframe>",
        "<embed>",
        "<hr>",
        "<p>\u{200b}</p>",
    ] {
        assert!(
            !empty_compose_target(&json!({"title":"new task","description":html})),
            "{html}"
        );
    }
    assert!(!empty_compose_target(
        &json!({"title":"Existing title","description":""})
    ));
    assert!(empty_compose_target(
        &json!({"title":"new task","description":"Legacy text","description_":{"content":"<p></p>"}})
    ));
}
#[test]
fn flagged_empty_writer_fill_uses_existing_id_exact_routes_and_refusal_rollback() {
    for (read_status, writer_status, save_status) in [
        (200, 200, 200),
        (401, 200, 200),
        (403, 200, 200),
        (200, 401, 200),
        (200, 403, 200),
        (200, 200, 401),
        (200, 200, 403),
    ] {
        let mut db = Db::memory().unwrap();
        seed(&mut db);
        let before = db.task(1).unwrap().unwrap();
        let (id, key) = queue(&mut db);
        let mut server = Server {
            read_status,
            writer_status,
            save_status,
            ..Default::default()
        };
        let ok = read_status == 200 && writer_status == 200 && save_status == 200;
        assert_eq!(
            db.drain_once(&mut server, 101).unwrap(),
            if ok {
                Drain::Applied(id)
            } else {
                Drain::Failed(id)
            }
        );
        let result: Value = serde_json::from_str(
            &db.kv_get(&format!("compose-result:{key}"))
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        if ok {
            assert_eq!(result["filledExistingTask"], true);
            assert_eq!(result["tasks"][0]["id"], 1);
            assert_eq!(db.task(1).unwrap().unwrap().title, "Written");
        } else {
            assert!(result["error"].is_string());
            assert!(result["tasks"].is_null());
            let after = db.task(1).unwrap().unwrap();
            assert_eq!(after.title, before.title);
            assert_eq!(after.description, before.description);
            assert_eq!(after.extra["priority"], before.extra["priority"]);
        }
        assert!(
            !server
                .calls
                .iter()
                .any(|(path, _)| path.contains("position=top"))
        );
        for (path, body) in &server.calls {
            if path == "/api/ai/task-writer" || path == "/api/tasks/createGlobally" {
                assert_eq!(body["existingTaskId"], 1);
            }
            if path == "/api/tasks/createGlobally" {
                assert_eq!(body["assignees"], json!([]));
                for key in [
                    "sectionId",
                    "section_title",
                    "ranking",
                    "priority",
                    "estimate",
                ] {
                    assert!(body.get(key).is_none(), "{key}");
                }
            }
        }
        assert_eq!(db.drain_once(&mut server, 1000).unwrap(), Drain::Empty);
    }
}
#[test]
fn empty_writer_fill_never_overwrites_real_tasks_or_ignores_flag_and_split_context() {
    for mode in ["flags-off", "real-content", "split"] {
        let mut db = Db::memory().unwrap();
        seed(&mut db);
        let (_, key) = queue(&mut db);
        let mut server = Server::default();
        if mode == "flags-off" {
            server.flags = false;
        }
        if mode == "real-content" {
            server.target["description_"]["content"] = json!("<p>Existing</p>");
        }
        if mode == "split" {
            server.split = true;
        }
        db.drain_once(&mut server, 101).unwrap();
        let result: Value = serde_json::from_str(
            &db.kv_get(&format!("compose-result:{key}"))
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        assert_eq!(result["filledExistingTask"], mode == "split");
        let writes: Vec<_> = server
            .calls
            .iter()
            .filter(|(path, _)| path == "/api/tasks/createGlobally")
            .collect();
        if mode == "split" {
            assert_eq!(writes.len(), 2);
            assert_eq!(writes[0].1["existingTaskId"], 1);
            assert!(writes[1].1["existingTaskId"].is_null());
            assert_eq!(writes[1].1["sectionId"], 10);
        } else {
            assert_eq!(writes.len(), 1);
            assert!(writes[0].1["existingTaskId"].is_null());
            assert_eq!(db.task(1).unwrap().unwrap().title, "New task");
        }
    }
}
#[test]
fn empty_writer_fill_unknown_ack_never_replays_after_restart_or_claims_success() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let mut db = Db::open(file.path()).unwrap();
    seed(&mut db);
    let (id, key) = queue(&mut db);
    let mut server = Server {
        wrong_ack: true,
        ..Default::default()
    };
    assert_eq!(db.drain_once(&mut server, 101).unwrap(), Drain::Failed(id));
    assert_eq!(db.task(1).unwrap().unwrap().title, "New task");
    drop(db);
    let mut db = Db::open(file.path()).unwrap();
    let count = server.calls.len();
    assert_eq!(db.drain_once(&mut server, 1000).unwrap(), Drain::Empty);
    assert_eq!(server.calls.len(), count);
    let result: Value = serde_json::from_str(
        &db.kv_get(&format!("compose-result:{key}"))
            .unwrap()
            .unwrap(),
    )
    .unwrap();
    assert!(result["error"].is_string());
}
