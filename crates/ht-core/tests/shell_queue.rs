use ht_core::{Db, Drain, ShellWrite, Transport, Write, http::{Response, TransportError}};
use serde_json::{Value, json};

struct Server { status: u16, body: Value, method: String, path: String, sent: Value, user: i64, keys: Vec<String> }
impl Transport for Server {
    fn get(&mut self, path: &str) -> Result<Response, TransportError> {
        assert_eq!(path, "/user/context");
        Ok(Response { status:200, body:json!({"success":true,"user":{"id":self.user}}) })
    }
    fn post(&mut self, path: &str, body: &Value, key: &str) -> Result<Response, TransportError> { self.mutate("POST", path, body, key) }
    fn mutate(&mut self, method: &str, path: &str, body: &Value, key: &str) -> Result<Response, TransportError> {
        assert_eq!(method, self.method); assert_eq!(path, self.path); assert_eq!(body, &self.sent);
        assert!(uuid::Uuid::parse_str(key).is_ok()); self.keys.push(key.into());
        Ok(Response { status:self.status, body:self.body.clone() })
    }
}

#[test]
fn shell_writes_survive_restart_use_exact_bearer_contracts_and_consume_real_acknowledgements() {
    for (method, path, body) in [
        ("PATCH", "/user/profile", json!({"displayName":"Updated"})),
        ("PUT", "/projects/15/instructions", json!({"custom_instruction":"Guide"})),
        ("POST", "/projects/15/members", json!({"userToAdd":"member@example.com"})),
        ("POST", "/skills", json!({"markdown":"Skill","scope":"user"})),
        ("PATCH", "/skills/2", json!({"enabled":false})),
        ("DELETE", "/skills/2", Value::Null),
        ("POST", "/skills/import", json!({"url":"https://github.com/example/skills","dry_run":true})),
        ("POST", "/webhooks", json!({"project_id":15,"url":"https://example.com/hook","events":["task.created"]})),
        ("POST", "/webhooks/wh1/test", json!({})),
        ("DELETE", "/webhooks?id=wh1", Value::Null),
        ("POST", "/projects/archive", json!({"project_id":15,"status":"Archive"})),
    ] {
        let file = tempfile::NamedTempFile::new().unwrap();
        let mut db = Db::open(file.path()).unwrap();
        let id = db.enqueue(Write::Shell { request:ShellWrite::new(42, method, path, Some(body.clone())).unwrap() }, 100).unwrap();
        let key = db.mutations().unwrap()[0].client_id.clone();
        assert!(db.take_shell_result(&key).unwrap().is_none());
        drop(db);
        let mut db = Db::open(file.path()).unwrap();
        let response = json!({"success":true,"project":{"id":15,"status":"Archive"},"secret":"one-time-fixture"});
        let mut server = Server { status:503, body:response.clone(), method:method.into(), path:path.into(), sent:body, user:42, keys:vec![] };
        assert_eq!(db.drain_once(&mut server,100).unwrap(), Drain::Retrying(id));
        assert!(db.take_shell_result(&key).unwrap().is_none());
        server.status = 200;
        assert_eq!(db.drain_once(&mut server,101).unwrap(), Drain::Applied(id));
        assert_eq!(server.keys, vec![key.clone(), key.clone()]);
        assert!(db.mutations().unwrap().is_empty());
        assert_eq!(db.take_shell_result(&key).unwrap().unwrap()["value"], response);
        assert!(db.take_shell_result(&key).unwrap().is_none());
        if path == "/projects/archive" { assert_eq!(db.projects().unwrap()[0]["status"], "Archive"); }
    }
}

#[test]
fn shell_queue_denial_and_account_switch_never_become_success_or_mutate_the_new_account() {
    for user in [42, 99] {
        let mut db = Db::memory().unwrap();
        let id = db.enqueue(Write::Shell { request:ShellWrite::new(42,"PATCH","/user/profile",Some(json!({"displayName":"Mine"}))).unwrap() },100).unwrap();
        let key = db.mutations().unwrap()[0].client_id.clone();
        let mut server = Server { status:403, body:json!({"success":false}), method:"PATCH".into(),path:"/user/profile".into(),sent:json!({"displayName":"Mine"}),user,keys:vec![] };
        assert_eq!(db.drain_once(&mut server,100).unwrap(), Drain::Failed(id));
        assert_eq!(server.keys.len(), usize::from(user == 42));
        let result = db.take_shell_result(&key).unwrap().unwrap();
        assert_eq!(result["error"], "HTTP 403"); assert!(result["value"].is_null());
        assert_eq!(db.mutations().unwrap()[0].status,"failed");
    }
}

#[test]
fn shell_queue_rejects_cookie_routes_and_route_injection_before_enqueue() {
    assert!(ShellWrite::new(42,"PATCH","/user/profile",Some(json!({"displayName":"Valid"}))).is_ok());
    for (method,path) in [
        ("POST","/users/preferences"),("POST","/api/users/preferences"),("POST","//evil.example/path"),
        ("POST","/projects/../members"),("POST","/projects/0/members"),("DELETE","/webhooks?id=wh1&agent_id=9"),
        ("POST","/webhooks/wh1/../test"),("POST","/ai/chat/stream"),("GET","/user/profile"),
    ] { assert!(ShellWrite::new(42,method,path,Some(json!({}))).is_err(), "{method} {path}"); }
    assert!(ShellWrite::new(0,"PATCH","/user/profile",Some(json!({}))).is_err());
}
