use ht_core::{Db, Drain, TaskPatch, Transport, Write, generated::Status, http::{Response, TransportError}};
use serde_json::{Value, json};

fn db() -> Db {
    let mut db = Db::memory().unwrap();
    db.apply_task(&json!({"id":1,"title":"Initial","projectId":15,"status":"Normal"})).unwrap();
    db
}
struct Server { expected: Value }
impl Transport for Server {
    fn get(&mut self, _: &str) -> Result<Response, TransportError> { panic!("unexpected GET") }
    fn post(&mut self, path: &str, body: &Value, key: &str) -> Result<Response, TransportError> {
        assert_eq!(path, "/tasks/update");
        assert_eq!(body, &self.expected);
        assert!(!key.is_empty());
        Ok(Response { status: 200, body: json!({"success":true,"tasks":[{"id":1,"title":"Server","projectId":15,"status":"Normal"}]}) })
    }
}
#[test]
fn typed_patches_are_optimistic_durable_and_use_authenticated_task_update() {
    for (patch, field, expected, request) in [
        (TaskPatch::Status(Status::Archive), "status", json!("Archive"), json!({"task_id":1,"status":"Archive"})),
        (TaskPatch::Status(Status::Deleted), "status", json!("Deleted"), json!({"task_id":1,"status":"Deleted"})),
        (TaskPatch::Description("<p>Description</p>".into()), "description", json!("<p>Description</p>"), json!({"task_id":1,"description":"<p>Description</p>","content_type":"html"})),
        (TaskPatch::Priority(2), "priority", json!({"priority_index":2,"Priority_Value":"High"}), json!({"task_id":1,"priority":2})),
        (TaskPatch::Estimate(3), "estimate", json!({"estimate_index":3,"estimate_value":"S"}), json!({"task_id":1,"estimate":3})),
        (TaskPatch::DueDate(Some("2026-10-24T00:00:00Z".into())), "dueDate", json!("2026-10-24T00:00:00Z"), json!({"task_id":1,"due_date":"2026-10-24T00:00:00Z"})),
        (TaskPatch::DueDate(None), "dueDate", Value::Null, json!({"task_id":1,"due_date":null})),
    ] {
        let file = tempfile::NamedTempFile::new().unwrap();
        let mut db = Db::open(file.path()).unwrap();
        db.apply_task(&json!({"id":1,"title":"Initial","projectId":15,"status":"Normal"})).unwrap();
        let id = db.enqueue(Write::TaskPatch { task_id:1, patch }, 100).unwrap();
        assert_eq!(db.raw("Task", &json!(1)).unwrap().unwrap()[field], expected);
        drop(db);
        let mut db = Db::open(file.path()).unwrap();
        assert_eq!(db.mutations().unwrap()[0].payload_json, request);
        assert_eq!(db.raw("Task", &json!(1)).unwrap().unwrap()[field], expected);
        assert_eq!(db.drain_once(&mut Server { expected:request }, 101).unwrap(), Drain::Applied(id));
        assert!(db.mutations().unwrap().is_empty());
        assert_eq!(db.task(1).unwrap().unwrap().title, "Server");
    }
}
#[test]
fn invalid_patch_never_enters_queue() {
    let mut db = db();
    for patch in [TaskPatch::Priority(5), TaskPatch::Estimate(1), TaskPatch::DueDate(Some("not a date".into()))] {
        assert!(db.enqueue(Write::TaskPatch { task_id:1, patch }, 100).is_err());
        assert!(db.mutations().unwrap().is_empty());
    }
    assert!(db.enqueue(Write::TaskPatch { task_id:999, patch:TaskPatch::Status(Status::Archive) }, 100).is_err());
}
