use ht_core::{
    CommentWrite, Db, Drain, TaskPatch, Transport, Write,
    http::{Response, TransportError},
};
use serde_json::{Value, json};

struct Server {
    method: &'static str,
    path: &'static str,
    body: Value,
    response: Value,
}
impl Transport for Server {
    fn get(&mut self, _: &str) -> Result<Response, TransportError> {
        panic!("unexpected GET")
    }
    fn post(&mut self, path: &str, body: &Value, key: &str) -> Result<Response, TransportError> {
        self.mutate("POST", path, body, key)
    }
    fn mutate(
        &mut self,
        method: &str,
        path: &str,
        body: &Value,
        key: &str,
    ) -> Result<Response, TransportError> {
        assert_eq!(method, self.method);
        assert_eq!(path, self.path);
        assert_eq!(body, &self.body);
        assert!(!key.is_empty());
        Ok(Response {
            status: 200,
            body: self.response.clone(),
        })
    }
}
#[test]
fn supported_web_html_survives_queue_restart_and_exact_bearer_wire_requests() {
    let formats = [
        "<p><strong>é🙂 &amp; bold</strong></p>",
        "<p><em>italic</em><u>underline</u></p>",
        "<blockquote><p>quote</p></blockquote>",
        "<p><code>inline</code></p>",
        "<pre><code class=\"language-javascript\">const x = 1;\nnext</code></pre>",
        "<ul><li><p>bullet</p></li></ul>",
        "<ol class=\"numbered-list\"><li><p>ordered</p></li></ol>",
        "<ul class=\"taskList\" data-type=\"taskList\"><li data-checked=\"false\" data-type=\"taskItem\"><label><input type=\"checkbox\"><span></span></label><div><p>task</p></div></li></ul>",
        "<h1>one</h1><h2>two</h2><h3>three</h3><h4>four</h4><h5>five</h5><h6>six</h6><p>normal<br>line</p>",
        "<p><a href=\"https://example.com\" rel=\"noopener noreferrer nofollow\" target=\"_blank\">link</a></p>",
    ];
    for html in formats {
        for surface in 0..3 {
            let file = tempfile::NamedTempFile::new().unwrap();
            let mut db = Db::open(file.path()).unwrap();
            db.apply_task(&json!({"id":1,"title":"Task","projectId":15,"status":"Normal"}))
                .unwrap();
            db.apply_comments(1,&json!({"success":true,"comments":[{"id":7,"text":"<p>Original</p>","creatorId":42,"createdAt":"2026-01-01T00:00:00Z"}]})).unwrap();
            db.kv_set("current-user-id", "42").unwrap();
            let (write, method, path, body, response) = match surface {
                0 => (
                    Write::TaskComment {
                        task_id: 1,
                        text: html.into(),
                    },
                    "POST",
                    "/comments",
                    json!({"task_id":1,"text":html,"kind":"qa","content_type":"html"}),
                    json!({"success":true,"comment":{"id":8,"text":html,"taskId":1,"creatorId":42,"createdAt":"2026-01-01T00:00:00Z"}}),
                ),
                1 => (
                    Write::CommentAction {
                        task_id: 1,
                        comment_id: 7,
                        change: CommentWrite::Edit { text: html.into() },
                    },
                    "PATCH",
                    "/comments/7",
                    json!({"text":html,"content_type":"html"}),
                    json!({"success":true,"comment":{"id":7,"text":html}}),
                ),
                _ => (
                    Write::TaskPatch {
                        task_id: 1,
                        patch: TaskPatch::Description(html.into()),
                    },
                    "POST",
                    "/tasks/update",
                    json!({"task_id":1,"description":html,"content_type":"html"}),
                    json!({"success":true,"tasks":[{"id":1,"title":"Task","projectId":15,"status":"Normal","description":html}]}),
                ),
            };
            let id = db.enqueue(write, 100).unwrap();
            let client_id = db.mutations().unwrap()[0].client_id.clone();
            drop(db);
            let mut db = Db::open(file.path()).unwrap();
            assert_eq!(db.mutations().unwrap()[0].client_id, client_id);
            assert_eq!(
                db.drain_once(
                    &mut Server {
                        method,
                        path,
                        body,
                        response
                    },
                    101
                )
                .unwrap(),
                Drain::Applied(id)
            );
            assert!(db.mutations().unwrap().is_empty());
            if surface == 2 {
                assert_eq!(db.task(1).unwrap().unwrap().description, html);
            } else {
                assert!(
                    db.task_detail(1)
                        .unwrap()
                        .unwrap()
                        .comments
                        .iter()
                        .any(|c| c.text == html)
                );
            }
        }
    }
}

#[test]
fn mcp_sanitizer_losses_are_explicit_blockers_not_faked_round_trips() {
    for html in [
        "<p><mark>highlight</mark></p>",
        "<p><mark data-color=\"#74c0fc\">color</mark></p>",
        "<p><span data-name=\"rocket\" data-type=\"emoji\">🚀</span></p>",
        "<p><a href=\"mailto:hello@example.com\">email</a></p>",
    ] {
        for surface in 0..3 {
            let mut db = Db::memory().unwrap();
            db.kv_set("current-user-id", "42").unwrap();
            db.apply_task(&json!({"id":1,"projectId":15,"title":"Task","status":"Normal","description":"<p>Published</p>"})).unwrap();
            db.apply_comments(1,&json!({"success":true,"comments":[{"id":7,"text":"<p>Original</p>","creatorId":42,"createdAt":"2026-01-01T00:00:00Z"}]})).unwrap();
            let write = match surface {
                0 => Write::TaskComment {
                    task_id: 1,
                    text: html.into(),
                },
                1 => Write::CommentAction {
                    task_id: 1,
                    comment_id: 7,
                    change: CommentWrite::Edit { text: html.into() },
                },
                _ => Write::TaskPatch {
                    task_id: 1,
                    patch: TaskPatch::Description(html.into()),
                },
            };
            let error = db.enqueue(write, 100).unwrap_err().to_string();
            assert!(
                error.contains("MCP") && error.contains("/api/mcp/"),
                "{error}"
            );
            assert!(db.mutations().unwrap().is_empty());
            assert_eq!(db.task(1).unwrap().unwrap().description, "<p>Published</p>");
        }
    }
    for html in [
        "<p><strong>kept</strong></p>",
        "<p>&lt;mark&gt; is plain text</p>",
        "<p><a href=\"https://example.com\">link</a></p>",
    ] {
        let mut db = Db::memory().unwrap();
        db.apply_task(&json!({"id":1,"projectId":15,"title":"Task","status":"Normal"}))
            .unwrap();
        db.enqueue(
            Write::TaskComment {
                task_id: 1,
                text: html.into(),
            },
            100,
        )
        .unwrap();
        assert_eq!(db.mutations().unwrap().len(), 1);
    }
}
