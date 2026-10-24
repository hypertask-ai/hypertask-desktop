use ht_core::{Db, Drain, Transport, ViewWrite, Write, http::{Response, TransportError}};
use serde_json::{Value, json};

struct Server {
    identity: Option<Result<Response, TransportError>>,
    writes: usize,
    lose_reply: bool,
}
impl Transport for Server {
    fn get(&mut self, path: &str) -> Result<Response, TransportError> {
        assert_eq!(path, "/user/context");
        self.identity.take().unwrap_or_else(|| Ok(Response { status: 200, body: json!({"success":true,"user":{"id":42}}) }))
    }
    fn post(&mut self, path: &str, body: &Value, key: &str) -> Result<Response, TransportError> {
        self.mutate("POST", path, body, key)
    }
    fn mutate(&mut self, method: &str, path: &str, body: &Value, key: &str) -> Result<Response, TransportError> {
        self.writes += 1;
        assert!(!key.is_empty());
        if self.lose_reply { return Err(TransportError::Timeout); }
        let reply = match path {
            "/api/tasks/create" => {assert_eq!(method,"POST"); json!({"id":51,"projectId":15,"sectionId":10,"title":body["title"],"status":"Normal"})},
            "/api/projects/views/create-view" => json!({"project_view_updated":{"id":"pv","projectId":15}}),
            "/api/projects/views/unsaved-view" => json!({"id":"pv","projectId":15}),
            "/api/favorites/addEditFavorites" => json!({"projectId":body["projectId"],"index":body["index"]}),
            "/api/favorites/reorderFavorites" => {assert_eq!(method,"PUT"); json!({"ok":true})},
            _ => panic!("unexpected {path}"),
        };
        Ok(Response { status:200, body:reply })
    }
}
fn seed(db: &mut Db) {
    db.kv_set("current-user-id","42").unwrap();
    db.apply_response(&json!({"success":true,"projects":[{"id":15,"title":"Board","project_view":{"id":"pv","projectId":15,"user_project_views":[{}]}}],"sections":[{"id":10,"projectId":15,"section_title":"Todo","deleted":false,"visibility":true,"ranking":"a"}]})).unwrap();
    db.cache_sidebar_profile(42,&json!({"id":42,"UserSettingId":99})).unwrap();
}
fn writes() -> Vec<Write> {
    vec![
        Write::CreateTask { project_id:15, section_id:10, title:"New task".into(), top:true },
        Write::View { change:ViewWrite { project:15, route:"create-view".into(), body:json!({"projectId":15,"viewTitle":"Daily","setAsDefault":false,"visibility":"Private","view_settings":{}}) } },
        Write::View { change:ViewWrite { project:15, route:"unsaved-view".into(), body:json!({"projectId":15,"board_sorting_mode":"Title","baseViewId":null}) } },
        Write::FavoriteSlot { index:0, project_id:Some(15) },
        Write::Favorites { project_ids:vec![15] },
    ]
}
#[test]
fn identity_outage_is_not_authorization_denial_and_never_spends_a_create_attempt() {
    for write in writes() {
        for outage in [Err(TransportError::Offline), Err(TransportError::Timeout), Ok(Response {status:500,body:json!({"message":"unavailable"})}), Ok(Response {status:429,body:json!({"message":"rate limited"})})] {
            let file=tempfile::NamedTempFile::new().unwrap();
            let mut db=Db::open(file.path()).unwrap(); seed(&mut db);
            let id=db.enqueue(write.clone(),100).unwrap();
            let mut server=Server { identity:Some(outage), writes:0, lose_reply:false };
            assert_eq!(db.drain_once(&mut server,100).unwrap(),Drain::Retrying(id));
            assert_eq!(server.writes,0);
            assert_eq!(db.mutations().unwrap()[0].status,"pending");
            drop(db); let mut db=Db::open(file.path()).unwrap();
            assert_eq!(db.drain_once(&mut server,1000).unwrap(),Drain::Applied(id));
            assert_eq!(server.writes,1);
        }
    }
}
#[test]
fn uncertain_create_acknowledgement_is_never_replayed_after_restart() {
    for write in writes().into_iter().take(2) {
        let file=tempfile::NamedTempFile::new().unwrap();
        let mut db=Db::open(file.path()).unwrap(); seed(&mut db);
        let id=db.enqueue(write,100).unwrap();
        let mut server=Server { identity:None, writes:0, lose_reply:true };
        let first=db.drain_once(&mut server,100).unwrap();
        assert!(matches!(first,Drain::Retrying(_) | Drain::Failed(_)));
        assert_eq!(server.writes,1);
        drop(db); let mut db=Db::open(file.path()).unwrap();
        let next=db.drain_once(&mut server,1000).unwrap();
        assert!(next==Drain::Failed(id) || next==Drain::Empty);
        assert_eq!(server.writes,1);
    }
}
