use ht_core::{
    BoardWrite, Db, Drain, Transport, Write,
    http::{Response, TransportError},
};
use serde_json::{Value, json};

fn seed(db: &mut Db) {
    db.kv_set("current-user-id", "42").unwrap();
    db.apply_response(&json!({"success":true,"projects":[{"id":15,"title":"Board"}],"sections":[
        {"id":10,"projectId":15,"section_title":"Todo","visibility":true,"deleted":false,"ranking":"a"},
        {"id":11,"projectId":15,"section_title":"Doing","visibility":true,"deleted":false,"ranking":"b"}
    ],"tasks":[{"id":1,"projectId":15,"sectionId":10,"title":"Task","status":"Normal","labels":[]}]})).unwrap();
}
struct Server {
    method: &'static str,
    path: &'static str,
    expected: Value,
    reply: Value,
    status: u16,
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
        assert_eq!(method, self.method);
        assert_eq!(path, self.path);
        assert_eq!(body, &self.expected);
        assert!(!key.is_empty());
        self.calls += 1;
        Ok(Response {
            status: self.status,
            body: self.reply.clone(),
        })
    }
}
fn server(method: &'static str, path: &'static str, expected: Value, reply: Value) -> Server {
    Server {
        method,
        path,
        expected,
        reply,
        status: 200,
        user: 42,
        calls: 0,
    }
}
#[test]
fn board_column_and_rename_writes_survive_restart_and_use_exact_bearer_contracts() {
    for (change, mut server) in [
        (
            BoardWrite::AddColumn {
                project: 15,
                title: "Review".into(),
            },
            server(
                "POST",
                "/projects/15/sections",
                json!({"title":"Review"}),
                json!({"success":true,"section":{"id":12,"projectId":15,"section_title":"Review","visibility":true,"deleted":false,"ranking":"c"}}),
            ),
        ),
        (
            BoardWrite::RenameColumn {
                project: 15,
                section: 10,
                title: "Backlog".into(),
            },
            server(
                "PATCH",
                "/projects/15/sections/10",
                json!({"title":"Backlog"}),
                json!({"success":true,"section":{"id":10,"projectId":15,"section_title":"Backlog","visibility":true,"deleted":false,"ranking":"a"}}),
            ),
        ),
        (
            BoardWrite::DeleteColumn {
                project: 15,
                section: 10,
            },
            server(
                "DELETE",
                "/projects/15/sections/10",
                Value::Null,
                json!({"success":true,"message":"Section deleted successfully"}),
            ),
        ),
        (
            BoardWrite::RenameBoard {
                project: 15,
                title: "Renamed".into(),
            },
            server(
                "PATCH",
                "/projects/15",
                json!({"title":"Renamed"}),
                json!({"success":true,"project":{"id":15,"title":"Renamed","name":"renamed"}}),
            ),
        ),
    ] {
        let file = tempfile::NamedTempFile::new().unwrap();
        let mut db = Db::open(file.path()).unwrap();
        seed(&mut db);
        let id = db.enqueue(Write::Board { change }, 100).unwrap();
        let key = db.mutations().unwrap()[0].client_id.clone();
        assert_eq!(db.board(15).unwrap()[0].title, "Todo");
        assert!(db.take_board_result(&key).unwrap().is_none());
        drop(db);
        let mut db = Db::open(file.path()).unwrap();
        assert_eq!(db.drain_once(&mut server, 101).unwrap(), Drain::Applied(id));
        assert_eq!(server.calls, 1);
        assert_eq!(
            db.take_board_result(&key).unwrap().unwrap()["value"]["success"],
            true
        );
        assert!(db.take_board_result(&key).unwrap().is_none());
        assert!(db.mutations().unwrap().is_empty());
        assert!(
            db.task(1).unwrap().is_some(),
            "column deletion must not invent task deletion"
        );
    }
}
#[test]
fn label_creation_acknowledgement_queues_assignment_only_with_real_server_id() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let mut db = Db::open(file.path()).unwrap();
    seed(&mut db);
    let id = db
        .enqueue(
            Write::Board {
                change: BoardWrite::CreateTaskLabel {
                    project: 15,
                    task: 1,
                    name: "New".into(),
                },
            },
            100,
        )
        .unwrap();
    assert!(
        db.task(1).unwrap().unwrap().extra["labels"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let mut create = server(
        "POST",
        "/projects/15/labels",
        json!({"name":"New"}),
        json!({"success":true,"label":{"id":"real-label","name":"New"}}),
    );
    assert_eq!(db.drain_once(&mut create, 101).unwrap(), Drain::Applied(id));
    drop(db);
    let mut db = Db::open(file.path()).unwrap();
    let rows = db.mutations().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0].payload_json,
        json!({"task_id":1,"labels":["real-label"]})
    );
    let mut assign = server(
        "POST",
        "/tasks/update",
        rows[0].payload_json.clone(),
        json!({"success":true,"task":{"id":1,"projectId":15,"title":"Task","status":"Normal","labels":[{"id":"real-label","name":"New"}]}}),
    );
    assert!(matches!(
        db.drain_once(&mut assign, 102).unwrap(),
        Drain::Applied(_)
    ));
    assert_eq!(
        db.task(1).unwrap().unwrap().extra["labels"][0]["id"],
        "real-label"
    );
    assert!(db.mutations().unwrap().is_empty());
}
#[test]
fn board_denial_account_change_and_malformed_ack_never_pretend_success() {
    for (user, status, reply) in [
        (99, 200, json!({"success":true})),
        (42, 403, json!({"success":false})),
        (
            42,
            200,
            json!({"success":true,"section":{"id":10,"projectId":99}}),
        ),
    ] {
        let mut db = Db::memory().unwrap();
        seed(&mut db);
        let id = db
            .enqueue(
                Write::Board {
                    change: BoardWrite::RenameColumn {
                        project: 15,
                        section: 10,
                        title: "Changed".into(),
                    },
                },
                100,
            )
            .unwrap();
        let mut s = server(
            "PATCH",
            "/projects/15/sections/10",
            json!({"title":"Changed"}),
            reply,
        );
        s.user = user;
        s.status = status;
        let result = db.drain_once(&mut s, 101).unwrap();
        assert!(matches!(result, Drain::Failed(_) | Drain::Retrying(_)));
        assert_eq!(db.board(15).unwrap()[0].title, "Todo");
        assert_eq!(db.mutations().unwrap()[0].id, id);
        if user == 99 {
            assert_eq!(s.calls, 0);
        }
    }
}
#[test]
fn move_to_board_uses_selected_destination_and_authoritative_task_without_guessing() {
    let mut db = Db::memory().unwrap();
    seed(&mut db);
    let id = db
        .enqueue(
            Write::Board {
                change: BoardWrite::MoveTask {
                    task: 1,
                    project: 16,
                    section: 20,
                },
            },
            100,
        )
        .unwrap();
    let mut s = server(
        "POST",
        "/tasks/move",
        json!({"task_id":1,"target_project_id":16,"target_section_id":20}),
        json!({"success":true,"task":{"id":1,"projectId":16,"sectionId":20,"title":"Task","status":"Normal"}}),
    );
    assert_eq!(db.task(1).unwrap().unwrap().project_id, 15);
    assert_eq!(db.drain_once(&mut s, 101).unwrap(), Drain::Applied(id));
    assert_eq!(db.task(1).unwrap().unwrap().project_id, 16);
    assert!(db.board(15).unwrap()[0].cards.is_empty());
}

#[test]
fn people_and_agent_assignments_preserve_each_other_and_denial_rolls_back() {
    let mut db = Db::memory().unwrap();
    seed(&mut db);
    db.kv_set(
        "members:15",
        &json!([
            {"id":7,"displayName":"Alex"},
            {"id":"agent-a","displayName":"Runner","owner":{"id":42,"displayName":"Owner"}}
        ])
        .to_string(),
    )
    .unwrap();
    let id = db
        .enqueue(
            Write::Board {
                change: BoardWrite::AssignAgent {
                    task: 1,
                    agent: "agent-a".into(),
                    active: true,
                },
            },
            100,
        )
        .unwrap();
    assert_eq!(
        db.task(1).unwrap().unwrap().extra["assignees"][0]["agent"]["id"],
        "agent-a"
    );
    db.enqueue(
        Write::TaskPatch {
            task_id: 1,
            patch: ht_core::TaskPatch::Assignees(vec![7]),
        },
        100,
    )
    .unwrap();
    assert_eq!(db.task(1).unwrap().unwrap().assignee_count, 2);
    let mut s = server(
        "POST",
        "/assignees/assign",
        json!({"task_id":1,"agent_id":"agent-a","intent":"assign"}),
        json!({"success":true,"assignees":[{"userId":42,"agent":{"id":"agent-a","displayName":"Runner"}}],"assignStatus":"Assigned"}),
    );
    assert_eq!(db.drain_once(&mut s, 101).unwrap(), Drain::Applied(id));
    assert_eq!(
        db.task(1).unwrap().unwrap().assignee_count,
        2,
        "later queued people change must be reapplied over agent acknowledgement"
    );
    let mut people = server(
        "POST",
        "/tasks/update",
        json!({"task_id":1,"assignee":[7]}),
        json!({"success":true,"task":{"id":1,"projectId":15,"title":"Task","status":"Normal","assignees":[{"id":7,"displayName":"Alex"},{"id":42,"displayName":"Runner","agent":{"id":"agent-a","displayName":"Runner"}}]}}),
    );
    db.drain_once(&mut people, 102).unwrap();
    db.enqueue(
        Write::Board {
            change: BoardWrite::AssignAgent {
                task: 1,
                agent: "agent-a".into(),
                active: false,
            },
        },
        103,
    )
    .unwrap();
    assert_eq!(db.task(1).unwrap().unwrap().assignee_count, 1);
    let mut deny = server(
        "POST",
        "/assignees/assign",
        json!({"task_id":1,"agent_id":"agent-a","intent":"unassign"}),
        json!({"success":false}),
    );
    deny.status = 403;
    assert!(matches!(
        db.drain_once(&mut deny, 104).unwrap(),
        Drain::Failed(_)
    ));
    assert_eq!(db.task(1).unwrap().unwrap().assignee_count, 2);
}

#[test]
fn archive_board_uses_existing_bearer_archive_contract_and_real_status_ack() {
    let mut db = Db::memory().unwrap();
    seed(&mut db);
    let id = db
        .enqueue(
            Write::Board {
                change: BoardWrite::ArchiveBoard { project: 15 },
            },
            100,
        )
        .unwrap();
    let mut s = server(
        "POST",
        "/projects/archive",
        json!({"project_id":15,"status":"Archive"}),
        json!({"success":true,"project":{"id":15,"title":"Board","status":"Archive"}}),
    );
    assert_eq!(db.drain_once(&mut s, 101).unwrap(), Drain::Applied(id));
    assert_eq!(db.projects().unwrap()[0]["status"], "Archive");
}

#[test]
fn self_assignment_uses_assign_intent_survives_restart_and_keeps_concurrent_people() {
    let file = std::env::temp_dir().join(format!("ht-self-assign-{}.db", uuid::Uuid::new_v4()));
    let mut db = Db::open(&file).unwrap();
    seed(&mut db);
    db.kv_set("members:15", &json!([
        {"id":7,"displayName":"Alex"},{"id":42,"displayName":"Me"}
    ]).to_string()).unwrap();
    db.apply_response(&json!({"success":true,"tasks":[{
        "id":1,"projectId":15,"sectionId":10,"title":"Task","status":"Normal",
        "assignees":[{"id":7,"displayName":"Alex"},{"id":42,"agent":{"id":"agent-a","displayName":"Runner"}}],"assigneeCount":2
    }]})).unwrap();
    let id = db.enqueue(Write::Board { change: BoardWrite::AssignToMe { task:1, user:42 } }, 100).unwrap();
    assert_eq!(db.task(1).unwrap().unwrap().assignee_count, 3);
    drop(db);
    let mut db = Db::open(&file).unwrap();
    let mut s = server(
        "POST", "/assignees/assign",
        json!({"task_id":1,"user_id":42,"intent":"assign"}),
        json!({"success":true,"assignees":[
            {"userId":7},{"userId":42},{"userId":8},
            {"userId":42,"agent":{"id":"agent-a","displayName":"Runner"}}
        ]}),
    );
    assert_eq!(db.drain_once(&mut s, 101).unwrap(), Drain::Applied(id));
    let task = db.task(1).unwrap().unwrap();
    assert_eq!(task.assignee_count, 4);
    let rows = task.extra["assignees"].as_array().unwrap();
    assert!(rows.iter().any(|row| row["id"] == 8 && row["agent"].is_null()));
    assert!(rows.iter().any(|row| row["id"] == 42 && row["agent"].is_null()));
    assert!(rows.iter().any(|row| row["agent"]["id"] == "agent-a"));
    assert_eq!(db.drain_once(&mut s, 102).unwrap(), Drain::Empty);
    assert_eq!(s.calls, 1);
    drop(db);
    std::fs::remove_file(file).unwrap();
}

#[test]
fn self_assignment_denial_restores_people_and_rejects_another_account() {
    let mut db = Db::memory().unwrap();
    seed(&mut db);
    db.kv_set("members:15", &json!([
        {"id":7,"displayName":"Alex"},{"id":42,"displayName":"Me"}
    ]).to_string()).unwrap();
    assert!(db.enqueue(Write::Board { change: BoardWrite::AssignToMe { task:1, user:7 } }, 100).is_err());
    assert!(db.mutations().unwrap().is_empty());
    let id = db.enqueue(Write::Board { change: BoardWrite::AssignToMe { task:1, user:42 } }, 100).unwrap();
    assert_eq!(db.task(1).unwrap().unwrap().assignee_count, 1);
    let mut deny = server("POST", "/assignees/assign", json!({"task_id":1,"user_id":42,"intent":"assign"}), json!({"success":false}));
    deny.status = 403;
    assert_eq!(db.drain_once(&mut deny, 101).unwrap(), Drain::Failed(id));
    let task = db.task(1).unwrap().unwrap();
    assert_eq!(task.assignee_count, 0);
    assert_eq!(task.extra["assignees"], json!([]));
}

#[test]
fn command_move_to_inbox_is_desired_state_and_account_bound() {
    for user in [42, 43] {
        let mut db = Db::memory().unwrap();
        seed(&mut db);
        let mut server = server("POST", "/tasks/card-actions", json!({"task_id":1,"project_id":15,"action":"inbox","active":true}), json!({"success":true,"action":"inbox","active":true}));
        server.user = user;
        db.enqueue(Write::Board { change: BoardWrite::MoveToInbox { task:1, project:15 } }, 1).unwrap();
        let result = db.drain_once(&mut server, 2).unwrap();
        if user == 42 {
            assert!(matches!(result, Drain::Applied(_)));
            assert_eq!(server.calls, 1);
            assert!(db.refetches().unwrap().iter().any(|(target, _, _)| target == "inbox"));
        } else {
            assert!(matches!(result, Drain::Failed(_)));
            assert_eq!(server.calls, 0);
        }
    }
    let mut db = Db::memory().unwrap();
    seed(&mut db);
    assert!(db.enqueue(Write::Board { change: BoardWrite::MoveToInbox { task:1, project:99 } }, 1).is_err());
    let mut server = server("POST", "/tasks/card-actions", json!({"task_id":1,"project_id":15,"action":"inbox","active":true}), json!({"success":true,"action":"inbox","active":false}));
    db.enqueue(Write::Board { change: BoardWrite::MoveToInbox { task:1, project:15 } }, 1).unwrap();
    assert!(matches!(db.drain_once(&mut server, 2).unwrap(), Drain::Retrying(_)));
    assert!(db.refetches().unwrap().iter().all(|(target, _, _)| target != "inbox"));
}

#[test]
fn projected_subtask_removal_survives_restart_and_updates_only_acknowledged_child() {
    for cached in [false, true] {
        let file = tempfile::NamedTempFile::new().unwrap();
        let mut db = Db::open(file.path()).unwrap();
        seed(&mut db);
        db.apply_task(&json!({"id":1,"projectId":15,"title":"Task","status":"Normal","sub_tasks":[
            {"id":98,"title":"First","ticketNumber":"HT-98"},
            {"id":99,"title":"Second","ticketNumber":"HT-99"}
        ]})).unwrap();
        if cached {
            db.apply_task(&json!({"id":99,"projectId":15,"title":"Second","status":"Normal","parent_id":1,"parent_task":{"id":1}})).unwrap();
        }
        let id = db.enqueue(Write::Board { change: BoardWrite::RemoveSubtask { task:99, parent:1 } }, 100).unwrap();
        let key = db.mutations().unwrap()[0].client_id.clone();
        assert_eq!(db.task(1).unwrap().unwrap().extra["sub_tasks"].as_array().unwrap().len(), 2);
        assert_eq!(db.task(99).unwrap().is_some(), cached);
        drop(db);
        let mut db = Db::open(file.path()).unwrap();
        let mut server = server("POST", "/tasks/update", json!({"task_id":99,"parent_task_id":null}),
            json!({"success":true,"tasks":[{"id":99,"projectId":15,"title":"Second","status":"Normal"}]}));
        assert_eq!(db.drain_once(&mut server, 101).unwrap(), Drain::Applied(id));
        assert_eq!(server.calls, 1);
        assert_eq!(db.take_board_result(&key).unwrap().unwrap()["value"]["success"], true);
        assert!(db.mutations().unwrap().is_empty());
        assert!(db.refetches().unwrap().iter().any(|row| row.0 == "task" && row.1 == 1));
        assert!(db.refetches().unwrap().iter().any(|row| row.0 == "task" && row.1 == 99));
        drop(db);
        let db = Db::open(file.path()).unwrap();
        assert_eq!(db.task(1).unwrap().unwrap().extra["sub_tasks"], json!([{"id":98,"title":"First","ticketNumber":"HT-98"}]));
        let child = db.task(99).unwrap().unwrap();
        assert!(child.extra["parentTaskId"].is_null());
        assert!(child.extra["parent_id"].is_null());
        assert!(child.extra["parent_task"].is_null());
    }
}

#[test]
fn projected_subtask_removal_rejects_foreign_children_and_changed_account() {
    let mut db = Db::memory().unwrap();
    seed(&mut db);
    db.apply_task(&json!({"id":1,"projectId":15,"title":"Task","status":"Normal","sub_tasks":[{"id":99,"title":"Child"}]})).unwrap();
    for (task, parent) in [(0,1), (99,0), (1,1), (98,1), (99,2)] {
        assert!(db.enqueue(Write::Board { change: BoardWrite::RemoveSubtask { task, parent } }, 100).is_err());
        assert!(db.mutations().unwrap().is_empty());
    }
    db.apply_task(&json!({"id":99,"projectId":15,"title":"Child","status":"Normal","parent_id":2})).unwrap();
    assert!(db.enqueue(Write::Board { change: BoardWrite::RemoveSubtask { task:99, parent:1 } }, 100).is_err());
    db.apply_task(&json!({"id":99,"projectId":15,"title":"Child","status":"Normal","parent_id":1})).unwrap();
    let id = db.enqueue(Write::Board { change: BoardWrite::RemoveSubtask { task:99, parent:1 } }, 100).unwrap();
    let mut server = server("POST", "/tasks/update", Value::Null, Value::Null);
    server.user = 43;
    assert_eq!(db.drain_once(&mut server, 101).unwrap(), Drain::Failed(id));
    assert_eq!(server.calls, 0);
    assert_eq!(db.task(99).unwrap().unwrap().extra["parentTaskId"], 1);
    assert_eq!(db.task(1).unwrap().unwrap().extra["sub_tasks"].as_array().unwrap().len(), 1);
}

#[test]
fn projected_subtask_removal_retains_parent_on_refusal_or_wrong_acknowledgement() {
    for (status, reply) in [
        (403, json!({"success":false,"error":"Forbidden"})),
        (200, json!({"success":true,"tasks":[{"id":98,"projectId":15,"title":"Wrong","status":"Normal"}]})),
        (200, json!({"success":true,"tasks":[{"id":99,"projectId":15,"title":"Child","status":"Normal","parent_id":1}]})),
        (200, json!({"success":true,"tasks":[{"id":99,"projectId":15,"title":"Child","status":"Normal","parent_task":{"id":1}}]})),
    ] {
        let mut db = Db::memory().unwrap();
        seed(&mut db);
        db.apply_task(&json!({"id":1,"projectId":15,"title":"Task","status":"Normal","sub_tasks":[{"id":99,"title":"Child"}]})).unwrap();
        let id = db.enqueue(Write::Board { change: BoardWrite::RemoveSubtask { task:99, parent:1 } }, 100).unwrap();
        let mut server = server("POST", "/tasks/update", json!({"task_id":99,"parent_task_id":null}), reply);
        server.status = status;
        assert_eq!(db.drain_once(&mut server, 101).unwrap(), if status == 403 { Drain::Failed(id) } else { Drain::Retrying(id) });
        assert_eq!(db.task(1).unwrap().unwrap().extra["sub_tasks"].as_array().unwrap().len(), 1);
        assert!(db.task(99).unwrap().is_none());
    }
}
