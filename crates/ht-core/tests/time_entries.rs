use ht_core::{
    Db, Drain, TimeEntryChange, TimeWrite, Transport, Write,
    http::{Response, TransportError},
};
use serde_json::{Value, json};
fn entry() -> Value {
    json!({"id":5,"userId":42,"userName":"Me","note":"Before","startedAt":"2026-10-01T12:00:00Z","endedAt":"2026-10-01T13:00:00Z","pausedAt":null,"createdAt":"2026-10-02T09:00:00Z","seconds":3600})
}
fn seed(db: &mut Db) {
    db.kv_set("current-user-id", "42").unwrap();
    db.apply_response(&json!({"success":true,"projects":[{"id":15,"title":"Board"}],"tasks":[{"id":1,"title":"Task","projectId":15,"status":"Normal"}]})).unwrap();
    db.cache_task_time(42,1,json!({"enabled":true,"runningEntry":null,"activeRunningEntryCount":0,"taskTotalSeconds":3600}),100).unwrap();
    db.cache_time_entries(
        42,
        1,
        &json!({"success":true,"canManage":false,"entries":[entry()]}),
    )
    .unwrap();
}
struct Server {
    status: u16,
    method: &'static str,
    body: Value,
    reply: Value,
    calls: usize,
    user: i64,
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
        self.mutate("POST", path, body, key)
    }
    fn mutate(
        &mut self,
        method: &str,
        path: &str,
        body: &Value,
        key: &str,
    ) -> Result<Response, TransportError> {
        assert_eq!(method, self.method);
        assert_eq!(path, "/api/time/entries");
        assert_eq!(*body, self.body);
        assert!(!key.is_empty());
        self.calls += 1;
        Ok(Response {
            status: self.status,
            body: if self.status == 200 {
                self.reply.clone()
            } else {
                json!({"error":"Forbidden"})
            },
        })
    }
}
fn changes() -> Vec<(TimeEntryChange, &'static str, Value, Value)> {
    let created = json!({"id":8,"taskId":1,"userId":42,"note":"New","startedAt":"2026-10-01T14:00:00Z","endedAt":"2026-10-01T14:30:00Z"});
    let mut edited = entry();
    edited["note"] = json!("Edited");
    edited["endedAt"] = json!("2026-10-01T13:30:00Z");
    edited["seconds"] = json!(5400);
    let moved = json!({"id":5,"userId":42,"note":null,"startedAt":"2026-10-02T06:30:00Z","endedAt":"2026-10-02T07:15:00Z"});
    vec![
        (
            TimeEntryChange::Update {
                entry: 5,
                date: Some("2026-10-02".into()),
                minutes: 45,
                offset: -330,
                note: "  ".into(),
            },
            "PATCH",
            json!({"entryId":5,"date":"2026-10-02","minutes":45,"timezoneOffsetMinutes":-330,"note":"  "}),
            json!({"success":true,"entry":moved}),
        ),
        (
            TimeEntryChange::Create {
                date: "2026-10-01".into(),
                minutes: 30,
                offset: 120,
                note: "New".into(),
            },
            "POST",
            json!({"taskId":1,"date":"2026-10-01","minutes":30,"timezoneOffsetMinutes":120,"note":"New"}),
            json!({"success":true,"entry":created}),
        ),
        (
            TimeEntryChange::Update {
                entry: 5,
                date: None,
                minutes: 90,
                offset: 120,
                note: "Edited".into(),
            },
            "PATCH",
            json!({"entryId":5,"minutes":90,"note":"Edited"}),
            json!({"success":true,"entry":edited}),
        ),
        (
            TimeEntryChange::Delete { entry: 5 },
            "DELETE",
            json!({"entryId":5}),
            json!({"success":true}),
        ),
    ]
}
#[test]
fn time_log_crud_persists_optimistically_and_accepts_or_rolls_back_exact_web_requests() {
    for (change, method, body, reply) in changes() {
        for status in [200, 401, 403] {
            let file = tempfile::NamedTempFile::new().unwrap();
            let mut db = Db::open(file.path()).unwrap();
            seed(&mut db);
            let before = db.time_entries(1).unwrap().unwrap();
            let id = db
                .enqueue(
                    Write::Time {
                        change: TimeWrite::Entry {
                            task: 1,
                            change: change.clone(),
                        },
                    },
                    100,
                )
                .unwrap();
            let key = db.mutations().unwrap()[0].client_id.clone();
            let optimistic = db.time_entries(1).unwrap().unwrap();
            assert_ne!(before, optimistic);
            drop(db);
            let mut db = Db::open(file.path()).unwrap();
            assert_eq!(db.time_entries(1).unwrap().unwrap(), optimistic);
            let mut server = Server {
                status,
                method,
                body: body.clone(),
                reply: reply.clone(),
                calls: 0,
                user: 42,
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
            let result: Value =
                serde_json::from_str(&db.kv_get(&format!("time-result:{key}")).unwrap().unwrap())
                    .unwrap();
            if status != 200 {
                assert_eq!(result["error"], "Forbidden");
                assert_eq!(db.time_entries(1).unwrap().unwrap(), before);
            } else {
                assert!(result["error"].is_null());
                assert!(db.mutations().unwrap().is_empty());
                assert!(
                    !db.time_entries(1).unwrap().unwrap()["entries"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|row| row["pending"] == true)
                );
            }
        }
    }
}
#[test]
fn time_log_refuses_wrong_ack_uncertain_replay_other_users_and_invalid_durations() {
    for (change, method, body, _) in changes() {
        for user in [42, 99] {
            for status in [200, 500] {
                let mut db = Db::memory().unwrap();
                seed(&mut db);
                let before = db.time_entries(1).unwrap().unwrap();
                let id = db
                    .enqueue(
                        Write::Time {
                            change: TimeWrite::Entry {
                                task: 1,
                                change: change.clone(),
                            },
                        },
                        100,
                    )
                    .unwrap();
                let mut server = Server {
                    status,
                    method,
                    body: body.clone(),
                    reply: json!({"success":true,"entry":{"id":999}}),
                    calls: 0,
                    user,
                };
                let expected = if matches!(change, TimeEntryChange::Delete { .. })
                    && status == 200
                    && user == 42
                {
                    Drain::Applied(id)
                } else {
                    Drain::Failed(id)
                };
                assert_eq!(db.drain_once(&mut server, 101).unwrap(), expected);
                assert_eq!(server.calls, usize::from(user == 42));
                if !matches!(expected, Drain::Applied(_)) {
                    assert_eq!(db.time_entries(1).unwrap().unwrap(), before);
                }
                assert_eq!(db.drain_once(&mut server, 1000).unwrap(), Drain::Empty);
            }
        }
    }
    for minutes in [0, 1441] {
        assert!(
            TimeEntryChange::Create {
                date: "2026-10-01".into(),
                minutes,
                offset: 0,
                note: String::new()
            }
            .request(1)
            .is_err()
        );
    }
    for date in ["2026-02-30", "2026-2-1", ""] {
        assert!(
            TimeEntryChange::Create {
                date: date.into(),
                minutes: 1,
                offset: 0,
                note: String::new()
            }
            .request(1)
            .is_err()
        );
    }
    let mut db = Db::memory().unwrap();
    seed(&mut db);
    let mut other = entry();
    other["userId"] = json!(7);
    db.cache_time_entries(42, 1, &json!({"canManage":false,"entries":[other]}))
        .unwrap();
    assert!(
        db.enqueue(
            Write::Time {
                change: TimeWrite::Entry {
                    task: 1,
                    change: TimeEntryChange::Delete { entry: 5 }
                }
            },
            100
        )
        .is_err()
    );
    assert!(db.mutations().unwrap().is_empty());
}
#[test]
fn time_log_reads_validate_entries_and_preserve_server_authentication_failures() {
    struct Read(u16);
    impl Transport for Read {
        fn get(&mut self, path: &str) -> Result<Response, TransportError> {
            assert_eq!(path, "/api/time/entries?taskId=1");
            Ok(Response {
                status: self.0,
                body: if self.0 == 200 {
                    json!({"success":true,"canManage":false,"entries":[entry()]})
                } else {
                    json!({"error":"Unauthorized"})
                },
            })
        }
        fn post(&mut self, _: &str, _: &Value, _: &str) -> Result<Response, TransportError> {
            panic!("read only")
        }
    }
    assert!(ht_core::time_entries::load(&mut Read(200), 1).is_ok());
    for status in [401, 403] {
        assert_eq!(
            ht_core::time_entries::load(&mut Read(status), 1)
                .unwrap_err()
                .to_string(),
            "Unauthorized"
        );
    }
}
