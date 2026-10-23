use ht_core::{
    Db, Drain, Transport, Write,
    http::{Response, TransportError},
};
use serde_json::{Value, json};

fn task(title: &str) -> Value {
    json!({"id":1,"title":title,"projectId":15,"status":"Normal","sectionId":1,"section":"Todo"})
}
fn db() -> Db {
    let mut db = Db::memory().unwrap();
    db.apply_task(&task("initial")).unwrap();
    db
}
struct Reply {
    status: u16,
    body: Value,
    calls: usize,
}
impl Transport for Reply {
    fn get(&mut self, _: &str) -> std::result::Result<Response, TransportError> {
        panic!("unexpected GET")
    }
    fn post(
        &mut self,
        _: &str,
        _: &Value,
        _: &str,
    ) -> std::result::Result<Response, TransportError> {
        self.calls += 1;
        Ok(Response {
            status: self.status,
            body: self.body.clone(),
        })
    }
}
fn reply(status: u16, title: &str) -> Reply {
    Reply {
        status,
        body: json!({"success":true,"tasks":[task(title)]}),
        calls: 0,
    }
}

#[test]
fn queue_optimistic_write_is_atomic_and_survives_restart() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let mut db = Db::open(file.path()).unwrap();
    db.apply_task(&task("initial")).unwrap();
    let id = db
        .enqueue(
            Write::TaskTitle {
                task_id: 1,
                title: "offline".into(),
            },
            100,
        )
        .unwrap();
    drop(db);
    let db = Db::open(file.path()).unwrap();
    assert_eq!(db.task(1).unwrap().unwrap().title, "offline");
    assert_eq!(db.mutations().unwrap().len(), 1);
    assert_eq!(db.mutations().unwrap()[0].id, id);
    assert_eq!(db.mutations().unwrap()[0].status, "pending");
    assert!(uuid::Uuid::parse_str(&db.mutations().unwrap()[0].client_id).is_ok());
}

#[test]
fn queue_permanent_errors_are_visible_and_transient_errors_back_off() {
    for (status, is_permanent) in [
        (400, true),
        (401, true),
        (403, true),
        (404, true),
        (408, false),
        (409, true),
        (422, true),
        (429, false),
        (500, false),
        (503, false),
    ] {
        let mut db = db();
        let id = db
            .enqueue(
                Write::TaskTitle {
                    task_id: 1,
                    title: "edit".into(),
                },
                100,
            )
            .unwrap();
        let mut server = reply(status, "server");
        let result = db.drain_once(&mut server, 100).unwrap();
        assert_eq!(
            result,
            if is_permanent {
                Drain::Failed(id)
            } else {
                Drain::Retrying(id)
            }
        );
        let rows = db.mutations().unwrap();
        assert_eq!(
            rows[0].status,
            if is_permanent { "failed" } else { "pending" }
        );
        assert_eq!(rows[0].last_error, Some(format!("HTTP {status}")));
        if is_permanent {
            assert_eq!(db.drain_once(&mut server, 101).unwrap(), Drain::Empty);
        } else {
            assert_eq!(
                db.drain_once(&mut server, 100).unwrap(),
                Drain::Waiting(101)
            );
        }
        assert_eq!(server.calls, 1);
    }
}

#[test]
fn queue_success_is_server_authoritative_and_preserves_later_optimism() {
    let mut db = db();
    db.enqueue(
        Write::TaskTitle {
            task_id: 1,
            title: "first".into(),
        },
        1,
    )
    .unwrap();
    db.enqueue(
        Write::TaskTitle {
            task_id: 1,
            title: "second".into(),
        },
        1,
    )
    .unwrap();
    let mut server = reply(200, "SERVER FIRST");
    db.drain_once(&mut server, 1).unwrap();
    assert_eq!(db.task(1).unwrap().unwrap().title, "second");
    server.body = json!({"success":true,"task":task("SERVER SECOND")});
    db.drain_once(&mut server, 1).unwrap();
    assert_eq!(db.task(1).unwrap().unwrap().title, "SERVER SECOND");
    assert!(db.mutations().unwrap().is_empty());
}

#[test]
fn queue_rejects_invalid_optimistic_writes_without_partial_rows() {
    let mut db = db();
    db.apply_response(&json!({"success":true,"sections":[{"id":7,"projectId":16,"section_title":"Other","deleted":false,"visibility":true,"ranking":"A"}]})).unwrap();
    for write in [
        Write::TaskTitle {
            task_id: 1,
            title: " ".into(),
        },
        Write::TaskTitle {
            task_id: 999,
            title: "title".into(),
        },
        Write::TaskSection {
            task_id: 1,
            section_id: 7,
        },
        Write::TaskComment {
            task_id: 1,
            text: "".into(),
        },
        Write::InboxArchive {
            notification_id: 999,
        },
    ] {
        assert!(db.enqueue(write, 1).is_err());
        assert!(db.mutations().unwrap().is_empty());
        assert_eq!(db.task(1).unwrap().unwrap().title, "initial");
    }
}

#[test]
fn queue_inflight_lease_blocks_other_drainers_and_recovers_after_crash() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let mut db = Db::open(file.path()).unwrap();
    db.apply_task(&task("initial")).unwrap();
    let id = db
        .enqueue(
            Write::TaskTitle {
                task_id: 1,
                title: "edit".into(),
            },
            100,
        )
        .unwrap();
    let client_id = db.mutations().unwrap()[0].client_id.clone();
    rusqlite::Connection::open(file.path())
        .unwrap()
        .execute(
            "UPDATE mutation_queue SET status='inflight',lease_until=220,attempts=1",
            [],
        )
        .unwrap();
    drop(db);
    let mut db = Db::open(file.path()).unwrap();
    let mut server = reply(200, "authoritative");
    assert_eq!(
        db.drain_once(&mut server, 219).unwrap(),
        Drain::Waiting(220)
    );
    assert_eq!(server.calls, 0);
    assert_eq!(db.mutations().unwrap()[0].client_id, client_id);
    assert_eq!(db.drain_once(&mut server, 220).unwrap(), Drain::Applied(id));
    assert_eq!(server.calls, 1);
    assert_eq!(db.task(1).unwrap().unwrap().title, "authoritative");
}

#[test]
fn queue_malformed_success_keeps_mutation_for_idempotent_retry() {
    let mut db = db();
    let id = db
        .enqueue(
            Write::TaskTitle {
                task_id: 1,
                title: "edit".into(),
            },
            1,
        )
        .unwrap();
    let mut server = Reply {
        status: 200,
        body: json!({"success":true,"tasks":[]}),
        calls: 0,
    };
    assert_eq!(db.drain_once(&mut server, 1).unwrap(), Drain::Retrying(id));
    assert_eq!(db.task(1).unwrap().unwrap().title, "edit");
    assert_eq!(db.mutations().unwrap()[0].status, "pending");
}

#[test]
fn queue_comment_replaces_negative_id_and_forces_durable_comment_routing() {
    struct CommentServer;
    impl Transport for CommentServer {
        fn get(&mut self, _: &str) -> std::result::Result<Response, TransportError> {
            unreachable!()
        }
        fn post(
            &mut self,
            path: &str,
            body: &Value,
            client: &str,
        ) -> std::result::Result<Response, TransportError> {
            assert_eq!(path, "/comments");
            assert_eq!(body["kind"], "qa");
            assert!(uuid::Uuid::parse_str(client).is_ok());
            Ok(Response {
                status: 201,
                body: json!({"success":true,"comment":{"id":500,"text":"server text","createdAt":"2026-01-01T00:00:00Z"}}),
            })
        }
    }
    let mut db = db();
    let id = db
        .enqueue(
            Write::TaskComment {
                task_id: 1,
                text: "optimistic".into(),
            },
            1,
        )
        .unwrap();
    assert_eq!(db.task_detail(1).unwrap().unwrap().comments[0].id, -id);
    db.drain_once(&mut CommentServer, 1).unwrap();
    let detail = db.task_detail(1).unwrap().unwrap();
    assert_eq!(detail.comments.len(), 1);
    assert_eq!(detail.comments[0].id, 500);
    assert_eq!(detail.comments[0].text, "server text");
}
