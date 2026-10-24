use ht_core::{
    Db, Drain, Transport, ViewWrite, Write, board_views,
    http::{Response, TransportError},
};
use serde_json::{Value, json};
fn project() -> Value {
    json!({"id":15,"title":"Board","project_view":{"id":"pv","projectId":15,"allViews":[{"id":"v1","title":"Daily","board_sorting_mode":"Priority"}],"user_project_views":[{}]},"section":[{"id":10,"projectId":15,"section_title":"Todo","visibility":true,"deleted":false,"ranking":"a"}]})
}
fn seed(db: &mut Db) {
    db.kv_set("current-user-id", "42").unwrap();
    db.apply_web_board(42,&json!({"project":project(),"allViews":project()["project_view"]["allViews"],"tasks":[{"id":1,"projectId":15,"sectionId":10,"title":"Low","status":"Normal","ranking":"A0100","priority":{"priority_index":4}},{"id":2,"projectId":15,"sectionId":10,"title":"Urgent","status":"Normal","ranking":"A0190","priority":{"priority_index":1}}]})).unwrap();
}
struct Server {
    status: u16,
    path: String,
    body: Value,
    reply: Value,
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
        assert_eq!(path, self.path);
        assert_eq!(body, &self.body);
        assert!(!key.is_empty());
        Ok(Response {
            status: self.status,
            body: if self.status == 200 {
                self.reply.clone()
            } else {
                json!({"message":"Unauthorized"})
            },
        })
    }
}
#[test]
fn unsaved_sort_and_filter_use_full_web_body_and_rollback_refusals_after_restart() {
    for status in [200, 401, 403] {
        let file = tempfile::NamedTempFile::new().unwrap();
        let mut db = Db::open(file.path()).unwrap();
        seed(&mut db);
        let mut body = board_views::settings(&db.board_project(15).unwrap());
        body["projectId"] = json!(15);
        body["board_sorting_mode"] = json!("Priority");
        body["baseViewId"] = Value::Null;
        let id = db
            .enqueue(
                Write::View {
                    change: ViewWrite {
                        project: 15,
                        route: "unsaved-view".into(),
                        body: body.clone(),
                    },
                },
                100,
            )
            .unwrap();
        assert_eq!(db.board(15).unwrap()[0].cards[0].id, 2);
        drop(db);
        let mut db = Db::open(file.path()).unwrap();
        let mut view = project()["project_view"].clone();
        view["user_project_views"][0]["unsavedView"] = body.clone();
        let mut server = Server {
            status,
            path: "/api/projects/views/unsaved-view".into(),
            body,
            reply: view,
        };
        assert_eq!(
            db.drain_once(&mut server, 100).unwrap(),
            if status == 200 {
                Drain::Applied(id)
            } else {
                Drain::Failed(id)
            }
        );
        assert_eq!(
            db.board(15).unwrap()[0].cards[0].id,
            if status == 200 { 2 } else { 1 }
        );
    }
}
#[test]
fn saved_view_and_order_routes_accept_the_web_response_shapes_or_refuse_without_success() {
    for route in [
        "switch-view",
        "create-view",
        "update-view",
        "update-order",
        "set-default-order",
        "reset-order",
        "reset-to-default",
    ] {
        for status in [200, 401, 403] {
            let mut db = Db::memory().unwrap();
            seed(&mut db);
            let body = match route {
                "switch-view" => json!({"projectId":15,"newViewId":"v1"}),
                "create-view" => {
                    json!({"projectId":15,"viewTitle":"Daily","setAsDefault":false,"visibility":"Private","view_settings":board_views::settings(&project())})
                }
                "update-view" => {
                    json!({"projectId":15,"viewId":"v1","view_settings":board_views::settings(&project())})
                }
                "update-order" | "set-default-order" => json!({"projectId":15,"viewOrder":["v1"]}),
                "reset-to-default" => json!({"projectId":15,"mode":"ResetToDefault"}),
                _ => json!({"projectId":15}),
            };
            let reply = match route {
                "create-view" => {
                    json!({"view":"daily","project_view_updated":project()["project_view"]})
                }
                "update-order" => json!({"viewOrder":["v1"]}),
                "set-default-order" => json!({"defaultViewOrder":["v1"]}),
                "reset-order" => json!({"viewOrder":null}),
                _ => project()["project_view"].clone(),
            };
            let id = db
                .enqueue(
                    Write::View {
                        change: ViewWrite {
                            project: 15,
                            route: route.into(),
                            body: body.clone(),
                        },
                    },
                    100,
                )
                .unwrap();
            assert_eq!(
                db.drain_once(
                    &mut Server {
                        status,
                        path: format!("/api/projects/views/{route}"),
                        body,
                        reply
                    },
                    100
                )
                .unwrap(),
                if status == 200 {
                    Drain::Applied(id)
                } else {
                    Drain::Failed(id)
                }
            );
        }
    }
}
#[test]
fn active_board_read_uses_same_post_and_body_as_web_and_rejects_denial() {
    for status in [200, 401, 403] {
        let mut server = Server {
            status,
            path: "/api/projects/boardTasks".into(),
            body: json!({"projectId":15,"userId":42}),
            reply: json!({"project":project(),"tasks":[],"allViews":[]}),
        };
        assert_eq!(board_views::load(&mut server, 15).is_ok(), status == 200);
    }
}

#[test]
fn null_unsaved_fields_inherit_applied_settings_without_resetting_other_view_fields() {
    let project = json!({"project_view":{"user_project_views":[{
        "unsavedView":{"board_sorting_mode":null,"board_filters":{"addedFilters":[],"matchFilters":"ALL"}},
        "appliedView":{"board_sorting_mode":"Title","board_empty_sections":"Hidden"}
    }],"default_view":{"board_sorting_order":"Ascending"}}});
    let settings = board_views::settings(&project);
    assert_eq!(settings["board_sorting_mode"], "Title");
    assert_eq!(settings["board_sorting_order"], "Ascending");
    assert_eq!(settings["board_empty_sections"], "Hidden");
    assert_eq!(settings["board_filters"]["matchFilters"], "ALL");
}

#[test]
fn rename_delete_and_view_order_are_optimistic_and_authoritative_after_refusal_or_restart(){
    struct Web {change:ViewWrite,status:u16,reply:Value}
    impl Transport for Web{
        fn get(&mut self,_:&str)->Result<Response,TransportError>{Ok(Response{status:200,body:json!({"success":true,"user":{"id":42}})})}
        fn post(&mut self,path:&str,body:&Value,key:&str)->Result<Response,TransportError>{self.mutate("POST",path,body,key)}
        fn mutate(&mut self,method:&str,path:&str,body:&Value,_:&str)->Result<Response,TransportError>{let(m,p,b)=self.change.request().unwrap();assert_eq!(method,m);assert_eq!(path,p);assert_eq!(body,&b);Ok(Response{status:self.status,body:if self.status==200{self.reply.clone()}else{json!({"message":"Unauthorized"})}})}
    }
    for route in ["rename-view","delete-view","update-order","reset-order","set-default-order"]{for status in [200,401,403]{
        let file=tempfile::NamedTempFile::new().unwrap();let mut db=Db::open(file.path()).unwrap();seed(&mut db);
        let before=db.board_project(15).unwrap();let body=match route{"rename-view"=>json!({"projectId":15,"viewId":"v1","title":"Renamed"}),"delete-view"=>json!({"projectId":15,"viewId":"v1"}),_=>json!({"projectId":15,"viewOrder":["v1"]})};
        let change=ViewWrite{project:15,route:route.into(),body};db.enqueue(Write::View{change:change.clone()},1).unwrap();
        match route{"rename-view"=>assert_eq!(db.board_project(15).unwrap()["project_view"]["allViews"][0]["title"],"Renamed"),"delete-view"=>assert!(board_views::ordered_views(&db.board_project(15).unwrap()).is_empty()),"update-order"=>assert_eq!(db.board_project(15).unwrap()["project_view"]["user_project_views"][0]["view_order"],json!(["v1"])),_=>()}
        let reply=match route{"rename-view"=>{let mut view=before["project_view"].clone();view["allViews"][0]["title"]=json!("Renamed");json!({"view":{"id":"v1","title":"Renamed"},"project_view_updated":view})},"delete-view"=>{let mut view=before["project_view"].clone();view["allViews"]=json!([]);view},"set-default-order"=>json!({"defaultViewOrder":["v1"]}),"reset-order"=>json!({"viewOrder":null}),_=>json!({"viewOrder":["v1"]})};
        drop(db);let mut db=Db::open(file.path()).unwrap();let result=db.drain_once(&mut Web{change,status,reply},2).unwrap();assert_eq!(matches!(result,Drain::Applied(_)),status==200);
        if status!=200{assert_eq!(db.board_project(15).unwrap()["project_view"],before["project_view"]);}else{match route{"rename-view"=>assert_eq!(db.board_project(15).unwrap()["project_view"]["allViews"][0]["title"],"Renamed"),"delete-view"=>assert!(board_views::ordered_views(&db.board_project(15).unwrap()).is_empty()),"set-default-order"=>assert_eq!(db.board_project(15).unwrap()["project_view"]["default_view_order"],json!(["v1"])),"reset-order"=>assert!(db.board_project(15).unwrap()["project_view"]["user_project_views"][0]["view_order"].is_null()),_=>()}}
    }}
}

#[test]
fn advanced_filter_types_are_optimistic_and_restore_the_board_after_both_refusals(){
    let today=chrono::Utc::now().to_rfc3339();
    for(kind,values)in [
        ("CreatedBy",json!([{"id":9,"uid":"u9"}])),("UpdatedBy",json!([{"id":9,"uid":"u9"}])),
        ("UpdatedBy",json!([{"id":"agent"}])),("BlockedByPerson",json!([{"id":0}])),
        ("DueDate",json!([{"dynamicRange":"TODAY"}])),("DueDate",json!([{"dynamicRange":"NO_DUE_DATE"}])),
        ("CreatedAt",json!([{"dynamicRange":"TODAY"}])),("UpdatedRange",json!([{"dynamicRange":"TODAY"}])),
        ("NoRecentComment",json!([])),("StuckInColumn",json!([])),("StaleOnBoard",json!([])),("NotStale",json!([])),
        ("RunningTimer",json!([])),("Inbox",json!([])),("Unread",json!([])),
        ("Labels",json!([{"id":"a"},{"id":"b"}])),("Assignees",json!([{"id":9,"uid":"u9"},{"id":"agent"}])),
    ]{for status in [200,401,403]{
        let mut db=Db::memory().unwrap();seed(&mut db);let mut project=project();project["timeTrackingEnabled"]=json!(true);
        db.apply_response(&json!({"success":true,"projects":[project],"tasks":[
            {"id":1,"projectId":15,"sectionId":10,"title":"Old","status":"Normal","ranking":"A0100","createdAt":"2000-01-01T00:00:00Z","lastCommentAt":"2000-01-01T00:00:00Z","sectionChangedAt":"2000-01-01T00:00:00Z"},
            {"id":2,"projectId":15,"sectionId":10,"title":"New","status":"Normal","ranking":"A0190","createdAt":today,"updatedAt":today,"dueDate":today,"sectionChangedAt":today,"lastCommentAt":today,"userId":9,"updatedByUserIds":[9],"agentId":"agent","waitingOnUserId":9,"notifications":[{"seen":false}],"taskLabels":[{"label":{"id":"a"}},{"label":{"id":"b"}}],"assignees":[{"userId":9,"agentId":null},{"agentId":"agent"}]}
        ]})).unwrap();db.kv_set("web-running-timers","[{\"taskId\":2}]").unwrap();
        let before=db.board_project(15).unwrap();let mut body=board_views::settings(&before);body["projectId"]=json!(15);body["baseViewId"]=Value::Null;body["board_filters"]=json!({"addedFilters":[{"type":kind,"searchPayload":values,"match":"ALL"}],"matchFilters":"ALL"});
        let expected=if matches!(kind,"NoRecentComment"|"StuckInColumn"|"StaleOnBoard") || (kind=="DueDate" && values[0]["dynamicRange"]=="NO_DUE_DATE"){1}else{2};
        db.enqueue(Write::View{change:ViewWrite{project:15,route:"unsaved-view".into(),body:body.clone()}},100).unwrap();
        assert_eq!(db.board(15).unwrap()[0].cards.iter().map(|task|task.id).collect::<Vec<_>>(),vec![expected],"{kind} optimistic");
        let mut reply=before["project_view"].clone();reply["user_project_views"][0]["unsavedView"]=body.clone();
        db.drain_once(&mut Server{status,path:"/api/projects/views/unsaved-view".into(),body,reply},101).unwrap();
        if status==200{assert_eq!(db.board(15).unwrap()[0].cards.len(),1);}else{assert_eq!(db.board_project(15).unwrap(),before);assert_eq!(db.board(15).unwrap()[0].cards.len(),2);}
    }}
}

#[test]
fn filter_people_and_timer_reads_reject_both_refusals_without_cached_success(){
    struct Reads(u16);
    impl Transport for Reads{
        fn get(&mut self,path:&str)->Result<Response,TransportError>{Ok(Response{status:self.0,body:if self.0!=200{json!({"message":"Unauthorized","error":"Unauthorized"})}else if path=="/api/members/getAllForAssignees?projectId=15"{json!({"id":15,"owner":{"id":42,"uid":"u42"},"members":[],"boardAgents":[]})}else{assert_eq!(path,"/api/time/running");json!({"success":true,"entries":[{"taskId":2}]})}})}
        fn post(&mut self,_:&str,_:&Value,_:&str)->Result<Response,TransportError>{panic!("read-only")}
    }
    for status in [200,401,403]{let mut server=Reads(status);assert_eq!(board_views::filter_people(&mut server,15).is_ok(),status==200);assert_eq!(board_views::running_timers(&mut server).is_ok(),status==200);}
}
