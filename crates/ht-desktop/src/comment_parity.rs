use super::*;

#[gpui::test]
fn focused_comment_copy_link_and_quote_target_the_selected_comment(cx: &mut gpui::TestAppContext) {
    cx.update(crate::input::init);
    let (view, cx) = cx.add_window_view(|window, cx| {
        let mut view = desktop(window, cx);
        view.db
            .apply_task(
                &json!({"id":1,"projectId":15,"title":"Task","status":"Normal","uniqueIndex":9}),
            )
            .unwrap();
        view.reload();
        view
    });
    view.update_in(cx, |this, window, cx| {
        this.interaction.task_part = 1;
        this.run_comment_action(Action::CopyCommentContent, window, cx);
        assert_eq!(
            cx.read_from_clipboard().unwrap().text().unwrap().trim(),
            "First é🙂"
        );
        this.run_comment_action(Action::CopyCommentLink, window, cx);
        assert_eq!(
            cx.read_from_clipboard().unwrap().text().unwrap(),
            "https://app.hypertask.ai/detail/project-15/9#comment-0"
        );
        this.interaction.task_part = 2;
        this.run_comment_action(Action::CopyCommentLink, window, cx);
        assert!(
            cx.read_from_clipboard()
                .unwrap()
                .text()
                .unwrap()
                .ends_with("#comment-1")
        );
        this.run_comment_action(Action::CopyCommentToAiChat, window, cx);
        assert!(this.shell.ai_open);
        assert!(
            this.ai_sidebar
                .as_ref()
                .unwrap()
                .read(cx)
                .has_focus(window, cx)
        );
        assert!(this.db.mutations().unwrap().is_empty());
        let mut commands = vec![];
        this.add_comment_commands(&mut commands);
        assert!(commands.iter().any(|c| c.key == "copyCommentContent"));
        assert!(commands.iter().any(|c| c.key == "copyCommentToAiChat"));
        assert!(!commands.iter().any(|c| c.key == "editcomment"));
    });
}

#[test]
fn timestamp_tooltip_uses_web_local_exact_date_not_relative_text() {
    let time = "2026-01-02T13:04:00Z";
    let date = chrono::DateTime::parse_from_rfc3339(time)
        .unwrap()
        .with_timezone(&chrono::Local);
    use chrono::{Datelike, Timelike};
    let (pm, hour) = date.hour12();
    let expected = format!(
        "{}, {} {} {} at {}:{:02} {}",
        date.format("%a"),
        date.format("%b"),
        date.day(),
        date.year(),
        hour,
        date.minute(),
        if pm { "PM" } else { "AM" }
    );
    assert_eq!(exact_comment_time(time), expected);
    assert_eq!(exact_comment_time("bad-time"), "bad-time");
}

#[gpui::test]
fn overflow_ellipsis_click_sets_exact_comment_before_opening_command_palette(
    cx: &mut gpui::TestAppContext,
) {
    use crate::ui::shell::tests::{click, draw};
    use gpui::{VisualTestContext, WindowHandle};
    cx.update(crate::input::init);
    let view: WindowHandle<Desktop> = cx.add_window(|window, cx| {
        let mut view = desktop(window, cx);
        view.db
            .apply_task(
                &json!({"id":1,"projectId":15,"title":"Task","status":"Normal","uniqueIndex":9}),
            )
            .unwrap();
        view.reload();
        view.layout = Some(Rc::new(RefCell::new(LayoutDump::default())));
        view.ready_scheduled = true;
        view
    });
    let mut cx = VisualTestContext::from_window(view.into(), cx);
    draw(view, &mut cx);
    click(view, &mut cx, "task.comment.0.options");
    view.read_with(&mut cx, |this, _| {
        assert_eq!(this.interaction.task_part, 1);
        assert_eq!(this.focused_comment().unwrap().id, 7);
        let mut rows = vec![];
        this.add_comment_commands(&mut rows);
        assert!(rows.iter().any(|r| r.key == "editcomment"));
        assert!(rows.iter().any(|r| r.key == "copyCommentLink"));
    })
    .unwrap();
    cx.simulate_input("Copy comment content");
    cx.simulate_keystrokes("enter");
    cx.update(|_, cx| {
        assert_eq!(
            cx.read_from_clipboard().unwrap().text().unwrap().trim(),
            "First é🙂"
        )
    });
}
