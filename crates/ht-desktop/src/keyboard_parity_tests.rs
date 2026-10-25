use super::*;
use super::shell::tests::{draw, measured, setup};
use super::shell_navigation::Destination;
use gpui::{ClipboardItem, TestAppContext};

#[gpui::test]
fn keyboard_shift_t_toggles_existing_board_table_without_changing_board(cx: &mut TestAppContext) {
    let (view, mut cx) = setup(cx);
    cx.simulate_keystrokes("2 shift-t");
    assert_eq!(measured(view, &mut cx, "shell.table.title")["text"], "Alpha");
    view.read_with(&cx, |this, _| {
        assert_eq!(this.options.board, 15);
        assert!(this.db.mutations().unwrap().is_empty());
    }).unwrap();
    cx.simulate_keystrokes("shift-t");
    view.read_with(&cx, |this, _| {
        assert!(this.shell.destination == Destination::Workspace);
        assert_eq!(this.screen, Screen::Board);
        assert_eq!(this.options.board, 15);
    }).unwrap();
}

#[gpui::test]
fn keyboard_ai_reset_aliases_work_from_workspace_and_focus_a_clean_composer(cx: &mut TestAppContext) {
    let (view, mut cx) = setup(cx);
    for chord in ["ctrl-shift-o", "cmd-shift-o", "ctrl-shift-j", "cmd-shift-j"] {
        view.update(&mut cx, |this, window, cx| window.focus(&this.focus, cx)).unwrap();
        cx.simulate_keystrokes("ctrl-q");
        cx.simulate_input("old draft");
        cx.simulate_keystrokes("ctrl-q");
        view.update(&mut cx, |this, window, cx| assert!(this.focus.is_focused(window))).unwrap();
        cx.simulate_keystrokes(chord);
        cx.simulate_input("fresh");
        cx.simulate_keystrokes("ctrl-a ctrl-c");
        cx.update(|cx| assert_eq!(cx.read_from_clipboard().and_then(|item| item.text()), Some("fresh".into()), "{chord}"));
        view.read_with(&cx, |this, _| assert!(this.db.mutations().unwrap().is_empty())).unwrap();
    }
}

#[gpui::test]
fn keyboard_ctrl_q_ignores_extra_modifiers_and_preserves_workspace_focus(cx: &mut TestAppContext) {
    let (view, mut cx) = setup(cx);
    for chord in ["ctrl-alt-q", "ctrl-shift-q", "ctrl-cmd-q"] {
        cx.simulate_keystrokes(chord);
        view.update(&mut cx, |this, window, _| assert!(this.focus.is_focused(window), "{chord}")).unwrap();
    }
    cx.simulate_keystrokes("ctrl-q ctrl-q");
    view.update(&mut cx, |this, window, _| assert!(this.focus.is_focused(window))).unwrap();
}
