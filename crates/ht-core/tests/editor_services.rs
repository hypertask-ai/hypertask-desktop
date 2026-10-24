use ht_core::{
    Db, DraftType, Drain, ImproveCommand, Sync, Transport, Write,
    http::{Response, TransportError},
};
use serde_json::{Value, json};
use std::collections::VecDeque;

struct Server {
    calls: VecDeque<(&'static str, String, Value, u16, Value)>,
    keys: Vec<(String, String)>,
}
impl Server {
    fn new(calls: Vec<(&'static str, &str, Value, u16, Value)>) -> Self {
        Self {
            calls: calls
                .into_iter()
                .map(|(m, p, b, s, r)| (m, p.into(), b, s, r))
                .collect(),
            keys: vec![],
        }
    }
    fn request(
        &mut self,
        method: &str,
        path: &str,
        body: &Value,
        key: &str,
    ) -> Result<Response, TransportError> {
        let (m, p, b, status, response) = self.calls.pop_front().expect("unexpected request");
        assert_eq!((method, path, body), (m, p.as_str(), &b));
        if method != "GET" {
            assert!(!key.is_empty());
            self.keys.push((path.into(), key.into()));
        }
        Ok(Response {
            status,
            body: response,
        })
    }
}
impl Transport for Server {
    fn get(&mut self, path: &str) -> Result<Response, TransportError> {
        self.request("GET", path, &Value::Null, "")
    }
    fn post(&mut self, path: &str, body: &Value, key: &str) -> Result<Response, TransportError> {
        self.request("POST", path, body, key)
    }
    fn mutate(
        &mut self,
        m: &str,
        path: &str,
        body: &Value,
        key: &str,
    ) -> Result<Response, TransportError> {
        self.request(m, path, body, key)
    }
}
fn seed(db: &mut Db) {
    db.kv_set("current-user-id", "42").unwrap();
    db.apply_task(&json!({"id":1,"projectId":15,"title":"Task","status":"Normal","description":"<p>Published</p>"})).unwrap();
}
fn drafts() -> Value {
    json!({"success":true,"drafts":[
        {"id":7,"taskId":1,"draftType":"comment","text":"<p>Comment draft</p>","createdBy":{"id":42}},
        {"id":8,"taskId":1,"draftType":"description","text":"<p>Description draft</p>","createdBy":{"id":42}}
    ]})
}
#[test]
fn all_web_ai_dropdown_commands_use_same_prompt_contract_through_durable_bearer_queue() {
    for (command, name) in [
        (ImproveCommand::Grammar, "FixSpellingAndGrammar"),
        (ImproveCommand::Readability, "ImproveReadability"),
        (ImproveCommand::Summarize, "Summarize"),
        (ImproveCommand::MakeShorter, "MakeShorter"),
        (ImproveCommand::English, "Translate:English"),
        (ImproveCommand::German, "Translate:German"),
        (ImproveCommand::Dutch, "Translate:Dutch"),
        (ImproveCommand::Spanish, "Translate:Spanish"),
        (ImproveCommand::French, "Translate:French"),
    ] {
        let file = tempfile::NamedTempFile::new().unwrap();
        let mut db = Db::open(file.path()).unwrap();
        seed(&mut db);
        let source = "<p>Original <strong>é🙂</strong></p>";
        let improved = "<p><em>Improved é🙂</em></p>";
        let id = db
            .enqueue(
                Write::EditorImprove {
                    task_id: 1,
                    command,
                    text: source.into(),
                },
                100,
            )
            .unwrap();
        let key = db.mutations().unwrap()[0].client_id.clone();
        drop(db);
        let mut db = Db::open(file.path()).unwrap();
        let body = json!({"task_id":1,"project_id":15,"command":name,"text":source});
        let mut server = Server::new(vec![
            (
                "POST",
                "/ai/improve",
                body.clone(),
                429,
                json!({"success":false}),
            ),
            (
                "POST",
                "/ai/improve",
                body,
                200,
                json!({"success":true,"html":improved,"command":name}),
            ),
        ]);
        assert_eq!(
            db.drain_once(&mut server, 100).unwrap(),
            Drain::Retrying(id)
        );
        assert!(db.editor_result(id).unwrap().is_none());
        assert_eq!(db.drain_once(&mut server, 101).unwrap(), Drain::Applied(id));
        assert_eq!(server.keys[0].1, key);
        assert_eq!(server.keys[0], server.keys[1]);
        assert!(server.calls.is_empty());
        drop(db);
        let db = Db::open(file.path()).unwrap();
        let result = db.editor_result(id).unwrap().unwrap();
        assert_eq!(result.source, source);
        assert_eq!(result.html.as_deref(), Some(improved));
        assert_eq!(result.task_id, 1);
        assert_eq!(db.task(1).unwrap().unwrap().description, "<p>Published</p>");
    }
}
#[test]
fn ai_denial_and_invalid_success_never_pretend_to_improve_or_publish_content() {
    for (status, response, failed) in [
        (403, json!({"success":false}), true),
        (200, json!({"success":true,"command":"Summarize"}), false),
    ] {
        let mut db = Db::memory().unwrap();
        seed(&mut db);
        let id = db
            .enqueue(
                Write::EditorImprove {
                    task_id: 1,
                    command: ImproveCommand::Summarize,
                    text: "<p>Source</p>".into(),
                },
                100,
            )
            .unwrap();
        let mut server = Server::new(vec![(
            "POST",
            "/ai/improve",
            json!({"task_id":1,"project_id":15,"command":"Summarize","text":"<p>Source</p>"}),
            status,
            response,
        )]);
        assert_eq!(
            db.drain_once(&mut server, 100).unwrap(),
            if failed {
                Drain::Failed(id)
            } else {
                Drain::Retrying(id)
            }
        );
        if failed {
            let result = db.editor_result(id).unwrap().unwrap();
            assert!(result.html.is_none());
            assert_eq!(result.error.as_deref(), Some("HTTP 403"));
        } else {
            assert!(db.editor_result(id).unwrap().is_none());
        }
        assert_eq!(db.task(1).unwrap().unwrap().description, "<p>Published</p>");
    }
}
#[test]
fn discard_resolves_fresh_own_ids_after_restart_and_preserves_other_draft_type() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let mut db = Db::open(file.path()).unwrap();
    seed(&mut db);
    let mut sync = Sync::new(
        db,
        Server::new(vec![(
            "GET",
            "/drafts?task_id=1",
            Value::Null,
            200,
            drafts(),
        )]),
    );
    sync.drafts(1).unwrap();
    let mut db = sync.db;
    let id = db
        .enqueue(
            Write::DiscardDraft {
                task_id: 1,
                draft_type: DraftType::Comment,
            },
            100,
        )
        .unwrap();
    assert_eq!(db.editor_drafts(1).unwrap().len(), 1);
    assert_eq!(db.task(1).unwrap().unwrap().extra["hasCommentDraft"], false);
    drop(db);
    let mut db = Db::open(file.path()).unwrap();
    let mut fresh = drafts();
    fresh["drafts"][0]["id"] = json!(77);
    let mut server = Server::new(vec![
        ("GET", "/drafts?task_id=1", Value::Null, 200, fresh),
        (
            "DELETE",
            "/drafts/77",
            json!({}),
            200,
            json!({"success":true}),
        ),
    ]);
    assert_eq!(db.drain_once(&mut server, 101).unwrap(), Drain::Applied(id));
    assert!(server.calls.is_empty());
    assert_eq!(db.editor_drafts(1).unwrap()[0]["draftType"], "description");
    assert_eq!(db.task(1).unwrap().unwrap().description, "<p>Published</p>");
    assert!(
        db.refetches()
            .unwrap()
            .iter()
            .any(|(target, task, _)| target == "drafts" && *task == 1)
    );
}
#[test]
fn discard_denial_restores_cached_draft_and_never_deletes_another_account() {
    for wrong_owner in [false, true] {
        let mut db = Db::memory().unwrap();
        seed(&mut db);
        let mut sync = Sync::new(
            db,
            Server::new(vec![(
                "GET",
                "/drafts?task_id=1",
                Value::Null,
                200,
                drafts(),
            )]),
        );
        sync.drafts(1).unwrap();
        let mut db = sync.db;
        let id = db
            .enqueue(
                Write::DiscardDraft {
                    task_id: 1,
                    draft_type: DraftType::Comment,
                },
                100,
            )
            .unwrap();
        let mut response = drafts();
        if wrong_owner {
            response["drafts"][0]["createdBy"]["id"] = json!(43);
        }
        let mut calls = vec![("GET", "/drafts?task_id=1", Value::Null, 200, response)];
        if !wrong_owner {
            calls.push((
                "DELETE",
                "/drafts/7",
                json!({}),
                403,
                json!({"success":false}),
            ));
        }
        let mut server = Server::new(calls);
        assert_eq!(db.drain_once(&mut server, 100).unwrap(), Drain::Failed(id));
        assert!(server.calls.is_empty());
        assert_eq!(db.editor_drafts(1).unwrap().len(), 2);
        assert_eq!(db.task(1).unwrap().unwrap().extra["hasCommentDraft"], true);
    }
}

#[test]
fn publishing_cleans_the_matching_draft_only_after_server_success_not_on_denial() {
    for success in [false, true] {
        let mut db = Db::memory().unwrap();
        seed(&mut db);
        let mut sync = Sync::new(
            db,
            Server::new(vec![(
                "GET",
                "/drafts?task_id=1",
                Value::Null,
                200,
                drafts(),
            )]),
        );
        sync.drafts(1).unwrap();
        let mut db = sync.db;
        let id = db
            .enqueue(
                Write::TaskComment {
                    task_id: 1,
                    text: "<p>Publish</p>".into(),
                },
                100,
            )
            .unwrap();
        let mut server = Server::new(vec![(
            "POST",
            "/comments",
            json!({"task_id":1,"text":"<p>Publish</p>","kind":"qa","content_type":"html"}),
            if success { 200 } else { 403 },
            if success {
                json!({"success":true,"comment":{"id":77,"text":"<p>Publish</p>","createdAt":"2026-01-01T00:00:00Z"}})
            } else {
                json!({"success":false})
            },
        )]);
        assert_eq!(
            db.drain_once(&mut server, 101).unwrap(),
            if success {
                Drain::Applied(id)
            } else {
                Drain::Failed(id)
            }
        );
        if success {
            let rows = db.mutations().unwrap();
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].kind, "draft_discard");
            assert_eq!(rows[0].payload_json["draft_type"], "comment");
            assert_eq!(db.editor_drafts(1).unwrap().len(), 1);
        } else {
            assert_eq!(db.editor_drafts(1).unwrap().len(), 2);
            assert!(
                db.mutations()
                    .unwrap()
                    .iter()
                    .all(|r| r.kind != "draft_discard")
            );
        }
    }
}
