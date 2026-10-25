use ht_core::{
    Db, Drain, TaskAction, Transport, Write,
    http::{Response, TransportError},
};
use serde_json::{Value, json};

fn seed(db: &mut Db) {
    db.kv_set("current-user-id", "42").unwrap();
    db.apply_response(&json!({"success":true,"projects":[{"id":15,"title":"Board"}],"sections":[
        {"id":10,"projectId":15,"section_title":"Triage","visibility":true,"deleted":false,"ranking":"a"},
        {"id":11,"projectId":15,"section_title":"Ready","visibility":true,"deleted":false,"ranking":"b"}
    ]})).unwrap();
    db.apply_task(&json!({"id":1,"title":"Task","projectId":15,"sectionId":10,"section":"Triage","ranking":"a","status":"Normal"}))
        .unwrap();
    db.kv_set(
        "members:15",
        &json!([{"id":42,"displayName":"Me"},{"id":7,"displayName":"Alex"}]).to_string(),
    )
    .unwrap();
    db.apply_task(&json!({"id":1,"title":"Task","projectId":15,"sectionId":10,"section":"Triage","ranking":"a","status":"Normal","relatedFromTasks":[{"id":77,"sourceTaskId":1,"targetTaskId":3,"relationType":"RelatedTo","targetTask":{"id":3,"projectId":15,"uniqueIndex":3,"title":"Existing"}}],"relatedToTasks":[]})).unwrap();
}
fn ack(change: &TaskAction) -> Value {
    match change {
        TaskAction::Relation { target, relation } => {
            json!([{"id":88,"sourceTaskId":1,"targetTaskId":target["id"],"targetTask":target,"relationType":relation}])
        }
        TaskAction::RemoveRelation(id) => {
            json!({"status":200,"json":{"id":id,"sourceTaskId":1,"targetTaskId":3}})
        }
        TaskAction::Accept {
            project,
            section,
            title,
        } => json!({"id":1,"projectId":project,"sectionId":section,"section":title,"ranking":"z"}),
        TaskAction::Decline => {
            json!({"id":1,"status":"Archive","dueDate":null,"archivedAt":"2026-10-01T09:00:00Z"})
        }
        TaskAction::SaveTemplate => json!({"success":true,"template":{"id":81,"name":"Task"}}),
        TaskAction::Recurrence(rule) => json!({"id":1,"recurrence":rule}),
        TaskAction::StartDate(date) => json!({"id":1,"startDate":date}),
        TaskAction::DueDate(date) => json!({"id":1,"dueDate":date}),
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
        self.mutate("POST", path, body, key)
    }
    fn mutate(
        &mut self,
        method: &str,
        path: &str,
        body: &Value,
        key: &str,
    ) -> Result<Response, TransportError> {
        assert_eq!(
            method,
            if matches!(self.change, TaskAction::Accept { .. }) {
                "PUT"
            } else {
                "POST"
            }
        );
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
fn target() -> Value {
    json!({"id":2,"projectId":15,"uniqueIndex":2,"title":"Other","ticketNumber":"HT-2"})
}
fn changes() -> Vec<TaskAction> {
    vec![
        TaskAction::Accept {
            project: 15,
            section: 11,
            title: "Ready".into(),
        },
        TaskAction::Decline,
        TaskAction::SaveTemplate,
        TaskAction::StartDate(Some("2026-10-01T09:00:00+02:00".into())),
        TaskAction::StartDate(None),
        TaskAction::DueDate(Some("2026-10-01T09:00:00+02:00".into())),
        TaskAction::DueDate(None),
        TaskAction::WaitingOn(Some(7)),
        TaskAction::WaitingOn(None),
        TaskAction::AssignMe,
        TaskAction::RemoveRelation(77),
        TaskAction::Relation {
            target: target(),
            relation: ht_core::RelationKind::RelatedTo,
        },
        TaskAction::Relation {
            target: target(),
            relation: ht_core::RelationKind::BlockedBy,
        },
        TaskAction::Relation {
            target: target(),
            relation: ht_core::RelationKind::BlockedTo,
        },
        TaskAction::Relation {
            target: target(),
            relation: ht_core::RelationKind::Duplicate,
        },
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
                TaskAction::StartDate(None) | TaskAction::DueDate(None) | TaskAction::WaitingOn(None)
            ) {
                db.apply_task(&json!({"id":1,"title":"Task","projectId":15,"sectionId":10,"section":"Triage","ranking":"a","status":"Normal","startDate":"2026-09-01T09:00:00Z","dueDate":"2026-10-03T09:00:00Z","waitingOnUserId":7,"waitingOnSetById":8,"waitingOnSetAt":"2026-09-01T09:00:00Z"})).unwrap();
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
                TaskAction::Accept { section, title, .. } => {
                    assert_eq!(optimistic["sectionId"], *section);
                    assert_eq!(optimistic["section"], *title);
                }
                TaskAction::Decline => {
                    assert_eq!(optimistic["status"], "Archive");
                    assert!(optimistic["dueDate"].is_null());
                }
                TaskAction::SaveTemplate => assert_eq!(optimistic, before),
                TaskAction::RemoveRelation(id) => assert!(
                    !optimistic["relatedFromTasks"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|row| row["id"] == *id)
                ),
                TaskAction::Relation { target, relation } => assert!(
                    optimistic["relatedFromTasks"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|row| row["targetTaskId"] == target["id"]
                            && row["relationType"] == json!(relation)
                            && row["pending"] == true)
                ),
                TaskAction::Recurrence(rule) => assert_eq!(optimistic["recurrence"], json!(rule)),
                TaskAction::StartDate(date) => assert_eq!(optimistic["startDate"], json!(date)),
                TaskAction::DueDate(date) => assert_eq!(optimistic["dueDate"], json!(date)),
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
                    "sectionId",
                    "section",
                    "ranking",
                    "status",
                    "archivedAt",
                    "dueDate",
                    "startDate",
                    "recurrence",
                    "relatedFromTasks",
                    "relatedToTasks",
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

#[test]
fn assign_to_me_preserves_people_and_owned_agents_as_distinct_assignees_on_accept_and_refusal() {
    for status in [200, 401, 403] {
        let mut db = Db::memory().unwrap();
        seed(&mut db);
        let agent = json!({"id":99,"userId":42,"agentId":"agent","user":{"id":42,"displayName":"Me"},"agent":{"id":"agent","displayName":"Bot"},"taskId":1});
        db.apply_task(&json!({"id":1,"title":"Task","projectId":15,"sectionId":10,"section":"Triage","ranking":"a","status":"Normal","assignees":[{"id":7,"displayName":"Alex"},agent],"assigneeCount":2})).unwrap();
        let change = TaskAction::AssignMe;
        let id = db
            .enqueue(
                Write::TaskAction {
                    task_id: 1,
                    change: change.clone(),
                },
                100,
            )
            .unwrap();
        assert_eq!(
            db.task(1).unwrap().unwrap().assignee_count,
            3,
            "owning an agent is not being assigned as a human"
        );
        let reply = json!({"body":[{"id":90,"taskId":1,"userId":7,"agentId":null,"user":{"id":7,"displayName":"Alex"}},agent,{"id":91,"taskId":1,"userId":42,"agentId":null,"user":{"id":42,"displayName":"Me"}}]});
        let mut server = Server {
            status,
            change,
            reply,
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
        let task = db.task(1).unwrap().unwrap();
        assert_eq!(task.assignee_count, if status == 200 { 3 } else { 2 });
        let rows = task.extra["assignees"].as_array().unwrap();
        assert!(rows.iter().any(|row| row["agent"]["id"] == "agent"));
        assert!(rows.iter().any(|row| row["id"] == 7));
    }
}

#[test]
fn empty_relation_acknowledgement_and_wrong_deleted_endpoints_are_refusals_not_success() {
    for (change, reply, message) in [
        (
            TaskAction::Relation {
                target: target(),
                relation: ht_core::RelationKind::RelatedTo,
            },
            json!([]),
            "Task relation already exists or cannot be added",
        ),
        (
            TaskAction::RemoveRelation(77),
            json!({"status":200,"json":{"id":77,"sourceTaskId":1,"targetTaskId":999}}),
            "Unable to remove task relation",
        ),
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
        let row = db.mutations().unwrap()[0].clone();
        assert_eq!(
            db.drain_once(
                &mut Server {
                    status: 200,
                    change,
                    reply,
                    user: 42,
                    calls: 0
                },
                101
            )
            .unwrap(),
            Drain::Failed(id)
        );
        let result: Value = serde_json::from_str(
            &db.kv_get(&format!("task-action-result:{}", row.client_id))
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        assert_eq!(result["error"], message);
        assert_eq!(
            db.task(1).unwrap().unwrap().extra["relatedFromTasks"][0]["id"],
            77
        );
    }
}

#[test]
fn accept_destination_matches_web_intake_skip_rank_order_hidden_columns_and_terminal_errors() {
    let mut db = Db::memory().unwrap();
    seed(&mut db);
    db.apply_response(&json!({"success":true,"sections":[
        {"id":11,"projectId":15,"section_title":"Intake / Inbox","visibility":true,"deleted":false,"ranking":"b"},
        {"id":12,"projectId":15,"section_title":"Ready","visibility":false,"deleted":false,"ranking":"c"},
        {"id":13,"projectId":15,"section_title":"Done","visibility":true,"deleted":false,"ranking":"d"}
    ]})).unwrap();
    assert_eq!(
        db.accept_task_action(1).unwrap(),
        TaskAction::Accept {
            project: 15,
            section: 12,
            title: "Ready".into()
        }
    );
    db.apply_task(&json!({"id":1,"projectId":15,"title":"Task","sectionId":12,"status":"Normal"}))
        .unwrap();
    assert_eq!(
        db.accept_task_action(1).unwrap(),
        TaskAction::Accept {
            project: 15,
            section: 13,
            title: "Done".into()
        }
    );
    db.apply_task(&json!({"id":1,"projectId":15,"title":"Task","sectionId":13,"status":"Normal"}))
        .unwrap();
    assert_eq!(
        db.accept_task_action(1).unwrap_err().to_string(),
        "No later column is available for this task"
    );
    assert!(
        db.enqueue(
            Write::TaskAction {
                task_id: 1,
                change: TaskAction::Accept {
                    project: 15,
                    section: 12,
                    title: "Ready".into()
                }
            },
            100
        )
        .is_err()
    );
    assert!(db.mutations().unwrap().is_empty());
}

#[test]
fn triage_and_template_commands_use_independent_exact_web_requests() {
    assert_eq!(
        TaskAction::Accept {
            project: 15,
            section: 11,
            title: "Ready".into()
        }
        .request(1, 42)
        .unwrap(),
        (
            "/api/tasks/moveTask",
            json!({"projectId":15,"taskId":1,"section_title":"Ready","sectionId":11})
        )
    );
    assert_eq!(
        TaskAction::Decline.request(1, 42).unwrap(),
        (
            "/api/tasks/(un)archive",
            json!({"taskId":1,"status":"Archive"})
        )
    );
    assert_eq!(
        TaskAction::SaveTemplate.request(1, 42).unwrap(),
        ("/api/task-templates", json!({"taskId":1}))
    );
}

#[test]
fn decline_duplicate_durably_chains_archive_after_relation_and_preserves_partial_success() {
    for (relation_status, archive_status) in
        [(401, 200), (403, 200), (200, 401), (200, 403), (200, 200)]
    {
        let file = tempfile::NamedTempFile::new().unwrap();
        let mut db = Db::open(file.path()).unwrap();
        seed(&mut db);
        let before = db.task(1).unwrap().unwrap();
        db.enqueue(
            Write::DeclineDuplicate {
                task_id: 1,
                target: target(),
            },
            100,
        )
        .unwrap();
        let first = db.mutations().unwrap().pop().unwrap();
        assert_eq!(
            db.task(1).unwrap().unwrap().status,
            ht_core::generated::Status::Normal
        );
        let relation = TaskAction::Relation {
            target: target(),
            relation: ht_core::RelationKind::Duplicate,
        };
        let mut server = Server {
            status: relation_status,
            reply: if relation_status == 200 {
                ack(&relation)
            } else {
                json!({"message":"Forbidden"})
            },
            change: relation,
            user: 42,
            calls: 0,
        };
        db.drain_once(&mut server, 101).unwrap();
        assert_eq!(server.calls, 1);
        if relation_status != 200 {
            assert_eq!(
                db.task(1).unwrap().unwrap().extra["relatedFromTasks"],
                before.extra["relatedFromTasks"]
            );
            assert!(
                db.mutations()
                    .unwrap()
                    .iter()
                    .all(|row| row.status == "failed")
            );
            continue;
        }
        let result: Value = serde_json::from_str(
            &db.kv_get(&format!("task-action-result:{}", first.client_id))
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        let next = db.mutations().unwrap().pop().unwrap();
        assert_eq!(result["next"], next.client_id);
        assert_eq!(next.payload_json["duplicate_archive"], true);
        assert_eq!(
            db.task(1).unwrap().unwrap().status,
            ht_core::generated::Status::Archive
        );
        drop(db);
        let mut db = Db::open(file.path()).unwrap();
        let mut server = Server {
            status: archive_status,
            reply: if archive_status == 200 {
                ack(&TaskAction::Decline)
            } else {
                json!({"message":"Forbidden"})
            },
            change: TaskAction::Decline,
            user: 42,
            calls: 0,
        };
        db.drain_once(&mut server, 102).unwrap();
        assert_eq!(server.calls, 1);
        let task = db.task(1).unwrap().unwrap();
        assert_eq!(task.extra["relatedFromTasks"][1]["id"], 88);
        assert_eq!(
            task.status,
            if archive_status == 200 {
                ht_core::generated::Status::Archive
            } else {
                ht_core::generated::Status::Normal
            }
        );
        let result: Value = serde_json::from_str(
            &db.kv_get(&format!("task-action-result:{}", next.client_id))
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        if archive_status != 200 {
            assert_eq!(
                result["error"],
                "Marked as duplicate, but could not archive the task"
            );
        } else {
            assert_eq!(result["success"], true);
        }
    }
}
