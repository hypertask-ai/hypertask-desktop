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
