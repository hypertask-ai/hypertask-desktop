use super::*;
use gpui::{point, size};
use std::{cell::RefCell, rc::Rc};

#[gpui::test]
fn quoted_comment_preserves_attribution_in_request_and_inserts_at_cursor(
    cx: &mut gpui::TestAppContext,
) {
    cx.update(crate::input::init);
    let view = cx.add_window(|window, cx| {
        let focus = cx.focus_handle();
        AiSidebar::new(crate::theme::THEMES[0], None, focus, window, cx)
    });
    view.update(cx,|this,window,cx| {
        this.input.update(cx,|input,cx| input.set_content("Before After".into(),cx));
        this.input.update(cx,|input,cx| {
            use gpui::EntityInputHandler;
            input.set_selected_text_range(7..7,window,cx);
        });
        let html = "<span data-type=\"mention\" data-id=\"42\" data-label=\"name\" text=\"A &amp; B\">A &amp; B</span> said <blockquote><p>Quoted é🙂</p></blockquote><p></p>".to_owned();
        this.insert_comment_quote(html.clone(),window,cx);
        let text = this.input.read(cx).content.to_string();
        assert!(text.starts_with("Before A & B said"));
        assert!(text.ends_with("After"));
        let prompt = this.prompt_body(&text);
        assert!(prompt.contains(&html));
        assert!(prompt.starts_with("<p>Before "));
        let body = ai_chat::request(&prompt,&TurnIds::new(this.chats[0].id.clone()),&ai_chat::session_id(),MODELS[0],&DefaultContext::default(),None,&[]);
        assert_eq!(body["message"],prompt);
        assert!(body["message"].as_str().unwrap().contains("data-id=\"42\""));
        assert!(this.chats[0].messages.is_empty());
        assert!(this.input.focus_handle(cx).is_focused(window));
    }).unwrap();
}

#[gpui::test]
fn response_copy_mouse_action_copies_rendered_unicode_text(cx: &mut gpui::TestAppContext) {
    let (view, cx) = cx.add_window_view(|window, cx| {
        let focus = cx.focus_handle();
        let mut this = AiSidebar::new(crate::theme::THEMES[0], None, focus, window, cx);
        this.capture = Some(Rc::new(RefCell::new(crate::layout::LayoutDump::default())));
        this.chats[0].messages.push(ChatMessage {
            message: Message {
                role: "assistant".into(),
                content: "**Answer** é🙂".into(),
            },
            rich: markdown("**Answer** é🙂", this.colors),
        });
        this
    });
    cx.run_until_parked();
    let element = view.read_with(cx, |this, _| {
        this.capture
            .as_ref()
            .unwrap()
            .borrow()
            .interaction_elements()
            .into_iter()
            .find(|e| e["key"] == "ai.response.0.copy")
            .expect("response copy painted")
    });
    cx.simulate_click(
        point(
            px(element["x"].as_f64().unwrap() as f32 + 2.),
            px(element["y"].as_f64().unwrap() as f32 + 2.),
        ),
        Default::default(),
    );
    cx.update(|_, cx| {
        assert_eq!(
            cx.read_from_clipboard().unwrap().text().unwrap().trim(),
            "Answer é🙂"
        )
    });
}

#[gpui::test]
fn incoming_stream_keeps_user_scroll_position_until_latest_is_clicked(
    cx: &mut gpui::TestAppContext,
) {
    let (view, cx) = cx.add_window_view(|window, cx| {
        let focus = cx.focus_handle();
        let mut this = AiSidebar::new(crate::theme::THEMES[0], None, focus, window, cx);
        this.capture = Some(Rc::new(RefCell::new(crate::layout::LayoutDump::default())));
        let content = "Paragraph with enough text to scroll.\n\n".repeat(100);
        this.chats[0].messages.push(ChatMessage {
            rich: markdown(&content, this.colors),
            message: Message {
                role: "assistant".into(),
                content,
            },
        });
        this
    });
    // A real constrained layout is essential: an unconstrained transcript has no scroll range.
    cx.draw(point(px(0.), px(0.)), size(px(420.), px(600.)), |_, _| {
        view.clone().into_element()
    });
    view.update_in(cx, |this, _, cx| {
        assert!(this.scroll.max_offset().y > px(100.));
        this.scroll.set_offset(point(px(0.), px(-60.)));
        assert!(this.away_from_latest());
        let (send, updates) = mpsc::sync_channel(64);
        this.active = Some(ActiveTurn {
            ids: TurnIds::new(this.chats[0].id.clone()),
            chat: 0,
            assistant: 0,
            updates,
            cancel_reply: None,
            stopping: false,
            client: Client::new("test-token".into()).unwrap(),
        });
        send.send(Update::Event(Event::Status("Running tool…".into())))
            .unwrap();
        send.send(Update::Event(Event::Content("More output".into())))
            .unwrap();
        send.send(Update::Event(Event::Done(ai_chat::Done {
            status: "complete".into(),
            cancelled: false,
            user_message_persisted: true,
            assistant_persisted: true,
        })))
        .unwrap();
        this.poll(cx);
        assert_eq!(this.scroll.offset().y, px(-60.));
        assert!(
            this.chats[0].messages[0]
                .message
                .content
                .ends_with("More output")
        );
    });
    cx.run_until_parked();
    let element = view.read_with(cx, |this, _| {
        this.capture
            .as_ref()
            .unwrap()
            .borrow()
            .interaction_elements()
            .into_iter()
            .find(|e| e["key"] == "ai.latest")
            .expect("scroll-to-latest painted")
    });
    cx.simulate_click(
        point(
            px(element["x"].as_f64().unwrap() as f32 + 2.),
            px(element["y"].as_f64().unwrap() as f32 + 2.),
        ),
        Default::default(),
    );
    cx.run_until_parked();
    view.read_with(cx, |this, _| assert!(!this.away_from_latest()));
}

fn element(view: &Entity<AiSidebar>, cx: &mut gpui::VisualTestContext, key: &str) -> serde_json::Value {
    cx.run_until_parked();
    view.read_with(cx, |this, _| {
        this.capture.as_ref().unwrap().borrow().interaction_elements().into_iter()
            .find(|row| row["key"] == key).unwrap_or_else(|| panic!("missing {key}"))
    })
}

fn click(view: &Entity<AiSidebar>, cx: &mut gpui::VisualTestContext, key: &str) {
    let row = element(view, cx, key);
    cx.simulate_click(point(px(row["x"].as_f64().unwrap() as f32 + 2.), px(row["y"].as_f64().unwrap() as f32 + 2.)), Default::default());
}

#[gpui::test]
fn sidebar_web_header_welcome_composer_and_tall_input_geometry(cx: &mut gpui::TestAppContext) {
    let (view, cx) = cx.add_window_view(|window, cx| {
        let focus = cx.focus_handle();
        let mut this = AiSidebar::new(crate::theme::THEMES[0], None, focus, window, cx);
        this.capture = Some(Rc::new(RefCell::new(crate::layout::LayoutDump::default())));
        this.set_scope(Scope::load(&Db::memory().unwrap(), Screen::Inbox, 0, None), cx);
        this
    });
    cx.draw(point(px(0.), px(0.)), size(px(420.), px(900.)), |_, _| view.clone().into_element());
    assert_eq!(element(&view, cx, "ai.header")["h"], 48.);
    let welcome = element(&view, cx, "ai.welcome");
    let suggestion = element(&view, cx, "ai.suggestion.0");
    assert_eq!(suggestion["x"].as_f64().unwrap() - welcome["x"].as_f64().unwrap(), 40.);
    let composer = element(&view, cx, "ai.composer");
    assert_eq!(composer["x"], 9.); // 1px panel border plus web p-2.
    assert_eq!(composer["w"], 403.);
    view.update_in(cx, |this, _, cx| {
        this.input.update(cx, |input, cx| input.set_content("Line\n".repeat(80), cx));
    });
    cx.draw(point(px(0.), px(0.)), size(px(420.), px(900.)), |_, _| view.clone().into_element());
    assert_eq!(element(&view, cx, "ai.input")["h"], 380.);
    click(&view, cx, "ai.suggestion.0");
    view.read_with(cx, |this, cx| {
        assert!(this.chats[0].error.as_deref().unwrap().contains("offline"));
        assert_eq!(this.input.read(cx).content.as_ref(), "Line\n".repeat(80));
    });
}

#[gpui::test]
fn human_message_copy_and_edit_are_real_mouse_actions(cx: &mut gpui::TestAppContext) {
    cx.update(crate::input::init);
    let (view, cx) = cx.add_window_view(|window, cx| {
        let focus = cx.focus_handle();
        let mut this = AiSidebar::new(crate::theme::THEMES[0], None, focus, window, cx);
        this.capture = Some(Rc::new(RefCell::new(crate::layout::LayoutDump::default())));
        this.chats[0].messages.push(ChatMessage {
            message: Message { role: "human".into(), content: "<p>Hello <strong>é🙂</strong></p>".into() },
            rich: RichText::default(),
        });
        this
    });
    click(&view, cx, "ai.response.0.copy");
    cx.update(|_, cx| assert_eq!(cx.read_from_clipboard().unwrap().text().unwrap().trim(), "Hello é🙂"));
    click(&view, cx, "ai.message.0.edit");
    cx.simulate_keystrokes("end");
    cx.simulate_input(" edited");
    view.read_with(cx, |this, cx| {
        assert_eq!(this.input.read(cx).content.as_ref(), "Hello é🙂 edited");
        assert_eq!(this.chats[0].messages.len(), 1);
        assert!(this.active.is_none());
    });
}

#[gpui::test]
fn idle_escape_keeps_composer_and_ctrl_q_only_returns_from_sidebar(cx: &mut gpui::TestAppContext) {
    cx.update(crate::input::init);
    let (view, cx) = cx.add_window_view(|window, cx| {
        let focus = cx.focus_handle();
        let mut this = AiSidebar::new(crate::theme::THEMES[0], None, focus, window, cx);
        this.focus_composer(window, cx);
        this
    });
    cx.simulate_input("Draft");
    cx.simulate_keystrokes("escape");
    cx.simulate_input(" retained");
    view.read_with(cx, |this, cx| assert_eq!(this.input.read(cx).content.as_ref(), "Draft retained"));
    for mode in [DisplayMode::Floating, DisplayMode::Fullscreen] {
        view.update_in(cx, |this, _, cx| this.set_mode(mode, cx));
        cx.simulate_keystrokes("ctrl-q");
        cx.simulate_input("!");
    }
    view.read_with(cx, |this, cx| assert_eq!(this.input.read(cx).content.as_ref(), "Draft retained!!"));
}

#[gpui::test]
fn cancelled_done_is_not_a_failure_and_refusal_survives_eof(cx: &mut gpui::TestAppContext) {
    let view = cx.add_window(|window, cx| {
        let focus = cx.focus_handle();
        AiSidebar::new(crate::theme::THEMES[0], None, focus, window, cx)
    });
    view.update(cx, |this, _, cx| {
        for partial in ["", "**Partial** é🙂"] {
            this.chats[0].messages = vec![ChatMessage {
                message: Message { role: "assistant".into(), content: partial.into() },
                rich: markdown(partial, this.colors),
            }];
            let (send, updates) = mpsc::sync_channel(64);
            this.active = Some(ActiveTurn {
                ids: TurnIds::new(this.chats[0].id.clone()), chat: 0, assistant: 0, updates,
                cancel_reply: None, stopping: true, client: Client::new("test-token".into()).unwrap(),
            });
            send.send(Update::Event(Event::Done(ai_chat::Done { status: "error".into(), cancelled: true, user_message_persisted: true, assistant_persisted: true }))).unwrap();
            this.poll(cx);
            assert!(this.active.is_none());
            assert!(this.chats[0].error.is_none());
            assert_eq!(this.chats[0].messages[0].message.content, if partial.is_empty() { "Stream cancelled." } else { partial });
        }
        let (send, updates) = mpsc::sync_channel(64);
        this.active = Some(ActiveTurn {
            ids: TurnIds::new(this.chats[0].id.clone()), chat: 0, assistant: 0, updates,
            cancel_reply: None, stopping: false, client: Client::new("test-token".into()).unwrap(),
        });
        send.send(Update::Event(Event::Status("Running".into()))).unwrap();
        send.send(Update::Event(Event::Error("Provider refused this model".into()))).unwrap();
        drop(send);
        this.poll(cx);
        assert_eq!(this.chats[0].error.as_deref(), Some("Provider refused this model"));
        assert!(this.chats[0].status.is_empty());
        assert_eq!(this.chats[0].messages[0].rich.text.trim(), "Partial é🙂");
        assert!(this.active.is_none());
    }).unwrap();
}

#[gpui::test]
fn streaming_status_stays_in_message_and_copy_waits_for_done(cx: &mut gpui::TestAppContext) {
    let (send, updates) = mpsc::sync_channel(64);
    let (view, cx) = cx.add_window_view(|window, cx| {
        let focus = cx.focus_handle();
        let mut this = AiSidebar::new(crate::theme::THEMES[0], None, focus, window, cx);
        this.capture = Some(Rc::new(RefCell::new(crate::layout::LayoutDump::default())));
        this.chats[0].messages.push(ChatMessage {
            message: Message { role: "assistant".into(), content: "Partial".into() },
            rich: markdown("Partial", this.colors),
        });
        this.active = Some(ActiveTurn {
            ids: TurnIds::new(this.chats[0].id.clone()), chat: 0, assistant: 0, updates,
            cancel_reply: None, stopping: false, client: Client::new("test-token".into()).unwrap(),
        });
        this
    });
    send.send(Update::Event(Event::Status("Reading tasks…".into()))).unwrap();
    view.update_in(cx, |this, _, cx| this.poll(cx));
    let status = element(&view, cx, "ai.stream.status");
    let message = element(&view, cx, "ai.message.0");
    assert_eq!(status["text"], "Reading tasks…");
    assert!(status["y"].as_f64().unwrap() >= message["y"].as_f64().unwrap());
    assert!(status["y"].as_f64().unwrap() + status["h"].as_f64().unwrap() <= message["y"].as_f64().unwrap() + message["h"].as_f64().unwrap());
    view.read_with(cx, |this, _| assert!(!this.capture.as_ref().unwrap().borrow().interaction_elements().iter().any(|row| row["key"] == "ai.response.0.copy")));
    send.send(Update::Event(Event::Done(ai_chat::Done { status: "complete".into(), cancelled: false, user_message_persisted: true, assistant_persisted: true }))).unwrap();
    view.update_in(cx, |this, _, cx| this.poll(cx));
    click(&view, cx, "ai.response.0.copy");
    cx.update(|_, cx| assert_eq!(cx.read_from_clipboard().unwrap().text().unwrap().trim(), "Partial"));
}

#[gpui::test]
fn ai_composer_rich_marks_keep_enter_send_and_skills_only(cx: &mut gpui::TestAppContext) {
    cx.update(crate::input::init);
    let (view, cx) = cx.add_window_view(|window, cx| {
        let focus = cx.focus_handle();
        let mut this = AiSidebar::new(crate::theme::THEMES[0], None, focus, window, cx);
        this.focus_composer(window, cx);
        this
    });
    cx.simulate_keystrokes("ctrl-b");
    cx.simulate_input("Bold é🙂");
    cx.simulate_keystrokes("ctrl-b shift-enter");
    cx.simulate_input("Normal");
    view.read_with(cx, |this, cx| {
        let input = this.input.read(cx);
        assert!(input.html().contains("<strong>Bold é🙂</strong>"), "{}", input.html());
        assert_eq!(input.content.as_ref(), "Bold é🙂\nNormal");
    });
    cx.simulate_keystrokes("enter");
    view.read_with(cx, |this, cx| {
        assert!(this.chats[0].error.as_deref().unwrap().contains("offline"));
        assert!(this.input.read(cx).html().contains("<strong>Bold é🙂</strong>"));
        assert!(this.chats[0].messages.is_empty());
    });
}

#[gpui::test]
fn task_summary_mouse_action_sends_exact_web_prompt_and_context(cx: &mut gpui::TestAppContext) {
    use std::io::{BufRead, BufReader, Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let server = thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
        let mut reader = BufReader::new(socket.try_clone().unwrap());
        let mut headers = String::new();
        let mut length = 0;
        loop {
            let mut line = String::new();
            assert!(reader.read_line(&mut line).unwrap() > 0);
            headers.push_str(&line);
            if line == "\r\n" { break; }
            if let Some((name, value)) = line.split_once(':') {
                if name.eq_ignore_ascii_case("content-length") { length = value.trim().parse().unwrap(); }
            }
        }
        let mut body = vec![0; length];
        reader.read_exact(&mut body).unwrap();
        socket.write_all(b"HTTP/1.1 401 Unauthorized\r\nContent-Type: application/json\r\nContent-Length: 24\r\nConnection: close\r\n\r\n{\"error\":\"Unauthorized\"}").unwrap();
        (headers, serde_json::from_slice::<serde_json::Value>(&body).unwrap())
    });
    let (view, cx) = cx.add_window_view(|window, cx| {
        let mut db = Db::memory().unwrap();
        db.apply_response(&serde_json::json!({"success":true, "projects":[{"id":123,"title":"Board"}], "tasks":[{"id":456,"projectId":123,"title":"Test","ticketNumber":"HTPR-123","status":"Normal"}]})).unwrap();
        let focus = cx.focus_handle();
        let mut this = AiSidebar::new(crate::theme::THEMES[0], None, focus, window, cx);
        this.client = Some(Client::with_base(&base, "synthetic-token".into()).unwrap());
        this.capture = Some(Rc::new(RefCell::new(crate::layout::LayoutDump::default())));
        this.set_scope(Scope::load(&db, Screen::Task, 123, Some(456)), cx);
        this
    });
    click(&view, cx, "ai.task.summary");
    let (headers, request) = server.join().unwrap();
    assert!(headers.starts_with("POST /api/ai/chat/stream "));
    assert_eq!(request["message"], "/i Summarize the current task");
    assert_eq!(request["default_context"]["task_id"], 456);
    assert_eq!(request["default_context"]["project_id"], 123);
}

#[gpui::test]
fn busy_composer_queues_with_enter_and_mouse_removes_without_stopping(cx: &mut gpui::TestAppContext) {
    cx.update(crate::input::init);
    let (_send, updates) = mpsc::sync_channel(64);
    let (view, cx) = cx.add_window_view(|window, cx| {
        let focus = cx.focus_handle();
        let mut this = AiSidebar::new(crate::theme::THEMES[0], None, focus, window, cx);
        this.capture = Some(Rc::new(RefCell::new(crate::layout::LayoutDump::default())));
        this.chats[0].messages.push(ChatMessage { message: Message { role: "assistant".into(), content: "Working".into() }, rich: markdown("Working", this.colors) });
        this.active = Some(ActiveTurn { ids: TurnIds::new(this.chats[0].id.clone()), chat: 0, assistant: 0, updates, cancel_reply: None, stopping: false, client: Client::new("test-token".into()).unwrap() });
        this.focus_composer(window, cx);
        this
    });
    cx.simulate_input("Next é🙂");
    cx.simulate_keystrokes("enter");
    assert_eq!(element(&view, cx, "ai.queue.0")["text"], "Next é🙂");
    view.read_with(cx, |this, cx| {
        assert!(this.input.read(cx).content.is_empty());
        assert_eq!(this.queued[0].prompt, "Next é🙂");
        assert!(!this.active.as_ref().unwrap().stopping);
    });
    cx.simulate_input("Another");
    click(&view, cx, "ai.send");
    assert_eq!(element(&view, cx, "ai.queue.1")["text"], "Another");
    click(&view, cx, "ai.queue.0.remove");
    assert_eq!(element(&view, cx, "ai.queue.0")["text"], "Another");
    click(&view, cx, "ai.new");
    view.read_with(cx, |this, _| assert!(this.queued.is_empty()));
}

#[gpui::test]
fn switching_local_chats_restores_each_transcript_scroll(cx: &mut gpui::TestAppContext) {
    let (view, cx) = cx.add_window_view(|window, cx| {
        let focus = cx.focus_handle();
        let mut this = AiSidebar::new(crate::theme::THEMES[0], None, focus, window, cx);
        this.capture = Some(Rc::new(RefCell::new(crate::layout::LayoutDump::default())));
        let content = "Long transcript paragraph.\n\n".repeat(100);
        this.chats[0].messages.push(ChatMessage { message: Message { role: "assistant".into(), content: content.clone() }, rich: markdown(&content, this.colors) });
        this
    });
    cx.draw(point(px(0.), px(0.)), size(px(420.), px(600.)), |_, _| view.clone().into_element());
    view.update_in(cx, |this, _, _| this.scroll.set_offset(point(px(0.), px(-80.))));
    click(&view, cx, "ai.new");
    click(&view, cx, "ai.history.toggle");
    click(&view, cx, "ai.history.0");
    cx.draw(point(px(0.), px(0.)), size(px(420.), px(600.)), |_, _| view.clone().into_element());
    view.read_with(cx, |this, _| {
        assert_eq!(this.selected, 0);
        assert_eq!(this.scroll.offset().y, px(-80.));
    });
}

#[gpui::test]
fn retry_mouse_action_preserves_real_offline_refusal(cx: &mut gpui::TestAppContext) {
    let (view, cx) = cx.add_window_view(|window, cx| {
        let focus = cx.focus_handle();
        let mut this = AiSidebar::new(crate::theme::THEMES[0], None, focus, window, cx);
        this.capture = Some(Rc::new(RefCell::new(crate::layout::LayoutDump::default())));
        this.chats[0].messages.push(ChatMessage { message: Message { role: "human".into(), content: "Original".into() }, rich: RichText::default() });
        this
    });
    click(&view, cx, "ai.message.0.retry");
    view.read_with(cx, |this, _| {
        assert!(this.active.is_none());
        assert!(this.chats[0].error.as_deref().unwrap().contains("offline"));
        assert_eq!(this.chats[0].messages.len(), 1);
    });
}
