use ht_core::{
    Db, Sync, Transport,
    http::{Response, TransportError},
    realtime::Event,
};
use serde_json::{Value, json};

#[test]
fn contract_realtime_server_invalidation_payloads() {
    let mut db = Db::memory().unwrap();
    for (event, channel, target, id) in [
        ("board:changed", "private-project-15", "board", 15),
        ("task:changed", "private-task-58006", "task", 58006),
        ("inbox:changed", "private-user-6", "inbox", 0),
    ] {
        for data in [json!({}), json!({"originUserId":6})] {
            let frame =
                json!({"event":event,"channel":channel,"data":data.to_string()}).to_string();
            db.apply_event(&frame).unwrap();
            assert!(
                db.refetches()
                    .unwrap()
                    .iter()
                    .any(|(t, i, g)| t == target && *i == id && *g > 0)
            );
        }
    }
    assert_eq!(db.refetches().unwrap().len(), 3);
    assert!(db.refetches().unwrap().iter().all(|(_, _, g)| *g == 2));
}

#[test]
fn contract_realtime_comment_payload_and_no_payload() {
    let mut db = Db::memory().unwrap();
    let comment = json!({"id":5,"text":"<p>push</p>","taskId":1,"createdAt":"2026-01-01T00:00:00Z","creatorId":6});
    let frame = json!({"event":"comment:changed","channel":"private-task-1","data":{"originUserId":6,"comment":comment}}).to_string();
    db.apply_event(&frame).unwrap();
    db.apply_event(&frame).unwrap();
    assert_eq!(db.dump().unwrap()["Comment"].as_array().unwrap().len(), 1);
    assert_eq!(db.dump().unwrap()["Comment"][0]["text"], "<p>push</p>");
    db.apply_event(&json!({"event":"comment:changed","channel":"private-task-1","data":"{\"originUserId\":6}"}).to_string()).unwrap();
    let requests = db.refetches().unwrap();
    assert_eq!(requests.len(), 2);
    assert!(
        requests
            .iter()
            .any(|(target, id, _)| target == "comments" && *id == 1)
    );
    assert!(
        requests
            .iter()
            .any(|(target, id, _)| target == "task" && *id == 1)
    );
}

#[test]
fn realtime_rejects_wrong_channels_and_preserves_unknown_events() {
    let mut db = Db::memory().unwrap();
    for frame in [
        json!({"event":"task:changed","channel":"private-project-1","data":{}}),
        json!({"event":"task:changed","channel":"private-task--1","data":{}}),
        json!({"event":"inbox:changed","data":{}}),
        json!({"event":"comment:changed","channel":"private-task-1","data":{"comment":{"id":5,"text":"bad","taskId":2,"createdAt":"2026-01-01T00:00:00Z"}}}),
    ] {
        assert!(db.apply_event(&frame.to_string()).is_err());
    }
    for event in [
        "time:changed",
        "feature-flags:changed",
        "agent-chat:changed",
        "pusher:ping",
        "pusher_internal:subscription_succeeded",
    ] {
        db.apply_event(&json!({"event":event,"data":{}}).to_string())
            .unwrap();
    }
    assert!(db.refetches().unwrap().is_empty());
    assert!(Event::parse("not JSON").is_err());
    assert!(db.dump().unwrap()["Comment"].as_array().unwrap().is_empty());
}

#[test]
fn realtime_invalidations_really_refetch_targeted_task_and_comments() {
    struct Server {
        paths: Vec<String>,
    }
    impl Transport for Server {
        fn get(&mut self, path: &str) -> std::result::Result<Response, TransportError> {
            self.paths.push(path.to_owned());
            let body = if path.starts_with("/comments?") {
                json!({"success":true,"comments":[{"id":10,"text":"from server","createdAt":"2026-01-01T00:00:00Z"}],"total":1})
            } else {
                json!({"success":true,"tasks":[{"id":1,"title":"server title","projectId":15,"status":"Normal"}]})
            };
            Ok(Response { status: 200, body })
        }
        fn post(
            &mut self,
            _: &str,
            _: &Value,
            _: &str,
        ) -> std::result::Result<Response, TransportError> {
            unreachable!()
        }
    }
    let mut sync = Sync::new(Db::memory().unwrap(), Server { paths: vec![] });
    sync.db
        .apply_event(
            &json!({"event":"comment:changed","channel":"private-task-1","data":{}}).to_string(),
        )
        .unwrap();
    sync.refetch_pending().unwrap();
    assert_eq!(
        sync.transport.paths,
        vec![
            "/comments?task_id=1&limit=50&offset=0&sort_order=desc",
            "/tasks?task_id=1"
        ]
    );
    assert!(sync.db.refetches().unwrap().is_empty());
    let task = sync.db.task_detail(1).unwrap().unwrap();
    assert_eq!(task.task.title, "server title");
    assert_eq!(task.comments[0].text, "from server");
}

#[test]
fn realtime_refetch_failures_remain_durable() {
    struct Offline;
    impl Transport for Offline {
        fn get(&mut self, _: &str) -> std::result::Result<Response, TransportError> {
            Err(TransportError::Offline)
        }
        fn post(
            &mut self,
            _: &str,
            _: &Value,
            _: &str,
        ) -> std::result::Result<Response, TransportError> {
            unreachable!()
        }
    }
    let file = tempfile::NamedTempFile::new().unwrap();
    let mut db = Db::open(file.path()).unwrap();
    db.apply_event("{\"event\":\"task:changed\",\"channel\":\"private-task-1\",\"data\":\"{}\"}")
        .unwrap();
    let mut sync = Sync::new(db, Offline);
    assert!(sync.refetch_pending().is_err());
    drop(sync);
    assert_eq!(Db::open(file.path()).unwrap().refetches().unwrap().len(), 1);
}
