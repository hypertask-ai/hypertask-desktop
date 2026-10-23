use ht_core::{
    Db, Sync, TaskPatch, Transport, Write,
    http::{Response, TransportError},
};
use serde_json::{Value, json};

struct Catalog;
impl Transport for Catalog {
    fn get(&mut self, path: &str) -> Result<Response, TransportError> {
        let body = match path {
            "/projects/15/members" => {
                json!({"success":true,"projectId":15,"members":[{"id":7,"displayName":"Alex"}]})
            }
            "/projects/15/labels" => {
                json!({"success":true,"projectId":15,"labels":[{"id":"label-1","name":"Bug"}]})
            }
            _ => panic!("unexpected GET {path}"),
        };
        Ok(Response { status: 200, body })
    }
    fn post(&mut self, _: &str, _: &Value, _: &str) -> Result<Response, TransportError> {
        panic!("unexpected POST")
    }
}
#[test]
fn catalogs_back_durable_label_and_assignment_writes_and_cached_search() {
    let mut sync = Sync::new(Db::memory().unwrap(), Catalog);
    sync.catalog(15).unwrap();
    sync.db.apply_task(&json!({"id":1,"projectId":15,"title":"Fix comment sending","description":"Keyboard focus", "ticketNumber":"HT-1", "status":"Normal"})).unwrap();
    sync.db
        .enqueue(
            Write::TaskPatch {
                task_id: 1,
                patch: TaskPatch::Labels(vec!["label-1".into()]),
            },
            100,
        )
        .unwrap();
    sync.db
        .enqueue(
            Write::TaskPatch {
                task_id: 1,
                patch: TaskPatch::Assignees(vec![7]),
            },
            101,
        )
        .unwrap();
    let task = sync.db.task(1).unwrap().unwrap();
    assert_eq!(task.label_count, 1);
    assert_eq!(task.assignee_count, 1);
    assert_eq!(task.extra["labels"][0]["name"], "Bug");
    assert_eq!(task.extra["assignees"][0]["displayName"], "Alex");
    let writes = sync.db.mutations().unwrap();
    assert_eq!(
        writes[0].payload_json,
        json!({"task_id":1,"labels":["label-1"]})
    );
    assert_eq!(writes[1].payload_json, json!({"task_id":1,"assignee":[7]}));
    assert_eq!(
        sync.db.search_tasks("FOCUS HT-1", Some(15)).unwrap().len(),
        1
    );
    assert!(
        sync.db
            .search_tasks("comment", Some(16))
            .unwrap()
            .is_empty()
    );
    assert!(sync.db.search_tasks("unmatched", None).unwrap().is_empty());
    sync.db
        .enqueue(
            Write::TaskPatch {
                task_id: 1,
                patch: TaskPatch::Labels(vec![]),
            },
            102,
        )
        .unwrap();
    sync.db
        .enqueue(
            Write::TaskPatch {
                task_id: 1,
                patch: TaskPatch::Assignees(vec![]),
            },
            103,
        )
        .unwrap();
    assert_eq!(sync.db.task(1).unwrap().unwrap().label_count, 0);
    assert_eq!(sync.db.task(1).unwrap().unwrap().assignee_count, 0);
    let count = sync.db.mutations().unwrap().len();
    assert!(
        sync.db
            .enqueue(
                Write::TaskPatch {
                    task_id: 1,
                    patch: TaskPatch::Labels(vec!["other-board-label".into()])
                },
                104
            )
            .is_err()
    );
    assert!(
        sync.db
            .enqueue(
                Write::TaskPatch {
                    task_id: 1,
                    patch: TaskPatch::Assignees(vec![8])
                },
                104
            )
            .is_err()
    );
    assert_eq!(sync.db.mutations().unwrap().len(), count);
}
