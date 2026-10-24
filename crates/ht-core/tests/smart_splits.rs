use ht_core::{Db, Drain, Transport, ViewWrite, Write, smart_splits, http::{Response, TransportError}};
use serde_json::{Value, json};

fn split() -> Value { json!({"id":"smart","title":"Bugs","slug":"bugs","project_view_id":"pv","board_filters":{"matchFilters":"ALL","addedFilters":[{"type":"Labels","match":"ANY","searchPayload":[{"id":"smart","value":"Bugs"}]}]}}) }
fn seed(db: &mut Db) {
    db.kv_set("current-user-id", "42").unwrap();
    db.apply_response(&json!({"success":true,"projects":[{"id":15,"title":"Board","desktop_smart_labels":[{"id":"smart","projectId":15,"value":"Bugs","ai_prompt":"Broken tasks"}],"project_view":{"id":"pv","projectId":15,"default_view_id":"default","default_view":{"id":"default","board_sorting_mode":"Priority"},"allViews":[split()],"user_project_views":[{"appliedView":split(),"view_order":["smart"]}],"default_view_order":["smart"]}}]})).unwrap();
}
struct Server { status: u16, method: &'static str, expected: Value, reply: Value, calls: usize, user: i64 }
impl Transport for Server {
    fn get(&mut self, path: &str) -> Result<Response, TransportError> {
        assert_eq!(path, "/user/context");
        Ok(Response { status:200, body:json!({"success":true,"user":{"id":self.user}}) })
    }
    fn post(&mut self, path: &str, body: &Value, key: &str) -> Result<Response, TransportError> { self.mutate("POST", path, body, key) }
    fn mutate(&mut self, method: &str, path: &str, body: &Value, key: &str) -> Result<Response, TransportError> {
        assert_eq!(method, self.method); assert_eq!(path, "/api/projects/views/smart-split"); assert_eq!(body, &self.expected); assert!(!key.is_empty()); self.calls += 1;
        Ok(Response { status:self.status, body:if self.status < 300 { self.reply.clone() } else { json!({"message":"Sign in to manage smart splits"}) } })
    }
}
#[test]
fn smart_split_create_edit_delete_queue_exact_web_requests_and_rollback_401_403_after_restart() {
    for (route, method, success) in [("smart-split-create","POST",201),("smart-split-edit","PATCH",200),("smart-split-delete","DELETE",200)] {
        for status in [success,401,403] {
            let file = tempfile::NamedTempFile::new().unwrap();
            let mut db = Db::open(file.path()).unwrap(); seed(&mut db);
            let before = db.board_project(15).unwrap();
            let body = match route {
                "smart-split-create" => json!({"projectId":15,"name":"Urgent bugs","prompt":"Broken urgent tasks"}),
                "smart-split-edit" => json!({"projectId":15,"viewId":"smart","name":"Urgent bugs","prompt":"Broken urgent tasks"}),
                _ => json!({"projectId":15,"viewId":"smart"}),
            };
            let id = db.enqueue(Write::View { change:ViewWrite { project:15, route:route.into(), body:body.clone() } },100).unwrap();
            let key = db.mutations().unwrap()[0].client_id.clone();
            let optimistic = db.board_project(15).unwrap();
            match route {
                "smart-split-create" => { assert_eq!(optimistic["project_view"]["allViews"][1]["pending"],true); assert_eq!(optimistic["project_view"]["allViews"][1]["board_sorting_mode"],"Priority"); }
                "smart-split-edit" => { assert_eq!(optimistic["project_view"]["allViews"][0]["title"],"Urgent bugs"); assert_eq!(optimistic["project_view"]["user_project_views"][0]["appliedView"]["board_filters"]["addedFilters"][0]["searchPayload"][0]["value"],"Urgent bugs"); assert_eq!(optimistic["desktop_smart_labels"][0]["ai_prompt"],"Broken urgent tasks"); }
                _ => { assert_eq!(optimistic["project_view"]["allViews"],json!([])); assert!(optimistic["project_view"]["user_project_views"][0]["appliedView"].is_null()); assert_eq!(optimistic["desktop_smart_labels"],json!([])); }
            }
            drop(db); let mut db = Db::open(file.path()).unwrap();
            let reply = match route {
                "smart-split-create" => json!({"view":{"id":"new","title":"Urgent bugs","slug":"urgent-bugs","project_view_id":"pv"},"label":{"id":"new","value":"Urgent bugs","ai_prompt":"Broken urgent tasks","projectId":15}}),
                "smart-split-edit" => json!({"success":true,"slug":"urgent-bugs"}),
                _ => json!({"success":true}),
            };
            let mut server = Server { status,method,expected:body,reply,calls:0,user:42 };
            assert_eq!(db.drain_once(&mut server,100).unwrap(), if status < 300 { Drain::Applied(id) } else { Drain::Failed(id) });
            assert_eq!(server.calls,1);
            let after = db.board_project(15).unwrap();
            let result: Value = serde_json::from_str(&db.kv_get(&format!("view-result:{key}")).unwrap().unwrap()).unwrap();
            if status >= 300 { assert_eq!(after,before); assert_eq!(result["error"],"Sign in to manage smart splits"); }
            else {
                assert!(result["error"].is_null());
                match route {
                    "smart-split-create" => { assert_eq!(after["project_view"]["allViews"][1]["id"],"new"); assert_eq!(after["desktop_smart_labels"][1]["id"],"new"); }
                    "smart-split-edit" => assert_eq!(after["project_view"]["user_project_views"][0]["appliedView"]["slug"],"urgent-bugs"),
                    _ => assert_eq!(after["project_view"]["default_view_order"],json!([])),
                }
            }
        }
    }
}
#[test]
fn smart_split_validation_shared_legacy_tags_and_account_switch_never_send_unsafe_writes() {
    let mut db=Db::memory().unwrap(); seed(&mut db);
    for prompt in ["".to_owned(),"😀".repeat(501)] {
        assert!(db.enqueue(Write::View { change:ViewWrite { project:15,route:"smart-split-create".into(),body:json!({"projectId":15,"name":"Name","prompt":prompt}) } },100).is_err());
    }
    let legacy=json!({"id":"legacy","board_filters":split()["board_filters"]});
    let labels=json!([{"id":"smart","ai_prompt":"Broken"}]);
    assert!(smart_splits::label(&legacy,labels.as_array().unwrap(),&[legacy.clone()]).is_some());
    assert!(smart_splits::label(&legacy,labels.as_array().unwrap(),&[legacy.clone(),split()]).is_none());
    assert!(smart_splits::label(&split(),labels.as_array().unwrap(),&[legacy,split()]).is_some());
    let body=json!({"projectId":15,"viewId":"smart","name":"Changed","prompt":"Broken"});
    let id=db.enqueue(Write::View { change:ViewWrite { project:15,route:"smart-split-edit".into(),body:body.clone() } },100).unwrap();
    let mut server=Server { status:200,method:"PATCH",expected:body,reply:json!({"success":true,"slug":"changed"}),calls:0,user:99 };
    assert_eq!(db.drain_once(&mut server,100).unwrap(),Drain::Failed(id)); assert_eq!(server.calls,0);
    assert_eq!(db.board_project(15).unwrap()["project_view"]["allViews"][0]["title"],"Bugs");
}
#[test]
fn smart_split_creation_uncertain_response_is_not_replayed_or_reported_success() {
    let mut db=Db::memory().unwrap(); seed(&mut db);
    let body=json!({"projectId":15,"name":"New","prompt":"New tasks"});
    let id=db.enqueue(Write::View { change:ViewWrite { project:15,route:"smart-split-create".into(),body:body.clone() } },100).unwrap();
    let key=db.mutations().unwrap()[0].client_id.clone();
    let mut server=Server { status:201,method:"POST",expected:body,reply:json!({"success":true}),calls:0,user:42 };
    assert_eq!(db.drain_once(&mut server,100).unwrap(),Drain::Retrying(id));
    assert_eq!(db.drain_once(&mut server,1000).unwrap(),Drain::Failed(id)); assert_eq!(server.calls,1);
    assert!(db.kv_get(&format!("view-result:{key}")).unwrap().unwrap().contains("uncertain"));
    assert_eq!(db.board_project(15).unwrap()["project_view"]["allViews"].as_array().unwrap().len(),1);
}
#[test]
fn smart_split_metadata_read_uses_web_route_and_surfaces_refusals() {
    struct Labels(u16);
    impl Transport for Labels {
        fn get(&mut self,path:&str)->Result<Response,TransportError> { assert_eq!(path,"/api/labels/getByProject?projectId=15"); Ok(Response { status:self.0,body:if self.0==200 {json!([{"id":"smart","projectId":15,"ai_prompt":"Broken"}])}else{json!({"message":"Forbidden"})} }) }
        fn post(&mut self,_:&str,_:&Value,_:&str)->Result<Response,TransportError> { panic!("No writes for metadata") }
    }
    assert!(smart_splits::labels(&mut Labels(200),15).is_ok());
    for status in [401,403] { assert_eq!(smart_splits::labels(&mut Labels(status),15).unwrap_err().to_string(),"Forbidden"); }
}
