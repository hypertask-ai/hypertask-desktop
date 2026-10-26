use super::*;
use super::shell::tests::{draw, measured, setup};
use gpui::TestAppContext;
use ht_core::generated::Status;

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
            {"id":1,"projectId":15,"sectionId":10,"title":"Needle","status":"Normal","assignees":[{"id":7,"displayName":"Alex"},{"id":"agent-a","agent":{"id":"agent-a","displayName":"Runner"}}],"assigneeCount":2}
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
            let writes: Vec<_> = this.db.mutations().unwrap().into_iter().filter(|row| row.kind == "board_write").collect();
            assert_eq!(writes.len(), expected_writes);
            assert_eq!(writes[0].payload_json["change"], serde_json::json!({"AssignToMe":{"task":1,"user":42}}));
            assert_eq!(this.interaction.modal, Some(super::interactions::Modal::Assign));
        });
        cx.simulate_keystrokes("escape");
    }
}

#[gpui::test]
fn cmdk_assign_to_me_is_absent_without_account_member_or_active_task(cx: &mut TestAppContext) {
    cx.update(crate::input::init);
    for (user, members, status) in [
        ("", serde_json::json!([{"id":42,"displayName":"Me"}]), "Normal"),
        ("42", serde_json::json!([{"id":7,"displayName":"Alex"}]), "Normal"),
        ("42", serde_json::json!([{"id":42,"displayName":"Me"}]), "Archive"),
    ] {
        let (view, cx) = cx.add_window_view(super::task_fields::tests::desktop);
        view.update_in(cx, |this, _, _| {
            this.db.kv_set("current-user-id", user).unwrap();
            this.db.kv_set("members:15", &members.to_string()).unwrap();
            this.db.apply_response(&serde_json::json!({"success":true,"tasks":[
                {"id":1,"projectId":15,"sectionId":10,"title":"Needle","status":status}
            ]})).unwrap();
            this.reload();
        });
        cx.simulate_keystrokes("ctrl-k");
        cx.simulate_input("Assign to me");
        view.read_with(cx, |this, cx| {
            assert!(!this.choices(cx).iter().any(|row| row.name == "Assign to me"));
            assert!(this.db.mutations().unwrap().is_empty());
        });
    }
}

#[gpui::test]
fn keyboard_escape_keeps_description_when_lossless_save_is_refused(cx: &mut TestAppContext) {
    cx.update(crate::input::init);
    let (view, cx) = cx.add_window_view(super::task_fields::tests::desktop);
    cx.simulate_keystrokes("ctrl-d ctrl-a ctrl-shift-h");
    cx.simulate_input("Highlighted draft");
    cx.simulate_keystrokes("escape");
    view.update_in(cx, |this, window, cx| {
        assert!(matches!(this.editing, Editing::Description));
        assert!(this.interaction.description.focus_handle(cx).is_focused(window));
        assert!(this.interaction.description.read(cx).html().contains("<mark"));
        assert!(this.interaction.description.read(cx).content.contains("Highlighted draft"));
        assert!(this.error.as_deref().unwrap().contains("strips highlights"));
        assert_eq!(this.db.task(1).unwrap().unwrap().description, "<p>Hidden</p>");
        assert!(this.db.mutations().unwrap().is_empty());
    });
}

struct KeyboardWriteServer {
    calls: Vec<(String, serde_json::Value)>,
    running: bool,
    enabled: bool,
    fail_effect: bool,
    user: i64,
}
impl KeyboardWriteServer {
    fn new() -> Self { Self { calls: vec![], running: false, enabled: true, fail_effect: false, user: 42 } }
}
impl ht_core::Transport for KeyboardWriteServer {
    fn get(&mut self, path: &str) -> Result<ht_core::http::Response, ht_core::http::TransportError> {
        assert_eq!(path, "/user/context");
        Ok(ht_core::http::Response { status: 200, body: serde_json::json!({"success":true,"user":{"id":self.user}}) })
    }
    fn post(&mut self, path: &str, body: &serde_json::Value, _: &str) -> Result<ht_core::http::Response, ht_core::http::TransportError> {
        use serde_json::json;
        self.calls.push((path.into(), body.clone()));
        let response = match path {
            "/tasks/card-actions" => {
                assert_eq!(body["project_id"], 15);
                if body["action"] == "inbox" {
                    assert_eq!(body["active"], true);
                    return Ok(ht_core::http::Response { status:200, body:json!({"success":true,"action":"inbox","active":true}) });
                }
                assert_eq!(body["action"], "star");
                let active = body["active"].as_bool().unwrap();
                json!({"success":true,"action":"star","active":active,"saved_content":if active {json!([{"id":"saved-real","type":"Private"}])} else {json!([])}})
            }
            "/tasks/waiting-on" => json!({"success":true,"task":{"id":body["taskId"],"waitingOnUserId":body["userId"],"waitingOnSetById":42,"waitingOnSetAt":"2026-10-26T12:00:00Z"}}),
            "/time/status" => json!({"success":true,"summary":{"enabled":self.enabled,"runningEntry":if self.running {json!({"id":90,"taskId":1,"endedAt":null})} else {json!(null)}}}),
            "/time/start" | "/time/stop" => {
                if self.fail_effect { return Err(ht_core::http::TransportError::Timeout); }
                self.running = path == "/time/start";
                json!({"success":true,"entry":{"id":90,"taskId":1,"endedAt":if self.running {json!(null)} else {json!("2026-10-26T12:00:00Z")}}})
            }
            _ => panic!("unexpected keyboard request {path}"),
        };
        Ok(ht_core::http::Response { status: 200, body: response })
    }
}

#[gpui::test]
fn keyboard_alt_s_toggles_private_task_star_on_every_surface_and_reconciles_real_ids(cx: &mut TestAppContext) {
    cx.update(crate::input::init);
    for screen in [Screen::Inbox, Screen::Board, Screen::Task] {
        let (view, cx) = cx.add_window_view(|window, cx| {
            let mut this = super::task_fields::tests::desktop(window, cx);
            this.db.apply_task(&serde_json::json!({"id":1,"projectId":15,"title":"Needle","status":"Normal","savedContent":[]})).unwrap();
            this.db.apply_response(&serde_json::json!({"success":true,"user_notifications":[{"id":101,"taskId":1,"projectId":15,"status":"Normal","seen":true,"createdAt":"2026-01-01T00:00:00Z"}]})).unwrap();
            this.screen = screen;
            this.reload();
            if screen == Screen::Inbox { this.split = this.splits.iter().position(|s| s == "All").unwrap(); this.filter_inbox(); }
            this
        });
        let mut server = KeyboardWriteServer::new();
        for active in [true, false] {
            cx.simulate_keystrokes("alt-s");
            view.update_in(cx, |this, _, cx| {
                let row = this.db.mutations().unwrap().into_iter().find(|r| r.status == "pending").unwrap();
                assert_eq!(row.payload_json["change"]["StarTask"], serde_json::json!({"task":1,"project":15,"active":active}));
                let optimistic = this.db.task(1).unwrap().unwrap();
                assert_eq!(!optimistic.extra["savedContent"].as_array().unwrap().is_empty(), active);
                assert!(matches!(this.db.drain_once(&mut server, chrono::Utc::now().timestamp()+1).unwrap(), ht_core::Drain::Applied(_)));
                this.reload();
                cx.notify();
                let task = this.db.task(1).unwrap().unwrap();
                assert_eq!(task.extra["savedContent"], if active {serde_json::json!([{"id":"saved-real","type":"Private"}])} else {serde_json::json!([])});
            });
        }
        assert_eq!(server.calls.len(), 2);
    }
}

#[gpui::test]
fn keyboard_shift_b_picks_only_people_and_clear_without_assigning_them(cx: &mut TestAppContext) {
    cx.update(crate::input::init);
    let (view, cx) = cx.add_window_view(super::task_fields::tests::desktop);
    let mut server = KeyboardWriteServer::new();
    for (query, user) in [("Alex", Some(7)), ("Nobody", None)] {
        cx.simulate_keystrokes("shift-b");
        cx.simulate_input(query);
        cx.run_until_parked();
        cx.simulate_keystrokes("enter");
        view.update_in(cx, |this, _, _| {
            let row = this.db.mutations().unwrap().into_iter().find(|r| r.status == "pending").unwrap();
            assert_eq!(row.payload_json["change"]["WaitingOn"], serde_json::json!({"task":1,"user":user}));
            assert!(matches!(this.db.drain_once(&mut server, chrono::Utc::now().timestamp()+1).unwrap(), ht_core::Drain::Applied(_)));
            assert_eq!(this.db.task(1).unwrap().unwrap().extra["waitingOnUserId"], serde_json::json!(user));
            assert!(this.db.task(1).unwrap().unwrap().extra.get("assignees").is_none());
        });
    }
    assert_eq!(server.calls[0], ("/tasks/waiting-on".into(), serde_json::json!({"taskId":1,"userId":7})));
    assert_eq!(server.calls[1].1["userId"], serde_json::Value::Null);
}

#[gpui::test]
fn keyboard_timer_aliases_read_authoritative_state_and_stop_disabled_running_timers(cx: &mut TestAppContext) {
    let (view, mut cx) = super::task_fields::tests::visual(cx);
    let mut server = KeyboardWriteServer::new();
    for (chord, running) in [("w", true), ("alt-t", false)] {
        cx.simulate_keystrokes(chord);
        cx.simulate_keystrokes(chord); // Held/repeated input cannot enqueue a second toggle.
        view.update(&mut cx, |this, _, cx| {
            assert_eq!(this.db.mutations().unwrap().iter().filter(|r| r.status == "pending").count(), 1);
            assert!(matches!(this.db.drain_once(&mut server, chrono::Utc::now().timestamp()+1).unwrap(), ht_core::Drain::Applied(_)));
            assert!(!this.poll_timer_shortcut(cx));
        }).unwrap();
        assert_eq!(measured(view, &mut cx, "task.timer.control")["text"], if running { "Stop" } else { "Start" });
    }
    assert_eq!(server.calls.iter().map(|(path, _)| path.as_str()).collect::<Vec<_>>(), ["/time/status","/time/start","/time/status","/time/stop"]);
    assert!(server.calls.iter().all(|(_, body)| *body == serde_json::json!({"task":"1"})));
    server.enabled = false;
    for running in [false, true] {
        server.running = running;
        let before = server.calls.len();
        cx.simulate_keystrokes("w");
        view.update(&mut cx, |this, _, cx| {
            assert!(matches!(this.db.drain_once(&mut server, chrono::Utc::now().timestamp()+1).unwrap(), ht_core::Drain::Applied(_)));
            this.poll_timer_shortcut(cx);
        }).unwrap();
        assert_eq!(server.calls.len()-before, if running {2} else {1});
        assert!(!server.running);
    }
}

#[gpui::test]
fn keyboard_timer_unknown_effect_is_never_automatically_replayed(cx: &mut TestAppContext) {
    cx.update(crate::input::init);
    let (view, cx) = cx.add_window_view(super::task_fields::tests::desktop);
    cx.simulate_keystrokes("w");
    let mut server = KeyboardWriteServer::new();
    server.fail_effect = true;
    view.update_in(cx, |this, _, cx| {
        let now = chrono::Utc::now().timestamp()+1;
        assert!(matches!(this.db.drain_once(&mut server, now).unwrap(), ht_core::Drain::Failed(_)));
        assert!(matches!(this.db.drain_once(&mut server, now+100).unwrap(), ht_core::Drain::Empty));
        this.poll_timer_shortcut(cx);
        assert!(this.error.as_ref().unwrap().contains("refresh before retrying"));
    });
    assert_eq!(server.calls.len(), 2);
}

#[gpui::test]
fn keyboard_new_task_actions_never_escape_text_inputs_or_extra_modifiers(cx: &mut TestAppContext) {
    cx.update(crate::input::init);
    for activate in ["f2", "ctrl-m", "ctrl-d"] {
        let (view, cx) = cx.add_window_view(super::task_fields::tests::desktop);
        cx.simulate_keystrokes(activate);
        for key in ["alt-s", "shift-b", "w", "alt-t"] { cx.simulate_keystrokes(key); }
        view.read_with(cx, |this, _| assert!(this.db.mutations().unwrap().is_empty()));
    }
}

#[gpui::test]
fn keyboard_board_w_uses_selected_task_and_changed_accounts_never_send_effects(cx: &mut TestAppContext) {
    cx.update(crate::input::init);
    for user in [42, 43] {
        let (view, cx) = cx.add_window_view(|window, cx| {
            let mut this = super::task_fields::tests::desktop(window, cx);
            this.screen = Screen::Board;
            this.reload();
            this
        });
        for chord in ["ctrl-w", "alt-w", "shift-w", "ctrl-alt-t", "ctrl-alt-s", "ctrl-shift-b"] { cx.simulate_keystrokes(chord); }
        view.read_with(cx, |this, _| assert!(this.db.mutations().unwrap().is_empty()));
        cx.simulate_keystrokes("w");
        let mut server = KeyboardWriteServer::new();
        server.user = user;
        view.update_in(cx, |this, _, cx| {
            let result = this.db.drain_once(&mut server, chrono::Utc::now().timestamp()+1).unwrap();
            if user == 42 { assert!(matches!(result, ht_core::Drain::Applied(_))); }
            else { assert!(matches!(result, ht_core::Drain::Failed(_))); }
            this.poll_timer_shortcut(cx);
        });
        assert_eq!(server.calls.len(), if user == 42 {2} else {0});
    }
}

#[gpui::test]
fn keyboard_star_refuses_unknown_state_and_rolls_back_on_account_refusal(cx: &mut TestAppContext) {
    cx.update(crate::input::init);
    let (view, cx) = cx.add_window_view(super::task_fields::tests::desktop);
    cx.simulate_keystrokes("alt-s");
    view.update_in(cx, |this, _, _| {
        assert!(this.db.mutations().unwrap().is_empty());
        assert!(this.error.as_ref().unwrap().contains("Sync this task"));
        this.db.apply_task(&serde_json::json!({"id":1,"projectId":15,"title":"Needle","status":"Normal","savedContent":[{"id":"old-star","type":"Private"}]})).unwrap();
    });
    cx.simulate_keystrokes("alt-s");
    let mut server = KeyboardWriteServer::new();
    server.user = 43;
    view.update_in(cx, |this, _, _| {
        assert!(this.db.task(1).unwrap().unwrap().extra["savedContent"].as_array().unwrap().is_empty());
        assert!(matches!(this.db.drain_once(&mut server, chrono::Utc::now().timestamp()+1).unwrap(), ht_core::Drain::Failed(_)));
        assert_eq!(this.db.task(1).unwrap().unwrap().extra["savedContent"], serde_json::json!([{"id":"old-star","type":"Private"}]));
    });
    assert!(server.calls.is_empty());
}

#[gpui::test]
fn cmdk_round2_existing_actions_use_web_labels_groups_and_native_effects(cx: &mut TestAppContext) {
    cx.update(crate::input::init);
    let (view, cx) = cx.add_window_view(super::task_fields::tests::desktop);
    view.update_in(cx, |this, _, _| {
        this.db.apply_task(&serde_json::json!({"id":1,"projectId":15,"title":"Needle","status":"Normal","savedContent":[],"project":{"id":15,"timeTrackingEnabled":true}})).unwrap();
        this.reload();
    });
    for (query, label) in [("Blocked by person", "Blocked by person…"), ("Star task", "Star task"), ("Start or stop timer", "Start or stop timer"), ("Decline task", "Decline task")] {
        cx.simulate_keystrokes("ctrl-k");
        cx.simulate_input(query);
        view.read_with(cx, |this, cx| assert_eq!(this.choices(cx).first().map(|row| row.name.as_str()), Some(label), "{query}"));
        cx.simulate_keystrokes("enter");
        if query == "Blocked by person" {
            cx.simulate_input("Alex");
            cx.simulate_keystrokes("enter");
            view.read_with(cx, |this, _| {
                let rows = this.db.mutations().unwrap();
                assert_eq!(rows.last().unwrap().payload_json["change"]["WaitingOn"], serde_json::json!({"task":1,"user":7}));
                assert_eq!(this.db.task(1).unwrap().unwrap().assignee_count, 0);
            });
        } else {
            view.read_with(cx, |this, _| {
                let rows = this.db.mutations().unwrap();
                let row = rows.last().unwrap();
                match query {
                    "Star task" => assert_eq!(row.payload_json["change"]["StarTask"], serde_json::json!({"task":1,"project":15,"active":true})),
                    "Start or stop timer" => assert_eq!(row.payload_json["change"]["ToggleTimer"], serde_json::json!({"task":1})),
                    "Decline task" => {
                        assert_eq!(row.payload_json["status"], "Archive");
                        assert_eq!(this.db.task(1).unwrap().unwrap().status, Status::Archive);
                    }
                    _ => unreachable!(),
                }
            });
        }
    }
}

#[gpui::test]
fn cmdk_round2_move_to_inbox_and_star_reconcile_without_faking_notifications(cx: &mut TestAppContext) {
    cx.update(crate::input::init);
    let (view, cx) = cx.add_window_view(super::task_fields::tests::desktop);
    view.update_in(cx, |this, _, _| {
        this.db.apply_task(&serde_json::json!({"id":1,"projectId":15,"title":"Needle","status":"Normal","savedContent":[]})).unwrap();
    });
    let mut server = KeyboardWriteServer::new();
    for (query, active) in [("Star task", true), ("Unstar task", false)] {
        cx.simulate_keystrokes("ctrl-k");
        cx.simulate_input(query);
        view.read_with(cx, |this, cx| assert_eq!(this.choices(cx).first().unwrap().name, query));
        cx.simulate_keystrokes("enter");
        view.update_in(cx, |this, _, _| {
            assert!(matches!(this.db.drain_once(&mut server, chrono::Utc::now().timestamp()+1).unwrap(), ht_core::Drain::Applied(_)));
            let task = this.db.task(1).unwrap().unwrap();
            assert_eq!(!task.extra["savedContent"].as_array().unwrap().is_empty(), active);
            if active { assert_eq!(task.extra["savedContent"][0]["id"], "saved-real"); }
        });
    }
    for count in [3, 4] {
        cx.simulate_keystrokes("ctrl-k");
        cx.simulate_input("Move task to inbox");
        view.read_with(cx, |this, cx| assert_eq!(this.choices(cx).first().unwrap().name, "Move task to inbox"));
        cx.simulate_keystrokes("enter");
        view.update_in(cx, |this, _, _| {
            assert!(matches!(this.db.drain_once(&mut server, chrono::Utc::now().timestamp()+1).unwrap(), ht_core::Drain::Applied(_)));
            assert!(this.db.inbox().unwrap().is_empty(), "notification identity must come from server refresh");
            assert!(this.db.refetches().unwrap().iter().any(|(target, _, _)| target == "inbox"));
        });
        assert_eq!(server.calls.len(), count);
        assert_eq!(server.calls.last().unwrap().1, serde_json::json!({"task_id":1,"project_id":15,"action":"inbox","active":true}));
    }
}

#[gpui::test]
fn cmdk_round2_omits_unknown_states_and_session_only_commands(cx: &mut TestAppContext) {
    cx.update(crate::input::init);
    let (view, cx) = cx.add_window_view(super::task_fields::tests::desktop);
    for (query, forbidden) in [
        ("Star task", "Star task"), ("Start or stop timer", "Start or stop timer"),
        ("Clear inbox to zero", "Clear inbox to zero"), ("Archive all read", "Archive all read"),
        ("Archive reactions", "Archive reactions"), ("Mark unread", "Mark unread"),
        ("Set reminder", "Set reminder"), ("Mark blocked by", "Mark blocked by"),
        ("Create task", "Create task"),
    ] {
        cx.simulate_keystrokes("ctrl-k");
        cx.simulate_input(query);
        view.read_with(cx, |this, cx| assert!(!this.choices(cx).iter().any(|row| row.name == forbidden), "{forbidden}"));
        cx.simulate_keystrokes("escape");
    }
    view.update_in(cx, |this, _, _| {
        this.db.kv_set("current-user-id", "").unwrap();
    });
    for query in ["Blocked by person…", "Move task to inbox", "Decline task"] {
        cx.simulate_keystrokes("ctrl-k");
        cx.simulate_input(query);
        view.read_with(cx, |this, cx| assert!(!this.choices(cx).iter().any(|row| row.name == query)));
        cx.simulate_keystrokes("escape");
    }
    view.read_with(cx, |this, _| assert!(this.db.mutations().unwrap().is_empty()));
}

#[gpui::test]
fn cmdk_round2_accept_uses_ranked_columns_skips_only_intake_and_keeps_hidden_targets(cx: &mut TestAppContext) {
    cx.update(crate::input::init);
    for (title, expected) in [("Triage", 13), ("Pretriage", 12)] {
        let (view, cx) = cx.add_window_view(super::task_fields::tests::desktop);
        view.update_in(cx, |this, _, _| {
            this.db.apply_response(&serde_json::json!({"success":true,"sections":[
                {"id":10,"projectId":15,"section_title":title,"visibility":true,"deleted":false,"ranking":"a"},
                {"id":11,"projectId":15,"section_title":"Doing","visibility":true,"deleted":false,"ranking":"z"},
                {"id":12,"projectId":15,"section_title":"Inbox","visibility":true,"deleted":false,"ranking":"b"},
                {"id":13,"projectId":15,"section_title":"Ready","visibility":false,"deleted":false,"ranking":"c"}
            ]})).unwrap();
            this.reload();
        });
        cx.simulate_keystrokes("ctrl-k");
        cx.simulate_input("Accept task");
        view.read_with(cx, |this, cx| assert_eq!(this.choices(cx).first().unwrap().name, "Accept task"));
        cx.simulate_keystrokes("enter");
        view.read_with(cx, |this, _| {
            let rows = this.db.mutations().unwrap();
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].payload_json["change"]["MoveTask"], serde_json::json!({"task":1,"project":15,"section":expected}));
        });
    }
    let (view, cx) = cx.add_window_view(super::task_fields::tests::desktop);
    view.update_in(cx, |this, _, _| {
        this.db.apply_task(&serde_json::json!({"id":1,"projectId":15,"sectionId":11,"title":"Last column","status":"Normal"})).unwrap();
        this.reload();
    });
    cx.simulate_keystrokes("ctrl-k");
    cx.simulate_input("Accept task");
    cx.simulate_keystrokes("enter");
    view.read_with(cx, |this, _| {
        assert!(this.db.mutations().unwrap().is_empty());
        assert_eq!(this.error.as_deref(), Some("No later column is available for this task"));
    });
}

#[gpui::test]
fn cmdk_round2_remove_parent_uses_nullable_parent_field_and_hides_after_unlink(cx: &mut TestAppContext) {
    cx.update(crate::input::init);
    let (view, cx) = cx.add_window_view(super::task_fields::tests::desktop);
    view.update_in(cx, |this, _, _| {
        this.db.apply_task(&serde_json::json!({"id":1,"projectId":15,"title":"Child","status":"Normal","parent_id":2,"parent_task":{"id":2,"title":"Parent"}})).unwrap();
        this.reload();
    });
    cx.simulate_keystrokes("ctrl-k");
    cx.simulate_input("Remove as sub-task");
    view.read_with(cx, |this, cx| assert_eq!(this.choices(cx).first().unwrap().name, "Remove as sub-task"));
    cx.simulate_keystrokes("enter");
    view.read_with(cx, |this, _| {
        let rows = this.db.mutations().unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].payload_json, serde_json::json!({"task_id":1,"parent_task_id":null}));
        assert!(this.db.task(1).unwrap().unwrap().extra["parentTaskId"].is_null());
    });
    cx.simulate_keystrokes("ctrl-k");
    cx.simulate_input("Remove as sub-task");
    view.read_with(cx, |this, cx| assert!(!this.choices(cx).iter().any(|row| row.name == "Remove as sub-task")));
}

#[gpui::test]
fn cmdk_round2_branch_comment_starts_clean_chat_with_attributed_quote(cx: &mut TestAppContext) {
    cx.update(crate::input::init);
    let (view, cx) = cx.add_window_view(super::task_fields::tests::desktop);
    view.update_in(cx, |this, _, _| {
        this.db.apply_comments(1, &serde_json::json!({"success":true,"comments":[
            {"id":7,"taskId":1,"text":"<p>Quoted é🙂</p>","creatorId":42,"creator":{"id":42,"displayName":"Alex"},"createdAt":"2026-01-01T00:00:00Z"}
        ]})).unwrap();
        this.reload();
        this.interaction.task_part = 1;
    });
    cx.simulate_keystrokes("ctrl-q");
    cx.simulate_input("Old draft must stay behind");
    view.update_in(cx, |this, window, cx| window.focus(&this.focus, cx));
    cx.simulate_keystrokes("ctrl-k");
    cx.simulate_input("Branch in new chat");
    view.read_with(cx, |this, cx| assert_eq!(this.choices(cx).first().unwrap().name, "Branch in new chat"));
    cx.simulate_keystrokes("enter");
    cx.simulate_keystrokes("ctrl-a ctrl-c");
    cx.update(|_, cx| {
        let text = cx.read_from_clipboard().unwrap().text().unwrap();
        assert!(text.contains("Alex said"));
        assert!(text.contains("Quoted é🙂"));
        assert!(!text.contains("Old draft"));
    });
    view.read_with(cx, |this, _| assert!(this.db.mutations().unwrap().is_empty()));
}

#[gpui::test]
fn cmdk_round2_summary_commands_reach_real_sender_and_preserve_offline_refusal(cx: &mut TestAppContext) {
    cx.update(crate::input::init);
    let (view, cx) = cx.add_window_view(super::task_fields::tests::desktop);
    view.update_in(cx, |this, _, _| {
        this.db.apply_comments(1, &serde_json::json!({"success":true,"comments":[
            {"id":7,"taskId":1,"text":"<p>Quoted é🙂</p>","creatorId":42,"creator":{"id":42,"displayName":"Alex"},"createdAt":"2026-01-01T00:00:00Z"}
        ]})).unwrap();
        this.reload();
        this.interaction.task_part = 1;
    });
    for (query, expected) in [
        ("Summarize comment", "Summarize this comment concisely."),
        ("Summarize ticket", "Summarize this ticket, its description and the key points from the comments, concisely."),
    ] {
        view.update_in(cx, |this, window, cx| window.focus(&this.focus, cx));
        cx.simulate_keystrokes("ctrl-k");
        cx.simulate_input(query);
        view.read_with(cx, |this, cx| assert_eq!(this.choices(cx).first().unwrap().name, query));
        cx.simulate_keystrokes("enter ctrl-a ctrl-c");
        cx.update(|_, cx| {
            let text = cx.read_from_clipboard().unwrap().text().unwrap();
            assert!(text.contains(expected), "{text}");
            assert_eq!(text.contains("Quoted é🙂"), query == "Summarize comment");
        });
        view.read_with(cx, |this, _| assert!(this.db.mutations().unwrap().is_empty()));
    }
}
