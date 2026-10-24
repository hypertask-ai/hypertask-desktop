use ht_core::{
    Db, Drain, Transport, Write,
    http::{Response, TransportError},
};
use serde_json::{Value, json};
struct Server {
    status: u16,
    calls: Vec<(String, Value)>,
}
impl Transport for Server {
    fn get(&mut self, path: &str) -> Result<Response, TransportError> {
        assert_eq!(path, "/user/context");
        Ok(Response {
            status: 200,
            body: json!({"success":true,"user":{"id":42}}),
        })
    }
    fn post(&mut self, path: &str, body: &Value, key: &str) -> Result<Response, TransportError> {
        assert!(!key.is_empty());
        self.calls.push((path.into(), body.clone()));
        if self.status == 0 {
            return Err(TransportError::Timeout);
        }
        let task = json!({"id":51,"projectId":15,"sectionId":10,"section":"Todo","title":body["title"],"ranking":body["ranking"],"status":"Normal"});
        Ok(Response {
            status: self.status,
            body: if path.ends_with("Globally") {
                json!({"newTask":task})
            } else {
                task
            },
        })
    }
}
fn seed(db: &mut Db, filtered: bool) {
    db.kv_set("current-user-id", "42").unwrap();
    let filters = if filtered {
        json!({"addedFilters":[{"type":"Priority","match":"ANY","searchPayload":[{"priority_index":2}]}],"matchFilters":"ANY"})
    } else {
        json!({"addedFilters":[]})
    };
    db.apply_response(&json!({"success":true,"projects":[{"id":15,"title":"Board","uniqueIdentifier":"HT","project_view":{"user_project_views":[{"unsavedView":{"board_filters":filters}}]}}],"sections":[{"id":10,"projectId":15,"section_title":"Todo","visibility":true,"deleted":false,"ranking":"a"}],"tasks":[{"id":1,"projectId":15,"sectionId":10,"title":"Existing","ranking":"A0100","status":"Normal"}]})).unwrap();
}
#[test]
fn inline_create_uses_web_route_body_active_filters_ranking_and_rolls_back_denial() {
    for filtered in [false, true] {
        for status in [200, 401, 403, 0] {
            let file = tempfile::NamedTempFile::new().unwrap();
            let mut db = Db::open(file.path()).unwrap();
            seed(&mut db, filtered);
            let id = db
                .enqueue(
                    Write::CreateTask {
                        project_id: 15,
                        section_id: 10,
                        title: "é & new".into(),
                        top: true,
                    },
                    100,
                )
                .unwrap();
            let key = db.mutations().unwrap()[0].client_id.clone();
            assert_eq!(db.board(15).unwrap()[0].cards.len(), 2);
            assert!(db.task(-id).unwrap().is_some());
            drop(db);
            let mut db = Db::open(file.path()).unwrap();
            let mut server = Server {
                status,
                calls: vec![],
            };
            assert_eq!(
                db.drain_once(&mut server, 100).unwrap(),
                if status == 200 {
                    Drain::Applied(id)
                } else {
                    Drain::Failed(id)
                }
            );
            assert_eq!(server.calls.len(), 1);
            let (path, body) = &server.calls[0];
            assert_eq!(
                path,
                if filtered {
                    "/api/tasks/createGlobally"
                } else {
                    "/api/tasks/create"
                }
            );
            let mut expected = json!({"title":"é & new","sectionId":10,"userId":42,"projectId":15,"ranking":"A0050.000000000000000","assignees":[]});
            if filtered {
                expected["fullScreenTask"] = json!(true);
                expected["section_title"] = json!("Todo");
                expected["projectIdentifier"] = json!("HT");
                expected["priority"] = ht_core::task_create::priority(2).unwrap();
            } else {
                expected["section"] = json!("Todo");
                expected["index"] = json!(1);
            }
            assert_eq!(body, &expected);
            assert!(db.task(-id).unwrap().is_none());
            assert_eq!(db.task(51).unwrap().is_some(), status == 200);
            assert!(
                db.kv_get(&format!("create-result:{key}"))
                    .unwrap()
                    .is_some()
            );
            assert_eq!(db.drain_once(&mut server, 1000).unwrap(), Drain::Empty);
            assert_eq!(server.calls.len(), 1);
        }
    }
}
#[test]
fn view_defaults_exclude_negative_matches_revoked_agents_and_invalid_ids() {
    let project = json!({"project_view":{"user_project_views":[{"unsavedView":{"board_filters":{"addedFilters":[
        {"type":"Labels","match":"NONE","searchPayload":[{"id":"11111111-1111-1111-1111-111111111111"}]},
        {"type":"Assignees","searchPayload":[{"id":3,"uid":"human"},{"id":"11111111-1111-1111-1111-111111111111","revokedAt":"today"},{"id":0,"uid":"bad"}]},
        {"type":"Priority","match":"ANY","searchPayload":[{"priority_index":0},{"priority_index":4}]},
        {"type":"Size","searchPayload":[{"estimate_index":1},{"estimate_index":3}]}
    ]}}}]}});
    let defaults = ht_core::task_create::view_defaults(&project);
    assert!(defaults["tags"].is_null());
    assert_eq!(defaults["assignees"], json!([{"id":3,"uid":"human"}]));
    assert_eq!(defaults["priority"]["priority_index"], 4);
    assert_eq!(defaults["estimate"]["estimate_index"], 3);
}
