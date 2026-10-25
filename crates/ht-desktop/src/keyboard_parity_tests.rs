use super::*;
use super::shell::tests::{draw, measured, setup};
use gpui::TestAppContext;

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
    draw(view, &mut cx);
    view.read_with(&cx, |this, _| {
        assert!(!this.layout.as_ref().unwrap().borrow().interaction_elements().iter().any(|e| e["key"] == "shell.table"));
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
        view.update(&mut cx, |this, window, _| assert!(this.focus.is_focused(window))).unwrap();
        cx.simulate_keystrokes(chord);
        cx.simulate_input("fresh");
        cx.simulate_keystrokes("ctrl-a ctrl-c");
        cx.update(|_, cx| assert_eq!(cx.read_from_clipboard().and_then(|item| item.text()), Some("fresh".into()), "{chord}"));
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

#[gpui::test]
fn keyboard_task_field_shortcuts_commit_the_selected_target_on_every_surface(cx: &mut TestAppContext) {
    cx.update(crate::input::init);
    for screen in [Screen::Inbox, Screen::Board, Screen::Task] {
        for (chord, query, field, expected) in [
            ("a", "Alex", "assignee", serde_json::json!([7])),
            ("m", "Doing", "sectionId", serde_json::json!(11)),
            ("p", "Urgent", "priority", serde_json::json!(1)),
            ("s", "XS", "estimate", serde_json::json!(2)),
            ("t", "Bug", "labels", serde_json::json!(["bug"])),
            ("d", "2030-06-20T10:30:00+02:00", "due_date", serde_json::json!("2030-06-20T10:30:00+02:00")),
        ] {
            let (view, cx) = cx.add_window_view(|window, cx| {
                let mut this = super::task_fields::tests::desktop(window, cx);
                this.db.apply_response(&serde_json::json!({"success":true,"user_notifications":[
                    {"id":101,"taskId":1,"projectId":15,"status":"Normal","seen":true,"createdAt":"2026-01-01T00:00:00Z"}
                ]})).unwrap();
                this.screen = screen;
                this.reload();
                if screen == Screen::Inbox {
                    this.split = this.splits.iter().position(|s| s == "All").unwrap();
                    this.filter_inbox();
                }
                this
            });
            cx.simulate_keystrokes(chord);
            cx.simulate_input(query);
            cx.run_until_parked();
            cx.simulate_keystrokes("enter");
            view.read_with(cx, |this, _| {
                let rows = this.db.mutations().unwrap();
                assert_eq!(rows.len(), 1, "{screen:?} {chord}");
                assert_eq!(rows[0].payload_json["task_id"], 1, "{screen:?} {chord}");
                assert_eq!(rows[0].payload_json[field], expected, "{screen:?} {chord}");
                assert_eq!(this.screen, screen);
            });
            cx.simulate_keystrokes("escape");
        }
    }
}

#[gpui::test]
fn keyboard_board_navigation_aliases_focus_columns_rows_and_open_the_selected_task(cx: &mut TestAppContext) {
    cx.update(crate::input::init);
    let (view, cx) = cx.add_window_view(|window, cx| {
        let mut this = super::task_fields::tests::desktop(window, cx);
        this.screen = Screen::Board;
        this.reload();
        this
    });
    for (chord, column, selected) in [
        ("j", 0, 1), ("k", 0, 0), ("down", 0, 1), ("up", 0, 0),
        ("tab", 1, 0), ("shift-tab", 0, 0), ("l", 1, 0), ("h", 0, 0),
        ("right", 1, 0), ("left", 0, 0), ("shift-g shift-g", 0, 1), ("g g", 0, 0),
    ] {
        cx.simulate_keystrokes(chord);
        view.read_with(cx, |this, _| {
            assert_eq!((this.column, this.selected), (column, selected), "{chord}");
            assert!(this.db.mutations().unwrap().is_empty());
        });
    }
    cx.simulate_keystrokes("tab enter");
    view.read_with(cx, |this, _| {
        assert_eq!(this.screen, Screen::Task);
        assert_eq!(this.options.task, 3);
    });
    cx.simulate_keystrokes("escape");
    view.read_with(cx, |this, _| {
        assert_eq!(this.screen, Screen::Board);
        assert_eq!((this.column, this.selected), (1, 0));
    });
}

#[gpui::test]
fn keyboard_archive_command_and_platform_aliases_archive_tasks_not_notifications(cx: &mut TestAppContext) {
    cx.update(crate::input::init);
    for screen in [Screen::Inbox, Screen::Board, Screen::Task] {
        for chord in ["ctrl-e", "cmd-e"] {
            let (view, cx) = cx.add_window_view(|window, cx| {
                let mut this = super::task_fields::tests::desktop(window, cx);
                this.db.apply_response(&serde_json::json!({"success":true,"user_notifications":[
                    {"id":101,"taskId":1,"projectId":15,"status":"Normal","seen":true,"createdAt":"2026-01-01T00:00:00Z"}
                ]})).unwrap();
                this.screen = screen;
                this.reload();
                if screen == Screen::Inbox {
                    this.split = this.splits.iter().position(|s| s == "All").unwrap();
                    this.filter_inbox();
                }
                this
            });
            cx.simulate_keystrokes(chord);
            view.read_with(cx, |this, _| {
                let rows = this.db.mutations().unwrap();
                assert_eq!(rows.len(), 1, "{screen:?} {chord}");
                assert_eq!(rows[0].kind, "task_patch");
                assert_eq!(rows[0].payload_json["task_id"], 1);
                assert_eq!(rows[0].payload_json["status"], "Archive");
                assert_eq!(this.db.task(1).unwrap().unwrap().status, ht_core::generated::Status::Archive);
                assert!(!rows.iter().any(|row| row.kind == "inbox_archive"));
            });
        }
    }
}

#[gpui::test]
fn keyboard_plain_shortcuts_do_not_escape_title_comment_or_description_inputs(cx: &mut TestAppContext) {
    cx.update(crate::input::init);
    for activate in ["f2", "ctrl-m", "ctrl-d"] {
        let (view, cx) = cx.add_window_view(super::task_fields::tests::desktop);
        cx.simulate_keystrokes(activate);
        for chord in ["a", "m", "p", "s", "t", "d", "e", "j", "k", "n", "shift-c", "shift-t", "/"] {
            cx.simulate_keystrokes(chord);
        }
        view.update_in(cx, |this, window, cx| {
            let input = match activate {
                "f2" => &this.title_input,
                "ctrl-m" => &this.comment_input,
                _ => &this.interaction.description,
            };
            assert!(input.focus_handle(cx).is_focused(window), "{activate}");
            assert!(this.interaction.modal.is_none());
            assert_eq!(this.screen, Screen::Task);
            assert!(this.db.mutations().unwrap().iter().all(|row| row.kind == "comment_draft"));
        });
    }
}

#[gpui::test]
fn keyboard_search_and_board_switch_shortcuts_use_existing_controls(cx: &mut TestAppContext) {
    let (view, mut cx) = setup(cx);
    for chord in ["/", "6"] {
        cx.simulate_keystrokes(chord);
        cx.simulate_input("Beta");
        draw(view, &mut cx);
        assert_eq!(measured(view, &mut cx, "shell.search.result.0")["text"], "Beta result");
        cx.simulate_keystrokes("escape");
    }
    for chord in ["ctrl-b", "cmd-b"] {
        cx.simulate_keystrokes(chord);
        cx.simulate_input("Beta");
        cx.simulate_keystrokes("enter");
        view.read_with(&cx, |this, _| {
            assert_eq!(this.options.board, 16, "{chord}");
            assert_eq!(this.screen, Screen::Board);
            assert!(this.db.mutations().unwrap().is_empty());
        }).unwrap();
        cx.simulate_keystrokes(chord);
        cx.simulate_input("Alpha");
        cx.simulate_keystrokes("enter");
    }
}

#[gpui::test]
fn keyboard_g_i_returns_to_the_default_inbox_split(cx: &mut TestAppContext) {
    cx.update(crate::input::init);
    let (view, cx) = cx.add_window_view(|window, cx| {
        let mut this = super::task_fields::tests::desktop(window, cx);
        this.screen = Screen::Inbox;
        this.reload();
        this.split = this.splits.len() - 1;
        this.filter_inbox();
        assert_ne!(this.split, 0);
        this
    });
    cx.simulate_keystrokes("g i");
    view.read_with(cx, |this, _| {
        assert_eq!(this.screen, Screen::Inbox);
        assert_eq!(this.split, 0);
        assert_eq!(this.selected, 0);
        assert!(this.db.mutations().unwrap().is_empty());
    });
}

#[gpui::test]
fn keyboard_alt_enter_sends_an_inbox_reply_without_archiving_or_advancing(cx: &mut TestAppContext) {
    cx.update(crate::input::init);
    for chord in ["ctrl-alt-enter", "cmd-alt-enter"] {
        let (view, cx) = cx.add_window_view(|window, cx| {
            let mut this = super::task_fields::tests::desktop(window, cx);
            this.db.apply_response(&serde_json::json!({"success":true,"user_notifications":[
                {"id":101,"taskId":1,"projectId":15,"status":"Normal","seen":true,"createdAt":"2026-01-02T00:00:00Z"},
                {"id":102,"taskId":2,"projectId":15,"status":"Normal","seen":true,"createdAt":"2026-01-01T00:00:00Z"}
            ]})).unwrap();
            this.screen = Screen::Inbox;
            this.reload();
            this.split = this.splits.iter().position(|s| s == "All").unwrap();
            this.filter_inbox();
            this
        });
        cx.simulate_keystrokes("ctrl-m");
        cx.simulate_input("Keep this task");
        cx.simulate_keystrokes(chord);
        view.read_with(cx, |this, _| {
            let rows = this.db.mutations().unwrap();
            let comments: Vec<_> = rows.iter().filter(|row| row.kind == "task_comment").collect();
            assert_eq!(comments.len(), 1, "{chord}");
            assert_eq!(comments[0].payload_json["task_id"], 1);
            assert_eq!(comments[0].payload_json["text"], "<p>Keep this task</p>");
            assert!(!rows.iter().any(|row| matches!(row.kind.as_str(), "inbox_archive" | "task_patch")));
            assert_eq!(this.screen, Screen::Task);
            assert_eq!(this.options.task, 1);
        });
    }
}

#[gpui::test]
fn keyboard_escape_saves_description_but_never_publishes_comment(cx: &mut TestAppContext) {
    cx.update(crate::input::init);
    let (view, cx) = cx.add_window_view(super::task_fields::tests::desktop);
    cx.simulate_keystrokes("ctrl-d ctrl-a");
    cx.simulate_input("Saved on Escape é🙂");
    cx.simulate_keystrokes("escape");
    view.update_in(cx, |this, window, cx| {
        let task = this.db.task(1).unwrap().unwrap();
        assert!(task.description.contains("Saved on Escape é🙂"), "{}", task.description);
        let rows = this.db.mutations().unwrap();
        let writes: Vec<_> = rows.iter().filter(|row| row.kind == "task_patch").collect();
        assert_eq!(writes.len(), 1);
        assert_eq!(writes[0].payload_json["task_id"], 1);
        assert!(writes[0].payload_json["description"].as_str().unwrap().contains("Saved on Escape é🙂"));
        assert!(matches!(this.editing, Editing::None));
        assert!(this.focus.is_focused(window));
        assert_eq!(this.screen, Screen::Task);
        assert!(this.interaction.modal.is_none());
    });
    cx.simulate_keystrokes("ctrl-m");
    cx.simulate_input("Unsent comment");
    cx.simulate_keystrokes("escape");
    view.read_with(cx, |this, cx| {
        assert!(this.comment_input.read(cx).content.contains("Unsent comment"));
        assert!(this.db.mutations().unwrap().iter().all(|row| row.kind != "task_comment"));
        assert_eq!(this.screen, Screen::Task);
    });
}

#[gpui::test]
fn cmdk_assign_to_me_preserves_people_agents_and_reopens_assignment(cx: &mut TestAppContext) {
    cx.update(crate::input::init);
    let (view, cx) = cx.add_window_view(super::task_fields::tests::desktop);
    view.update_in(cx, |this, _, _| {
        this.db.kv_set("members:15", &serde_json::json!([
            {"id":7,"displayName":"Alex"}, {"id":42,"displayName":"Me"},
            {"id":"agent-a","displayName":"Runner"}
        ]).to_string()).unwrap();
        this.db.apply_response(&serde_json::json!({"success":true,"tasks":[
            {"id":1,"assignees":[{"id":7,"displayName":"Alex"},{"id":"agent-a","agent":{"id":"agent-a","displayName":"Runner"}}],"assigneeCount":2}
        ]})).unwrap();
        this.reload();
    });
    for expected_writes in [1, 1] {
        cx.simulate_keystrokes("ctrl-k");
        cx.simulate_input("Assign to me");
        view.read_with(cx, |this, cx| {
            assert!(this.choices(cx).iter().any(|row| row.name == "Assign to me"));
        });
        cx.simulate_keystrokes("enter");
        view.read_with(cx, |this, _| {
            let task = this.db.task(1).unwrap().unwrap();
            assert_eq!(task.assignee_count, 3);
            assert!(task.extra["assignees"].as_array().unwrap().iter().any(|row| row["id"] == 7));
            assert!(task.extra["assignees"].as_array().unwrap().iter().any(|row| row["id"] == 42));
            assert!(task.extra["assignees"].as_array().unwrap().iter().any(|row| row["agent"]["id"] == "agent-a"));
            let writes: Vec<_> = this.db.mutations().unwrap().into_iter().filter(|row| row.kind == "task_patch").collect();
            assert_eq!(writes.len(), expected_writes);
            assert_eq!(writes[0].payload_json["assignee"], serde_json::json!([7,42]));
            assert_eq!(this.interaction.modal, Some(super::interactions::Modal::Assign));
        });
        cx.simulate_keystrokes("escape");
    }
}
