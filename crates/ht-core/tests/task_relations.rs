use ht_core::{
    Db, Transport,
    http::{Response, TransportError},
    task_relations,
};
use serde_json::{Value, json};

struct Server {
    status: u16,
    projects_status: u16,
    query: String,
    calls: usize,
}
impl Transport for Server {
    fn get(&mut self, path: &str) -> Result<Response, TransportError> {
        if path == "/api/projects/getAllMinimal?mode=ExtraMinimal" {
            return Ok(Response {
                status: self.projects_status,
                body: json!([{"id":15,"title":"Board"}]),
            });
        }
        assert_eq!(path, "/api/tasks/single?id=1");
        Ok(Response {
            status: self.status,
            body: json!({"id":1,"relatedFromTasks":[{"id":7,"sourceTaskId":1,"targetTaskId":2,"relationType":"BlockedBy","targetTask":{"id":2,"projectId":15,"uniqueIndex":2,"title":"Other"}}],"relatedToTasks":[]}),
        })
    }
    fn post(&mut self, path: &str, body: &Value, key: &str) -> Result<Response, TransportError> {
        assert_eq!(path, "/api/tasks/searchAll");
        assert!(!key.is_empty());
        self.calls += 1;
        assert_eq!(
            *body,
            if self.query.is_empty() {
                json!({"projectIds":[15],"mode":"recent","currentTaskId":1})
            } else {
                json!({"projectIds":[15],"searchQuery":self.query})
            }
        );
        Ok(Response {
            status: self.status,
            body: json!([{"id":1,"projectId":15,"uniqueIndex":1,"title":"Current"},{"id":2,"projectId":15,"uniqueIndex":2,"title":"Other"}]),
        })
    }
}
#[test]
fn relation_picker_uses_web_authoritative_boards_recent_and_search_requests_and_rejects_refusals() {
    for query in ["", "Other"] {
        for status in [200, 401, 403] {
            let mut server = Server {
                status,
                projects_status: 200,
                query: query.into(),
                calls: 0,
            };
            let result = task_relations::search(&mut server, 1, query);
            if status == 200 {
                let rows = result.unwrap();
                assert_eq!(rows.len(), 1);
                assert_eq!(rows[0]["id"], 2);
            } else {
                assert!(result.is_err());
            }
            assert_eq!(server.calls, 1);
            let mut server = Server {
                status: 200,
                projects_status: status,
                query: query.into(),
                calls: 0,
            };
            let result = task_relations::search(&mut server, 1, query);
            assert_eq!(result.is_ok(), status == 200);
            assert_eq!(server.calls, if status == 200 { 1 } else { 0 });
        }
    }
}
#[test]
fn task_relation_hydration_accepts_both_directions_preserves_edits_and_refuses_auth_failures() {
    for status in [200, 401, 403] {
        let mut server = Server {
            status,
            projects_status: 200,
            query: String::new(),
            calls: 0,
        };
        let result = task_relations::details(&mut server, 1);
        assert_eq!(result.is_ok(), status == 200);
        if status == 200 {
            let mut db = Db::memory().unwrap();
            db.kv_set("current-user-id", "42").unwrap();
            db.apply_task(&json!({"id":1,"title":"Keep title","projectId":15,"status":"Normal","description":"Keep description"})).unwrap();
            db.cache_task_relations(42, 1, &result.unwrap()).unwrap();
            let task = db.task(1).unwrap().unwrap();
            assert_eq!(task.title, "Keep title");
            assert_eq!(task.description, "Keep description");
            assert_eq!(task.extra["relatedFromTasks"][0]["id"], 7);
            assert!(
                db.cache_task_relations(
                    99,
                    1,
                    &json!({"id":1,"relatedFromTasks":[],"relatedToTasks":[]})
                )
                .is_err()
            );
        }
    }
}
#[test]
fn relation_section_titles_match_web_direction_inversion() {
    use ht_core::RelationKind::*;
    for (kind, from, to) in [
        (RelatedTo, "Related", "Related"),
        (BlockedBy, "Blocked by", "Blocks"),
        (BlockedTo, "Blocks", "Blocked by"),
        (Duplicate, "Duplicate of", "Duplicate"),
    ] {
        assert_eq!(kind.section(true), from);
        assert_eq!(kind.section(false), to);
    }
}
