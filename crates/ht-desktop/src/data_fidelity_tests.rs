use super::*;
use crate::ui::shell::tests::{click, draw, measured};
use gpui::VisualTestContext;
use ht_core::{Sync, Transport, http::{Response, TransportError}};
use serde_json::Value;
use std::collections::VecDeque;

struct Fixture(VecDeque<Value>);
impl Transport for Fixture {
    fn get(&mut self, _: &str) -> std::result::Result<Response, TransportError> {
        Ok(Response { status: 200, body: self.0.pop_front().expect("unexpected fixture request") })
    }
    fn post(&mut self, _: &str, _: &Value, _: &str) -> std::result::Result<Response, TransportError> {
        panic!("fidelity rendering must not write to the server")
    }
}

#[gpui::test]
fn synced_fidelity_fixture_renders_real_authors_saved_summary_and_web_draft_markers(
    cx: &mut gpui::TestAppContext,
) {
    cx.update(crate::input::init);
    let view = cx.open_window(gpui::size(px(1440.), px(900.)), |window, cx| {
        let f: Value = serde_json::from_str(include_str!("../../../fixtures/task-fidelity.json")).unwrap();
        let mut sync = Sync::new(Db::memory().unwrap(), Fixture(VecDeque::from([
            f["sections"].clone(), f["list"].clone(), f["detail"].clone(), f["drafts"].clone(),
        ])));
        sync.board(15).unwrap();
        sync.drafts(101).unwrap();
        assert!(sync.transport.0.is_empty());
        let options = Options::parse([
            "--offline", "--screen", "task", "--task", "101", "--board", "15",
        ].map(String::from)).unwrap();
        let mut view = Desktop::new(sync.db, options, window, cx);
        view.layout = Some(Rc::new(RefCell::new(LayoutDump::default())));
        view.ready_scheduled = true;
        window.focus(&view.focus, cx);
        view
    });
    let mut cx = VisualTestContext::from_window(view.into(), cx);
    draw(view, &mut cx);
    assert_eq!(measured(view, &mut cx, "task.summary.toggle")["text"], "Stored summary, not generated.");
    assert_eq!(measured(view, &mut cx, "task.description.author")["text"], "Real Human");
    let marker = measured(view, &mut cx, "task.description.draft");
    assert_eq!(marker["bg"], "#c2cfa5");
    assert_eq!(marker["w"], 4.0);
    assert!(marker["text"].is_null());
    view.read_with(&mut cx, |this, _| {
        assert!(!this.layout.as_ref().unwrap().borrow().interaction_elements().iter()
            .any(|e| e["key"] == "task.composer.draft"));
    }).unwrap();
    let teaser = measured(view, &mut cx, "task.summary.toggle");
    assert_eq!(teaser["fontSize"], 13.0);
    assert_eq!(teaser["fontWeight"], 500.0);
    assert_eq!(teaser["lineHeight"], 19.5);
    view.read_with(&mut cx, |this, _| {
        assert_eq!(this.summary.as_ref(), "Stored summary, not generated.");
        assert_eq!(this.detail.as_ref().unwrap().task.description, "<p>Published description</p>");
        assert_eq!(full_task_summary(&this.detail.as_ref().unwrap().task.extra),
            "## Summary\n- **Stored** summary, not generated. [1]\n- Another item.");
        assert!(this.db.mutations().unwrap().is_empty());
    }).unwrap();
    cx.simulate_keystrokes("i");
    assert_eq!(measured(view, &mut cx, "task.summary.full")["text"],
        "## Summary\n- **Stored** summary, not generated. [1]\n- Another item."
            .chars().take(40).collect::<String>());
    let expanded = measured(view, &mut cx, "task.summary.full");
    let header = measured(view, &mut cx, "task.header");
    let content = measured(view, &mut cx, "task.summary.content");
    assert_eq!(expanded["x"], header["x"]);
    assert_eq!(expanded["w"], header["w"]);
    assert_eq!(content["x"], teaser["x"]);
    let column = measured(view, &mut cx, "task.summary.column");
    assert!((content["w"].as_f64().unwrap() - column["w"].as_f64().unwrap() * 0.65).abs() < 1.0);
    assert!((teaser["w"].as_f64().unwrap() - column["w"].as_f64().unwrap() * 0.65).abs() < 1.0);
    cx.simulate_keystrokes("escape");
    view.update(&mut cx, |this, window, cx| {
        this.open_task(102, cx);
        window.focus(&this.focus, cx);
    }).unwrap();
    draw(view, &mut cx);
    assert_eq!(measured(view, &mut cx, "task.description.author")["text"], "Actual Runner");
    view.read_with(&mut cx, |this, _| assert!(this.summary.is_empty())).unwrap();
    view.update(&mut cx, |this, _, cx| this.open_task(103, cx)).unwrap();
    draw(view, &mut cx);
    assert_eq!(measured(view, &mut cx, "task.description.author")["text"], "");
    view.update(&mut cx, |this, _, cx| this.switch(Screen::Board, cx)).unwrap();
    draw(view, &mut cx);
    let badge = measured(view, &mut cx, "board.card.0.draft");
    assert_eq!(badge["text"], "Draft");
    assert_eq!(badge["color"], "#c2cfa5");
    assert_eq!(badge["fontWeight"], 500.0);
    assert_eq!(badge["lineHeight"], 22.0);
    view.read_with(&mut cx, |this, _| {
        assert_eq!(this.columns[0].cards[0].extra["createdBy"]["displayName"], "Real Human");
        assert!(this.db.mutations().unwrap().is_empty());
    }).unwrap();
}

#[test]
fn creator_prefers_the_agent_not_the_agents_owner_and_handles_missing_agent_names() {
    for (extra, expected) in [
        (serde_json::json!({"createdBy":{"displayName":"Owner","agent":{"displayName":"Runner"}}}), "Runner"),
        (serde_json::json!({"agent":{"displayName":"Runner"},"user":{"displayName":"Owner"}}), "Runner"),
        (serde_json::json!({"agent":null,"createdBy":{"displayName":"Human"}}), "Human"),
        (serde_json::json!({"createdBy":{"displayName":"","email":"human@example.test"}}), ""),
        (serde_json::json!({}), ""),
    ] {
        assert_eq!(creator_name(extra.as_object().unwrap()), expected);
    }
}

#[gpui::test]
fn synced_fidelity_refresh_removes_markers_and_keeps_web_summary_presence(
    cx: &mut gpui::TestAppContext,
) {
    cx.update(crate::input::init);
    let view = cx.open_window(gpui::size(px(1440.), px(900.)), |window, cx| {
        let f: Value = serde_json::from_str(include_str!("../../../fixtures/task-fidelity.json")).unwrap();
        let mut sync = Sync::new(Db::memory().unwrap(), Fixture(VecDeque::from([
            f["sections"].clone(), f["list"].clone(), f["detail"].clone(), f["drafts"].clone(),
        ])));
        sync.board(15).unwrap();
        sync.drafts(101).unwrap();
        let options = Options::parse([
            "--offline", "--screen", "task", "--task", "101", "--board", "15",
        ].map(String::from)).unwrap();
        let mut view = Desktop::new(sync.db, options, window, cx);
        view.layout = Some(Rc::new(RefCell::new(LayoutDump::default())));
        view.ready_scheduled = true;
        window.focus(&view.focus, cx);
        view
    });
    let mut cx = VisualTestContext::from_window(view.into(), cx);
    draw(view, &mut cx);
    for (drafts, description, _comment, board) in [
        (serde_json::json!([]), false, false, false),
        (serde_json::json!([{"id":1,"taskId":101,"draftType":"description","text":"<p>Private draft</p>"}]), true, false, false),
        (serde_json::json!([{"id":2,"taskId":101,"draftType":"comment","text":"   "}]), false, false, true),
    ] {
        view.update(&mut cx, |this, _, _| {
            let db = std::mem::replace(&mut this.db, Db::memory().unwrap());
            let mut sync = Sync::new(db, Fixture(VecDeque::from([
                serde_json::json!({"success":true,"drafts":drafts}),
            ])));
            sync.drafts(101).unwrap();
            assert!(sync.transport.0.is_empty());
            this.db = sync.db;
            this.reload();
        }).unwrap();
        draw(view, &mut cx);
        view.read_with(&mut cx, |this, _| {
            let elements = this.layout.as_ref().unwrap().borrow().interaction_elements();
            for (key, visible) in [("task.description.draft", description), ("task.composer.draft", false)] {
                assert_eq!(elements.iter().any(|e| e["key"] == key), visible, "{key}");
            }
        }).unwrap();
        assert_eq!(measured(view, &mut cx, "task.description.author")["text"], "Real Human");
        view.update(&mut cx, |this, _, cx| this.switch(Screen::Board, cx)).unwrap();
        draw(view, &mut cx);
        view.read_with(&mut cx, |this, _| {
            let elements = this.layout.as_ref().unwrap().borrow().interaction_elements();
            assert_eq!(elements.iter().any(|e| e["key"] == "board.card.0.draft"), board);
            assert!(!elements.iter().any(|e| e["key"].as_str().is_some_and(|key|
                key.starts_with("board.card.") && (key.ends_with(".author") || key.ends_with(".summary")))));
        }).unwrap();
        view.update(&mut cx, |this, window, cx| {
            this.open_task(101, cx);
            window.focus(&this.focus, cx);
        }).unwrap();
    }
    for summary in [
        Some(serde_json::json!({"content":"#","updated_at":"2026-10-20T13:00:00Z"})),
        Some(Value::Null),
        Some(serde_json::json!({"content":"#","updated_at":"2026-10-20T13:00:00Z"})),
        None,
    ] {
        let present = summary.as_ref().is_some_and(|s| !s.is_null());
        view.update(&mut cx, |this, _, _| {
            let f: Value = serde_json::from_str(include_str!("../../../fixtures/task-fidelity.json")).unwrap();
            let mut task = f["detail"]["tasks"][0].clone();
            match summary {
                Some(summary) => task["ai_summary"] = summary,
                None => { task.as_object_mut().unwrap().remove("ai_summary"); }
            }
            let db = std::mem::replace(&mut this.db, Db::memory().unwrap());
            let mut sync = Sync::new(db, Fixture(VecDeque::from([
                serde_json::json!({"success":true,"tasks":[task]}),
            ])));
            sync.task(101).unwrap();
            assert!(sync.transport.0.is_empty());
            this.db = sync.db;
            this.reload();
        }).unwrap();
        draw(view, &mut cx);
        view.read_with(&mut cx, |this, _| {
            let elements = this.layout.as_ref().unwrap().borrow().interaction_elements();
            assert_eq!(elements.iter().any(|e| e["key"] == "task.summary.toggle"), present);
        }).unwrap();
        cx.simulate_keystrokes("i");
        if present {
            assert_eq!(measured(view, &mut cx, "task.summary.full")["text"], "#");
            cx.simulate_keystrokes("escape");
            click(view, &mut cx, "task.summary.toggle");
            assert_eq!(measured(view, &mut cx, "task.summary.full")["text"], "#");
            cx.simulate_keystrokes("escape");
        } else {
            draw(view, &mut cx);
            view.read_with(&mut cx, |this, _| {
                let elements = this.layout.as_ref().unwrap().borrow().interaction_elements();
                assert!(!elements.iter().any(|e| e["key"] == "task.summary.full"));
            }).unwrap();
        }
    }
    view.read_with(&mut cx, |this, _| {
        assert_eq!(this.detail.as_ref().unwrap().task.description, "<p>Published description</p>");
        assert!(this.db.mutations().unwrap().is_empty());
    }).unwrap();
}

#[gpui::test]
fn synced_summary_teaser_preserves_unpaired_markers_like_web(cx: &mut gpui::TestAppContext) {
    cx.update(crate::input::init);
    let view = cx.open_window(gpui::size(px(1440.), px(900.)), |window, cx| {
        let f: Value = serde_json::from_str(include_str!("../../../fixtures/task-fidelity.json")).unwrap();
        let mut sync = Sync::new(Db::memory().unwrap(), Fixture(VecDeque::from([
            f["sections"].clone(), f["list"].clone(), f["detail"].clone(),
        ])));
        sync.board(15).unwrap();
        let options = Options::parse(["--offline", "--screen", "task", "--task", "101", "--board", "15"]
            .map(String::from)).unwrap();
        let mut view = Desktop::new(sync.db, options, window, cx);
        view.layout = Some(Rc::new(RefCell::new(LayoutDump::default())));
        view.ready_scheduled = true;
        window.focus(&view.focus, cx);
        view
    });
    let mut cx = VisualTestContext::from_window(view.into(), cx);
    for (markdown, expected) in [
        ("- Keep unmatched* and `tick", "Keep unmatched* and `tick"),
        ("- **Bold** *em* `code` [12]", "Bold em code"),
        ("-   Indented bullet", "Indented bullet"),
        ("### Heading only", "Heading only"),
        ("## Title\nText with **broken", "Text with **broken"),
    ] {
        view.update(&mut cx, |this, _, _| {
            let f: Value = serde_json::from_str(include_str!("../../../fixtures/task-fidelity.json")).unwrap();
            let mut task = f["detail"]["tasks"][0].clone();
            task["ai_summary"]["content"] = serde_json::json!(markdown);
            let db = std::mem::replace(&mut this.db, Db::memory().unwrap());
            let mut sync = Sync::new(db, Fixture(VecDeque::from([
                serde_json::json!({"success":true,"tasks":[task]}),
            ])));
            sync.task(101).unwrap();
            assert!(sync.transport.0.is_empty());
            this.db = sync.db;
            this.reload();
        }).unwrap();
        assert_eq!(measured(view, &mut cx, "task.summary.toggle")["text"], expected);
    }
}

#[gpui::test]
fn data4_task_fields_use_web_names_indices_and_relative_dates(cx: &mut gpui::TestAppContext) {
    use serde_json::json;
    cx.update(crate::input::init);
    let view = cx.open_window(gpui::size(px(1440.), px(900.)), |window, cx| {
        let now = chrono::Local::now().to_rfc3339();
        let mut sync = Sync::new(Db::memory().unwrap(), Fixture(VecDeque::from([
            json!({"success":true,"tasks":[{"id":101,"projectId":15,"sectionId":10,"section":"Doing","boardTitle":"Real board","status":"Normal","title":"Read actual fields","ticketNumber":"real-42","createdAt":now,
                "createdBy":{"email":"not-a-display-name@example.test"},
                "priority":{"priority_index":2,"Priority_Value":"Wrong label"},
                "estimate":{"estimate_index":3,"estimate_value":"Wrong size"},
                "dueDate":"2030-01-02T13:00:00Z","labels":[{"id":"tag","name":"Real tag"}],
                "assignees":[{"id":1,"email":"not-a-name@example.test"},{"id":2,"displayName":"Person Name"},{"id":3,"displayName":"Owner","agent":{"id":"runner","displayName":"Agent Name"}}]}]}),
            json!({"success":true,"comments":[{"id":7,"taskId":101,"text":"<p>Real comment</p>","createdAt":now,"agent":{"displayName":"Live agent"},"agent_display_name":"Old stored name","creator":{"displayName":"Owner"}}],"total":1}),
        ])));
        sync.task(101).unwrap();
        sync.comments(101).unwrap();
        let options = Options::parse(["--offline","--screen","task","--task","101","--board","15"].map(String::from)).unwrap();
        let mut view = Desktop::new(sync.db, options, window, cx);
        view.layout = Some(Rc::new(RefCell::new(LayoutDump::default())));
        view.ready_scheduled = true;
        view
    });
    let mut cx = VisualTestContext::from_window(view.into(), cx);
    draw(view, &mut cx);
    assert_eq!(measured(view, &mut cx, "task.description.author")["text"], "");
    assert_eq!(measured(view, &mut cx, "task.description.timestamp")["text"], "Just now");
    for (index, expected) in [(0,"Person Name"),(1,"Agent Name"),(2,"REAL-42"),(3,"Doing"),(5,"Real board"),(6,"High"),(7,"Small"),(8,"Real tag")] {
        assert_eq!(measured(view, &mut cx, &if index == 2 { "task.id".into() } else { format!("task.property.{index}.value") })["text"], expected);
    }
    let due = chrono::DateTime::parse_from_rfc3339("2030-01-02T13:00:00Z").unwrap().with_timezone(&chrono::Local).format("%b %d, %Y").to_string();
    assert_eq!(measured(view, &mut cx, "task.property.4.value")["text"], due);
    assert_eq!(measured(view, &mut cx, "task.comment.0.author")["text"], "Live agent");
    assert_eq!(measured(view, &mut cx, "task.comment.0.timestamp")["text"], "Just now");
    view.update(&mut cx, |this, _, cx| {
        this.db.apply_task(&json!({"id":101,"projectId":15,"status":"Normal","title":"Read actual fields","ticketNumber":null,"createdBy":{"displayName":"First Middle Last"},"assignees":[],"priority":null,"estimate":null,"dueDate":null,"labels":[]})).unwrap();
        this.db.apply_comments(101,&json!({"success":true,"comments":[{"id":8,"taskId":101,"text":"<p>Anonymous</p>","createdAt":"2025-01-02T13:00:00Z","creator":{"email":"no-name@example.test"}}]})).unwrap();
        this.reload();cx.notify();
    }).unwrap();
    draw(view, &mut cx);
    for (index, expected) in [(0,"The Assignees"),(1,"No agents"),(4,"-"),(6,"No Priority"),(7,"-"),(8,"No tags")] {
        assert_eq!(measured(view, &mut cx, &if index == 2 { "task.id".into() } else { format!("task.property.{index}.value") })["text"], expected);
    }
    view.read_with(&mut cx, |this, _| assert!(!this.layout.as_ref().unwrap().borrow().interaction_elements().iter().any(|e|e["key"]=="task.id"))).unwrap();
    assert_eq!(measured(view, &mut cx, "task.comment.0.author")["text"], "");
    assert_eq!(measured(view, &mut cx, "task.comment.0.timestamp")["text"], "2 Jan 2025");
    view.read_with(&mut cx, |this, _| {
        assert_eq!(this.creator_initials.as_ref(), "FL");
        assert!(this.db.mutations().unwrap().is_empty());
    }).unwrap();
}

#[gpui::test]
fn data4_board_shows_real_assignees_size_and_three_tags(cx: &mut gpui::TestAppContext) {
    use serde_json::json;
    cx.update(crate::input::init);
    let view = cx.open_window(gpui::size(px(1440.), px(900.)), |window, cx| {
        let task = json!({"id":101,"projectId":15,"sectionId":10,"section":"Doing","status":"Normal","title":"Board fields","ticketNumber":"REAL-1",
            "priority":{"priority_index":2,"Priority_Value":"Wrong"},"estimate":{"estimate_index":3,"estimate_value":"Wrong"},
            "dueDate":"2030-01-02T13:00:00Z","totalComments":4,
            "assignees":[{"id":1,"displayName":"First Person"},{"id":2,"displayName":"Owner","agent":{"id":"runner","displayName":"Real Runner"}}],
            "labels":[{"id":"1","name":"One"},{"id":"empty","name":""},{"id":"2","name":"Two"},{"id":"3","name":"Three"},{"id":"4","name":"Four"}]});
        let mut sync = Sync::new(Db::memory().unwrap(),Fixture(VecDeque::from([
            json!({"success":true,"projectId":15,"sections":[{"id":10,"projectId":15,"section_title":"Doing","ranking":"a","visibility":true,"deleted":false}]}),
            json!({"success":true,"tasks":[task],"total":1}),json!({"success":true,"tasks":[task]}),
        ])));
        sync.board(15).unwrap();
        let options=Options::parse(["--offline","--screen","board","--board","15"].map(String::from)).unwrap();
        let mut view=Desktop::new(sync.db,options,window,cx);
        view.layout=Some(Rc::new(RefCell::new(LayoutDump::default())));view.ready_scheduled=true;view
    });
    let mut cx=VisualTestContext::from_window(view.into(),cx);
    draw(view,&mut cx);
    for (key,text) in [("priority","High"),("size","S"),("label","One"),("label.1","Two"),("label.2","Three"),("labels.more","+1"),("assignee.0","First Person"),("agent.0","Real Runner"),("comments","4")] {
        assert_eq!(measured(view,&mut cx,&format!("board.card.0.{key}"))["text"],text);
    }
    view.read_with(&mut cx,|this,_| {
        let elements=this.layout.as_ref().unwrap().borrow().interaction_elements();
        assert!(!elements.iter().any(|e|e["key"]=="board.card.0.label.3"));
        assert_eq!(this.columns[0].cards[0].assignee_count,2);
        assert!(this.db.mutations().unwrap().is_empty());
    }).unwrap();
}

#[gpui::test]
fn data4_inbox_renders_web_kinds_actors_indicators_and_metadata(cx: &mut gpui::TestAppContext) {
    use serde_json::json;
    cx.update(crate::input::init);
    let view = cx.open_window(gpui::size(px(1440.),px(900.)), |window,cx| {
        let options=Options::parse(["--offline","--screen","inbox"].map(String::from)).unwrap();
        let mut view=Desktop::new(Db::memory().unwrap(),options,window,cx);
        view.layout=Some(Rc::new(RefCell::new(LayoutDump::default())));view.ready_scheduled=true;view
    });
    let mut cx=VisualTestContext::from_window(view.into(),cx);
    let base=json!({"id":100,"taskId":101,"userId":42,"status":"Normal","seen":false,"createdAt":"2026-10-20T12:00:00Z","unreadCount":3,
        "fromUser":{"displayName":"Owner"},"fromAgent":{"displayName":"Real actor"},
        "comment":{"text":"<blockquote>Old quote</blockquote><p><span data-type=\"mention\" data-label=\"name-1\">Old author</span> said:</p><p>Fresh &amp; real</p>"},
        "task":{"id":101,"title":"Real task","ticketNumber":"REAL-1","status":"Normal","section":"Review","dueDate":"2030-01-02T13:00:00Z",
            "priority":{"priority_index":2,"Priority_Value":"Wrong"},"estimate":{"estimate_index":3,"estimate_value":"Wrong"},
            "taskLabels":[{"label":{"value":"One"}},{"label":{"value":"Two"}},{"label":{"value":"Three"}},{"label":{"value":"Four"}}]}});
    for (kind, patch, expected) in [
        ("Assigned",json!({}),"Assigned to you"),
        ("Comment",json!({}),"Fresh & real"),
        ("TaskArchived",json!({}),"Unarchived this task"),
        ("TaskMoved",json!({}),"Moved to Review"),
        ("AddedToFollowerInTask",json!({}),"Added you to followers and mentioned you"),
        ("Mentioned",json!({}),"Mentioned in"),
        ("Mentioned",json!({"commentId":7,"comment":{"text":"<p>Other paragraph</p><p>Hello <span data-type=\"mention\" data-label=\"name-42\">Alex</span> here</p>"}}),"Hello @Alex here"),
        ("Invited",json!({}),"Click or press ENTER to accept invitation"),
        ("TaskDueDate",json!({}),"Due date changed to 02 Jan 2030"),
        ("TaskReminder",json!({}),"Task Reminder"),
        ("TaskReminder",json!({"staleNudgeDays":5}),"No activity for 5 days"),
        ("TaskMovedToInbox",json!({}),"Task moved to inbox"),
        ("TaskOverdue",json!({}),""),
        ("TaskUpdateDescription",json!({}),"Description updated"),
        ("AgentMessage",json!({"message":"\u{001e}ht-agent-message:v1:12345678-1234-1234-1234-123456789012:<p>Actual agent message</p>"}),"Actual agent message"),
        ("Reacted",json!({"reaction":{"emoji":"👍"}}),"Reacted with 👍 on description"),
    ] {
        let mut notification=base.clone(); notification["type"]=json!(kind);
        for (key,value) in patch.as_object().unwrap() {notification[key]=value.clone();}
        view.update(&mut cx,|this,_,cx| {
            let db=std::mem::replace(&mut this.db,Db::memory().unwrap());
            let mut sync=Sync::new(db,Fixture(VecDeque::from([json!({"success":true,"user_notifications":[notification]})])));
            sync.inbox().unwrap();this.db=sync.db;this.reload();
            this.split=this.splits.iter().position(|s|s=="All").unwrap();this.filter_inbox();cx.notify();
        }).unwrap();
        draw(view,&mut cx);
        assert_eq!(measured(view,&mut cx,"inbox.row.0.kind")["text"],expected.chars().take(40).collect::<String>(),"{kind}");
        assert_eq!(measured(view,&mut cx,"inbox.row.0.author")["text"],if kind=="Invited" {"Real actor"} else {"Real actor (3)"});
        if kind == "Invited" {
            view.read_with(&mut cx,|this,_| assert!(!this.layout.as_ref().unwrap().borrow().interaction_elements().iter().any(|e|e["key"]=="inbox.row.0.meta"))).unwrap();
        } else {
            let metadata=measured(view,&mut cx,"inbox.row.0.meta");
            assert!(metadata["text"].as_str().unwrap().starts_with("Jan 02, 2030 · High · S · One · Two"),"{metadata}");
        }
    }
    for (patch,colors,actor) in [
        (json!({"seen":true}),vec![],"Real actor (3)"),
        (json!({"returnedFromReminders":true}),vec!["#c2cfa5"],"Real actor (3)"),
        (json!({"type":"TaskOverdue"}),vec!["#f88f9c"],"Real actor (3)"),
        (json!({"task":{"title":"Real task","savedContent":[{"id":"saved"}],"waitingOnUserId":42}}),vec!["#ffcb33","#7f1d1d"],"Real actor (3)"),
        (json!({"recentActors":[{"displayName":"First"},{"displayName":"Second"}]}),vec!["#5896f1"],"First, Second (3)"),
        (json!({"fromAgent":{"displayName":""}}),vec!["#5896f1"]," (3)"),
    ] {
        let mut notification=base.clone();notification["type"]=json!("Comment");
        for (key,value) in patch.as_object().unwrap(){notification[key]=value.clone();}
        view.update(&mut cx,|this,_,cx| {
            let db=std::mem::replace(&mut this.db,Db::memory().unwrap());
            let mut sync=Sync::new(db,Fixture(VecDeque::from([json!({"success":true,"user_notifications":[notification]})])));
            sync.inbox().unwrap();this.db=sync.db;this.reload();this.filter_inbox();cx.notify();
        }).unwrap();draw(view,&mut cx);
        assert_eq!(measured(view,&mut cx,"inbox.row.0.author")["text"],actor);
        view.read_with(&mut cx,|this,_| {
            let elements=this.layout.as_ref().unwrap().borrow().interaction_elements();
            let actual: Vec<_>=elements.iter().filter(|e|e["key"].as_str().is_some_and(|k|k.starts_with("inbox.row.0.indicator."))).map(|e|e["bg"].as_str().unwrap()).collect();
            assert_eq!(actual,colors);assert!(this.db.mutations().unwrap().is_empty());
        }).unwrap();
    }
}
