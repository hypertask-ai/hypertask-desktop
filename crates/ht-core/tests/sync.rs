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

fn fidelity() -> Value {
    serde_json::from_str(include_str!("../../../fixtures/data-fidelity.json")).unwrap()
}

#[test]
fn board_hydrates_creator_and_preserves_detail_across_list_deltas() {
    let f = fidelity();
    let mut sync = Sync::new(
        Db::memory().unwrap(),
        Script::new(vec![
            f["sections"].clone(),
            f["list"].clone(),
            f["detail"].clone(),
            f["list"].clone(),
            page(vec![], 0),
            page(vec![], 0),
        ]),
    );
    sync.board(15).unwrap();
    assert_eq!(sync.transport.paths[2], "/tasks?task_id=1");
    sync.delta(15).unwrap();
    let task = sync.db.task_detail(1).unwrap().unwrap().task;
    assert_eq!(task.extra["createdBy"]["displayName"], "Example Creator");
    assert_eq!(task.extra["priority"], "High");
    assert_eq!(task.due_date.as_deref(), Some("2026-10-20T12:00:00Z"));
    assert_eq!(
        (task.comment_count, task.assignee_count, task.label_count),
        (3, 2, 1)
    );
    assert_eq!(task.extra["labels"][0]["name"], "Example tag");
    assert_eq!(
        task.extra["assignees"][1]["agent"]["displayName"],
        "Example Agent"
    );
    assert_eq!(sync.db.board(15).unwrap()[0].cards[0], task);
}

#[test]
fn bearer_snapshots_clear_removed_optional_fields_without_erasing_partial_updates() {
    let f = fidelity();
    let mut cleared = f["detail"].clone();
    for key in ["dueDate", "priority", "createdBy"] {
        cleared["tasks"][0].as_object_mut().unwrap().remove(key);
    }
    let mut sync = Sync::new(
        Db::memory().unwrap(),
        Script::new(vec![f["detail"].clone(), cleared]),
    );
    sync.task(1).unwrap();
    sync.db
        .apply_task(&json!({"id":1,"title":"partial","projectId":15,"status":"Normal"}))
        .unwrap();
    assert!(sync.db.task(1).unwrap().unwrap().due_date.is_some());
    sync.task(1).unwrap();
    let task = sync.db.task(1).unwrap().unwrap();
    assert!(task.due_date.is_none());
    assert!(task.extra["priority"].is_null());
    assert!(task.extra["createdBy"].is_null());

    let mut list = f["list"].clone();
    list["tasks"][0].as_object_mut().unwrap().remove("dueDate");
    list["tasks"][0].as_object_mut().unwrap().remove("priority");
    let mut sync = Sync::new(Db::memory().unwrap(), Script::new(vec![list]));
    sync.db.apply_response(&f["detail"]).unwrap();
    sync.delta(15).unwrap();
    let task = sync.db.task(1).unwrap().unwrap();
    assert!(task.due_date.is_none());
    assert!(task.extra["priority"].is_null());
    assert_eq!(task.extra["createdBy"]["displayName"], "Example Creator");
}

#[test]
fn optional_ai_summary_survives_partial_updates_but_detail_absence_clears_it() {
    let f = fidelity();
    let mut present = f["detail"].clone();
    present["tasks"][0]["ai_summary"] = json!({
        "content":"A saved AI summary", "updated_at":"2026-10-23T00:00:00Z"
    });
    let mut null = f["detail"].clone();
    null["tasks"][0]["ai_summary"] = Value::Null;
    let mut sync = Sync::new(Db::memory().unwrap(), Script::new(vec![
        present.clone(), null, present, f["detail"].clone(),
    ]));
    sync.task(1).unwrap();
    sync.db.apply_task(&json!({"id":1,"title":"partial","projectId":15,"status":"Normal"})).unwrap();
    assert_eq!(sync.db.task(1).unwrap().unwrap().extra["ai_summary"]["content"], "A saved AI summary");
    sync.task(1).unwrap();
    assert!(sync.db.task(1).unwrap().unwrap().extra["ai_summary"].is_null());
    sync.task(1).unwrap();
    assert!(sync.db.task(1).unwrap().unwrap().extra["ai_summary"].is_object());
    sync.task(1).unwrap();
    assert!(sync.db.task(1).unwrap().unwrap().extra["ai_summary"].is_null());
}

#[test]
fn draft_flags_are_server_owned_and_empty_snapshots_remove_markers() {
    let f = fidelity();
    let mut sync = Sync::new(
        Db::memory().unwrap(),
        Script::new(vec![f["drafts"].clone(), f["emptyDrafts"].clone()]),
    );
    sync.db.apply_response(&f["detail"]).unwrap();
    sync.db
        .enqueue(
            ht_core::Write::TaskTitle {
                task_id: 1,
                title: "Local title".into(),
            },
            1,
        )
        .unwrap();
    sync.drafts(1).unwrap();
    let task = sync.db.task(1).unwrap().unwrap();
    assert_eq!(task.extra["hasDescriptionDraft"], true);
    assert_eq!(task.extra["hasCommentDraft"], true);
    assert_eq!(task.extra["hasBoardDraft"], true);
    assert_eq!(task.description, "Published description");
    assert_eq!(task.title, "Local title");
    sync.drafts(1).unwrap();
    let task = sync.db.task(1).unwrap().unwrap();
    assert_eq!(task.extra["hasDescriptionDraft"], false);
    assert_eq!(task.extra["hasCommentDraft"], false);
    assert_eq!(task.extra["hasBoardDraft"], false);
    assert_eq!(
        sync.transport.paths,
        ["/drafts?task_id=1", "/drafts?task_id=1"]
    );
}

#[test]
fn draft_emptiness_matches_use_has_drafts_and_wrong_task_is_rejected() {
    let f = fidelity();
    for (kind, text, expected) in [
        ("description", "", false),
        ("description", " <p></p> ", false),
        (
            "description",
            "<html><head></head><body><p></p></body></html>",
            false,
        ),
        (
            "description",
            "<html><head></head><body></body></html>",
            false,
        ),
        ("description", "<p>Example</p>", true),
        ("comment", " \n", false),
        ("comment", "<p></p>", false),
        ("comment", "<html><head></head><body></body></html>", true),
        ("comment", "Example", true),
    ] {
        let mut sync = Sync::new(
            Db::memory().unwrap(),
            Script::new(vec![json!({
                "success":true,"drafts":[{"taskId":1,"draftType":kind,"text":text}]
            })]),
        );
        sync.db.apply_response(&f["detail"]).unwrap();
        sync.drafts(1).unwrap();
        let task = sync.db.task(1).unwrap().unwrap();
        let key = if kind == "description" {
            "hasDescriptionDraft"
        } else {
            "hasCommentDraft"
        };
        assert_eq!(task.extra[key], expected, "{kind}: {text}");
        assert_eq!(
            task.extra["hasBoardDraft"],
            kind == "comment" && !text.is_empty() && text != "<p></p>"
        );
    }
    let mut bad = f["drafts"].clone();
    bad["drafts"][0]["taskId"] = json!(2);
    let mut sync = Sync::new(Db::memory().unwrap(), Script::new(vec![bad]));
    sync.db.apply_response(&f["detail"]).unwrap();
    assert!(sync.drafts(1).is_err());
    assert!(
        !sync
            .db
            .task(1)
            .unwrap()
            .unwrap()
            .extra
            .contains_key("hasDescriptionDraft")
    );
}

#[test]
fn detail_mismatches_do_not_pollute_cache() {
    let f = fidelity();
    for response in [
        page(vec![], 0),
        page(vec![task(2, "Normal", "2026-01-01T00:00:00Z")], 1),
    ] {
        let mut sync = Sync::new(Db::memory().unwrap(), Script::new(vec![response]));
        sync.db.apply_response(&f["detail"]).unwrap();
        assert!(sync.task(1).is_err());
        assert_eq!(
            sync.db.task(1).unwrap().unwrap().extra["createdBy"]["displayName"],
            "Example Creator"
        );
        assert!(sync.db.task(2).unwrap().is_none());
    }
}

#[test]
fn inbox_refresh_updates_cached_titles_but_reapplies_local_edits() {
    let f = fidelity();
    let mut sync = Sync::new(
        Db::memory().unwrap(),
        Script::new(vec![f["inbox"].clone(), f["inbox"].clone()]),
    );
    sync.db.apply_response(&f["detail"]).unwrap();
    sync.inbox().unwrap();
    assert_eq!(sync.db.inbox().unwrap()[0].title, "Server renamed task");
    assert_eq!(
        sync.db.task(1).unwrap().unwrap().title,
        "Server renamed task"
    );
    sync.db
        .enqueue(
            ht_core::Write::TaskTitle {
                task_id: 1,
                title: "Local title".into(),
            },
            1,
        )
        .unwrap();
    sync.inbox().unwrap();
    assert_eq!(sync.db.inbox().unwrap()[0].title, "Local title");
    assert_eq!(sync.db.task(1).unwrap().unwrap().title, "Local title");
}

#[test]
fn creator_hydration_batches_at_one_hundred_and_does_not_refetch_cached_creators() {
    let f = fidelity();
    let tasks: Vec<_> = (1..=101)
        .map(|id| {
            let mut task = f["list"]["tasks"][0].clone();
            task["id"] = json!(id);
            task
        })
        .collect();
    let details: Vec<_> = tasks
        .iter()
        .map(|task| {
            let mut task = task.clone();
            task["createdBy"] = f["detail"]["tasks"][0]["createdBy"].clone();
            task
        })
        .collect();
    let mut sync = Sync::new(
        Db::memory().unwrap(),
        Script::new(vec![
            f["sections"].clone(),
            page(tasks[..100].to_vec(), 101),
            page(tasks[100..].to_vec(), 101),
            page(details[..100].to_vec(), 100),
            page(details[100..].to_vec(), 1),
            f["sections"].clone(),
            page(vec![], 0),
            page(vec![], 0),
            page(vec![], 0),
        ]),
    );
    sync.board(15).unwrap();
    assert_eq!(
        query(&sync.transport.paths[3])["task_id"]
            .split(',')
            .count(),
        100
    );
    assert_eq!(query(&sync.transport.paths[4])["task_id"], "101");
    sync.board(15).unwrap();
    assert_eq!(sync.transport.paths.len(), 9);
    assert_eq!(sync.db.board(15).unwrap()[0].cards.len(), 101);
}

#[test]
fn worker_refreshes_board_drafts_progressively_without_opening_task_pages() {
    use std::{sync::mpsc, time::Duration};
    struct Server {
        send: mpsc::Sender<i64>,
        fixture: Value,
    }
    impl Transport for Server {
        fn get(&mut self, path: &str) -> std::result::Result<Response, TransportError> {
            let f = &self.fixture;
            let body = if path.starts_with("/projects?") {
                json!({"success":true,"projects":[],"total":0})
            } else if path.starts_with("/projects/15/sections?") {
                f["sections"].clone()
            } else if path.starts_with("/drafts?") {
                let id: i64 = query(path)["task_id"].parse().unwrap();
                self.send.send(id).unwrap();
                json!({"success":true,"drafts":[{"id":id,"taskId":id,"draftType":"comment","text":"Example draft"}]})
            } else {
                let key = if query(path).contains_key("task_id") {
                    "detail"
                } else {
                    "list"
                };
                let mut response = f[key].clone();
                let mut second = response["tasks"][0].clone();
                second["id"] = json!(2);
                response["tasks"].as_array_mut().unwrap().push(second);
                response["total"] = json!(2);
                response
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
    let file = tempfile::NamedTempFile::new().unwrap();
    let (send, receive) = mpsc::channel();
    let worker = ht_core::Worker::start(
        file.path(),
        Server {
            send,
            fixture: fidelity(),
        },
    )
    .unwrap();
    worker.handle().sync_board(15).unwrap();
    let first = receive.recv_timeout(Duration::from_secs(10)).unwrap();
    let second = receive.recv_timeout(Duration::from_secs(10)).unwrap();
    assert_ne!(first, second);
    drop(worker);
    let db = Db::open(file.path()).unwrap();
    for id in [1, 2] {
        assert_eq!(db.task(id).unwrap().unwrap().extra["hasBoardDraft"], true);
    }
}

#[test]
fn inbox_sender_uses_recent_actors_and_null_agent_does_not_shadow_person() {
    let f = fidelity();
    let mut response = f["inbox"].clone();
    response["user_notifications"][0]["fromAgent"] = json!({"displayName":null});
    response["user_notifications"][0]["unreadCount"] = json!(3);
    let mut sync = Sync::new(Db::memory().unwrap(), Script::new(vec![response.clone()]));
    sync.inbox().unwrap();
    assert_eq!(
        sync.db.inbox().unwrap()[0].source.as_deref(),
        Some("Example Person")
    );
    assert_eq!(sync.db.inbox().unwrap()[0].raw_json["unreadCount"], 3);
    response["user_notifications"][0]["recentActors"] = json!([
        {"displayName":"Example Person"}, {"displayName":"Example Agent"}
    ]);
    sync.transport.responses.push_back(Ok(Response {
        status: 200,
        body: response,
    }));
    sync.inbox().unwrap();
    assert_eq!(
        sync.db.inbox().unwrap()[0].source.as_deref(),
        Some("Example Person, Example Agent")
    );
}

#[test]
fn inbox_stale_or_undated_snapshots_cannot_regress_an_authoritative_task() {
    let f = fidelity();
    for date in [None, Some("2026-10-18T12:00:00Z")] {
        let mut response = f["inbox"].clone();
        response["user_notifications"][0]["task"]["title"] = json!("Stale notification title");
        match date {
            Some(date) => response["user_notifications"][0]["task"]["updatedAt"] = json!(date),
            None => {
                response["user_notifications"][0]["task"]
                    .as_object_mut()
                    .unwrap()
                    .remove("updatedAt");
            }
        }
        let mut sync = Sync::new(Db::memory().unwrap(), Script::new(vec![response]));
        sync.db.apply_response(&f["detail"]).unwrap();
        sync.inbox().unwrap();
        assert_eq!(
            sync.db.task(1).unwrap().unwrap().title,
            "Synthetic fidelity task"
        );
        assert_eq!(sync.db.inbox().unwrap()[0].title, "Synthetic fidelity task");
    }
}

#[test]
fn fidelity_upgrades_list_only_database_and_persists_authors_summary_and_drafts() {
    let f: Value = serde_json::from_str(include_str!("../../../fixtures/task-fidelity.json")).unwrap();
    let file = tempfile::NamedTempFile::new().unwrap();
    {
        let mut db = Db::open(file.path()).unwrap();
        db.apply_response(&f["list"]).unwrap();
        assert!(!db.task(101).unwrap().unwrap().extra.contains_key("createdBy"));
    }
    let mut sync = Sync::new(Db::open(file.path()).unwrap(), Script::new(vec![
        f["sections"].clone(), f["list"].clone(), f["detail"].clone(), f["drafts"].clone(),
        f["list"].clone(), page(vec![], 0), page(vec![], 0),
    ]));
    sync.board(15).unwrap();
    assert_eq!(sync.transport.paths[2], "/tasks?task_id=101,102,103");
    sync.drafts(101).unwrap();
    sync.delta(15).unwrap();
    drop(sync);
    let db = Db::open(file.path()).unwrap();
    let cards = db.board(15).unwrap().remove(0).cards;
    for (index, id) in [101, 102, 103].into_iter().enumerate() {
        let task = db.task_detail(id).unwrap().unwrap().task;
        assert_eq!(cards[index], task);
        assert_eq!(task.description, "<p>Published description</p>");
        if id == 103 {
            assert!(task.extra["createdBy"].is_null());
        } else {
            assert_eq!(task.extra["createdBy"], f["detail"]["tasks"][index]["createdBy"]);
        }
    }
    let human = db.task(101).unwrap().unwrap();
    assert_eq!(human.extra["ai_summary"], f["detail"]["tasks"][0]["ai_summary"]);
    for key in ["hasDescriptionDraft", "hasCommentDraft", "hasBoardDraft"] {
        assert_eq!(human.extra[key], true, "{key}");
    }
    assert!(db.task(102).unwrap().unwrap().extra["ai_summary"].is_null());
}

#[test]
fn bounded_sync_cache_evicts_pages_without_losing_persisted_offline_content() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let mut db = Db::open(file.path()).unwrap();
    let description = "offline content ".repeat(1024);
    for id in 1..=128 {
        db.apply_task(&json!({"id":id,"title":format!("task {id}"),"projectId":15,
            "status":"Normal","description":description})).unwrap();
    }
    if cfg!(feature = "bounded-sync-cache") {
        assert!(db.memory_usage().unwrap()["SqliteCacheBytes"].as_u64().unwrap() < 600_000);
    }
    drop(db);
    let db = Db::open(file.path()).unwrap();
    for id in 1..=128 {
        assert_eq!(db.task_detail(id).unwrap().unwrap().task.description, description);
    }
}
