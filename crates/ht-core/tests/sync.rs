use ht_core::{
    Db, Sync, Transport,
    http::{Response, TransportError},
    sync::cursor_minus_second,
};
use serde_json::{Value, json};
use std::collections::VecDeque;

struct Script {
    responses: VecDeque<std::result::Result<Response, TransportError>>,
    paths: Vec<String>,
}
impl Script {
    fn new(values: Vec<Value>) -> Self {
        Self {
            responses: values
                .into_iter()
                .map(|body| Ok(Response { status: 200, body }))
                .collect(),
            paths: vec![],
        }
    }
}
impl Transport for Script {
    fn get(&mut self, path: &str) -> std::result::Result<Response, TransportError> {
        self.paths.push(path.to_owned());
        self.responses.pop_front().expect("unexpected HTTP request")
    }
    fn post(
        &mut self,
        _: &str,
        _: &Value,
        _: &str,
    ) -> std::result::Result<Response, TransportError> {
        panic!("unexpected write")
    }
}
fn task(id: i64, status: &str, date: &str) -> Value {
    json!({"id":id,"title":format!("task {id}"),"projectId":15,"sectionId":1,"section":"Todo","status":status,"updatedAt":date,"description":"body"})
}
fn page(tasks: Vec<Value>, total: usize) -> Value {
    json!({"success":true,"tasks":tasks,"total":total})
}
fn query(path: &str) -> std::collections::HashMap<String, String> {
    url::Url::parse(&format!("https://example.test{path}"))
        .unwrap()
        .query_pairs()
        .into_owned()
        .collect()
}

#[test]
fn delta_pages_all_statuses_with_encoded_overlapping_cursors() {
    let tasks = (1..=100)
        .map(|id| task(id, "Normal", "2026-10-19T23:59:59.123Z"))
        .collect();
    let mut sync = Sync::new(
        Db::memory().unwrap(),
        Script::new(vec![
            page(tasks, 101),
            page(vec![task(101, "Normal", "2026-10-20T00:00:00.123Z")], 101),
            page(vec![task(101, "Normal", "2026-10-20T00:00:01.123Z")], 1),
            page(vec![task(1, "Archive", "2026-10-20T00:00:02.123Z")], 1),
            page(vec![task(2, "Deleted", "2026-10-20T00:00:03.123Z")], 1),
            page(vec![], 0),
            page(vec![], 0),
            page(vec![], 0),
        ]),
    );
    sync.delta(15).unwrap();
    assert_eq!(sync.transport.paths.len(), 2);
    assert_eq!(query(&sync.transport.paths[0])["limit"], "100");
    assert_eq!(query(&sync.transport.paths[1])["offset"], "100");
    assert!(!query(&sync.transport.paths[0]).contains_key("updated_since"));
    assert_eq!(
        sync.db.cursor(15, "Normal").unwrap().as_deref(),
        Some("2026-10-20T00:00:00.123Z")
    );
    sync.delta(15).unwrap();
    assert_eq!(
        query(&sync.transport.paths[2])["updated_since"],
        "2026-10-19T23:59:59.123Z"
    );
    assert!(sync.transport.paths[2].contains("%3A"));
    assert_eq!(query(&sync.transport.paths[3])["status"], "Archive");
    assert_eq!(query(&sync.transport.paths[4])["status"], "Deleted");
    assert_eq!(
        sync.db.task(1).unwrap().unwrap().status,
        ht_core::generated::Status::Archive
    );
    assert_eq!(
        sync.db.task(2).unwrap().unwrap().status,
        ht_core::generated::Status::Deleted
    );
    sync.delta(15).unwrap();
    assert_eq!(
        query(&sync.transport.paths[6])["updated_since"],
        "2026-10-20T00:00:01.123Z"
    );
    assert_eq!(
        query(&sync.transport.paths[7])["updated_since"],
        "2026-10-20T00:00:02.123Z"
    );
    assert_eq!(
        sync.db.cursor(15, "Archive").unwrap().as_deref(),
        Some("2026-10-20T00:00:02.123Z")
    );
}

#[test]
fn delta_incomplete_page_does_not_advance_cursor_or_apply_partial_batch() {
    let mut script = Script::new(vec![page(
        vec![task(1, "Normal", "2026-01-01T00:00:00Z")],
        2,
    )]);
    script.responses.push_back(Ok(Response {
        status: 500,
        body: Value::Null,
    }));
    let mut sync = Sync::new(Db::memory().unwrap(), script);
    assert!(sync.delta(15).is_err());
    assert!(sync.db.cursor(15, "Normal").unwrap().is_none());
    assert!(sync.db.task(1).unwrap().is_none());
}

#[test]
fn delta_empty_bootstrap_still_observes_later_archive_and_deleted() {
    let mut sync = Sync::new(
        Db::memory().unwrap(),
        Script::new(vec![
            page(vec![], 0),
            page(vec![], 0),
            page(vec![task(1, "Archive", "2026-01-01T00:00:00Z")], 1),
            page(vec![], 0),
        ]),
    );
    sync.delta(15).unwrap();
    sync.delta(15).unwrap();
    assert_eq!(sync.transport.paths.len(), 4);
    assert_eq!(
        sync.db.task(1).unwrap().unwrap().status,
        ht_core::generated::Status::Archive
    );
}

#[test]
fn delta_rejects_non_progress_wrong_board_and_bad_timestamp() {
    for body in [
        page(vec![], 1),
        page(
            vec![json!({"id":1,"title":"bad","projectId":16,"status":"Normal"})],
            1,
        ),
        page(vec![task(1, "Normal", "broken")], 1),
        json!({"success":false,"tasks":[],"total":0}),
    ] {
        let mut sync = Sync::new(Db::memory().unwrap(), Script::new(vec![body]));
        assert!(sync.delta(15).is_err());
        assert!(sync.db.task(1).unwrap().is_none());
    }
}

#[test]
fn cursor_handles_midnight_leap_day_and_timezones() {
    assert_eq!(
        cursor_minus_second("2024-03-01T00:00:00.123Z").unwrap(),
        "2024-02-29T23:59:59.123Z"
    );
    assert_eq!(
        cursor_minus_second("2026-10-20T02:00:00+02:00").unwrap(),
        "2026-10-19T23:59:59.000Z"
    );
    assert!(cursor_minus_second("2026-13-31").is_err());
}

#[test]
fn comments_page_and_snapshot_remove_deleted_preserve_local_pending() {
    let mut db = Db::memory().unwrap();
    db.apply_task(&task(1, "Normal", "2026-01-01T00:00:00Z"))
        .unwrap();
    db.enqueue(
        ht_core::Write::TaskComment {
            task_id: 1,
            text: "pending".into(),
        },
        1,
    )
    .unwrap();
    let c = |id| json!({"id":id,"text":format!("comment {id}"),"createdAt":"2026-01-01T00:00:00Z"});
    db.apply_comments(1, &json!({"success":true,"comments":[c(99)]}))
        .unwrap();
    let mut sync = Sync::new(
        db,
        Script::new(vec![
            json!({"success":true,"comments":[c(1)],"total":2}),
            json!({"success":true,"comments":[c(2)],"total":2}),
        ]),
    );
    sync.comments(1).unwrap();
    let detail = sync.db.task_detail(1).unwrap().unwrap();
    assert_eq!(detail.comments.len(), 3);
    assert!(!detail.comments.iter().any(|c| c.id == 99));
    assert!(detail.comments.iter().any(|c| c.id < 0));
    assert_eq!(query(&sync.transport.paths[1])["offset"], "1");
}

#[test]
fn inbox_snapshot_hides_remotely_archived_notifications() {
    let n = |id| json!({"id":id,"status":"Normal","archivedAt":null,"taskId":null,"createdAt":"2026-01-01T00:00:00Z","seen":false});
    let mut db = Db::memory().unwrap();
    db.apply_response(&json!({"success":true,"user_notifications":[n(1),n(2)]}))
        .unwrap();
    db.enqueue(ht_core::Write::InboxArchive { notification_id: 1 }, 1)
        .unwrap();
    let mut sync = Sync::new(
        db,
        Script::new(vec![json!({"success":true,"user_notifications":[n(1)]})]),
    );
    sync.inbox().unwrap();
    assert!(sync.db.inbox().unwrap().is_empty());
}
