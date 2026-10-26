use super::*;
use gpui::MouseButton;
use serde_json::Value;
use std::ops::Range;

pub(super) struct Mentions {
    generation: u64,
    token: Option<(gpui::EntityId, i64, Range<usize>, String)>,
    input: Option<Entity<TextInput>>,
    rows: Vec<Value>,
    choice: usize,
    loading: bool,
    error: bool,
    dismissed: bool,
    scroll: ScrollHandle,
}
impl Mentions {
    pub fn new() -> Self {
        Self {
            generation: 0,
            token: None,
            input: None,
            rows: vec![],
            choice: 0,
            loading: false,
            error: false,
            dismissed: false,
            scroll: ScrollHandle::new(),
        }
    }
    pub fn invalidate(&mut self) {
        self.generation += 1;
        self.token = None;
        self.input = None;
        self.rows.clear();
        self.loading = false;
    }
}
impl Desktop {
    pub(super) fn bind_mentions(&mut self, cx: &mut Context<Self>) {
        for input in [
            self.comment_input.clone(),
            self.comment_actions.input.clone(),
            self.interaction.description.clone(),
        ] {
            cx.observe(&input, |this, input, cx| this.observe_mention(input, cx))
                .detach();
        }
    }
    fn observe_mention(&mut self, input: Entity<TextInput>, cx: &mut Context<Self>) {
        if self.screen != Screen::Task || self.compose_open() {
            return;
        }
        let project = self.detail.as_ref().map_or(0, |d| d.task.project_id);
        let next = input
            .read(cx)
            .mention_token()
            .map(|(range, query)| (input.entity_id(), project, range, query));
        if self.mentions.token == next {
            return;
        }
        self.mentions.invalidate();
        self.mentions.token = next.clone();
        self.mentions.dismissed = false;
        self.mentions.choice = 0;
        self.mentions.error = false;
        let Some((_, project, _, query)) = next else {
            cx.notify();
            return;
        };
        self.mentions.input = Some(input.clone());
        self.mentions.loading = true;
        let generation = self.mentions.generation;
        let task = self.options.task;
        let user = self.db.current_user_id();
        let Some(token) = self.cli_token() else {
            self.mentions.loading = false;
            self.mentions.error = true;
            cx.notify();
            return;
        };
        cx.spawn(async move |view, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(180))
                .await;
            let current = view.update(cx, |this, _| this.mentions.generation == generation);
            if !matches!(current, Ok(true)) {
                return;
            }
            let (send, receive) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                let result = (|| {
                    let http = ht_core::Http::new(token)?;
                    let request = ht_core::desktop_cli::mentions_request(project, &query);
                    let value = http.json_request("GET", &request.path, None)?;
                    anyhow::ensure!(value.is_array(), "invalid mention response");
                    Ok::<_, anyhow::Error>(value.as_array().unwrap().clone())
                })();
                let _ = send.send(result.map_err(|_| ()));
            });
            loop {
                match receive.try_recv() {
                    Ok(result) => {
                        let _ = view.update(cx, |this, cx| {
                            this.apply_mention_results(generation, task, user, result, cx);
                        });
                        break;
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => {
                        cx.background_executor()
                            .timer(Duration::from_millis(50))
                            .await
                    }
                    Err(_) => break,
                }
            }
        })
        .detach();
        cx.notify();
    }
    fn apply_mention_results(
        &mut self,
        generation: u64,
        task: i64,
        user: Option<i64>,
        result: Result<Vec<Value>, ()>,
        cx: &mut Context<Self>,
    ) {
        let valid = self.mentions.generation == generation
            && self.options.task == task
            && self.db.current_user_id() == user
            && self.cli_token().is_some()
            && self.mentions.input.as_ref().is_some_and(|input| {
                self.mentions.token.as_ref().is_some_and(|(id, project, range, query)| {
                    input.entity_id() == *id
                        && self.detail.as_ref().is_some_and(|d| d.task.project_id == *project)
                        && input.read(cx).mention_token() == Some((range.clone(), query.clone()))
                })
            });
        if !valid {
            return;
        }
        self.mentions.loading = false;
        self.mentions.rows.clear();
        self.mentions.error = result.is_err();
        if let Ok(rows) = result {
            self.mentions.rows = rows;
        }
        self.mentions.choice = self.mention_selectable().first().copied().unwrap_or(0);
        cx.notify();
    }
    fn mention_selectable(&self) -> Vec<usize> {
        self.mentions
            .rows
            .iter()
            .enumerate()
            .filter(|(_, row)| ht_core::desktop_cli::mention_html(row).is_some())
            .map(|(i, _)| i)
            .collect()
    }
    fn insert_mention_choice(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(input) = self.mentions.input.clone() else {
            return;
        };
        let Some((_, project, range, query)) = self.mentions.token.clone() else {
            return;
        };
        if input.read(cx).mention_token() != Some((range.clone(), query))
            || self
                .detail
                .as_ref()
                .is_none_or(|d| d.task.project_id != project)
        {
            self.mentions.invalidate();
            return;
        }
        let Some(html) = self
            .mentions
            .rows
            .get(index)
            .and_then(ht_core::desktop_cli::mention_html)
        else {
            return;
        };
        // Dismiss before the editor notification so insertion cannot reopen an old request.
        self.mentions.invalidate();
        input.update(cx, |input, cx| {
            input.insert_mention(range, &html, window, cx)
        });
        window.focus(&input.focus_handle(cx), cx);
        cx.notify();
    }
    pub(super) fn mention_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let modifiers = event.keystroke.modifiers;
        if modifiers.control || modifiers.platform || modifiers.alt || modifiers.shift {
            return false;
        }
        self.mention_navigation(&event.keystroke.key, window, cx)
    }
    pub(super) fn mention_navigation(
        &mut self,
        key: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.mentions.dismissed
            || self.mentions.token.is_none()
            || !self
                .mentions
                .input
                .as_ref()
                .is_some_and(|i| i.focus_handle(cx).is_focused(window))
        {
            return false;
        }
        match key {
            "escape" => self.mentions.dismissed = true,
            "up" | "down" => {
                let rows = self.mention_selectable();
                if !rows.is_empty() {
                    let at = rows
                        .iter()
                        .position(|i| *i == self.mentions.choice)
                        .unwrap_or(0);
                    self.mentions.choice = rows[if key == "down" {
                        (at + 1) % rows.len()
                    } else {
                        (at + rows.len() - 1) % rows.len()
                    }];
                    self.mentions.scroll.scroll_to_item(self.mentions.choice);
                }
            }
            "enter" => self.insert_mention_choice(self.mentions.choice, window, cx),
            _ => return false,
        }
        cx.stop_propagation();
        cx.notify();
        true
    }
    pub(super) fn mention_overlay(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let Some(input) = &self.mentions.input else {
            return div().into_any_element();
        };
        if self.mentions.dismissed
            || self.mentions.token.is_none()
            || !input.focus_handle(cx).is_focused(window)
            || self.screen != Screen::Task
            || self.compose_open()
        {
            return div().into_any_element();
        }
        let t = self.colors;
        let bounds = input.read(cx).input_bounds();
        let left = bounds
            .map_or(180., |b| f32::from(b.left()))
            .min((f32::from(window.viewport_size().width) - 510.).max(0.));
        let top = bounds.map_or(100., |b| (f32::from(b.top()) - 290.).max(0.));
        let mut menu = div()
            .id("mention-menu")
            .absolute()
            .left(px(left))
            .top(px(top))
            .w(px(500.))
            .max_h(px(280.))
            .overflow_y_scroll()
            .track_scroll(&self.mentions.scroll)
            .bg(color(t.comment))
            .border_1()
            .border_color(color(t.border))
            .rounded(px(5.))
            .shadow(shadow())
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|_, _, _, cx| cx.stop_propagation()),
            );
        if self.mentions.loading {
            menu = menu.child(div().p(px(12.)).child("Loading mentions…"));
        } else if self.mentions.error {
            menu = menu.child(div().p(px(12.)).child("No results found").layout_key(&self.layout, "comment.mentions.error", Some("No results found".into())));
        } else if self.mention_selectable().is_empty() {
            menu = menu.child(div().p(px(12.)).child("No results found"));
        } else {
            for (i, row) in self.mentions.rows.iter().enumerate() {
                let selectable = ht_core::desktop_cli::mention_html(row).is_some();
                menu = menu.child(
                    div()
                        .id(("mention-result", i))
                        .px(px(12.))
                        .py(px(8.))
                        .when(!selectable, |d| {
                            d.text_size(px(11.)).text_color(color(t.muted))
                        })
                        .when(selectable, |d| {
                            d.cursor_pointer()
                                .when(i == self.mentions.choice, |d| d.bg(color(t.hover)))
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(move |this, _, window, cx| {
                                        cx.stop_propagation();
                                        this.insert_mention_choice(i, window, cx);
                                    }),
                                )
                        })
                        .child(row["name"].as_str().unwrap_or("").to_owned())
                        .layout_key(&self.layout, format!("comment.mention.{i}"), None),
                );
            }
        }
        menu.layout_key(&self.layout, "comment.mentions", None)
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn desktop(window: &mut Window, cx: &mut Context<Desktop>) -> Desktop {
        let mut db = Db::memory().unwrap();
        db.kv_set("current-user-id", "985").unwrap();
        db.apply_task(
            &json!({"id":1,"projectId":7283,"title":"QA","status":"Normal","uniqueIndex":5}),
        )
        .unwrap();
        db.apply_comments(1,&json!({"success":true,"comments":[{"id":7,"creatorId":985,"text":"<p>Comment</p>","createdAt":"2026-01-01T00:00:00Z"}]})).unwrap();
        let options = Options::parse(
            [
                "--offline",
                "--screen",
                "task",
                "--task",
                "1",
                "--board",
                "7283",
            ]
            .map(String::from),
        )
        .unwrap();
        let view = Desktop::new(db, options, window, cx);
        window.focus(&view.focus, cx);
        view
    }
    fn results(this: &mut Desktop, cx: &mut Context<Desktop>) {
        let input = this.comment_input.clone();
        let (range, query) = input.read(cx).mention_token().unwrap();
        this.mentions.token = Some((
            input.entity_id(),
            this.detail.as_ref().unwrap().task.project_id,
            range,
            query,
        ));
        this.mentions.input = Some(input);
        this.mentions.loading = false;
        this.mentions.error = false;
        this.mentions.dismissed = false;
        this.mentions.choice = 1;
        this.mentions.rows = vec![
            json!({"type":"peopleHeading","name":"People"}),
            json!({"type":"name","id":985,"name":"QA é🙂"}),
            json!({"type":"taskHeading","name":"Tasks"}),
            json!({"type":"task","id":5,"name":"Linked task","project_id":7283,"index":5,"ticketNumber":"QA-5"}),
        ];
        cx.notify();
    }
    #[gpui::test]
    fn comment_mentions_skip_headers_insert_rich_task_links_and_undo_without_sending(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(crate::input::init);
        let (view, cx) = cx.add_window_view(desktop);
        cx.simulate_keystrokes("ctrl-m");
        cx.simulate_input("Before @QA");
        view.update_in(cx, |this, _, cx| results(this, cx));
        cx.simulate_keystrokes("down");
        view.read_with(cx, |this, _| assert_eq!(this.mentions.choice, 3));
        cx.simulate_keystrokes("enter");
        view.read_with(cx, |this, cx| {
            let html = this.comment_input.read(cx).html();
            assert!(html.contains("Before "));
            assert!(
                html.contains("data-label=\"task\""),
                "inserted HTML: {html}"
            );
            assert!(html.contains("href=\"https://app.hypertask.ai/detail/project-7283/5\""));
            assert!(!html.contains("@QA"));
            assert!(this.db.mutations().unwrap().is_empty());
        });
        cx.simulate_keystrokes("ctrl-z");
        view.read_with(cx, |this, cx| {
            assert!(this.comment_input.read(cx).content.ends_with("@QA"))
        });
    }
    #[gpui::test]
    fn mention_escape_empty_error_and_stale_selection_do_not_insert_or_send(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(crate::input::init);
        let (view, cx) = cx.add_window_view(desktop);
        cx.simulate_keystrokes("ctrl-m");
        cx.simulate_input("@QA");
        view.update_in(cx, |this, _, cx| results(this, cx));
        cx.simulate_keystrokes("escape");
        view.read_with(cx, |this, cx| {
            assert!(this.mentions.dismissed);
            assert_eq!(this.comment_input.read(cx).content.as_ref(), "@QA");
        });
        view.update_in(cx, |this, window, cx| {
            results(this, cx);
            this.mentions.rows.clear();
            this.insert_mention_choice(0, window, cx);
            assert_eq!(this.comment_input.read(cx).content.as_ref(), "@QA");
            results(this, cx);
            this.options.task = 2;
            this.detail = None;
            this.insert_mention_choice(1, window, cx);
            assert_eq!(this.comment_input.read(cx).content.as_ref(), "@QA");
            assert!(this.db.mutations().unwrap().is_empty());
        });
    }
    #[gpui::test]
    fn comment_star_pin_shortcuts_and_command_rows_target_focused_comment_only(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(crate::input::init);
        let (view, cx) = cx.add_window_view(desktop);
        view.update_in(cx, |this, _, _| this.interaction.task_part = 1);
        cx.simulate_keystrokes("ctrl-shift-s ctrl-shift-p");
        view.update_in(cx, |this, _, _| {
            let rows = this.db.mutations().unwrap();
            assert_eq!(rows.len(), 2);
            assert_eq!(
                rows[0].payload_json["change"],
                json!({"action":"saved","public":false})
            );
            assert_eq!(
                rows[1].payload_json["change"],
                json!({"action":"saved","public":true})
            );
            assert_eq!(rows[1].payload_json["comment_id"], 7);
            let mut commands = vec![];
            this.add_comment_commands(&mut commands);
            assert!(commands.iter().any(|c| c.key == "starComment"));
            assert!(commands.iter().any(|c| c.key == "pinComment"));
            this.interaction.task_part = 0;
        });
        cx.simulate_keystrokes("ctrl-shift-s ctrl-shift-p");
        view.read_with(cx, |this, _| {
            assert_eq!(this.db.mutations().unwrap().len(), 2)
        });
        cx.simulate_keystrokes("ctrl-m ctrl-shift-s ctrl-shift-p");
        view.read_with(cx, |this, _| {
            assert_eq!(this.db.mutations().unwrap().len(), 2)
        });
    }
    #[gpui::test]
    fn mention_mouse_selection_inserts_notification_metadata_without_blurring_or_sending(
        cx: &mut gpui::TestAppContext,
    ) {
        use super::super::shell::tests::{click, setup};
        let (view, mut visual) = setup(cx);
        view.update(&mut visual, |this, _, cx| {
            this.open_task(101, cx);
        })
        .unwrap();
        visual.simulate_keystrokes("ctrl-m");
        visual.simulate_input("@QA");
        view.update(&mut visual, |this, _, cx| results(this, cx))
            .unwrap();
        click(view, &mut visual, "comment.mention.1");
        view.read_with(&visual, |this, cx| {
            let html = this.comment_input.read(cx).html();
            assert!(html.contains("data-label=\"name-985\""), "{html}");
            assert!(!html.contains("@QA"));
            assert!(this.db.mutations().unwrap().is_empty());
        })
        .unwrap();
    }
    #[gpui::test]
    fn cli7699_native_mentions_apply_mocked_http_results_and_refusals(
        cx: &mut gpui::TestAppContext,
    ) {
        use ht_core::desktop_cli::mentions_request;
        use std::{io::{Read, Write}, net::TcpListener};
        cx.update(crate::input::init);
        for status in [200, 401] {
            let (view, visual) = cx.add_window_view(desktop);
            visual.simulate_keystrokes("ctrl-m");
            visual.simulate_input("Before @QA");
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let base = format!("http://{}/api/mcp", listener.local_addr().unwrap());
            let server = std::thread::spawn(move || {
                let (mut socket, _) = listener.accept().unwrap();
                socket.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
                let mut request = vec![];
                while !request.windows(4).any(|bytes| bytes == b"\r\n\r\n") {
                    let mut buf = [0; 1024];
                    let len = socket.read(&mut buf).unwrap();
                    assert!(len > 0);
                    request.extend_from_slice(&buf[..len]);
                }
                let request = String::from_utf8(request).unwrap();
                assert!(request.starts_with("GET /api/tasks/searchByParam?param=QA&projectId=7283 HTTP/1.1"));
                assert!(request.to_ascii_lowercase().contains("authorization: bearer test-human-token\r\n"));
                assert!(!request.to_ascii_lowercase().contains("cookie:"));
                let body = if status == 200 {
                    json!([{"type":"peopleHeading","name":"People"},{"type":"name","id":985,"name":"QA"},{"type":"task","id":5,"name":"Linked task","project_id":7283,"index":5,"ticketNumber":"QA-5"}])
                } else { json!({"message":"Unauthorized"}) }.to_string();
                write!(socket, "HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
            });
            let request = mentions_request(7283, "QA");
            let result = ht_core::Http::with_base(&base, "test-human-token".into()).unwrap()
                .json_request("GET", &request.path, None)
                .and_then(|value| value.as_array().cloned().ok_or_else(|| anyhow::anyhow!("invalid mention response")))
                .map_err(|_| ());
            server.join().unwrap();
            view.update_in(visual, |this, _, cx| {
                this.shell.set_token(Some("test-human-token".into()));
                let generation = this.mentions.generation;
                this.apply_mention_results(generation.wrapping_sub(1), 1, Some(985), Ok(vec![]), cx);
                this.apply_mention_results(generation, 1, Some(985), result, cx);
                assert!(!this.mentions.loading);
                assert_eq!(this.mentions.error, status != 200);
            });
            if status == 200 {
                visual.simulate_keystrokes("down enter");
            } else {
                visual.simulate_keystrokes("enter escape");
            }
            view.read_with(visual, |this, cx| {
                let html = this.comment_input.read(cx).html();
                if status == 200 {
                    assert!(html.contains("href=\"https://app.hypertask.ai/detail/project-7283/5\""), "{html}");
                    assert!(!html.contains("@QA"));
                } else {
                    assert!(html.contains("Before @QA"), "{html}");
                }
                assert!(this.db.mutations().unwrap().is_empty());
            });
        }
    }

    #[gpui::test]
    fn cli7699_native_star_pin_mocked_save_remove_and_flag_off_are_authoritative(
        cx: &mut gpui::TestAppContext,
    ) {
        struct Server(u16);
        impl ht_core::Transport for Server {
            fn get(&mut self, path: &str) -> Result<ht_core::http::Response, ht_core::http::TransportError> {
                assert_eq!(path, "/user/context");
                Ok(ht_core::http::Response { status: 200, body: json!({"success":true,"user":{"id":985}}) })
            }
            fn post(&mut self, path: &str, body: &Value, _: &str) -> Result<ht_core::http::Response, ht_core::http::TransportError> {
                assert_eq!(path, "/api/savedContent/toggleSaved");
                assert_eq!(body, &json!({"taskId":1,"projectId":7283,"commentId":"7","type":body["type"]}));
                Ok(ht_core::http::Response { status: self.0, body: match self.0 {
                    200 => json!({"saved":{"id":"real-saved-id","commentId":7,"type":body["type"],"userId":985}}),
                    201 => json!({}),
                    _ => json!({"message":"Unauthorized"}),
                } })
            }
        }
        cx.update(crate::input::init);
        for (shortcut, kind) in [("ctrl-shift-s", "Private"), ("ctrl-shift-p", "Public")] {
            let (view, visual) = cx.add_window_view(desktop);
            view.update_in(visual, |this, _, _| this.interaction.task_part = 1);
            for status in [200, 201, 401] {
                visual.simulate_keystrokes(shortcut);
                view.update_in(visual, |this, _, cx| {
                    let change = &this.db.mutations().unwrap()[0].payload_json["change"];
                    assert_eq!(change["public"], kind == "Public");
                    let result = this.db.drain_once(&mut Server(status), chrono::Utc::now().timestamp()+1).unwrap();
                    assert!(if status == 401 { matches!(result, ht_core::Drain::Failed(_)) } else { matches!(result, ht_core::Drain::Applied(_)) });
                    this.poll_saved_comments(cx);
                    let comment = &this.db.task_detail(1).unwrap().unwrap().comments[0];
                    let saved = comment.extra.get("savedContent").and_then(Value::as_array).cloned().unwrap_or_default();
                    assert_eq!(saved.len(), usize::from(status == 200));
                    if status == 200 { assert_eq!(saved[0]["type"], kind); }
                    if status == 401 { assert_eq!(this.error.as_deref(), Some("Unauthorized")); }
                });
            }
        }
    }

    #[gpui::test]
    fn cli7699_mention_http_refusal_paints_the_web_empty_result_without_inserting(
        cx: &mut gpui::TestAppContext,
    ) {
        use super::super::shell::tests::{measured, setup};
        let (view, mut visual) = setup(cx);
        view.update(&mut visual, |this, _, cx| { this.open_task(101, cx); }).unwrap();
        visual.simulate_keystrokes("ctrl-m");
        visual.simulate_input("@QA");
        view.update(&mut visual, |this, _, cx| {
            this.shell.set_token(Some("test-human-token".into()));
            this.apply_mention_results(this.mentions.generation, 101, Some(1), Err(()), cx);
        }).unwrap();
        let error = measured(view, &mut visual, "comment.mentions.error");
        assert_eq!(error["text"], "No results found");
        assert!(error["w"].as_f64().unwrap() > 0.);
        visual.simulate_keystrokes("enter");
        view.read_with(&visual, |this, cx| {
            assert_eq!(this.comment_input.read(cx).content.as_ref(), "@QA");
            assert!(this.db.mutations().unwrap().is_empty());
        }).unwrap();
    }

}
