use ht_core::{Db, Drain, Transport, Write, http::{Response, TransportError}};
use serde_json::{Value, json};

fn seed(db: &mut Db) {
    db.apply_response(&json!({"success":true,"user_notifications":[
        {"id":1,"taskId":10,"projectId":15,"status":"Normal","seen":false,"unreadCount":3,"createdAt":"2026-01-01T00:00:00Z"},
        {"id":2,"taskId":11,"projectId":15,"status":"Normal","seen":false,"createdAt":"2026-01-02T00:00:00Z"}
    ]})).unwrap();
}
struct Server { path: &'static str, body: Value, status: u16, result: Value }
impl Transport for Server {
    fn get(&mut self, _: &str) -> Result<Response, TransportError> { panic!("unexpected GET") }
    fn post(&mut self, path: &str, body: &Value, id: &str) -> Result<Response, TransportError> {
        assert_eq!(path, self.path);
        assert_eq!(body, &self.body);
        assert!(uuid::Uuid::parse_str(id).is_ok());
        Ok(Response { status: self.status, body: self.result.clone() })
    }
}
#[test]
fn inbox_seen_and_remind_survive_restart_and_send_exact_bearer_contracts() {
    for remind in [false, true] {
        let file = tempfile::NamedTempFile::new().unwrap();
        let mut db = Db::open(file.path()).unwrap();
        seed(&mut db);
        let at = "2026-01-03T09:00:00Z";
        let now = chrono::DateTime::parse_from_rfc3339("2026-01-02T00:00:00Z").unwrap().timestamp();
        let write = if remind { Write::InboxRemind { notification_ids:vec![2,1,2], remind_at:at.into() } } else { Write::InboxSeen {notification_id:1} };
        let id = db.enqueue(write, now).unwrap();
        if remind { assert!(db.inbox().unwrap().is_empty()); }
        else { assert!(db.inbox().unwrap().iter().find(|n|n.id==1).unwrap().seen); assert!(!db.inbox().unwrap().iter().find(|n|n.id==2).unwrap().seen); }
        drop(db);
        let mut db = Db::open(file.path()).unwrap();
        let mut server = Server {
            path:if remind {"/inbox/archive"} else {"/inbox/seen"},
            body:if remind {json!({"notification_ids":[1,2],"remind_at":at})} else {json!({"notification_ids":[1]})},
            status:if remind {202} else {200},
            result:if remind {json!({"success":true,"archived_count":2,"reminder_count":2,"delivery_pending":true})} else {json!({"success":true,"seen_count":1})},
        };
        assert_eq!(db.drain_once(&mut server, now).unwrap(), Drain::Applied(id));
        assert!(db.mutations().unwrap().is_empty());
    }
}
#[test]
fn inbox_seen_and_remind_denials_restore_rows_and_invalid_success_retries() {
    for remind in [false,true] {
        for status in [403,200,503] {
            let mut db=Db::memory().unwrap(); seed(&mut db);
            let at="2026-01-03T09:00:00Z";
            let now=chrono::DateTime::parse_from_rfc3339("2026-01-02T00:00:00Z").unwrap().timestamp();
            let id=db.enqueue(if remind {Write::InboxRemind{notification_ids:vec![1],remind_at:at.into()}} else {Write::InboxSeen{notification_id:1}},now).unwrap();
            let mut server=Server{path:if remind {"/inbox/archive"}else{"/inbox/seen"},body:if remind{json!({"notification_ids":[1],"remind_at":at})}else{json!({"notification_ids":[1]})},status,result:json!({"success":true})};
            assert_eq!(db.drain_once(&mut server,now).unwrap(),if status==403{Drain::Failed(id)}else{Drain::Retrying(id)});
            if status==403 {assert_eq!(db.inbox().unwrap().len(),2);assert!(!db.inbox().unwrap().iter().find(|n|n.id==1).unwrap().seen);}
        }
    }
}
#[test]
fn inbox_reminders_validate_selection_and_time_before_any_optimistic_write() {
    let mut db=Db::memory().unwrap();seed(&mut db);
    for (ids,at) in [(vec![],"2026-01-03T09:00:00Z"),(vec![999],"2026-01-03T09:00:00Z"),(vec![1],"not-a-date"),(vec![1],"1970-01-01T00:00:00Z"),(vec![1],"2030-01-01T00:00:00Z")] {
        assert!(db.enqueue(Write::InboxRemind{notification_ids:ids,remind_at:at.into()},1767312000).is_err());
        assert!(db.mutations().unwrap().is_empty());assert_eq!(db.inbox().unwrap().len(),2);
    }
}
