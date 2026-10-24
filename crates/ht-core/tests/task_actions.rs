use ht_core::{
    Db, Drain, TaskAction, Transport, Write,
    http::{Response, TransportError},
};
use serde_json::{Value, json};

fn seed(db: &mut Db) {
    db.kv_set("current-user-id", "42").unwrap();
    db.apply_task(&json!({"id":1,"title":"Task","projectId":15,"status":"Normal"}))
        .unwrap();
    db.kv_set(
        "members:15",
        &json!([{"id":42,"displayName":"Me"},{"id":7,"displayName":"Alex"}]).to_string(),
    )
    .unwrap();
}
fn ack(change: &TaskAction) -> Value {
    match change {
        TaskAction::Recurrence(rule) => json!({"id":1,"recurrence":rule}),
        TaskAction::StartDate(date) => json!({"id":1,"startDate":date}),
        TaskAction::WaitingOn(user) => {
            json!({"id":1,"waitingOnUserId":user,"waitingOnSetById":user.map(|_|42),"waitingOnSetAt":user.map(|_|"2026-10-01T09:00:00Z")})
        }
        TaskAction::AssignMe => {
            json!({"body":[{"id":99,"taskId":1,"userId":42,"agentId":null,"user":{"id":42,"displayName":"Me"}}],"assignStatus":"Assigned"})
        }
    }
}
struct Server {
    status: u16,
    change: TaskAction,
    reply: Value,
    user: i64,
    calls: usize,
}
impl Transport for Server {
    fn get(&mut self, path: &str) -> Result<Response, TransportError> {
        assert_eq!(path, "/user/context");
        Ok(Response {
            status: 200,
            body: json!({"success":true,"user":{"id":self.user}}),
        })
    }
    fn post(&mut self, path: &str, body: &Value, key: &str) -> Result<Response, TransportError> {
        let (expected_path, expected_body) = self.change.request(1, 42).unwrap();
        assert_eq!(path, expected_path);
        assert_eq!(*body, expected_body);
        assert!(!key.is_empty());
        self.calls += 1;
        Ok(Response {
            status: self.status,
            body: self.reply.clone(),
        })
    }
}
fn changes() -> Vec<TaskAction> {
    vec![
        TaskAction::StartDate(Some("2026-10-01T09:00:00+02:00".into())),
        TaskAction::StartDate(None),
        TaskAction::WaitingOn(Some(7)),
        TaskAction::WaitingOn(None),
        TaskAction::AssignMe,
        TaskAction::Recurrence(None),
        TaskAction::Recurrence(Some("Daily".into())),
        TaskAction::Recurrence(Some("Weekdays".into())),
        TaskAction::Recurrence(Some("Weekly".into())),
        TaskAction::Recurrence(Some("Biweekly".into())),
        TaskAction::Recurrence(Some("Monthly".into())),
    ]
}
#[test]
fn task_daily_commands_persist_optimistically_and_accept_or_rollback_exact_web_writes() {
    for change in changes() {
        for status in [200, 401, 403] {
            let file = tempfile::NamedTempFile::new().unwrap();
            let mut db = Db::open(file.path()).unwrap();
            seed(&mut db);
            if matches!(
                change,
                TaskAction::StartDate(None) | TaskAction::WaitingOn(None)
            ) {
                db.apply_task(&json!({"id":1,"title":"Task","projectId":15,"status":"Normal","startDate":"2026-09-01T09:00:00Z","waitingOnUserId":7,"waitingOnSetById":8,"waitingOnSetAt":"2026-09-01T09:00:00Z"})).unwrap();
            }
            let before = serde_json::to_value(db.task(1).unwrap().unwrap()).unwrap();
            let id = db
                .enqueue(
                    Write::TaskAction {
                        task_id: 1,
                        change: change.clone(),
                    },
                    100,
                )
                .unwrap();
            let row = db.mutations().unwrap()[0].clone();
            let optimistic = serde_json::to_value(db.task(1).unwrap().unwrap()).unwrap();
            match &change {
                TaskAction::Recurrence(rule) => assert_eq!(optimistic["recurrence"], json!(rule)),
                TaskAction::StartDate(date) => assert_eq!(optimistic["startDate"], json!(date)),
                TaskAction::WaitingOn(user) => {
                    assert_eq!(optimistic["waitingOnUserId"], json!(user))
                }
                TaskAction::AssignMe => assert_eq!(optimistic["assigneeCount"], 1),
            }
            assert!(
                db.enqueue(
                    Write::TaskAction {
                        task_id: 1,
                        change: change.clone()
                    },
                    100
                )
                .is_err()
            );
            drop(db);
            let mut db = Db::open(file.path()).unwrap();
            assert_eq!(
                serde_json::to_value(db.task(1).unwrap().unwrap()).unwrap(),
                optimistic
            );
            let mut server = Server {
                status,
                change: change.clone(),
                reply: ack(&change),
                user: 42,
                calls: 0,
            };
            assert_eq!(
                db.drain_once(&mut server, 101).unwrap(),
                if status == 200 {
                    Drain::Applied(id)
                } else {
                    Drain::Failed(id)
                }
            );
            assert_eq!(server.calls, 1);
            let result: Value = serde_json::from_str(
                &db.kv_get(&format!("task-action-result:{}", row.client_id))
                    .unwrap()
                    .unwrap(),
            )
            .unwrap();
            if status == 200 {
                assert_eq!(result["success"], true);
                assert!(db.mutations().unwrap().is_empty());
            } else {
                assert_eq!(result["error"], change.failure());
                let after = serde_json::to_value(db.task(1).unwrap().unwrap()).unwrap();
                for field in [
                    "startDate",
                    "recurrence",
                    "waitingOnUserId",
                    "waitingOnSetById",
                    "waitingOnSetAt",
                    "assignees",
                    "assigneeCount",
                ] {
                    assert_eq!(after[field], before[field], "{change:?} {status} {field}");
                }
            }
        }
    }
}
#[test]
fn malformed_or_uncertain_task_writes_never_claim_success_or_replay_activities() {
    for change in changes() {
        for (status, reply) in [
            (200, json!({})),
            (200, json!({"id":999,"startDate":null})),
            (200, json!({"success":false})),
            (500, ack(&change)),
        ] {
            let mut db = Db::memory().unwrap();
            seed(&mut db);
            let id = db
                .enqueue(
                    Write::TaskAction {
                        task_id: 1,
                        change: change.clone(),
                    },
                    100,
                )
                .unwrap();
            let mut server = Server {
                status,
                change: change.clone(),
                reply,
                user: 42,
                calls: 0,
            };
            assert_eq!(db.drain_once(&mut server, 101).unwrap(), Drain::Failed(id));
            assert_eq!(db.drain_once(&mut server, 1000).unwrap(), Drain::Empty);
            assert_eq!(server.calls, 1);
            assert_eq!(db.task(1).unwrap().unwrap().assignee_count, 0);
        }
    }
}
#[test]
fn task_commands_reject_bad_dates_nonmembers_and_cross_account_replays_without_sending() {
    let mut db = Db::memory().unwrap();
    seed(&mut db);
    for change in [
        TaskAction::Recurrence(Some("Yearly".into())),
        TaskAction::StartDate(Some("invalid".into())),
        TaskAction::WaitingOn(Some(-1)),
        TaskAction::WaitingOn(Some(999)),
    ] {
        assert!(
            db.enqueue(Write::TaskAction { task_id: 1, change }, 100)
                .is_err()
        );
    }
    assert!(db.mutations().unwrap().is_empty());
    for switch_local in [false, true] {
        let mut db = Db::memory().unwrap();
        seed(&mut db);
        let change = TaskAction::WaitingOn(Some(7));
        let id = db
            .enqueue(
                Write::TaskAction {
                    task_id: 1,
                    change: change.clone(),
                },
                100,
            )
            .unwrap();
        if switch_local {
            db.kv_set("current-user-id", "99").unwrap();
        }
        let mut server = Server {
            status: 200,
            reply: ack(&change),
            change,
            user: if switch_local { 42 } else { 99 },
            calls: 0,
        };
        assert_eq!(db.drain_once(&mut server, 101).unwrap(), Drain::Failed(id));
        assert_eq!(server.calls, 0);
    }
}
#[test]
fn refused_assignment_restores_people_but_preserves_later_title() {
    let mut db = Db::memory().unwrap();
    seed(&mut db);
    let change = TaskAction::AssignMe;
    db.enqueue(
        Write::TaskAction {
            task_id: 1,
            change: change.clone(),
        },
        100,
    )
    .unwrap();
    db.enqueue(
        Write::TaskTitle {
            task_id: 1,
            title: "Later title".into(),
        },
        100,
    )
    .unwrap();
    let mut server = Server {
        status: 403,
        reply: json!({"message":"Forbidden"}),
        change,
        user: 42,
        calls: 0,
    };
    db.drain_once(&mut server, 101).unwrap();
    assert_eq!(db.task(1).unwrap().unwrap().title, "Later title");
    assert_eq!(db.task(1).unwrap().unwrap().assignee_count, 0);
}
