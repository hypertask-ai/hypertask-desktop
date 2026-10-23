use super::*;
use crate::shortcuts::Action;
use gpui::MouseButton;
use serde_json::Value;

pub(super) struct Shell {
    expanded: bool,
    pub ai_open: bool,
    boards: bool,
    pub settings: bool,
    query: Entity<TextInput>,
    choice: usize,
    recent: bool,
    scroll: ScrollHandle,
    settings_state: settings::Settings,
    token: Option<String>,
    theme_preference: Option<String>,
    agents: bool,
    shortcuts: bool,
    feedback: bool,
    feedback_kind: &'static str,
    feedback_notify: bool,
    feedback_error: bool,
    feedback_input: Entity<TextInput>,
}

impl Shell {
    pub fn set_token(&mut self, token: Option<String>) {
        self.token = token;
    }
    pub fn new(db: &Db, cx: &mut Context<Desktop>) -> Self {
        let query =
            cx.new(|cx| TextInput::new(String::new(), "Search boards", color(0x8e9093), cx));
        cx.observe(&query, |this, _, cx| {
            this.shell.choice = 0;
            cx.notify();
        })
        .detach();
        let feedback_input = cx.new(|cx| {
            TextInput::new(
                String::new(),
                "What's broken, missing, or annoying?",
                color(0x8e9093),
                cx,
            )
            .multiline()
        });
        cx.observe(&feedback_input, |_, _, cx| cx.notify()).detach();
        Self {
            agents: false,
            shortcuts: false,
            feedback: false,
            feedback_kind: "Bug",
            feedback_notify: true,
            feedback_error: false,
            feedback_input,
            expanded: db.kv_get("appShellRailExpanded").ok().flatten().as_deref() == Some("true"),
            ai_open: true,
            boards: false,
            settings: false,
            query,
            choice: 0,
            recent: db.kv_get("boardSortMode").ok().flatten().as_deref() == Some("lastUsed"),
            scroll: ScrollHandle::new(),
            settings_state: settings::Settings::new(cx),
            token: None,
            theme_preference: db.kv_get("theme").ok().flatten(),
        }
    }

    fn width(&self) -> f32 {
        if self.expanded { 130. } else { 48. }
    }
}

impl Desktop {
    fn toggle_rail(&mut self, cx: &mut Context<Self>) {
        self.shell.expanded = !self.shell.expanded;
        if let Err(error) = self.db.kv_set(
            "appShellRailExpanded",
            if self.shell.expanded { "true" } else { "false" },
        ) {
            self.error = Some(error.to_string());
        }
        cx.notify();
    }

    fn toggle_ai(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.shell.ai_open = !self.shell.ai_open;
        if self.shell.ai_open {
            if let Some(sidebar) = &self.ai_sidebar {
                sidebar.update(cx, |sidebar, cx| sidebar.expand(cx));
            }
        } else {
            window.focus(&self.focus, cx);
        }
        cx.notify();
    }

    fn toggle_boards(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.shell.boards = !self.shell.boards;
        self.shell
            .query
            .update(cx, |input, cx| input.set_content(String::new(), cx));
        self.shell.choice = 0;
        let focus = if self.shell.boards {
            self.shell.query.focus_handle(cx)
        } else {
            self.focus.clone()
        };
        window.focus(&focus, cx);
        cx.notify();
    }

    pub(super) fn shell_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let key = &event.keystroke;
        let m = key.modifiers;
        let ctrl = m.control || m.platform;
        if self.shell.feedback {
            match (key.key.as_str(), ctrl) {
                ("escape", false) => {
                    self.shell.feedback = false;
                    window.focus(&self.focus, cx);
                    cx.notify();
                }
                ("enter", true) => self.send_feedback(cx),
                _ => return true,
            }
            cx.stop_propagation();
            return true;
        }
        if self.shell.shortcuts {
            if key.key == "escape" {
                self.shell.shortcuts = false;
                window.focus(&self.focus, cx);
                cx.stop_propagation();
                cx.notify();
            }
            return true;
        }
        if self.shell.settings {
            return self.settings_key(event, window, cx);
        }
        if self.shell.boards {
            match key.key.as_str() {
                "escape" => self.toggle_boards(window, cx),
                "b" if ctrl && !m.alt && !m.shift => self.toggle_boards(window, cx),
                "down" => {
                    self.shell.choice =
                        (self.shell.choice + 1).min(self.shell_boards(cx).len().saturating_sub(1));
                    cx.notify();
                }
                "up" => {
                    self.shell.choice = self.shell.choice.saturating_sub(1);
                    cx.notify();
                }
                "enter" => {
                    if let Some(id) = self
                        .shell_boards(cx)
                        .get(self.shell.choice)
                        .and_then(|b| b["id"].as_i64())
                    {
                        self.choose_board(id, window, cx);
                    }
                }
                _ => return true,
            }
            cx.stop_propagation();
            return true;
        }
        let editor = self.title_input.focus_handle(cx).is_focused(window)
            || self.comment_input.focus_handle(cx).is_focused(window)
            || self
                .interaction
                .description
                .focus_handle(cx)
                .is_focused(window)
            || self
                .ai_sidebar
                .as_ref()
                .is_some_and(|s| s.read(cx).has_focus(window, cx));
        if editor || self.interaction.modal.is_some() {
            return false;
        }
        match (key.key.as_str(), ctrl, m.shift, m.alt) {
            ("[", false, false, false) => self.toggle_rail(cx),
            ("5" | "]", false, false, false) | ("/" | "?", true, true, false) => {
                self.toggle_ai(window, cx)
            }
            ("?", false, _, false) => self.open_shortcuts(window, cx),
            ("b", true, false, false) => self.toggle_boards(window, cx),
            ("\\", false, false, false) => self.open_settings(window, cx),
            _ => return false,
        }
        cx.stop_propagation();
        true
    }

    pub(super) fn rail(&self, cx: &mut Context<Self>) -> AnyElement {
        let t = self.colors;
        let wide = self.shell.expanded;
        let button = |i: usize, name: &'static str, label: &'static str, selected: bool| {
            div()
                .w(px(if wide { 116. } else { 36. }))
                .h(px(34.))
                .flex()
                .items_center()
                .when(!wide, |d| d.justify_center())
                .when(wide, |d| d.pl(px(10.)).gap(px(10.)))
                .rounded(px(5.))
                .text_size(px(14.))
                .text_color(color(if selected { t.text } else { t.muted }))
                .when(selected, |d| d.bg(color(t.hover)))
                .child(icon(name, if selected { t.text } else { t.muted }, 18.))
                .when(wide, |d| d.child(label))
                .layout_key(&self.layout, format!("rail.icon.{i}"), Some(label.into()))
        };
        div()
            .w(px(self.shell.width()))
            .h_full()
            .flex_shrink_0()
            .bg(color(t.rail))
            .border_r_1()
            .border_color(color(t.rail_border))
            .flex()
            .flex_col()
            .items_center()
            .py(px(8.))
            .child(
                div()
                    .id("rail-expand")
                    .cursor_pointer()
                    .w_full()
                    .h(px(20.))
                    .mb(px(4.))
                    .flex()
                    .when(wide, |d| d.justify_end().pr(px(10.)))
                    .when(!wide, |d| d.justify_center())
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_rail(cx)))
                    .child(icon(
                        if wide {
                            "chevron-left"
                        } else {
                            "chevron-right"
                        },
                        t.muted,
                        16.,
                    ))
                    .layout_key(&self.layout, "rail.icon.0", None),
            )
            .child(
                div()
                    .id("rail-boards")
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, window, cx| this.toggle_boards(window, cx)))
                    .child(button(1, "panel-left", "Boards", self.shell.boards)),
            )
            .child(div().h(px(12.)).flex_shrink_0())
            .child(
                div()
                    .id("rail-inbox")
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, _, cx| this.switch(Screen::Inbox, cx)))
                    .child(button(2, "inbox", "Inbox", self.screen == Screen::Inbox)),
            )
            .child(div().h(px(4.)))
            .child(
                div()
                    .id("rail-board")
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.shell.agents = false;
                        this.run_shortcut(Action::Board, window, cx);
                    }))
                    .child(button(
                        3,
                        "columns-2",
                        "Board",
                        self.screen == Screen::Board,
                    )),
            )
            .child(div().mt(px(4.)).child(button(4, "rows-3", "Table", false)))
            .child(
                div()
                    .mt(px(4.))
                    .child(button(5, "calendar", "Calendar", false)),
            )
            .child(
                div()
                    .id("rail-ai")
                    .mt(px(4.))
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, window, cx| this.toggle_ai(window, cx)))
                    .child(button(6, "sparkles", "AI Chat", false)),
            )
            .child(
                div()
                    .id("rail-search")
                    .mt(px(4.))
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.run_shortcut(Action::Search, window, cx)
                    }))
                    .child(button(7, "search", "Search", false)),
            )
            .child(div().flex_1())
            .child(
                div()
                    .id("rail-agents")
                    .mb(px(4.))
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.shell.agents = true;
                        window.focus(&this.focus, cx);
                        cx.notify();
                    }))
                    .child(button(8, "bot", "Agents", self.shell.agents)),
            )
            .child(
                div()
                    .mb(px(4.))
                    .child(button(9, "user-plus", "Invite", false)),
            )
            .child(div().mb(px(4.)).child(button(10, "rocket", "News", false)))
            .child(
                div()
                    .id("rail-feedback")
                    .mb(px(4.))
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, window, cx| this.open_feedback(window, cx)))
                    .child(button(11, "message-circle", "Feedback", false)),
            )
            .child(
                div()
                    .id("rail-settings")
                    .mb(px(4.))
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, window, cx| this.open_settings(window, cx)))
                    .child(button(12, "settings", "Settings", false)),
            )
            .layout_key(&self.layout, "rail", None)
    }

    fn shell_boards(&self, cx: &App) -> Vec<Value> {
        let query = self.shell.query.read(cx).content.to_lowercase();
        let mut boards: Vec<_> = self
            .db
            .projects()
            .unwrap_or_default()
            .into_iter()
            .filter(|b| {
                b["title"]
                    .as_str()
                    .or(b["name"].as_str())
                    .unwrap_or("")
                    .to_lowercase()
                    .contains(&query)
            })
            .collect();
        if self.shell.recent {
            boards.sort_by_cached_key(|b| {
                std::cmp::Reverse(
                    self.db
                        .kv_get(&format!("boardUsed:{}", b["id"]))
                        .ok()
                        .flatten()
                        .and_then(|s| s.parse::<u128>().ok())
                        .unwrap_or(0),
                )
            });
        }
        boards
    }

    fn choose_board(&mut self, id: i64, window: &mut Window, cx: &mut Context<Self>) {
        self.options.board = id;
        let _ = self.db.kv_set("last_board", &id.to_string());
        let _ = self.db.kv_set(
            &format!("boardUsed:{id}"),
            &SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
                .to_string(),
        );
        self.toggle_boards(window, cx);
        self.switch(Screen::Board, cx);
    }

    pub(super) fn shell_overlay(&self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        if self.shell.feedback {
            return self.feedback_overlay(cx);
        }
        if self.shell.shortcuts {
            return self.shortcut_overlay(cx);
        }
        if !self.shell.boards {
            return div().into_any_element();
        }
        let t = self.colors;
        div()
            .absolute()
            .inset_0()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, window, cx| {
                    this.toggle_boards(window, cx);
                }),
            )
            .child(
                div()
                    .id("boards-flyout")
                    .absolute()
                    .left(px(self.shell.width() + 8.))
                    .top_0()
                    .bottom_0()
                    .w(px(461.))
                    .bg(color(t.container))
                    .text_size(px(14.))
                    .flex()
                    .flex_col()
                    .shadow(shadow())
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child(
                        div()
                            .m(px(16.))
                            .p(px(8.))
                            .bg(color(t.card))
                            .child(self.shell.query.clone())
                            .layout_key(&self.layout, "shell.boards.search", None),
                    )
                    .child(
                        div()
                            .px(px(16.))
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .child("Group by")
                            .children([(false, "Teams"), (true, "Recent")].map(
                                |(recent, label)| {
                                    div()
                                        .id(label)
                                        .px(px(8.))
                                        .py(px(2.))
                                        .rounded(px(4.))
                                        .when(self.shell.recent == recent, |d| {
                                            d.bg(color(t.active))
                                        })
                                        .cursor_pointer()
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.shell.recent = recent;
                                            let _ = this.db.kv_set(
                                                "boardSortMode",
                                                if recent { "lastUsed" } else { "teams" },
                                            );
                                            cx.notify();
                                        }))
                                        .child(label)
                                },
                            )),
                    )
                    .child(
                        div()
                            .id("boards-list")
                            .flex_1()
                            .overflow_y_scroll()
                            .track_scroll(&self.shell.scroll)
                            .children(self.shell_boards(cx).into_iter().enumerate().map(
                                |(i, board)| {
                                    let id = board["id"].as_i64().unwrap_or(0);
                                    let title = board["title"]
                                        .as_str()
                                        .or(board["name"].as_str())
                                        .unwrap_or("")
                                        .to_owned();
                                    div()
                                        .id(("shell-board", id as u64))
                                        .px(px(16.))
                                        .py(px(8.))
                                        .cursor_pointer()
                                        .when(i == self.shell.choice, |d| d.bg(color(t.active)))
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            this.choose_board(id, window, cx)
                                        }))
                                        .child(title.clone())
                                        .layout_key(
                                            &self.layout,
                                            format!("shell.board.{i}"),
                                            Some(title),
                                        )
                                },
                            )),
                    )
                    .child(
                        div()
                            .id("boards-settings")
                            .p(px(16.))
                            .cursor_pointer()
                            .child("Settings")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.toggle_boards(window, cx);
                                this.open_settings(window, cx);
                            })),
                    )
                    .layout_key(&self.layout, "shell.boards", None),
            )
            .into_any_element()
    }
}

#[path = "shell_settings.rs"]
mod settings;

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use gpui::{TestAppContext, VisualTestContext, WindowHandle, size};

    pub(in super::super) fn setup(
        cx: &mut TestAppContext,
    ) -> (WindowHandle<Desktop>, VisualTestContext) {
        cx.update(crate::input::init);
        let view = cx.open_window(size(px(1440.), px(900.)), |window, cx| {
            let mut db = Db::memory().unwrap();
            db.apply_response(&serde_json::json!({"success": true, "projects": [
                {"id": 15, "title": "Alpha", "name": "Alpha", "status": "Normal"},
                {"id": 16, "title": "Beta", "name": "Beta", "status": "Normal"}
            ]}))
            .unwrap();
            let options =
                Options::parse(["--offline", "--board", "15"].map(str::to_owned)).unwrap();
            let mut desktop = Desktop::new(db, options, window, cx);
            desktop.layout = Some(Rc::new(RefCell::new(LayoutDump::default())));
            desktop.ready_scheduled = true;
            window.focus(&desktop.focus, cx);
            desktop
        });
        let visual = VisualTestContext::from_window(view.into(), cx);
        (view, visual)
    }

    pub(in super::super) fn draw(view: WindowHandle<Desktop>, cx: &mut VisualTestContext) {
        let entity = view.entity(cx).unwrap();
        cx.draw(point(px(0.), px(0.)), size(px(1440.), px(900.)), |_, _| {
            entity.into_element()
        });
    }

    pub(in super::super) fn measured(
        view: WindowHandle<Desktop>,
        cx: &mut VisualTestContext,
        key: &str,
    ) -> Value {
        draw(view, cx);
        view.read_with(cx, |this, _| {
            this.layout
                .as_ref()
                .unwrap()
                .borrow()
                .interaction_elements()
                .into_iter()
                .find(|e| e["key"] == key)
                .unwrap_or_else(|| panic!("visible element {key} missing"))
        })
        .unwrap()
    }

    pub(in super::super) fn click(
        view: WindowHandle<Desktop>,
        cx: &mut VisualTestContext,
        key: &str,
    ) {
        let e = measured(view, cx, key);
        cx.simulate_click(
            point(
                px(e["x"].as_f64().unwrap() as f32 + e["w"].as_f64().unwrap() as f32 / 2.),
                px(e["y"].as_f64().unwrap() as f32 + e["h"].as_f64().unwrap() as f32 / 2.),
            ),
            Default::default(),
        );
        draw(view, cx);
    }

    #[gpui::test]
    fn shell_chevron_and_bracket_move_content_and_persist(cx: &mut TestAppContext) {
        let (view, mut cx) = setup(cx);
        assert_eq!(measured(view, &mut cx, "rail")["w"], 48.);
        let before = measured(view, &mut cx, "inbox.tabs")["x"].as_f64().unwrap();
        click(view, &mut cx, "rail.icon.0");
        assert_eq!(measured(view, &mut cx, "rail")["w"], 130.);
        assert_eq!(
            measured(view, &mut cx, "inbox.tabs")["x"].as_f64().unwrap() - before,
            82.
        );
        assert_eq!(measured(view, &mut cx, "rail.icon.1")["text"], "Boards");
        view.read_with(&cx, |this, _| {
            assert_eq!(
                this.db.kv_get("appShellRailExpanded").unwrap().as_deref(),
                Some("true")
            )
        })
        .unwrap();
        cx.simulate_keystrokes("[");
        assert_eq!(measured(view, &mut cx, "rail")["w"], 48.);
    }

    #[gpui::test]
    fn shell_ai_icon_and_all_toggle_keys_resize_workspace(cx: &mut TestAppContext) {
        let (view, mut cx) = setup(cx);
        draw(view, &mut cx);
        for key in [None, Some("5"), Some("]"), Some("ctrl-shift-/")] {
            if let Some(key) = key {
                cx.simulate_keystrokes(key);
            } else {
                click(view, &mut cx, "rail.icon.6");
            }
            let width = measured(view, &mut cx, "main")["w"].as_f64().unwrap();
            let open = view.read_with(&cx, |this, _| this.shell.ai_open).unwrap();
            assert_eq!(width, if open { 1020. } else { 1440. });
        }
    }

    #[gpui::test]
    fn shell_boards_flyout_click_search_enter_escape_and_outside(cx: &mut TestAppContext) {
        let (view, mut cx) = setup(cx);
        click(view, &mut cx, "rail.icon.1");
        assert_eq!(measured(view, &mut cx, "shell.boards")["w"], 461.);
        cx.simulate_input("Beta");
        assert_eq!(measured(view, &mut cx, "shell.board.0")["text"], "Beta");
        cx.simulate_keystrokes("enter");
        view.read_with(&cx, |this, _| {
            assert_eq!(this.screen, Screen::Board);
            assert_eq!(this.options.board, 16);
            assert!(!this.shell.boards);
        })
        .unwrap();
        cx.simulate_keystrokes("ctrl-b escape");
        view.update(&mut cx, |this, window, _| {
            assert!(this.focus.is_focused(window))
        })
        .unwrap();
        cx.simulate_keystrokes("ctrl-b");
        draw(view, &mut cx);
        cx.simulate_click(point(px(800.), px(300.)), Default::default());
        view.read_with(&cx, |this, _| assert!(!this.shell.boards))
            .unwrap();
    }

    #[gpui::test]
    fn shell_settings_icon_key_back_search_and_copy(cx: &mut TestAppContext) {
        let (view, mut cx) = setup(cx);
        click(view, &mut cx, "rail.icon.12");
        assert_eq!(measured(view, &mut cx, "settings.title")["text"], "General");
        click(view, &mut cx, "settings.search");
        cx.simulate_input("CLI");
        click(view, &mut cx, "settings.nav.cli");
        assert_eq!(measured(view, &mut cx, "settings.title")["text"], "CLI");
        click(view, &mut cx, "settings.copy.login");
        assert_eq!(
            cx.read_from_clipboard().unwrap().text().unwrap(),
            "hypertask login"
        );
        click(view, &mut cx, "settings.back");
        assert_eq!(measured(view, &mut cx, "rail")["w"], 48.);
        cx.simulate_keystrokes("\\");
        assert_eq!(measured(view, &mut cx, "settings.title")["text"], "CLI");
        cx.simulate_keystrokes("escape");
        view.read_with(&cx, |this, _| {
            assert!(!this.shell.settings);
            assert_eq!(this.screen, Screen::Inbox);
        })
        .unwrap();
    }
}

impl Desktop {
    pub(super) fn init_shell(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(preference) = self.shell.theme_preference.clone() {
            let theme = if preference == "system" {
                system_theme(window.appearance())
            } else {
                ThemeName::parse(&preference).unwrap_or(self.options.theme)
            };
            self.apply_shell_theme(theme, cx);
        }
        cx.observe_window_appearance(window, |this, window, cx| {
            if this.shell.theme_preference.as_deref() == Some("system") {
                this.apply_shell_theme(system_theme(window.appearance()), cx);
            }
        })
        .detach();
        self.profile_blur(window, cx);
    }

    fn apply_shell_theme(&mut self, theme: ThemeName, cx: &mut Context<Self>) {
        self.options.theme = theme;
        self.colors = theme.colors();
        self.reload();
        if let Some(sidebar) = &self.ai_sidebar {
            sidebar.update(cx, |sidebar, cx| sidebar.set_theme(self.colors, cx));
        }
        cx.notify();
    }

    pub(super) fn shell_reopen(&self, cx: &mut Context<Self>) -> AnyElement {
        div()
            .when(!self.shell.ai_open, |d| {
                d.child(
                    div()
                        .id("shell-ai-reopen")
                        .absolute()
                        .right(px(4.))
                        .top(px(8.))
                        .h(px(20.))
                        .cursor_pointer()
                        .child(icon("chevron-left", self.colors.muted, 16.))
                        .on_click(cx.listener(|this, _, window, cx| this.toggle_ai(window, cx)))
                        .layout_key(&self.layout, "shell.ai.reopen", None),
                )
            })
            .into_any_element()
    }
}

fn system_theme(appearance: gpui::WindowAppearance) -> ThemeName {
    match appearance {
        gpui::WindowAppearance::Dark | gpui::WindowAppearance::VibrantDark => ThemeName::Amoled,
        _ => ThemeName::Porcelain,
    }
}

impl Desktop {
    pub(super) fn shell_body(&self, body: AnyElement) -> AnyElement {
        if !self.shell.agents {
            return body;
        }
        div()
            .size_full()
            .bg(color(self.colors.page))
            .flex()
            .justify_center()
            .child(
                div()
                    .w_full()
                    .max_w(px(1120.))
                    .px(px(24.))
                    .py(px(28.))
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .text_size(px(22.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Agents")
                            .layout_key(&self.layout, "shell.agents.title", Some("Agents".into())),
                    )
                    .child(
                        div()
                            .mt(px(24.))
                            .text_size(px(13.))
                            .text_color(color(0xcf222e))
                            .child("Unauthorized")
                            .layout_key(
                                &self.layout,
                                "shell.agents.error",
                                Some("Unauthorized".into()),
                            ),
                    ),
            )
            .into_any_element()
    }

    fn open_feedback(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.shell.feedback = true;
        self.shell.feedback_error = false;
        window.focus(&self.shell.feedback_input.focus_handle(cx), cx);
        cx.notify();
    }

    fn send_feedback(&mut self, cx: &mut Context<Self>) {
        if !self.shell.feedback_input.read(cx).content.trim().is_empty() {
            // /api/feedback requires a cookie session; /api/mcp/feedback files agent diagnostics, not user feedback.
            self.shell.feedback_error = true;
            cx.notify();
        }
    }

    fn feedback_overlay(&self, cx: &mut Context<Self>) -> AnyElement {
        let t = self.colors;
        div()
            .absolute()
            .inset_0()
            .bg(rgba(0x00000066))
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .id("feedback-modal")
                    .w(px(600.))
                    .rounded(px(5.))
                    .bg(color(t.container))
                    .p(px(24.))
                    .shadow(shadow())
                    .flex()
                    .flex_col()
                    .gap(px(12.))
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child(
                        div()
                            .text_size(px(20.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Send feedback")
                            .layout_key(
                                &self.layout,
                                "shell.feedback.title",
                                Some("Send feedback".into()),
                            ),
                    )
                    .child(div().flex().gap(px(6.)).children(
                        ["Bug", "Idea", "Question", "Praise"].map(|kind| {
                            div()
                                .id(kind)
                                .flex_1()
                                .px(px(12.))
                                .py(px(8.))
                                .rounded(px(4.))
                                .cursor_pointer()
                                .when(self.shell.feedback_kind == kind, |d| d.bg(color(t.active)))
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.shell.feedback_kind = kind;
                                    window.focus(&this.shell.feedback_input.focus_handle(cx), cx);
                                    cx.notify();
                                }))
                                .child(kind)
                                .layout_key(
                                    &self.layout,
                                    format!("shell.feedback.kind.{kind}"),
                                    Some(kind.into()),
                                )
                        }),
                    ))
                    .child(
                        div()
                            .min_h(px(180.))
                            .px(px(16.))
                            .py(px(12.))
                            .child(self.shell.feedback_input.clone())
                            .layout_key(&self.layout, "shell.feedback.input", None),
                    )
                    .child(
                        div()
                            .id("feedback-notify")
                            .p(px(8.))
                            .cursor_pointer()
                            .child(format!(
                                "{} Email me when this is fixed",
                                if self.shell.feedback_notify {
                                    "☑"
                                } else {
                                    "☐"
                                }
                            ))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.shell.feedback_notify = !this.shell.feedback_notify;
                                cx.notify();
                            })),
                    )
                    .when(self.shell.feedback_error, |d| {
                        d.child(
                            div()
                                .text_color(color(0xcf222e))
                                .child("Unable to send feedback. Please try again.")
                                .layout_key(
                                    &self.layout,
                                    "shell.feedback.error",
                                    Some("Unable to send feedback. Please try again.".into()),
                                ),
                        )
                    })
                    .child(
                        div()
                            .flex()
                            .justify_end()
                            .gap(px(8.))
                            .child(
                                div()
                                    .id("feedback-cancel")
                                    .p(px(8.))
                                    .cursor_pointer()
                                    .child("Cancel")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.shell.feedback = false;
                                        window.focus(&this.focus, cx);
                                        cx.notify();
                                    }))
                                    .layout_key(&self.layout, "shell.feedback.cancel", None),
                            )
                            .child(
                                div()
                                    .id("feedback-send")
                                    .p(px(8.))
                                    .cursor_pointer()
                                    .child("Send")
                                    .on_click(cx.listener(|this, _, _, cx| this.send_feedback(cx)))
                                    .layout_key(&self.layout, "shell.feedback.send", None),
                            ),
                    )
                    .layout_key(&self.layout, "shell.feedback", None),
            )
            .into_any_element()
    }

    fn open_shortcuts(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.shell.shortcuts = true;
        self.shortcut_search_focus(window, cx);
        cx.notify();
    }

    fn shortcut_overlay(&self, cx: &mut Context<Self>) -> AnyElement {
        div()
            .absolute()
            .inset_0()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, window, cx| {
                    this.shell.shortcuts = false;
                    window.focus(&this.focus, cx);
                    cx.notify();
                }),
            )
            .child(
                div()
                    .id("shortcut-sidebar")
                    .absolute()
                    .right_0()
                    .top_0()
                    .bottom_0()
                    .w(gpui::relative(0.26))
                    .bg(color(self.colors.container))
                    .px(px(16.))
                    .overflow_y_scroll()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child(self.shortcut_settings(cx))
                    .layout_key(&self.layout, "shell.shortcuts", None),
            )
            .into_any_element()
    }
}
