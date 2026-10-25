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
        json!({"event":"time:changed","channel":"private-task-1","data":{}}),
        json!({"event":"time:changed","channel":"private-time-task-0","data":{}}),
        json!({"event":"time:changed","channel":"private-time-project--1","data":{}}),
        json!({"event":"comment:changed","channel":"private-task-1","data":{"comment":{"id":5,"text":"bad","taskId":2,"createdAt":"2026-01-01T00:00:00Z"}}}),
    ] {
        assert!(db.apply_event(&frame.to_string()).is_err());
    }
    for event in [
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

struct TimeServer {
    status: u16,
    user: i64,
    running: bool,
    paths: Vec<String>,
}
impl Transport for TimeServer {
    fn get(&mut self, path: &str) -> Result<Response, TransportError> {
        self.paths.push(path.into());
        if path == "/user/context" {
            return Ok(Response { status:200, body:json!({"success":true,"user":{"id":self.user}}) });
        }
        let body = match path {
            "/api/time/task?taskId=1" => json!({"success":true,"enabled":true,"runningEntry":null,"activeRunningEntryCount":if self.running {1}else{0},"taskTotalSeconds":900,"myTotalSeconds":300,"otherEntriesSeconds":600}),
            "/api/time/entries?taskId=1" => json!({"success":true,"canManage":false,"entries":[{"id":5,"taskId":1,"userId":42,"userName":"Me","seconds":900,"startedAt":"2026-10-01T09:00:00Z","endedAt":"2026-10-01T09:15:00Z","pausedAt":null}]}),
            "/api/time/board-summary?board=15" => json!({"success":true,"entries":if self.running {json!([{"taskId":1,"runningEntries":[{"startedAt":"2026-10-01T09:00:00Z","pausedAt":null,"userName":"Other person"}]}])}else{json!([])}}),
            _ => panic!("unexpected time route: {path}"),
        };
        Ok(Response { status:self.status, body:if self.status==200 {body}else{json!({"success":false,"error":"Not authenticated"})} })
    }
    fn post(&mut self, _: &str, _: &Value, _: &str) -> Result<Response, TransportError> { panic!("invalidation must never write"); }
}
fn time_seed(db: &mut Db) {
    db.kv_set("current-user-id", "42").unwrap();
    db.apply_web_board(42, &json!({"project":{"id":15,"title":"Board","timeTrackingEnabled":true,"section":[{"id":10,"projectId":15,"section_title":"Todo","visibility":true,"ranking":"a","deleted":false}]},"allViews":[],"tasks":[{"id":1,"projectId":15,"sectionId":10,"title":"Task","status":"Normal","ranking":"a"}]})).unwrap();
    db.cache_task_time(42,1,json!({"enabled":true,"runningEntry":null,"activeRunningEntryCount":0,"taskTotalSeconds":60}),1).unwrap();
    db.cache_time_entries(42,1,&json!({"success":true,"canManage":false,"entries":[]})).unwrap();
    db.kv_set("web-running-timers:42:15", "[]").unwrap();
}
#[test]
fn time_events_refresh_summary_entries_and_live_board_timer_filter_or_preserve_refused_state() {
    for status in [401, 403, 200] {
        let file = tempfile::NamedTempFile::new().unwrap();
        let mut db = Db::open(file.path()).unwrap();
        time_seed(&mut db);
        for channel in ["private-time-task-1", "private-time-project-15"] {
            db.apply_event(&json!({"event":"time:changed","channel":channel,"data":"{}"}).to_string()).unwrap();
        }
        drop(db);
        let mut sync = Sync::new(Db::open(file.path()).unwrap(), TimeServer {status,user:42,running:true,paths:vec![]});
        let result = sync.refetch_pending();
        if status == 200 {
            result.unwrap();
            assert!(sync.db.refetches().unwrap().is_empty());
            assert_eq!(sync.db.task_time(1).unwrap().unwrap()["taskTotalSeconds"],900);
            assert_eq!(sync.db.time_entries(1).unwrap().unwrap()["entries"][0]["id"],5);
            assert!(sync.transport.paths.contains(&"/api/time/task?taskId=1".into()));
            assert!(sync.transport.paths.contains(&"/api/time/entries?taskId=1".into()));
            let view = json!({"board_columns_view":[{"id":10,"visibility":true}],"board_filters":{"addedFilters":[{"type":"RunningTimer","searchPayload":[]}]}});
            assert_eq!(sync.db.view_task_count(15,&view).unwrap(),1);
            sync.transport.running = false;
            sync.db.apply_event(&json!({"event":"time:changed","channel":"private-time-project-15","data":{}}).to_string()).unwrap();
            sync.refetch_pending().unwrap();
            assert_eq!(sync.db.view_task_count(15,&view).unwrap(),0);
        } else {
            assert_eq!(result.unwrap_err().to_string(), "Not authenticated");
            assert_eq!(sync.db.refetches().unwrap().len(),2);
            assert_eq!(sync.db.task_time(1).unwrap().unwrap()["taskTotalSeconds"],60);
            assert!(sync.db.time_entries(1).unwrap().unwrap()["entries"].as_array().unwrap().is_empty());
            assert_eq!(sync.db.kv_get("web-running-timers:42:15").unwrap().as_deref(),Some("[]"));
            sync.transport.status = 200;
            sync.refetch_pending().unwrap();
            assert!(sync.db.refetches().unwrap().is_empty());
        }
    }
}

#[test]
fn time_event_reads_never_replace_pending_optimistic_writes_or_cross_account_caches() {
    for user in [42,99] {
        let mut db = Db::memory().unwrap();
        time_seed(&mut db);
        db.enqueue(ht_core::Write::Time {change:ht_core::TimeWrite::Start {task:1}},100).unwrap();
        let before = db.task_time(1).unwrap().unwrap();
        let mut sync = Sync::new(db,TimeServer {status:200,user,running:false,paths:vec![]});
        sync.db.apply_event(&json!({"event":"time:changed","channel":"private-time-task-1","data":{}}).to_string()).unwrap();
        assert!(sync.refetch_pending().is_err());
        assert_eq!(sync.db.task_time(1).unwrap().unwrap(),before);
        assert_eq!(sync.db.refetches().unwrap().len(),1);
        if user == 99 { assert_eq!(sync.transport.paths,vec!["/user/context"]); }
    }
}
