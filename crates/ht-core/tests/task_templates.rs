use ht_core::{
    Db, Drain, Transport, Write,
    http::{Response, TransportError},
    task_create,
};
use serde_json::{Value, json};
fn catalog() -> Value {
    json!({"success":true,"templates":[{"id":81,"name":"Daily template","title":"Check everything","descriptionHtml":"<p>Checklist</p>","priorityIndex":2,"estimateIndex":4,"labelIds":["live","gone"]}],"labels":[{"id":"live","value":"Ready","projectId":15}],"targetSection":{"id":10,"section_title":"Hidden"}})
}
struct Server(u16);
impl Transport for Server {
    fn get(&mut self, path: &str) -> Result<Response, TransportError> {
        Ok(Response {
            status: if path == "/user/context" {
                200
            } else {
                assert_eq!(path, "/api/task-templates?projectId=15");
                self.0
            },
            body: if path == "/user/context" {
                json!({"success":true,"user":{"id":42}})
            } else if self.0 == 200 {
                catalog()
            } else {
                json!({"error":"Unauthorized"})
            },
        })
    }
    fn post(&mut self, path: &str, body: &Value, key: &str) -> Result<Response, TransportError> {
        assert_eq!(path, "/api/tasks/createGlobally");
        assert!(!key.is_empty());
        assert_eq!(body["title"], "Check everything");
        assert_eq!(body["description"], "<p>Checklist</p>");
        assert_eq!(body["sectionId"], 10);
        assert_eq!(body["priority"]["priority_index"], 2);
        assert_eq!(body["estimate"]["estimate_index"], 4);
        assert_eq!(
            body["tags"],
            json!([{"id":"live","value":"Ready","projectId":15}])
        );
        let mut task = body.clone();
        task["id"] = json!(500);
        task["status"] = json!("Normal");
        Ok(Response {
            status: self.0,
            body: if self.0 == 200 {
                json!({"newTask":task})
            } else {
                json!({"error":"Unauthorized"})
            },
        })
    }
}
#[test]
fn task_template_catalog_and_hidden_column_create_match_web_with_live_labels_and_refusal_rollback()
{
    for status in [200, 401, 403] {
        let mut server = Server(status);
        let result = task_create::templates(&mut server, 15);
        if status == 200 {
            assert_eq!(result.unwrap(), catalog());
        } else {
            assert_eq!(result.unwrap_err().to_string(), "Unauthorized");
        }
    }
    for status in [200, 401, 403] {
        let mut db = Db::memory().unwrap();
        db.kv_set("current-user-id", "42").unwrap();
        db.apply_response(&json!({"success":true,"projects":[{"id":15,"title":"Board"}],"sections":[{"id":10,"projectId":15,"section_title":"Hidden","visibility":false,"deleted":false,"ranking":"a"}]})).unwrap();
        assert!(
            db.enqueue(
                Write::CreateTask {
                    project_id: 15,
                    section_id: 10,
                    title: "Inline".into(),
                    top: true
                },
                100
            )
            .is_err()
        );
        let source = task_create::template_source(&catalog(), 15, 81).unwrap();
        let fields = json!({"description":source["description"],"priority":source["priority"],"estimate":source["estimate"],"tags":source["tags"]});
        let id = db
            .enqueue(
                Write::CreateTaskForm {
                    project_id: 15,
                    section_id: 10,
                    title: source["title"].as_str().unwrap().into(),
                    fields,
                },
                100,
            )
            .unwrap();
        assert!(db.task(-id).unwrap().is_some());
        assert_eq!(
            db.drain_once(&mut Server(status), 101).unwrap(),
            if status == 200 {
                Drain::Applied(id)
            } else {
                Drain::Failed(id)
            }
        );
        assert!(db.task(-id).unwrap().is_none());
        assert_eq!(db.task(500).unwrap().is_some(), status == 200);
    }
    let mut empty = catalog();
    empty["targetSection"] = Value::Null;
    assert_eq!(
        task_create::template_source(&empty, 15, 81)
            .unwrap_err()
            .to_string(),
        "Could not find an open column for this task"
    );
    assert!(task_create::template_source(&catalog(), 15, 999).is_err());
}
