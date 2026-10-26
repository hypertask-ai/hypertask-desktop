use super::*;
use crate::ui::shell::tests::{click, draw, measured};
use gpui::{VisualTestContext, WindowHandle};
use ht_core::{Transport, http::{Response, TransportError}};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex, mpsc};

struct TaskServer {
    response: Option<mpsc::Receiver<std::result::Result<Value, TransportError>>>,
    requests: Arc<Mutex<Vec<String>>>,
}

impl Transport for TaskServer {
    fn get(&mut self, path: &str) -> std::result::Result<Response, TransportError> {
        self.requests.lock().unwrap().push(path.into());
        let body = match path {
            "/tasks?task_id=101" => self.response.take().ok_or(TransportError::Offline)?
                .recv_timeout(Duration::from_secs(10)).unwrap()?,
            "/projects?limit=100&offset=0" => json!({"success":true,"projects":[],"total":0}),
            "/projects/15/sections?include_hidden=false" => json!({"success":true,"projectId":15,"sections":[]}),
            "/projects/15/members" => json!({"success":true,"projectId":15,"members":[]}),
            "/projects/15/labels" => json!({"success":true,"projectId":15,"labels":[]}),
            "/user/context" => json!({"success":true,"user":{"id":42}}),
            "/comments?task_id=101&limit=50&offset=0&sort_order=desc" => json!({"success":true,"comments":[],"total":0}),
            "/drafts?task_id=101" => json!({"success":true,"drafts":[]}),
            _ => return Err(TransportError::InvalidResponse),
        };
        Ok(Response { status: 200, body })
    }

    fn post(&mut self, _: &str, _: &Value, _: &str) -> std::result::Result<Response, TransportError> {
        panic!("opening a task must not mutate the server")
    }
}

fn setup(
    cx: &mut gpui::TestAppContext,
) -> (
    WindowHandle<Desktop>,
    VisualTestContext,
    mpsc::Sender<std::result::Result<Value, TransportError>>,
    Arc<Mutex<Vec<String>>>,
    PathBuf,
) {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let directory = std::env::temp_dir().join(format!("ht-task-open-{}-{}", std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("local.db");
    let (send, receive) = mpsc::channel();
    let requests = Arc::new(Mutex::new(vec![]));
    let worker = Worker::start(&path, TaskServer { response: Some(receive), requests: requests.clone() }).unwrap();
    cx.update(crate::input::init);
    let view = cx.open_window(gpui::size(px(1440.), px(900.)), |window, cx| {
        let options = Options::parse(["--offline", "--screen", "inbox", "--db", path.to_str().unwrap()]
            .map(String::from)).unwrap();
        let mut view = Desktop::new(Db::open(&path).unwrap(), options, window, cx);
        view.worker = Some(worker);
        view.layout = Some(Rc::new(RefCell::new(LayoutDump::default())));
        view.ready_scheduled = true;
        window.focus(&view.focus, cx);
        view
    });
    let visual = VisualTestContext::from_window(view.into(), cx);
    (view, visual, send, requests, directory)
}

fn response() -> Value {
    let fixture: Value = serde_json::from_str(include_str!("../../../fixtures/task-fidelity.json")).unwrap();
    json!({"success":true,"tasks":[fixture["detail"]["tasks"][0].clone()]})
}

fn finish(view: WindowHandle<Desktop>, cx: &mut VisualTestContext) {
    let worker = view.update(cx, |this, _, _| this.worker.take()).unwrap();
    drop(worker);
    view.update(cx, |this, _, cx| { this.reload(); cx.notify(); }).unwrap();
    draw(view, cx);
}

#[gpui::test]
fn opening_uncached_task_fetches_renders_and_caches_server_response(cx: &mut gpui::TestAppContext) {
    let (view, mut cx, send, requests, directory) = setup(cx);
    view.update(&mut cx, |this, _, cx| {
        assert!(this.db.task(101).unwrap().is_none());
        this.db.apply_response(&json!({"success":true,"user_notifications":[{
            "id":901,"taskId":101,"projectId":15,"status":"Normal","seen":true,
            "createdAt":"2026-10-20T12:00:00Z",
            "task":{"title":"Human-authored task","ticketNumber":"FID-101"}
        }]})).unwrap();
        this.reload();
        this.split = this.splits.iter().position(|split| split == "All").unwrap();
        this.filter_inbox();
        cx.notify();
    }).unwrap();
    click(view, &mut cx, "inbox.row.0");
    assert_eq!(measured(view, &mut cx, "task.unavailable")["text"], "Loading...");
    let response = response();
    let title = response["tasks"][0]["title"].clone();
    send.send(Ok(response)).unwrap();
    finish(view, &mut cx);
    assert_eq!(measured(view, &mut cx, "task.title")["text"], title);
    view.read_with(&mut cx, |this, _| {
        assert_eq!(this.db.task(101).unwrap().unwrap().title, title.as_str().unwrap());
        assert!(this.task_load.is_none());
    }).unwrap();
    assert_eq!(requests.lock().unwrap().iter().filter(|p| *p == "/tasks?task_id=101").count(), 1);
    assert!(Db::open(directory.join("local.db")).unwrap().task(101).unwrap().is_some());
}

#[gpui::test]
fn failed_uncached_task_request_shows_offline_only_after_failure(cx: &mut gpui::TestAppContext) {
    let (view, mut cx, send, requests, _) = setup(cx);
    view.update(&mut cx, |this, _, cx| this.open_task(101, cx)).unwrap();
    assert_eq!(measured(view, &mut cx, "task.unavailable")["text"], "Loading...");
    send.send(Err(TransportError::Offline)).unwrap();
    finish(view, &mut cx);
    assert_eq!(measured(view, &mut cx, "task.unavailable")["text"],
        "Task is not available offline. Sync this task first.".chars().take(40).collect::<String>());
    assert!(requests.lock().unwrap().iter().any(|p| p == "/tasks?task_id=101"));
    view.read_with(&mut cx, |this, _| assert!(this.db.task(101).unwrap().is_none())).unwrap();
}

#[gpui::test]
fn late_task_response_caches_without_replacing_newer_task(cx: &mut gpui::TestAppContext) {
    let (view, mut cx, send, _, _) = setup(cx);
    view.update(&mut cx, |this, _, cx| {
        let fixture: Value = serde_json::from_str(include_str!("../../../fixtures/task-fidelity.json")).unwrap();
        this.db.apply_response(&json!({"success":true,"tasks":[fixture["detail"]["tasks"][1].clone()]})).unwrap();
        this.open_task(101, cx);
        this.open_task(102, cx);
    }).unwrap();
    assert!(measured(view, &mut cx, "task.title")["text"].is_string());
    send.send(Ok(response())).unwrap();
    finish(view, &mut cx);
    view.read_with(&mut cx, |this, _| {
        assert_eq!(this.detail.as_ref().unwrap().task.id, 102);
        assert!(this.db.task(101).unwrap().is_some());
        assert!(this.task_load.is_none());
    }).unwrap();
}
