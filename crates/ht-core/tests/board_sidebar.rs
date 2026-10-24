use ht_core::{
    Db, Drain, Transport, Write, board_sidebar,
    http::{Response, TransportError},
};
use serde_json::{Value, json};
struct Server {
    status: u16,
    calls: Vec<(String, String, Value)>,
}
fn teams() -> Value {
    json!([{"id":"team-3","title":"Team","projects":[{"id":15,"title":"Alpha","status":"Normal"},{"id":16,"title":"Beta","status":"Normal"}]}])
}
fn favorites() -> Value {
    json!([{"projectId":15,"index":1,"project":{"id":15,"title":"Alpha"}}])
}
impl Transport for Server {
    fn get(&mut self, path: &str) -> Result<Response, TransportError> {
        self.calls.push(("GET".into(), path.into(), Value::Null));
        Ok(if path == "/user/context" {
            Response {
                status: 200,
                body: json!({"success":true,"user":{"id":42}}),
            }
        } else {
            assert_eq!(path, "/api/favorites/getFavorites");
            Response {
                status: self.status,
                body: if self.status == 200 {
                    favorites()
                } else {
                    json!({"error":"Unauthorized"})
                },
            }
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
        _: &str,
    ) -> Result<Response, TransportError> {
        self.calls.push((method.into(), path.into(), body.clone()));
        let reply = match path {
            "/api/teams/getAllSidebar" => {
                assert_eq!(method, "POST");
                assert_eq!(body, &json!({"userId":42}));
                teams()
            }
            "/api/favorites/reorderFavorites" => {
                assert_eq!(method, "PUT");
                assert_eq!(
                    body,
                    &json!({"favorites":[{"projectId":16},{"projectId":15}]})
                );
                json!({"ok":true})
            }
            _ => panic!("unexpected {path}"),
        };
        Ok(Response {
            status: self.status,
            body: if self.status == 200 {
                reply
            } else {
                json!({"message":"Unauthorized"})
            },
        })
    }
}
#[test]
fn switcher_reads_actual_post_teams_and_get_favorites_and_never_guesses_on_denial() {
    for status in [200, 401, 403] {
        let mut server = Server {
            status,
            calls: vec![],
        };
        let result = board_sidebar::load(&mut server);
        assert!(result.is_ok());
        assert_eq!(
            server
                .calls
                .iter()
                .map(|c| (c.0.as_str(), c.1.as_str()))
                .collect::<Vec<_>>(),
            vec![
                ("GET", "/user/context"),
                ("POST", "/api/teams/getAllSidebar"),
                ("GET", "/api/favorites/getFavorites")
            ]
        );
        if let Ok((user, t, f)) = result {
            assert_eq!(t.is_array(), status == 200);
            assert_eq!(f.is_array(), status == 200);
            if status != 200 {
                continue;
            }
            let mut db = Db::memory().unwrap();
            db.kv_set("current-user-id", "42").unwrap();
            db.cache_sidebar(user, &t, &f).unwrap();
            assert_eq!(
                db.projects()
                    .unwrap()
                    .iter()
                    .find(|p| p["id"] == 15)
                    .unwrap()["teamId"],
                "team-3"
            );
            assert_eq!(db.sidebar_data("favorites"), favorites());
            db.kv_set("current-user-id", "43").unwrap();
            assert_eq!(db.sidebar_data("favorites"), json!([]));
            assert!(db.cache_sidebar(user, &t, &f).is_err());
        }
    }
}
#[test]
fn favorite_replacement_is_durable_optimistic_and_rolls_back_401_and_403() {
    for status in [200, 401, 403] {
        let file = tempfile::NamedTempFile::new().unwrap();
        let mut db = Db::open(file.path()).unwrap();
        db.kv_set("current-user-id", "42").unwrap();
        db.cache_sidebar(42, &teams(), &favorites()).unwrap();
        let id = db
            .enqueue(
                Write::Favorites {
                    project_ids: vec![16, 15],
                },
                100,
            )
            .unwrap();
        assert_eq!(db.sidebar_data("favorites")[0]["projectId"], 16);
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
        assert_eq!(
            db.sidebar_data("favorites")[0]["projectId"],
            if status == 200 { 16 } else { 15 }
        );
    }
}

#[test]
fn independent_metadata_refusal_retains_the_other_authoritative_query_and_cached_values() {
    let mut db = Db::memory().unwrap();
    db.kv_set("current-user-id", "42").unwrap();
    db.cache_sidebar(42, &teams(), &favorites()).unwrap();
    db.cache_sidebar(
        42,
        &json!([{"id":"new-team","title":"New","projects":[]}]),
        &Value::Null,
    )
    .unwrap();
    assert_eq!(db.sidebar_data("teams")[0]["id"], "new-team");
    assert_eq!(db.sidebar_data("favorites"), favorites());
    db.cache_sidebar(42, &Value::Null, &json!([])).unwrap();
    assert_eq!(db.sidebar_data("teams")[0]["id"], "new-team");
    assert_eq!(db.sidebar_data("favorites"), json!([]));
}

#[test]
fn ten_slot_favorites_use_web_post_put_reset_and_rollback_after_restart() {
    struct Slots { status: u16, method: &'static str, body: Value }
    impl Transport for Slots {
        fn get(&mut self, path: &str) -> Result<Response, TransportError> {
            assert_eq!(path,"/user/context");
            Ok(Response { status:200, body:json!({"success":true,"user":{"id":42}}) })
        }
        fn post(&mut self, _: &str, _: &Value, _: &str) -> Result<Response,TransportError> {panic!("use method")}
        fn mutate(&mut self, method:&str, path:&str, body:&Value, _: &str) -> Result<Response,TransportError> {
            assert_eq!(method,self.method); assert_eq!(path,"/api/favorites/addEditFavorites"); assert_eq!(body,&self.body);
            Ok(Response { status:self.status, body:if self.status!=200 {json!({"message":"Unauthorized"})}else if method=="POST" {json!({"projectId":body["projectId"],"index":body["index"]})}else{json!({"count":1})} })
        }
    }
    for status in [200,401,403] { for (index, project, method) in [(0,Some(16),"POST"),(1,Some(16),"PUT"),(1,None,"PUT")] {
        let file=tempfile::NamedTempFile::new().unwrap(); let mut db=Db::open(file.path()).unwrap();
        db.kv_set("current-user-id","42").unwrap();db.cache_sidebar(42,&teams(),&favorites()).unwrap();
        assert!(db.enqueue(Write::FavoriteSlot{index,project_id:project},100).is_err());
        assert!(db.cache_sidebar_profile(42,&json!({"id":43,"UserSettingId":9})).is_err());
        db.cache_sidebar_profile(42,&json!({"id":42,"UserSettingId":99})).unwrap();
        let before=db.sidebar_data("favorites");
        let id=db.enqueue(Write::FavoriteSlot{index,project_id:project},100).unwrap();
        let optimistic=db.sidebar_data("favorites");assert_ne!(optimistic,before);
        assert!(db.enqueue(Write::Favorites{project_ids:vec![]},100).is_err());
        drop(db);let mut db=Db::open(file.path()).unwrap();
        let mut server=Slots{status,method,body:json!({"userSettingId":99,"index":index,"projectId":project.unwrap_or(-1)})};
        assert_eq!(db.drain_once(&mut server,100).unwrap(),if status==200{Drain::Applied(id)}else{Drain::Failed(id)});
        assert_eq!(db.sidebar_data("favorites"),if status==200{optimistic}else{before});
    }}
}
