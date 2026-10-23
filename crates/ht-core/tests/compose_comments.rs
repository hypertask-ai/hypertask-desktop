use ht_core::{
    CommentWrite, Db, Drain, Transport, Write,
    http::{Response, TransportError},
};
use serde_json::{Value, json};

fn db() -> Db {
    let mut db = Db::memory().unwrap();
    db.apply_task(&json!({"id":1,"projectId":15,"title":"Task","status":"Normal"}))
        .unwrap();
    db.apply_comments(1, &json!({"success":true,"comments":[{"id":7,"text":"<p>Original</p>","createdAt":"2026-01-01T00:00:00Z","creatorId":42,"creator":{"displayName":"Owner"},"reactions":[]}]})).unwrap();
    db.kv_set("current-user-id", "42").unwrap();
    db
}
fn change(db: &mut Db, change: CommentWrite) -> i64 {
    db.enqueue(
        Write::CommentAction {
            task_id: 1,
            comment_id: 7,
            change,
        },
        100,
    )
    .unwrap()
}
fn comment(db: &Db) -> Value {
    serde_json::to_value(&db.task_detail(1).unwrap().unwrap().comments[0]).unwrap()
}
struct Server {
    method: &'static str,
    path: &'static str,
    body: Value,
    response: Response,
    keys: Vec<String>,
}
impl Transport for Server {
    fn get(&mut self, _: &str) -> Result<Response, TransportError> {
        panic!("unexpected get")
    }
    fn post(&mut self, path: &str, body: &Value, id: &str) -> Result<Response, TransportError> {
        self.mutate("POST", path, body, id)
    }
    fn mutate(
        &mut self,
        method: &str,
        path: &str,
        body: &Value,
        id: &str,
    ) -> Result<Response, TransportError> {
        assert_eq!(method, self.method);
        assert_eq!(path, self.path);
        assert_eq!(body, &self.body);
        assert!(uuid::Uuid::parse_str(id).is_ok());
        self.keys.push(id.into());
        Ok(self.response.clone())
    }
}
#[test]
fn comment_edit_queue_uses_patch_html_and_preserves_author_metadata() {
    let mut db = db();
    let text = "<p>Edited<br>é🙂 &amp; safe</p>";
    let id = change(&mut db, CommentWrite::Edit { text: text.into() });
    assert_eq!(comment(&db)["text"], text);
    let mut server = Server {
        method: "PATCH",
        path: "/comments/7",
        body: json!({"text":text,"content_type":"html"}),
        response: Response {
            status: 200,
            body: json!({"success":true,"comment":{"id":7,"text":text,"editedAt":"2026-02-01T00:00:00Z"}}),
        },
        keys: vec![],
    };
    assert_eq!(db.drain_once(&mut server, 100).unwrap(), Drain::Applied(id));
    assert_eq!(comment(&db)["creator"]["displayName"], "Owner");
    assert_eq!(comment(&db)["commentText"], text);
    assert!(db.mutations().unwrap().is_empty());
}
#[test]
fn comment_delete_queue_confirms_server_delete_and_restores_on_denial() {
    for status in [200, 403] {
        let mut db = db();
        let before = comment(&db);
        let id = change(&mut db, CommentWrite::Delete);
        assert!(db.task_detail(1).unwrap().unwrap().comments.is_empty());
        let mut server = Server {
            method: "DELETE",
            path: "/comments/7",
            body: json!({}),
            response: Response {
                status,
                body: json!({"success":status==200}),
            },
            keys: vec![],
        };
        assert_eq!(
            db.drain_once(&mut server, 100).unwrap(),
            if status == 200 {
                Drain::Applied(id)
            } else {
                Drain::Failed(id)
            }
        );
        if status == 403 {
            assert_eq!(comment(&db), before);
        } else {
            assert!(db.task_detail(1).unwrap().unwrap().comments.is_empty());
        }
    }
}
#[test]
fn comment_reactions_are_set_not_toggled_on_retry_and_keep_later_optimism() {
    let mut db = db();
    change(
        &mut db,
        CommentWrite::Reaction {
            emoji: "👍".into(),
            active: true,
        },
    );
    let mut server = Server {
        method: "POST",
        path: "/comments/7/reactions",
        body: json!({"emoji":"👍","active":true}),
        response: Response {
            status: 503,
            body: json!({"success":false}),
        },
        keys: vec![],
    };
    assert!(matches!(
        db.drain_once(&mut server, 100).unwrap(),
        Drain::Retrying(_)
    ));
    change(
        &mut db,
        CommentWrite::Reaction {
            emoji: "👍".into(),
            active: false,
        },
    );
    server.response = Response {
        status: 200,
        body: json!({"success":true,"active":true,"reactions":[{"id":"server","emoji":"👍","userId":42}]}),
    };
    assert!(matches!(
        db.drain_once(&mut server, 101).unwrap(),
        Drain::Applied(_)
    ));
    assert_eq!(server.keys[0], server.keys[1]);
    assert_eq!(comment(&db)["reactions"], json!([]));
    server.body["active"] = json!(false);
    server.response.body["active"] = json!(false);
    server.response.body["reactions"] = json!([]);
    assert!(matches!(
        db.drain_once(&mut server, 101).unwrap(),
        Drain::Applied(_)
    ));
}
#[test]
fn comment_actions_validate_target_owner_text_and_emoji_atomically() {
    let mut db = db();
    for (task_id, comment_id, action) in [
        (2, 7, CommentWrite::Delete),
        (1, -1, CommentWrite::Delete),
        (1, 7, CommentWrite::Edit { text: " ".into() }),
        (
            1,
            7,
            CommentWrite::Edit {
                text: "x".repeat(5001),
            },
        ),
        (
            1,
            7,
            CommentWrite::Reaction {
                emoji: "word".into(),
                active: true,
            },
        ),
    ] {
        assert!(
            db.enqueue(
                Write::CommentAction {
                    task_id,
                    comment_id,
                    change: action
                },
                100
            )
            .is_err()
        );
    }
    db.kv_set("current-user-id", "43").unwrap();
    assert!(
        db.enqueue(
            Write::CommentAction {
                task_id: 1,
                comment_id: 7,
                change: CommentWrite::Edit {
                    text: "not mine".into()
                }
            },
            100
        )
        .is_err()
    );
    assert!(
        db.enqueue(
            Write::CommentAction {
                task_id: 1,
                comment_id: 7,
                change: CommentWrite::Delete
            },
            100
        )
        .is_err()
    );
    assert!(db.mutations().unwrap().is_empty());
    assert_eq!(comment(&db)["text"], "<p>Original</p>");
}
#[test]
fn comment_edit_survives_restart_and_rejects_wrong_server_identity() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let seed = db();
    let original = comment(&seed);
    let mut db = Db::open(file.path()).unwrap();
    db.apply_task(&json!({"id":1,"projectId":15,"title":"Task","status":"Normal"}))
        .unwrap();
    db.apply_comments(1, &json!({"success":true,"comments":[original]}))
        .unwrap();
    db.kv_set("current-user-id", "42").unwrap();
    change(
        &mut db,
        CommentWrite::Edit {
            text: "edited".into(),
        },
    );
    drop(db);
    let mut db = Db::open(file.path()).unwrap();
    assert_eq!(comment(&db)["text"], "edited");
    let mut server = Server {
        method: "PATCH",
        path: "/comments/7",
        body: json!({"text":"edited","content_type":"html"}),
        response: Response {
            status: 200,
            body: json!({"success":true,"comment":{"id":8,"text":"wrong"}}),
        },
        keys: vec![],
    };
    assert!(matches!(
        db.drain_once(&mut server, 100).unwrap(),
        Drain::Retrying(_)
    ));
    assert_eq!(comment(&db)["text"], "edited");
}

#[test]
fn quoted_reply_create_body_is_html_and_never_uses_agent_only_parent_id() {
    let mut db = db();
    let quote = "<span data-type=\"mention\" data-label=\"name-42\">Owner</span> said <blockquote data-quote-reply=\"true\"><p>Original</p></blockquote><p>Reply<br>Line</p>";
    let id = db
        .enqueue(
            Write::TaskComment {
                task_id: 1,
                text: quote.into(),
            },
            100,
        )
        .unwrap();
    let mut server = Server {
        method: "POST",
        path: "/comments",
        body: json!({"task_id":1,"text":quote,"kind":"qa","content_type":"html"}),
        response: Response {
            status: 200,
            body: json!({"success":true,"comment":{"id":9,"text":quote,"createdAt":"2026-01-03T00:00:00Z"}}),
        },
        keys: vec![],
    };
    assert_eq!(db.drain_once(&mut server, 100).unwrap(), Drain::Applied(id));
    assert!(
        db.task_detail(1)
            .unwrap()
            .unwrap()
            .comments
            .iter()
            .any(|c| c.id == 9 && c.text == quote)
    );
}
