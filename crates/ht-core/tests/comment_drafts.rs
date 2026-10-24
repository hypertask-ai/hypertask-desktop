use ht_core::{
    Db, Drain, Sync, Transport, Write,
    http::{Response, TransportError},
};
use serde_json::{Value, json};
use std::collections::VecDeque;

struct Server(VecDeque<(&'static str, &'static str, Value, u16, Value)>);
impl Transport for Server {
    fn get(&mut self, path: &str) -> Result<Response, TransportError> {
        self.mutate("GET", path, &Value::Null, "")
    }
    fn post(&mut self, path: &str, body: &Value, client: &str) -> Result<Response, TransportError> {
        self.mutate("POST", path, body, client)
    }
    fn mutate(
        &mut self,
        method: &str,
        path: &str,
        body: &Value,
        client: &str,
    ) -> Result<Response, TransportError> {
        let (expected_method, expected_path, expected_body, status, response) =
            self.0.pop_front().expect("unexpected draft request");
        assert_eq!(
            (method, path, body),
            (expected_method, expected_path, &expected_body)
        );
        if method != "GET" {
            assert!(uuid::Uuid::parse_str(client).is_ok());
        }
        Ok(Response {
            status,
            body: response,
        })
    }
}
fn db() -> Db {
    let mut db = Db::memory().unwrap();
    db.kv_set("current-user-id", "42").unwrap();
    for id in [1, 2] {
        db.apply_task(&json!({"id":id,"projectId":15,"title":"Task","status":"Normal"}))
            .unwrap();
    }
    db
}
fn draft(text: &str) -> Value {
    json!({"id":7,"taskId":1,"draftType":"comment","text":text,"createdBy":{"id":42}})
}
fn save(text: &str) -> Write {
    Write::CommentDraft {
        task_id: 1,
        text: Some(text.into()),
    }
}
fn clear() -> Write {
    Write::CommentDraft {
        task_id: 1,
        text: None,
    }
}
fn save_reply(text: &str) -> (&'static str, &'static str, Value, u16, Value) {
    (
        "POST",
        "/drafts",
        json!({"task_id":1,"draft_type":"comment","text":text}),
        200,
        json!({"success":true,"draft":draft(text)}),
    )
}
fn clear_replies() -> Vec<(&'static str, &'static str, Value, u16, Value)> {
    vec![
        (
            "GET",
            "/drafts?task_id=1",
            Value::Null,
            200,
            json!({"success":true,"drafts":[draft("saved"),{"id":8,"taskId":1,"draftType":"description","text":"leave alone","createdBy":{"id":42}},{"id":9,"taskId":1,"draftType":"comment","text":"other user's draft","createdBy":{"id":43}}]}),
        ),
        (
            "DELETE",
            "/drafts/7",
            json!({}),
            200,
            json!({"success":true}),
        ),
    ]
}

#[test]
fn draft_queue_uses_bearer_routes_exact_body_and_coalesces_unattempted_saves() {
    let mut db = db();
    db.enqueue(save("first"), 1).unwrap();
    db.enqueue(save("<p>Latest é🙂</p>"), 2).unwrap();
    assert_eq!(db.mutations().unwrap().len(), 1);
    assert_eq!(
        db.comment_draft(1).unwrap().as_deref(),
        Some("<p>Latest é🙂</p>")
    );
    assert!(db.comment_draft(2).unwrap().is_none());
    let mut server = Server(VecDeque::from([save_reply("<p>Latest é🙂</p>")]));
    assert!(matches!(
        db.drain_once(&mut server, 2).unwrap(),
        Drain::Applied(_)
    ));
    assert!(db.mutations().unwrap().is_empty());
    assert_eq!(
        db.comment_draft(1).unwrap().as_deref(),
        Some("<p>Latest é🙂</p>")
    );
    db.enqueue(clear(), 3).unwrap();
    server.0.extend(clear_replies());
    assert!(matches!(
        db.drain_once(&mut server, 3).unwrap(),
        Drain::Applied(_)
    ));
    assert_eq!(db.comment_draft(1).unwrap().as_deref(), Some(""));
    assert!(server.0.is_empty());
}

#[test]
fn server_draft_restore_is_scoped_and_never_overwrites_a_newer_local_edit() {
    let mut db = db();
    db.save_local_comment_draft(1, "<p>New local</p>").unwrap();
    let mut sync = Sync::new(
        db,
        Server(VecDeque::from([
            (
                "GET",
                "/drafts?task_id=1",
                Value::Null,
                200,
                json!({"success":true,"drafts":[draft("Old server")]}),
            ),
            (
                "GET",
                "/drafts?task_id=2",
                Value::Null,
                200,
                json!({"success":true,"drafts":[{"id":8,"taskId":2,"draftType":"comment","text":"Remote draft","createdBy":{"id":42}}]}),
            ),
        ])),
    );
    sync.drafts(1).unwrap();
    sync.drafts(2).unwrap();
    assert_eq!(
        sync.db.comment_draft(1).unwrap().as_deref(),
        Some("<p>New local</p>")
    );
    assert_eq!(
        sync.db.comment_draft(2).unwrap().as_deref(),
        Some("Remote draft")
    );
    sync.db.kv_set("current-user-id", "43").unwrap();
    assert!(sync.db.comment_draft(1).unwrap().is_none());
    assert!(sync.db.comment_draft(2).unwrap().is_none());
}

#[test]
fn failed_comment_preserves_draft_and_success_clears_only_after_acknowledgement() {
    for status in [403, 201] {
        let mut db = db();
        db.save_local_comment_draft(1, "<p>Unsent</p>").unwrap();
        db.enqueue(
            Write::TaskComment {
                task_id: 1,
                text: "<p>Unsent</p>".into(),
            },
            10,
        )
        .unwrap();
        let mut server = Server(VecDeque::from([(
            "POST",
            "/comments",
            json!({"task_id":1,"text":"<p>Unsent</p>","kind":"qa","content_type":"html"}),
            status,
            json!({"success":status==201,"comment":{"id":99,"text":"<p>Unsent</p>","createdAt":"2026-01-01T00:00:00Z"}}),
        )]));
        db.drain_once(&mut server, 10).unwrap();
        if status == 403 {
            assert_eq!(
                db.comment_draft(1).unwrap().as_deref(),
                Some("<p>Unsent</p>")
            );
            assert_eq!(db.mutations().unwrap().len(), 1);
            assert_eq!(db.mutations().unwrap()[0].status, "failed");
        } else {
            assert_eq!(db.mutations().unwrap().len(), 1);
            assert_eq!(db.mutations().unwrap()[0].kind, "comment_draft");
            server.0.extend(clear_replies());
            db.drain_once(&mut server, 10).unwrap();
            assert_eq!(db.comment_draft(1).unwrap().as_deref(), Some(""));
            assert!(db.mutations().unwrap().is_empty());
        }
        assert!(server.0.is_empty());
    }
}

#[test]
fn sending_older_comment_never_clears_a_new_draft_or_replays_old_draft_over_it() {
    let mut db = db();
    db.enqueue(save("Old"), 1).unwrap();
    db.enqueue(
        Write::TaskComment {
            task_id: 1,
            text: "Old".into(),
        },
        2,
    )
    .unwrap();
    db.save_local_comment_draft(1, "New draft still being typed")
        .unwrap();
    let mut server = Server(VecDeque::from([
        save_reply("Old"),
        (
            "POST",
            "/comments",
            json!({"task_id":1,"text":"Old","kind":"qa","content_type":"html"}),
            201,
            json!({"success":true,"comment":{"id":99,"text":"Old","createdAt":"2026-01-01T00:00:00Z"}}),
        ),
    ]));
    db.drain_once(&mut server, 2).unwrap();
    assert_eq!(
        db.comment_draft(1).unwrap().as_deref(),
        Some("New draft still being typed")
    );
    db.drain_once(&mut server, 2).unwrap();
    assert!(db.mutations().unwrap().is_empty());
    assert_eq!(
        db.comment_draft(1).unwrap().as_deref(),
        Some("New draft still being typed")
    );
    assert!(server.0.is_empty());
}

#[test]
fn drafts_survive_restart_and_rejected_saves_are_not_reported_as_success() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let mut db = Db::open(file.path()).unwrap();
    db.kv_set("current-user-id", "42").unwrap();
    db.apply_task(&json!({"id":1,"projectId":15,"title":"Task","status":"Normal"}))
        .unwrap();
    db.enqueue(save("Draft"), 1).unwrap();
    drop(db);
    let mut db = Db::open(file.path()).unwrap();
    assert_eq!(db.comment_draft(1).unwrap().as_deref(), Some("Draft"));
    let mut server = Server(VecDeque::from([(
        "POST",
        "/drafts",
        json!({"task_id":1,"draft_type":"comment","text":"Draft"}),
        403,
        json!({"success":false}),
    )]));
    assert!(matches!(
        db.drain_once(&mut server, 1).unwrap(),
        Drain::Failed(_)
    ));
    assert_eq!(db.comment_draft(1).unwrap().as_deref(), Some("Draft"));
}
