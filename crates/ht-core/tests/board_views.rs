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
