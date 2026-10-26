use super::*;
use crate::ui::shell::tests::{click, draw, measured};
use gpui::{VisualTestContext, WindowHandle};
use ht_core::{Transport, http::{Response, TransportError}};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex, mpsc};
use std::time::Instant;

type Gate = Arc<Mutex<Option<(String, mpsc::Receiver<()>)>>>;

struct Server {
    gate: Gate,
    requests: Arc<Mutex<Vec<String>>>,
    fail_comments: bool,
}

fn task(id: i64) -> Value {
    json!({"id":id,"projectId":15,"uniqueIndex":id,"ticketNumber":format!("NEW-{id}"),
        "title":format!("Server task {id}"),"description":"<p>Server description</p>",
        "sectionId":51,"section":"In progress","boardTitle":"Server board","status":"Normal",
        "createdBy":{"id":42,"displayName":"Server author"},"ai_summary":null,
        "assignees":[{"id":43,"displayName":"Server assignee"}],
        "labels":[{"id":71,"name":"Server tag"}],"updatedAt":"2026-10-20T12:00:00Z"})
}

impl Transport for Server {
    fn get(&mut self, path: &str) -> std::result::Result<Response, TransportError> {
        self.requests.lock().unwrap().push(path.into());
        let gate = {
            let mut gate = self.gate.lock().unwrap();
            if gate.as_ref().is_some_and(|(expected, _)| path.starts_with(expected)) {
                gate.take().map(|(_, receive)| receive)
            } else { None }
        };
        if let Some(gate) = gate { gate.recv_timeout(Duration::from_secs(10)).unwrap(); }
        let body = if path == "/inbox/list" {
            json!({"success":true,"user_notifications":[{"id":901,"taskId":101,"projectId":15,
                "status":"Normal","seen":true,"type":"TaskMention","createdAt":"2026-10-20T12:00:00Z",
                "fromUser":{"id":42,"displayName":"Server author"},
                "project":{"id":15,"title":"Server board"},"task":{
                    "id":101,"title":"Server task 101","ticketNumber":"NEW-101","projectId":15,"section":"In progress",
                    "taskLabels":[{"label":{"id":71,"value":"Server tag"}}]}}]})
        } else if path.starts_with("/projects?limit=") {
            json!({"success":true,"projects":[{"id":15,"title":"Server board","status":"Normal","ownerId":42}],"total":1})
        } else if path == "/projects/15/sections?include_hidden=false" {
            json!({"success":true,"projectId":15,"sections":[{"id":51,"projectId":15,
                "section_title":"In progress","ranking":"a","visibility":true,"deleted":false}]})
        } else if path == "/projects/15/members" {
            json!({"success":true,"projectId":15,"members":[{"id":43,"displayName":"Server assignee"}]})
        } else if path == "/projects/15/labels" {
            json!({"success":true,"projectId":15,"labels":[{"id":71,"name":"Server tag"}]})
        } else if path == "/user/context" {
            json!({"success":true,"user":{"id":42}})
        } else if path.starts_with("/comments?") {
            if self.fail_comments { return Err(TransportError::Offline); }
            let id = if path.contains("task_id=102&") {102} else {101};
            json!({"success":true,"comments":[{"id":id+1000,"taskId":id,"text":"<p>Server comment</p>",
                "createdAt":"2026-10-20T12:00:00Z","creatorId":42,"creator":{"id":42,"displayName":"Server author"}}],"total":1})
        } else if path.starts_with("/drafts?") {
            let id = if path.ends_with("=102") {102} else {101};
            json!({"success":true,"drafts":[{"id":201,"taskId":id,"draftType":"comment",
                "text":"<p>Server draft</p>","createdBy":{"id":42,"displayName":"Server author"}}]})
        } else if path.starts_with("/tasks?task_id=") {
            let ids = path.split_once('=').unwrap().1.split(',').map(|id|task(id.parse().unwrap())).collect::<Vec<_>>();
            json!({"success":true,"tasks":ids})
        } else if path.starts_with("/tasks?project_id=15&") {
            let tasks = if path.contains("status=Normal") {[101,102].into_iter().map(|id| {
                let mut task = task(id);
                task.as_object_mut().unwrap().remove("createdBy");
                task.as_object_mut().unwrap().remove("ai_summary");
                task
            }).collect::<Vec<_>>()} else {vec![]};
            json!({"success":true,"total":tasks.len(),"tasks":tasks})
        } else if path.starts_with("/tasks?search=") || path == "/tasks?status=Normal&limit=100&offset=0" {
            json!({"success":true,"tasks":[{"id":102,"projectId":15,"status":"Normal","title":"Server task 102"}],"total":1})
        } else { panic!("unexpected first-run request {path}"); };
        Ok(Response { status:200, body })
    }
    fn post(&mut self, path: &str, _: &Value, _: &str) -> std::result::Result<Response, TransportError> {
        panic!("read-only walkthrough must not mutate {path}")
    }
}

struct Harness {
    view: WindowHandle<Desktop>,
    cx: VisualTestContext,
    gate: Gate,
    requests: Arc<Mutex<Vec<String>>>,
    directory: PathBuf,
}

fn setup(cx: &mut gpui::TestAppContext, fail_comments: bool) -> Harness {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let directory = std::env::temp_dir().join(format!("ht-first-run-{}-{}",std::process::id(),
        NEXT.fetch_add(1,std::sync::atomic::Ordering::Relaxed)));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("local.db");
    let gate = Arc::new(Mutex::new(None));
    let requests = Arc::new(Mutex::new(vec![]));
    let worker = Worker::start(&path, Server {gate:gate.clone(),requests:requests.clone(),fail_comments}).unwrap();
    cx.update(crate::input::init);
    let view = cx.open_window(gpui::size(px(1440.),px(900.)), |window,cx| {
        let options = Options::parse(["--offline","--screen","inbox","--board","15","--db",path.to_str().unwrap()].map(str::to_owned)).unwrap();
        let mut view = Desktop::new(Db::open(&path).unwrap(),options,window,cx);
        assert!(view.db.inbox().unwrap().is_empty());
        assert!(view.db.projects().unwrap().is_empty());
        assert!(view.db.task(101).unwrap().is_none());
        view.worker = Some(worker);
        view.layout = Some(Rc::new(RefCell::new(LayoutDump::default())));
        view.ready_scheduled = true;
        window.focus(&view.focus,cx);
        view
    });
    let visual = VisualTestContext::from_window(view.into(),cx);
    Harness {view,cx:visual,gate,requests,directory}
}

impl Harness {
    fn block(&self, path: &str) -> mpsc::Sender<()> {
        let (send,receive) = mpsc::channel();
        assert!(self.gate.lock().unwrap().replace((path.into(),receive)).is_none());
        send
    }
    fn wait(&mut self, ready: impl Fn(&Desktop) -> bool) {
        let deadline = Instant::now()+Duration::from_secs(10);
        loop {
            let ready = self.view.update(&mut self.cx, |this,_,_| {this.reload();ready(this)}).unwrap();
            if ready {break;}
            assert!(Instant::now()<deadline,"first-run fetch did not complete; requests={:?}",self.requests.lock().unwrap());
            std::thread::sleep(Duration::from_millis(10));
        }
        draw(self.view,&mut self.cx);
        self.view.read_with(&self.cx,|this,_| {
            assert!(!this.layout.as_ref().unwrap().borrow().interaction_elements().iter()
                .any(|e|e["text"].as_str().is_some_and(|s|s.contains("not available offline"))));
        }).unwrap();
    }
    fn related_ready(this: &Desktop) -> bool {
        this.db.current_user_id()==Some(42) && !this.db.projects().unwrap().is_empty()
            && !this.db.sections(15).unwrap().is_empty()
            && this.db.comment_draft(this.options.task).unwrap().is_some()
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        self.view.update(&mut self.cx,|this,_,_|drop(this.worker.take())).unwrap();
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

#[gpui::test]
fn fresh_signin_inbox_row_board_card_walkthrough_fetches_real_content(cx: &mut gpui::TestAppContext) {
    let mut h = setup(cx,false);
    let send = h.block("/inbox/list");
    // Sign-in installs the worker then enters Inbox through this same request boundary.
    h.view.update(&mut h.cx,|this,_,_|this.request_sync()).unwrap();
    assert_eq!(measured(h.view,&mut h.cx,"inbox.empty")["text"],"Loading...");
    send.send(()).unwrap();
    h.wait(|this| !this.inbox.is_empty() && this.collection_load.is_none() && !this.db.projects().unwrap().is_empty());
    assert_eq!(measured(h.view,&mut h.cx,"inbox.row.0.title")["text"],"Server task 101");
    assert_eq!(measured(h.view,&mut h.cx,"inbox.row.0.author")["text"],"Server author");
    assert_eq!(measured(h.view,&mut h.cx,"inbox.row.0.meta")["text"],"Server tag");
    let send = h.block("/tasks?task_id=101");
    click(h.view,&mut h.cx,"inbox.row.0");
    assert_eq!(measured(h.view,&mut h.cx,"task.unavailable")["text"],"Loading...");
    assert_eq!(measured(h.view,&mut h.cx,"ai.context")["text"],"Loading...");
    send.send(()).unwrap();
    h.wait(|this| this.task_load.is_none() && task_ready(this,101));
    assert_eq!(measured(h.view,&mut h.cx,"task.title")["text"],"Server task 101");
    assert_eq!(measured(h.view,&mut h.cx,"task.description.author")["text"],"Server author");
    for (row,expected) in [(0,"Server assignee"),(3,"In progress"),(5,"Server board"),(8,"Server tag")] {
        assert_eq!(measured(h.view,&mut h.cx,&format!("task.property.{row}.value"))["text"],expected);
    }
    h.view.read_with(&h.cx,|this,cx| {
        assert_eq!(this.comment_input.read(cx).content.as_ref(),"Server draft");
        assert_eq!(this.detail.as_ref().unwrap().comments[0].text,"<p>Server comment</p>");
    }).unwrap();
    let send = h.block("/projects/15/sections");
    click(h.view,&mut h.cx,"rail.icon.3");
    send.send(()).unwrap();
    h.wait(|this| this.collection_load.is_none() && this.columns.iter().map(|c|c.cards.len()).sum::<usize>()==2);
    assert_eq!(measured(h.view,&mut h.cx,"board.title")["text"],"Server board");
    assert_eq!(measured(h.view,&mut h.cx,"ai.context")["text"],"Server board");
    assert_eq!(measured(h.view,&mut h.cx,"board.card.0.title")["text"],"Server task 101");
    click(h.view,&mut h.cx,"board.card.0");
    h.wait(|this| task_ready(this,101));
    assert_eq!(measured(h.view,&mut h.cx,"task.title")["text"],"Server task 101");
    let db = Db::open(h.directory.join("local.db")).unwrap();
    assert_eq!(db.task(101).unwrap().unwrap().board_title,"Server board");
    assert_eq!(db.inbox().unwrap().len(),1);
    assert_eq!(db.board(15).unwrap()[0].cards.len(),2);
}

fn task_ready(this: &Desktop,id:i64) -> bool {
    this.detail.as_ref().is_some_and(|d|d.task.id==id && !d.comments.is_empty()) && Harness::related_ready(this)
}

#[gpui::test]
fn never_synced_board_loads_instead_of_blank_and_embeds_uncached_metadata(cx: &mut gpui::TestAppContext) {
    let mut h = setup(cx,false);
    let send = h.block("/projects/15/sections");
    click(h.view,&mut h.cx,"rail.icon.3");
    assert_eq!(measured(h.view,&mut h.cx,"board.loading")["text"],"Loading board…");
    send.send(()).unwrap();
    h.wait(|this|this.collection_load.is_none() && !this.columns.is_empty() && !this.db.projects().unwrap().is_empty()
        && !this.db.project_members(15).unwrap().is_empty());
    assert_eq!(measured(h.view,&mut h.cx,"board.title")["text"],"Server board");
    assert_eq!(measured(h.view,&mut h.cx,"ai.context")["text"],"Server board");
    h.view.read_with(&h.cx,|this,_| {
        let task = &this.columns[0].cards[0];
        assert_eq!(task.extra["createdBy"]["displayName"],"Server author");
        assert_eq!(task.extra["assignees"][0]["displayName"],"Server assignee");
        assert_eq!(task.extra["labels"][0]["name"],"Server tag");
        assert_eq!(task.section,"In progress");
        assert_eq!(task.board_title,"Server board");
    }).unwrap();
}

#[gpui::test]
fn fresh_task_loads_catalog_and_draft_even_when_comments_fail(cx: &mut gpui::TestAppContext) {
    let mut h = setup(cx,true);
    h.view.update(&mut h.cx,|this,_,cx|this.open_task(101,cx)).unwrap();
    h.wait(Harness::related_ready);
    assert_eq!(measured(h.view,&mut h.cx,"task.title")["text"],"Server task 101");
    h.view.read_with(&h.cx,|this,cx| {
        assert_eq!(this.comment_input.read(cx).content.as_ref(),"Server draft");
        assert_eq!(this.db.project_labels(15).unwrap()[0]["name"],"Server tag");
        assert_eq!(this.db.project_members(15).unwrap()[0]["displayName"],"Server assignee");
    }).unwrap();
    assert!(h.requests.lock().unwrap().iter().any(|p|p.starts_with("/comments?task_id=101")));
}

#[gpui::test]
fn fresh_search_fetches_uncached_result_then_opens_full_task(cx: &mut gpui::TestAppContext) {
    let mut h = setup(cx,false);
    let send = h.block("/tasks?search=");
    click(h.view,&mut h.cx,"rail.icon.7");
    h.cx.simulate_input("Server");
    assert_eq!(measured(h.view,&mut h.cx,"shell.search.empty")["text"],"Loading...");
    send.send(()).unwrap();
    h.wait(|this|this.search_ids.as_ref().is_some_and(|ids|ids==&vec![102]));
    assert_eq!(measured(h.view,&mut h.cx,"shell.search.result.0")["text"],"Server task 102");
    let send = h.block("/tasks?task_id=102");
    click(h.view,&mut h.cx,"shell.search.result.0");
    assert_eq!(measured(h.view,&mut h.cx,"task.unavailable")["text"],"Loading...");
    assert_eq!(measured(h.view,&mut h.cx,"ai.context")["text"],"Current task Server task 102");
    send.send(()).unwrap();
    h.wait(|this|task_ready(this,102));
    assert_eq!(measured(h.view,&mut h.cx,"task.title")["text"],"Server task 102");
    assert_eq!(measured(h.view,&mut h.cx,"ai.context")["text"],"NEW-102 Server task 102");
}

#[gpui::test]
fn fresh_command_search_fetches_and_keyboard_opens_uncached_task(cx: &mut gpui::TestAppContext) {
    let mut h = setup(cx,false);
    let send = h.block("/tasks?search=");
    h.cx.simulate_keystrokes("ctrl-k");
    h.cx.simulate_input("Search");
    h.cx.simulate_keystrokes("enter");
    h.cx.simulate_input("Server");
    assert_eq!(measured(h.view,&mut h.cx,"shell.search.empty")["text"],"Loading...");
    send.send(()).unwrap();
    h.wait(|this|this.search_ids.as_ref().is_some_and(|ids|ids==&vec![102]));
    h.cx.simulate_keystrokes("enter");
    h.wait(|this|task_ready(this,102));
    assert_eq!(measured(h.view,&mut h.cx,"task.title")["text"],"Server task 102");
    assert_eq!(measured(h.view,&mut h.cx,"ai.context")["text"],"NEW-102 Server task 102");
}

#[gpui::test]
fn fresh_task_waits_for_draft_owner_before_allowing_comment_input(cx: &mut gpui::TestAppContext) {
    let mut h = setup(cx,false);
    let send = h.block("/user/context");
    h.view.update(&mut h.cx,|this,_,cx|this.open_task(101,cx)).unwrap();
    h.wait(|this|this.task_load.is_none() && this.detail.is_some());
    assert_eq!(measured(h.view,&mut h.cx,"task.title")["text"],"Server task 101");
    h.cx.simulate_keystrokes("ctrl-m");
    h.view.read_with(&h.cx,|this,_| {
        assert!(this.db.current_user_id().is_none());
        assert!(this.editing == Editing::None);
        assert!(!this.layout.as_ref().unwrap().borrow().interaction_elements().iter().any(|e|e["key"]=="task.composer"));
    }).unwrap();
    send.send(()).unwrap();
    h.wait(|this|task_ready(this,101));
    measured(h.view,&mut h.cx,"task.composer");
    h.view.read_with(&h.cx,|this,cx|assert_eq!(this.comment_input.read(cx).content.as_ref(),"Server draft")).unwrap();
}
