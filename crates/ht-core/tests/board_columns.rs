use ht_core::{Db, Drain, Transport, ViewWrite, Write, board_columns, http::{Response, TransportError}};
use serde_json::{Value,json};
fn section(id:i64,title:&str,rank:&str)->Value {json!({"id":id,"projectId":15,"section_title":title,"ranking":rank,"visibility":true,"deleted":false,"isDone":null,"autoAssignUserId":null,"autoAssignAgentId":null})}
fn seed(db:&mut Db) {
    db.kv_set("current-user-id","42").unwrap();
    let columns=json!([section(10,"Todo","A0100"),section(11,"Done","A0200")]);
    let view=json!({"id":"default","board_columns_view":columns});
    db.apply_response(&json!({"success":true,"projects":[{"id":15,"title":"Board","project_view":{"id":"pv","projectId":15,"default_view":view,"allViews":[view],"user_project_views":[{}]}}],"sections":columns,"tasks":[{"id":1,"projectId":15,"sectionId":10,"section":"Todo","title":"Task","status":"Normal","ranking":"A0100"}]})).unwrap();
}
struct Server { status:u16,path:&'static str,body:Value,reply:Value,calls:usize,user:i64 }
impl Transport for Server {
    fn get(&mut self,path:&str)->Result<Response,TransportError>{assert_eq!(path,"/user/context");Ok(Response{status:200,body:json!({"success":true,"user":{"id":self.user}})})}
    fn post(&mut self,path:&str,body:&Value,key:&str)->Result<Response,TransportError>{self.mutate("POST",path,body,key)}
    fn mutate(&mut self,method:&str,path:&str,body:&Value,key:&str)->Result<Response,TransportError>{assert_eq!(method,"POST");assert_eq!(path,self.path);assert_eq!(body,&self.body);assert!(!key.is_empty());self.calls+=1;Ok(Response{status:self.status,body:if self.status<300{self.reply.clone()}else{json!({"message":"Forbidden"})}})}
}
#[test]
fn all_column_controls_use_exact_web_routes_and_rollback_refusals_after_restart() {
    for (route,path,body,reply,accepted) in [
        ("column-create","/api/section/create",json!({"projectId":15,"title":"Review"}),json!({"section":section(12,"Review","A0300"),"project_view":{"id":"pv","projectId":15,"default_view":{"id":"default","board_columns_view":[section(10,"Todo","A0100"),section(11,"Done","A0200"),section(12,"Review","A0300")]},"allViews":[],"user_project_views":[{}]}}),200),
        ("column-update","/api/section/update",json!({"userId":42,"sectionId":10,"newSection":{"section_title":"Backlog"}}),section(10,"Backlog","A0100"),200),
        ("column-update","/api/section/update",json!({"userId":42,"sectionId":10,"newSection":{"isDone":true}}),{let mut value=section(10,"Todo","A0100");value["isDone"]=json!(true);value},200),
        ("column-update","/api/section/update",json!({"userId":42,"sectionId":10,"newSection":{"ranking":"A0300"}}),section(10,"Todo","A0300"),200),
        ("column-update","/api/section/update",json!({"userId":42,"sectionId":10,"newSection":{"deleted":true}}),Value::Null,204),
        ("column-visibility","/api/sections/view-visibility",json!({"sectionId":10,"visible":false}),json!({"success":true,"sectionId":10,"visible":false}),200),
        ("column-visibility","/api/sections/view-visibility",json!({"sectionId":10,"visible":true}),json!({"success":true,"sectionId":10,"visible":true}),200),
        ("column-autoassign","/api/sections/auto-assign",json!({"sectionId":10,"autoAssignUserId":7,"autoAssignAgentId":null}),json!({"success":true,"sectionId":10,"autoAssignUserId":7,"autoAssignAgentId":null}),200),
        ("column-autoassign","/api/sections/auto-assign",json!({"sectionId":10,"autoAssignUserId":null,"autoAssignAgentId":"agent"}),json!({"success":true,"sectionId":10,"autoAssignUserId":null,"autoAssignAgentId":"agent"}),200),
        ("column-autoassign","/api/sections/auto-assign",json!({"sectionId":10,"autoAssignUserId":null,"autoAssignAgentId":null}),json!({"success":true,"sectionId":10,"autoAssignUserId":null,"autoAssignAgentId":null}),200),
    ] {
        for status in [accepted,401,403] {
            let file=tempfile::NamedTempFile::new().unwrap();let mut db=Db::open(file.path()).unwrap();seed(&mut db);
            let before=db.board_project(15).unwrap();let task=db.task(1).unwrap().unwrap();
            let mut queued=body.clone();queued["projectId"]=json!(15);
            let id=db.enqueue(Write::View{change:ViewWrite{project:15,route:route.into(),body:queued}},100).unwrap();let key=db.mutations().unwrap()[0].client_id.clone();
            if route=="column-create" {assert!(db.board(15).unwrap().iter().any(|column|column.id==-id && column.title=="Review"));}
            if body["newSection"]["deleted"]==true {assert_eq!(db.task(1).unwrap().unwrap().section_id,Some(11));assert!(!db.board(15).unwrap().iter().any(|column|column.id==10));}
            if body["newSection"]["section_title"].is_string() {assert_eq!(db.task(1).unwrap().unwrap().section,"Backlog");}
            drop(db);let mut db=Db::open(file.path()).unwrap();
            assert_eq!(db.drain_once(&mut Server{status,path,body:body.clone(),reply:reply.clone(),calls:0,user:42},100).unwrap(),if status<300{Drain::Applied(id)}else{Drain::Failed(id)},"{route} {body}");
            let result:Value=serde_json::from_str(&db.kv_get(&format!("view-result:{key}")).unwrap().unwrap()).unwrap();
            if status>=300 {assert_eq!(db.board_project(15).unwrap(),before,"{route} {body}");let restored=db.task(1).unwrap().unwrap();assert_eq!(restored.section,task.section);assert_eq!(restored.section_id,task.section_id);assert_eq!(result["error"],"Forbidden");}
            else {
                assert!(result["error"].is_null());
                if route=="column-create" {assert!(db.board(15).unwrap().iter().any(|column|column.id==12));assert!(!db.board_project(15).unwrap()["section"].as_array().unwrap().iter().any(|column|column["id"]==-id));}
                if route=="column-visibility" {assert_eq!(board_columns::visible_counts(&db.board_project(15).unwrap(),10).0,if body["visible"]==true{1}else{0});assert_eq!(db.board_project(15).unwrap()["section"][0]["visibility"],true);}
                if body["newSection"]["ranking"].is_string() {assert_eq!(db.board(15).unwrap()[0].id,11);assert_eq!(board_columns::reordered_settings(&db.board_project(15).unwrap())["board_columns_view"][0]["id"],11);}
                if route=="column-autoassign" {let project=db.board_project(15).unwrap();assert_eq!(project["section"][0]["autoAssignUserId"],body["autoAssignUserId"]);assert_eq!(project["section"][0]["autoAssignAgentId"],body["autoAssignAgentId"]);}
            }
        }
    }
}
#[test]
fn column_last_card_guard_invalid_selection_account_switch_and_bad_ack_never_succeed() {
    let mut db=Db::memory().unwrap();seed(&mut db);
    let delete=|id|ViewWrite{project:15,route:"column-update".into(),body:json!({"projectId":15,"userId":42,"sectionId":id,"newSection":{"deleted":true}})};
    db.apply_response(&json!({"success":true,"sections":[{"id":11,"projectId":15,"deleted":true}]})).unwrap();
    assert!(db.enqueue(Write::View{change:delete(10)},100).unwrap_err().to_string().contains("last column"));
    assert!(db.enqueue(Write::View{change:delete(999)},100).is_err());
    let mut db=Db::memory().unwrap();seed(&mut db);
    let body=json!({"projectId":15,"sectionId":10,"autoAssignUserId":7,"autoAssignAgentId":"agent"});
    assert!(db.enqueue(Write::View{change:ViewWrite{project:15,route:"column-autoassign".into(),body}},100).is_err());
    let id=db.enqueue(Write::View{change:delete(10)},100).unwrap();
    let mut server=Server{status:204,path:"/api/section/update",body:json!({"userId":42,"sectionId":10,"newSection":{"deleted":true}}),reply:Value::Null,calls:0,user:99};
    assert_eq!(db.drain_once(&mut server,100).unwrap(),Drain::Failed(id));assert_eq!(server.calls,0);assert_eq!(db.task(1).unwrap().unwrap().section_id,Some(10));
}
#[test]
fn column_counts_are_not_capped_at_the_task_search_limit() {
    let mut db=Db::memory().unwrap();seed(&mut db);
    let tasks:Vec<_>=(2..=125).map(|id|json!({"id":id,"projectId":15,"sectionId":10,"title":"Task","status":"Normal"})).collect();
    db.apply_response(&json!({"success":true,"tasks":tasks})).unwrap();
    assert_eq!(db.column_task_count(15,10).unwrap(),125);
}

#[test]
fn refused_column_delete_does_not_overwrite_a_newer_acknowledged_task_move() {
    let mut db=Db::memory().unwrap();seed(&mut db);
    db.apply_response(&json!({"success":true,"sections":[section(12,"Review","A0300")]})).unwrap();
    let body=json!({"projectId":15,"userId":42,"sectionId":10,"newSection":{"deleted":true}});
    let id=db.enqueue(Write::View{change:ViewWrite{project:15,route:"column-update".into(),body}},100).unwrap();
    assert_eq!(db.task(1).unwrap().unwrap().section_id,Some(11));
    db.apply_response(&json!({"success":true,"tasks":[{"id":1,"projectId":15,"sectionId":12,"section":"Review","title":"New title","status":"Normal"}]})).unwrap();
    let result=db.drain_once(&mut Server{status:403,path:"/api/section/update",body:json!({"userId":42,"sectionId":10,"newSection":{"deleted":true}}),reply:Value::Null,calls:0,user:42},100).unwrap();
    assert_eq!(result,Drain::Failed(id));
    assert_eq!(db.task(1).unwrap().unwrap().section_id,Some(12));assert_eq!(db.task(1).unwrap().unwrap().title,"New title");
}

#[test]
fn created_column_accepts_the_webs_nullable_project_view_without_keeping_provisional_ids() {
    let mut db=Db::memory().unwrap();seed(&mut db);
    let id=db.enqueue(Write::View{change:ViewWrite{project:15,route:"column-create".into(),body:json!({"projectId":15,"title":"Review"})}},100).unwrap();
    assert_eq!(db.drain_once(&mut Server{status:200,path:"/api/section/create",body:json!({"projectId":15,"title":"Review"}),reply:json!({"section":section(12,"Review","A0300"),"project_view":null}),calls:0,user:42},100).unwrap(),Drain::Applied(id));
    let project=db.board_project(15).unwrap();assert!(project["project_view"].is_null());assert!(!project["section"].as_array().unwrap().iter().any(|section|section["id"]==-id));assert_eq!(db.board(15).unwrap().last().unwrap().id,12);
}

#[test]
fn refused_rename_and_delete_restore_missing_legacy_task_section_as_a_string() {
    for deleted in [false,true] {for status in [401,403] {
        let mut db=Db::memory().unwrap();seed(&mut db);
        db.apply_response(&json!({"success":true,"tasks":[{"id":1,"projectId":15,"sectionId":10,"section":null,"title":"Task","status":"Normal"}]})).unwrap();
        let fields=if deleted{json!({"deleted":true})}else{json!({"section_title":"Backlog"})};
        let body=json!({"projectId":15,"userId":42,"sectionId":10,"newSection":fields});
        let id=db.enqueue(Write::View{change:ViewWrite{project:15,route:"column-update".into(),body}},100).unwrap();
        assert_eq!(db.drain_once(&mut Server{status,path:"/api/section/update",body:json!({"userId":42,"sectionId":10,"newSection":fields}),reply:Value::Null,calls:0,user:42},100).unwrap(),Drain::Failed(id));
        assert_eq!(db.task(1).unwrap().unwrap().section,"Todo");assert_eq!(db.task(1).unwrap().unwrap().section_id,Some(10));
    }}
}
