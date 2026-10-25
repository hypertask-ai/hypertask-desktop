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
        let task = json!({"id":51,"projectId":15,"sectionId":10,"section":"Todo","title":body["title"],"ranking":body["ranking"],"priority":body["priority"],"estimate":body["estimate"],"status":"Normal"});
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
            let visible = db.board(15).unwrap();
            assert_eq!(visible[0].cards.len(), if filtered { 1 } else { 2 });
            if filtered {
                assert_eq!(visible[0].cards[0].id, -id);
                assert_eq!(visible[0].cards[0].extra["priority"]["priority_index"], 2);
            }
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

#[test]
fn expanded_create_form_uses_global_web_body_optimism_and_refusal_rollback_after_restart(){
    for status in [200,401,403,0]{
        let file=tempfile::NamedTempFile::new().unwrap();let mut db=Db::open(file.path()).unwrap();seed(&mut db,false);
        let fields=json!({"description":"<p>Keep <strong>rich</strong> text</p>","priority":ht_core::task_create::priority(2).unwrap(),"estimate":ht_core::task_create::estimate(3).unwrap(),"dueDate":"2026-12-01T09:00:00Z","startDate":"2026-11-01T09:00:00Z","tags":[{"id":"11111111-1111-1111-1111-111111111111"}],"assignees":[{"id":42,"uid":"u42","displayName":"Owner"}]});
        let id=db.enqueue(Write::CreateTaskForm{project_id:15,section_id:10,title:"Expanded task".into(),fields:fields.clone()},100).unwrap();let row=db.mutations().unwrap()[0].clone();assert_eq!(row.kind,"web_create");assert_eq!(row.payload_json["global"],true);
        let optimistic=db.task(-id).unwrap().unwrap();assert_eq!(optimistic.description,"<p>Keep <strong>rich</strong> text</p>");assert_eq!(optimistic.due_date.as_deref(),Some("2026-12-01T09:00:00Z"));
        drop(db);let mut db=Db::open(file.path()).unwrap();let mut server=Server{status,calls:vec![]};db.drain_once(&mut server,101).unwrap();assert_eq!(server.calls[0].0,"/api/tasks/createGlobally");
        let body=&server.calls[0].1;for(key,value)in fields.as_object().unwrap(){assert_eq!(&body[key],value);}
        assert_eq!(body["fullScreenTask"],true);assert_eq!(body["section_title"],"Todo");assert_eq!(body["projectIdentifier"],"HT");assert!(body.get("section").is_none());assert!(body.get("index").is_none());
        assert!(db.task(-id).unwrap().is_none());assert_eq!(db.task(51).unwrap().is_some(),status==200);assert_eq!(db.drain_once(&mut server,200).unwrap(),Drain::Empty);assert_eq!(server.calls.len(),1);
    }
}

#[test]
fn expanded_form_cannot_override_identity_placement_or_route_fields(){
    for fields in [json!({"userId":999}),json!({"projectId":16}),json!({"fullScreenTask":false}),json!({"dueDate":"invalid"}),json!({"tags":"invalid"}),json!({"description":{}})]{
        let mut db=Db::memory().unwrap();seed(&mut db,false);assert!(db.enqueue(Write::CreateTaskForm{project_id:15,section_id:10,title:"Task".into(),fields},100).is_err());assert!(db.mutations().unwrap().is_empty());assert_eq!(db.board(15).unwrap()[0].cards.len(),1);
    }
}

#[test]
fn duplicate_reads_the_web_single_task_route_and_refusals_never_become_a_source() {
    struct Source {status:u16,body:Value}
    impl Transport for Source {
        fn get(&mut self,path:&str)->Result<Response,TransportError>{assert_eq!(path,"/api/tasks/single?id=1");Ok(Response{status:self.status,body:self.body.clone()})}
        fn post(&mut self,_:&str,_:&Value,_:&str)->Result<Response,TransportError>{panic!("duplicate reads must not write")}
    }
    for status in [200,401,403] {
        let mut source=Source{status,body:if status==200{json!({"id":1,"projectId":15,"sectionId":10,"title":"Original","description_":{"content":"<p>Rich</p>"}})}else{json!({"message":"Task access denied"})}};
        let result=ht_core::task_create::duplicate_source(&mut source,1);assert_eq!(result.is_ok(),status==200);if status!=200{assert_eq!(result.unwrap_err().to_string(),"Task access denied");}
    }
    assert!(ht_core::task_create::duplicate_source(&mut Source{status:200,body:json!({"id":2,"projectId":15,"sectionId":10,"title":"Wrong task"})},1).is_err());
}

#[test]
fn source_comment_create_preserves_web_context_and_attachments_through_restart_acceptance_or_refusal() {
    for status in [200, 401, 403, 0] {
        let file = tempfile::NamedTempFile::new().unwrap();
        let mut db = Db::open(file.path()).unwrap();
        seed(&mut db, false);
        db.apply_task(&json!({"id":1,"projectId":15,"sectionId":10,"section":"Todo","uniqueIndex":7,"ticketNumber":"HT-7","title":"Source","status":"Normal"})).unwrap();
        let fields = json!({"description":"<p>Quoted source</p>","createTaskFromComment":{"task":{"id":1,"projectId":15,"uniqueIndex":7},"commentIndex":2},"urlsToAdd":[{"urlString":"https://files.example/source.pdf","Attachment":true,"fileSize":1024,"title":"source.pdf","attachmentType":"application/pdf"}]});
        let id = db.enqueue(Write::CreateTaskForm { project_id:15, section_id:10, title:"Follow up".into(), fields:fields.clone() }, 100).unwrap();
        let key = db.mutations().unwrap()[0].client_id.clone();
        assert_eq!(db.task(-id).unwrap().unwrap().description, "<p>Quoted source</p>");
        drop(db);
        let mut db = Db::open(file.path()).unwrap();
        let mut server = Server { status, calls:vec![] };
        assert_eq!(db.drain_once(&mut server, 101).unwrap(), if status==200 {Drain::Applied(id)} else {Drain::Failed(id)});
        assert_eq!(server.calls[0].0, "/api/tasks/createGlobally");
        for (field,value) in fields.as_object().unwrap() { assert_eq!(&server.calls[0].1[field], value); }
        assert!(server.calls[0].1.get("parentTask").is_none());
        assert!(db.task(-id).unwrap().is_none());
        assert_eq!(db.task(51).unwrap().is_some(), status==200);
        let receipt:Value=serde_json::from_str(&db.kv_get(&format!("create-result:{key}")).unwrap().unwrap()).unwrap();
        assert_eq!(receipt["error"].is_null(), status==200);
        assert_eq!(db.drain_once(&mut server, 1000).unwrap(), Drain::Empty);
        assert_eq!(server.calls.len(), 1);
    }
}

#[test]
fn source_comment_create_rejects_malformed_or_mismatched_context_before_any_optimistic_write() {
    let valid = json!({"task":{"id":1,"projectId":15,"uniqueIndex":7},"commentIndex":0});
    for context in [json!(null),json!({"task":{"id":1,"projectId":16,"uniqueIndex":7},"commentIndex":0}),json!({"task":{"id":999,"projectId":15,"uniqueIndex":7},"commentIndex":0}),json!({"task":{"id":1,"projectId":15,"uniqueIndex":8},"commentIndex":0}),json!({"task":{"id":1,"projectId":15,"uniqueIndex":7},"commentIndex":-1}),json!({"task":{"id":1,"projectId":15,"uniqueIndex":7},"commentIndex":"0"})] {
        let mut db=Db::memory().unwrap();seed(&mut db,false);
        db.apply_task(&json!({"id":1,"projectId":15,"sectionId":10,"title":"Source","uniqueIndex":7,"status":"Normal"})).unwrap();
        assert!(db.enqueue(Write::CreateTaskForm {project_id:15,section_id:10,title:"Task".into(),fields:json!({"createTaskFromComment":context})},100).is_err());
        assert!(db.mutations().unwrap().is_empty());
    }
    for attachment in [json!({"urlString":"javascript:alert(1)","Attachment":true,"fileSize":1,"title":"bad","attachmentType":"text/plain"}),json!({"urlString":"https://files.example/a","Attachment":true,"fileSize":-1,"title":"bad","attachmentType":"text/plain"}),json!({"urlString":"https://files.example/a","Attachment":true,"fileSize":1,"attachmentType":"text/plain"})] {
        let mut db=Db::memory().unwrap();seed(&mut db,false);
        db.apply_task(&json!({"id":1,"projectId":15,"sectionId":10,"title":"Source","uniqueIndex":7,"status":"Normal"})).unwrap();
        assert!(db.enqueue(Write::CreateTaskForm {project_id:15,section_id:10,title:"Task".into(),fields:json!({"createTaskFromComment":valid,"urlsToAdd":[attachment]})},100).is_err());
        assert!(db.mutations().unwrap().is_empty());
    }
}
