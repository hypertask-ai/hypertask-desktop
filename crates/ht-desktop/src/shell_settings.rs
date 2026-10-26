use super::*;
use gpui::ClipboardItem;
use std::collections::BTreeMap;
#[cfg(not(test))]
use std::sync::mpsc;

#[derive(Clone, Debug)]
struct Request {
    serial: u64,
    client_id: Option<String>,
    invite: bool,
    method: &'static str,
    path: String,
    body: Option<Value>,
    section: &'static str,
    board: i64,
}

pub(super) struct Settings {
    webhooks: webhooks::Webhooks,
    section: &'static str,
    return_to_workspace: bool,
    calendar_sunday: bool,
    calendar_weekends: bool,
    search: Entity<TextInput>,
    scroll: ScrollHandle,
    board: i64,
    board_picker: bool,
    data: BTreeMap<String, Value>,
    pending: Option<Request>,
    serial: u64,
    error: Option<String>,
    message: Option<String>,
    draft: Entity<TextInput>,
    import_url: Entity<TextInput>,
    editing: Option<i64>,
    delete: Option<i64>,
    lifecycle: Option<&'static str>,
    integration: usize,
    bearer_config: bool,
    copied: Option<&'static str>,
    preview: Vec<Value>,
    selected_slugs: std::collections::BTreeSet<String>,
    #[cfg(test)]
    requests: Vec<Request>,
}
impl Settings {
    pub fn new(db: &Db, cx: &mut Context<Desktop>) -> Self {
        let calendar: Value = db
            .kv_get("calendarSettingsAtom")
            .ok()
            .flatten()
            .and_then(|value| serde_json::from_str(&value).ok())
            .unwrap_or(Value::Null);
        let search =
            cx.new(|cx| TextInput::new(String::new(), "Search settings", color(0x8e9093), cx));
        cx.observe(&search, |_, _, cx| cx.notify()).detach();
        let draft = cx.new(|cx| TextInput::new(String::new(), "Display name", color(0x8e9093), cx));
        let import_url = cx.new(|cx| {
            TextInput::new(
                String::new(),
                "https://github.com/owner/repository",
                color(0x8e9093),
                cx,
            )
        });
        cx.observe(&draft, |_, _, cx| cx.notify()).detach();
        cx.observe(&import_url, |_, _, cx| cx.notify()).detach();
        Self {
            webhooks: webhooks::Webhooks::new(cx),
            section: "general",
            return_to_workspace: false,
            calendar_sunday: calendar["weekStartsOn"] == "sunday",
            calendar_weekends: calendar["showWeekends"].as_bool().unwrap_or(false),
            search,
            scroll: ScrollHandle::new(),
            board: 0,
            board_picker: false,
            data: BTreeMap::new(),
            pending: None,
            serial: 0,
            error: None,
            message: None,
            draft,
            import_url,
            editing: None,
            delete: None,
            lifecycle: None,
            integration: 0,
            bearer_config: false,
            copied: None,
            preview: vec![],
            selected_slugs: Default::default(),
            #[cfg(test)]
            requests: vec![],
        }
    }
}

const SECTIONS: &[(&str, &str, &str)] = &[
    ("profile", "general", "General"),
    ("profile", "appearance", "Appearance"),
    ("profile", "calendar", "Calendar"),
    ("team", "skills", "Skills"),
    ("board", "board-general", "General"),
    ("board", "board-members", "Members"),
    ("board", "board-ai", "Custom instructions"),
    ("board", "board-skills", "Skills"),
    ("board", "board-files", "Files"),
    ("connect", "mcp", "MCP"),
    ("connect", "webhooks", "Webhooks"),
    ("connect", "cli", "CLI"),
    ("connect", "api", "REST API"),
    ("help", "shortcuts", "Shortcuts"),
    ("help", "learn", "Learn Hypertask"),
];
const LINKS: &[(&str, &str)] = &[
    ("Changelog", "https://docs.hypertask.ai/changelog"),
    ("Terms of Service", "https://hypertask.ai/terms"),
    ("Privacy", "https://hypertask.ai/privacy"),
];
const INSTALL: &str = "curl -fsSL https://hypertask.ai/install | sh";
const LOGIN: &str = "hypertask login";
const API: &str = "https://app.hypertask.ai/api/mcp";
const EXAMPLE: &str = "curl \"https://app.hypertask.ai/api/mcp/projects\" \\\n  -H \"Authorization: Bearer $HYPERTASK_TOKEN\"";

impl Desktop {
    pub(in super::super) fn open_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.shell.settings { self.shell.settings_state.return_to_workspace = true; }
        self.shell.settings = true;
        self.shell.settings_state.board = self.options.board;
        self.load_settings(window, cx);
        self.shell
            .settings_state
            .search
            .update(cx, |input, cx| input.set_content(String::new(), cx));
        window.focus(&self.focus, cx);
        cx.notify();
    }

    pub(in super::super) fn enter_settings_directly(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.open_settings(window, cx);
        self.shell.settings_state.return_to_workspace = false;
    }

    fn close_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.shell.settings = false;
        if !self.shell.settings_state.return_to_workspace {
            let board = self.db.kv_get("last_board").ok().flatten().and_then(|value| value.parse::<i64>().ok())
                .filter(|id| self.db.projects().unwrap_or_default().iter().any(|board| board["id"] == *id));
            if let Some(board) = board {
                self.options.board = board;
                self.switch(Screen::Board, cx);
            } else {
                self.switch(Screen::Inbox, cx);
            }
        }
        self.shell.settings_state.return_to_workspace = false;
        window.focus(&self.focus, cx);
        cx.notify();
    }

    pub(in super::super) fn settings_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if event.keystroke.key == "enter"
            && self.shell.settings_state.section == "general"
            && self
                .shell
                .settings_state
                .draft
                .focus_handle(cx)
                .is_focused(window)
        {
            self.save_profile(window, cx);
            window.focus(&self.focus, cx);
            cx.stop_propagation();
        } else if event.keystroke.key == "escape" {
            self.close_settings(window, cx);
            cx.stop_propagation();
        }
        true
    }

    pub(in super::super) fn settings_screen(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let t = self.colors;
        let section = self.shell.settings_state.section;
        let tab = SECTIONS
            .iter()
            .find(|(_, id, _)| *id == section)
            .map_or("help", |s| s.0);
        let query = self
            .shell
            .settings_state
            .search
            .read(cx)
            .content
            .to_lowercase();
        let title = SECTIONS
            .iter()
            .find(|(_, id, _)| *id == section)
            .map_or("Help", |s| s.2);
        div()
            .size_full()
            .flex()
            .bg(color(t.container))
            .text_color(color(t.text))
            .font_family(self.options.theme.font())
            .text_size(px(14.))
            .track_focus(&self.focus)
            .capture_key_down(cx.listener(Self::key_down))
            .child(
                div()
                    .w(px(250.))
                    .h_full()
                    .flex_shrink_0()
                    .border_r_1()
                    .border_color(color(t.border))
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .p(px(16.))
                            .border_b_1()
                            .border_color(color(t.border))
                            .child(
                                div()
                                    .id("settings-back")
                                    .p(px(8.))
                                    .mb(px(16.))
                                    .cursor_pointer()
                                    .child("←  Back to app")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.close_settings(window, cx)
                                    }))
                                    .layout_key(
                                        &self.layout,
                                        "settings.back",
                                        Some("Back to app".into()),
                                    ),
                            )
                            .child(
                                div()
                                    .h(px(36.))
                                    .px(px(12.))
                                    .child(self.shell.settings_state.search.clone())
                                    .layout_key(&self.layout, "settings.search", None),
                            ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .p(px(12.))
                            .when(tab == "board", |d| d.child(self.settings_board_picker(cx)))
                            .children(
                                SECTIONS
                                    .iter()
                                    .filter(|(scope, _, label)| {
                                        (query.is_empty() && *scope == tab)
                                            || (!query.is_empty()
                                                && label.to_lowercase().contains(&query))
                                    })
                                    .map(|&(_, id, label)| {
                                        div()
                                            .id(id)
                                            .px(px(8.))
                                            .py(px(8.))
                                            .rounded(px(5.))
                                            .cursor_pointer()
                                            .when(id == section, |d| d.bg(color(t.active)))
                                            .on_click(cx.listener(move |this, _, window, cx| {
                                                this.select_settings(id, window, cx);
                                                window.focus(&this.focus, cx);
                                                cx.notify();
                                            }))
                                            .child(label)
                                            .layout_key(
                                                &self.layout,
                                                format!("settings.nav.{id}"),
                                                Some(label.into()),
                                            )
                                    }),
                            )
                            .when(tab == "help" || !query.is_empty(), |d| {
                                d.children(
                                    LINKS
                                        .iter()
                                        .filter(|(label, _)| label.to_lowercase().contains(&query))
                                        .map(|&(label, url)| self.settings_link(label, url)),
                                )
                            }),
                    )
                    .child(
                        div()
                            .p(px(12.))
                            .child(self.settings_link("Contact us", "mailto:help@hypertask.ai"))
                            .child(
                                div()
                                    .id("settings-feedback")
                                    .p(px(8.))
                                    .cursor_pointer()
                                    .child("Send feedback")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.open_feedback(window, cx)
                                    }))
                                    .layout_key(&self.layout, "settings.feedback", None),
                            ),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .flex()
                    .flex_col()
                    .child(
                        div().h(px(60.)).w_full().flex().justify_center().child(
                            div()
                                .w_full()
                                .max_w(px(760.))
                                .px(px(24.))
                                .py(px(12.))
                                .flex()
                                .gap(px(4.))
                                .children(
                                    [
                                        ("profile", "Profile", "general"),
                                        ("team", "Team", "skills"),
                                        ("board", "Board", "board-members"),
                                        ("connect", "Connect", "mcp"),
                                        ("help", "Help", "shortcuts"),
                                    ]
                                    .map(
                                        |(id, label, default)| {
                                            div()
                                                .id(id)
                                                .px(px(12.))
                                                .py(px(6.))
                                                .rounded(px(5.))
                                                .cursor_pointer()
                                                .when(tab == id, |d| d.bg(color(t.active)))
                                                .on_click(cx.listener(
                                                    move |this, _, window, cx| {
                                                        this.select_settings(default, window, cx);
                                                    },
                                                ))
                                                .child(label)
                                                .layout_key(
                                                    &self.layout,
                                                    format!("settings.tab.{id}"),
                                                    Some(label.into()),
                                                )
                                        },
                                    ),
                                ),
                        ),
                    )
                    .child(
                        div()
                            .id("settings-content-scroll")
                            .flex_1()
                            .min_h_0()
                            .overflow_y_scroll()
                            .track_scroll(&self.shell.settings_state.scroll)
                            .flex()
                            .items_start()
                            .justify_center()
                            .child(
                                div()
                                    .w_full()
                                    .min_h_full()
                                    .max_w(px(760.))
                                    .px(px(24.))
                                    .pt(px(20.))
                                    .pb(px(32.))
                                    .flex()
                                    .flex_col()
                                    .gap(px(32.))
                                    .child(
                                        div()
                                            .text_size(px(24.))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .child(title)
                                            .layout_key(
                                                &self.layout,
                                                "settings.title",
                                                Some(title.into()),
                                            ),
                                    )
                                    .when_some(
                                        self.shell.settings_state.error.clone(),
                                        |d, error| {
                                            d.child(
                                                div()
                                                    .text_color(color(0xcf222e))
                                                    .child(error.clone())
                                                    .layout_key(
                                                        &self.layout,
                                                        "settings.error",
                                                        Some(error),
                                                    ),
                                            )
                                        },
                                    )
                                    .when_some(
                                        self.shell.settings_state.message.clone(),
                                        |d, message| {
                                            d.child(div().child(message.clone()).layout_key(
                                                &self.layout,
                                                "settings.message",
                                                Some(message),
                                            ))
                                        },
                                    )
                                    .child(self.settings_content(cx)),
                            ),
                    ),
            )
            .relative()
            .child(self.shell_overlay(window, cx))
            .layout_key(&self.layout, "settings", None)
    }

    fn settings_link(&self, label: &'static str, url: &'static str) -> AnyElement {
        div()
            .id(label)
            .px(px(8.))
            .py(px(8.))
            .cursor_pointer()
            .child(label)
            .on_click(move |_, _, cx| cx.open_url(url))
            .layout_key(
                &self.layout,
                format!("settings.link.{label}"),
                Some(label.into()),
            )
    }

    fn code_row(
        &self,
        id: &'static str,
        value: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let value = value.into();
        let copied = self.shell.settings_state.copied == Some(id);
        div()
            .px(px(12.))
            .py(px(8.))
            .rounded(px(5.))
            .bg(color(self.colors.card))
            .flex()
            .items_start()
            .gap(px(12.))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_size(px(12.))
                    .child(value.clone()),
            )
            .child(
                div()
                    .id(id)
                    .px(px(8.))
                    .py(px(4.))
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, window, cx| {
                        cx.write_to_clipboard(ClipboardItem::new_string(value.to_string()));
                        this.shell.settings_state.copied = Some(id);
                        cx.spawn_in(window, async move |view, cx| {
                            cx.background_executor().timer(Duration::from_secs(2)).await;
                            let _ = view.update(cx, |this, cx| {
                                if this.shell.settings_state.copied == Some(id) {
                                    this.shell.settings_state.copied = None;
                                    cx.notify();
                                }
                            });
                        })
                        .detach();
                        cx.notify();
                    }))
                    .child(if copied { "Copied" } else { "Copy" })
                    .layout_key(
                        &self.layout,
                        format!("settings.copy.{id}"),
                        Some(if copied { "Copied" } else { "Copy" }.into()),
                    ),
            )
            .into_any_element()
    }

    fn settings_content(&self, cx: &mut Context<Self>) -> AnyElement {
        let t = self.colors;
        match self.shell.settings_state.section {
            "shortcuts" => self.shortcut_settings(cx),
            "webhooks" => self.webhook_settings(cx),
            "learn" => self.learn_settings(cx),
            "mcp" => self.mcp_settings(cx),
            "general" => self.profile_settings(cx),
            "board-general" => self.lifecycle_settings(cx),
            "board-ai" => self.instruction_settings(cx),
            "board-members" => self.member_settings(),
            "board-files" => self.file_settings(),
            "skills" | "board-skills" => self.skill_settings(cx),
            "calendar" => self.calendar_settings(cx),
            "appearance" => div().flex().flex_col().gap(px(4.))
                .child(div().text_color(color(t.muted)).child("APPEARANCE"))
                .children([
                    ("system", "Follow system"),
                    ("porcelain", "Light · Porcelain"),
                    ("graphite", "Dark · Graphite"),
                    ("amoled", "OLED black · AMOLED"),
                    ("dia", "Paper · Dia"),
                ].into_iter().enumerate().map(|(i, (preference, label))| {
                    let selected = self.shell.theme_preference.as_deref().map_or_else(|| ThemeName::parse(preference).ok() == Some(self.options.theme), |p| p == preference);
                    div().id(("theme", i)).p(px(8.)).rounded(px(5.)).cursor_pointer()
                        .when(selected, |d| d.bg(color(t.active)))
                        .child(if selected { format!("{label}    ✓") } else { label.into() })
                        .on_click(cx.listener(move |this, _, window, cx| {
                            let theme = if preference == "system" { system_theme(window.appearance()) } else { ThemeName::parse(preference).unwrap() };
                            this.shell.theme_preference = Some(preference.into());
                            if let Err(error) = this.db.kv_set("theme", preference) { this.shell.settings_state.error = Some(error.to_string()); }
                            this.apply_shell_theme(theme, cx);
                        })).layout_key(&self.layout, format!("settings.theme.{i}"), Some(label.into()))
                })).into_any_element(),
            "cli" => div().flex().flex_col().gap(px(12.))
                .child("INSTALL").child("Install the Hypertask command-line interface on macOS or Linux.")
                .child(self.code_row("install", INSTALL, cx))
                .child("SIGN IN").child("Authorize the CLI in your browser for this Hypertask account.")
                .child(self.code_row("login", LOGIN, cx))
                .child("NEXT STEPS").child("Run hypertask --help to see every available task, project, comment, and inbox command.")
                .child(self.settings_link("Read the CLI docs", "https://docs.hypertask.ai")).into_any_element(),
            "api" => div().flex().flex_col().gap(px(12.))
                .child("AUTHENTICATION").child("The REST API uses the same bearer token as MCP. Send it in the Authorization header with every request.")
                .child("BASE URL").child(self.code_row("base-url", API, cx))
                .child("EXAMPLE").child("List the projects available to the authenticated account.")
                .child(self.code_row("example", EXAMPLE, cx))
                .child("DOCUMENTATION").child("Browse endpoint details, request fields, and response examples.")
                .child(self.settings_link("Read the API docs", "https://docs.hypertask.ai/")).into_any_element(),
            _ => div().into_any_element(),
        }
    }
}

impl Desktop {
    fn calendar_settings(&self, cx: &mut Context<Self>) -> AnyElement {
        let t = self.colors;
        let settings = &self.shell.settings_state;
        div()
            .flex()
            .flex_col()
            .gap(px(12.))
            .child("Calendar")
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child("Week starts on")
                    .child(
                        div().flex().gap(px(4.)).children(
                            [(false, "Monday"), (true, "Sunday")].into_iter().map(
                                |(sunday, label)| {
                                    let selected = settings.calendar_sunday == sunday;
                                    div()
                                        .id(label)
                                        .p(px(8.))
                                        .rounded(px(4.))
                                        .cursor_pointer()
                                        .when(selected, |d| d.bg(color(t.active)))
                                        .child(label)
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.shell.settings_state.calendar_sunday = sunday;
                                            this.save_calendar_settings(cx);
                                        }))
                                        .layout_key(
                                            &self.layout,
                                            format!("settings.calendar.{label}"),
                                            Some(label.into()),
                                        )
                                },
                            ),
                        ),
                    ),
            )
            .child(
                div()
                    .id("calendar-weekends")
                    .p(px(8.))
                    .cursor_pointer()
                    .child(if settings.calendar_weekends {
                        "Show weekends    ✓"
                    } else {
                        "Show weekends"
                    })
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.shell.settings_state.calendar_weekends =
                            !this.shell.settings_state.calendar_weekends;
                        this.save_calendar_settings(cx);
                    }))
                    .layout_key(
                        &self.layout,
                        "settings.calendar.weekends",
                        Some(
                            if settings.calendar_weekends {
                                "Show weekends    ✓"
                            } else {
                                "Show weekends"
                            }
                            .into(),
                        ),
                    ),
            )
            .into_any_element()
    }

    fn save_calendar_settings(&mut self, cx: &mut Context<Self>) {
        let settings = &self.shell.settings_state;
        let value = serde_json::json!({
            "weekStartsOn": if settings.calendar_sunday { "sunday" } else { "monday" },
            "showWeekends": settings.calendar_weekends,
        });
        if let Err(error) = self.db.kv_set("calendarSettingsAtom", &value.to_string()) {
            self.shell.settings_state.error = Some(error.to_string());
        }
        cx.notify();
    }

    fn select_settings(
        &mut self,
        section: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self
            .shell
            .settings_state
            .pending
            .as_ref()
            .is_some_and(|r| r.method != "GET")
        {
            return;
        }
        self.shell.settings_state.section = section;
        self.shell.settings_state.error = None;
        self.shell.settings_state.message = None;
        self.shell.settings_state.editing = None;
        self.shell.settings_state.delete = None;
        self.shell.settings_state.preview.clear();
        self.shell.settings_state.webhooks.secret = None;
        self.shell
            .settings_state
            .draft
            .update(cx, |input, cx| input.set_content(String::new(), cx));
        let (placeholder, multiline) = match section {
            "board-ai" => ("e.g. Keep the responses concise", true),
            "skills" | "board-skills" => (
                "---\nname: minimalist-review\ndescription: Review a decision.\nargument-hint: [describe your decision]\n---\nYour skill prompt...",
                true,
            ),
            _ => ("Display name", false),
        };
        self.shell
            .settings_state
            .draft
            .update(cx, |input, cx| input.configure(placeholder, multiline, cx));
        self.shell
            .settings_state
            .import_url
            .update(cx, |input, cx| {
                input.set_content(String::new(), cx);
                input.configure(
                    if section == "shortcuts" {
                        "Search shortcuts"
                    } else {
                        "https://github.com/owner/repository"
                    },
                    false,
                    cx,
                );
            });
        self.load_settings(window, cx);
        cx.notify();
    }

    fn settings_path(&self) -> Option<String> {
        let s = &self.shell.settings_state;
        match s.section {
            "general" | "board-general" => Some("/user/context".into()),
            "board-ai" | "board-files" => Some(format!("/projects/{}/instructions", s.board)),
            "board-members" => Some(format!("/projects/{}/members", s.board)),
            "webhooks" => Some(format!("/webhooks?project_id={}", s.board)),
            "skills" => Some("/skills".into()),
            "board-skills" => Some(format!("/skills?project_id={}", s.board)),
            _ => None,
        }
    }

    fn load_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(path) = self.settings_path() {
            self.settings_request("GET", path, None, window, cx);
        }
    }

    fn settings_request(
        &mut self,
        method: &'static str,
        path: String,
        body: Option<Value>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.shell.settings_state.serial += 1;
        let mut request = Request {
            client_id: None,
            invite: (self.shell.invite || !self.shell.invite_emails.is_empty())
                && method == "POST"
                && path.ends_with("/members"),
            serial: self.shell.settings_state.serial,
            method,
            path,
            body,
            section: self.shell.settings_state.section,
            board: self.shell.settings_state.board,
        };
        // A write is never replaced by a second input while it is in flight.
        if self
            .shell
            .settings_state
            .pending
            .as_ref()
            .is_some_and(|r| r.method != "GET")
        {
            return;
        }
        if method != "GET" {
            let queued = ht_core::ShellWrite::new(self.db.current_user_id().unwrap_or(0), method, &request.path, request.body.clone())
                .and_then(|write| self.db.enqueue(Write::Shell { request: write }, chrono::Utc::now().timestamp()))
                .and_then(|id| self.db.mutations()?.into_iter().find(|row| row.id == id).ok_or_else(|| anyhow::anyhow!("queued shell write missing")));
            match queued {
                Ok(row) => request.client_id = Some(row.client_id),
                Err(error) => { self.shell.settings_state.error = Some(error.to_string()); cx.notify(); return; }
            }
            if let Some(worker) = &self.worker { let _ = worker.handle().wake(); }
        }
        self.shell.settings_state.pending = Some(request.clone());
        self.shell.settings_state.error = None;
        self.shell.settings_state.message = None;
        #[cfg(test)]
        {
            let _ = window;
            self.shell.settings_state.requests.push(request);
            cx.notify();
            return;
        }
        #[cfg(not(test))]
        if method != "GET" {
            cx.spawn_in(window, async move |view, cx| {
                loop {
                    cx.background_executor().timer(Duration::from_millis(100)).await;
                    match view.update_in(cx, |this, window, cx| this.poll_settings_write(window, cx)) {
                        Ok(true) => (),
                        _ => break,
                    }
                }
            }).detach();
            cx.notify();
            return;
        }
        #[cfg(not(test))]
        {
            let token = self.shell.token.clone();
            let (send, receive) = mpsc::channel();
            let job = request.clone();
            std::thread::spawn(move || {
                let result = token
                    .ok_or_else(|| anyhow::anyhow!("Sign in to continue"))
                    .and_then(ht_core::Http::new)
                    .and_then(|http| http.json_request("GET", &job.path, None))
                    .map_err(|error| error.to_string());
                let _ = send.send(result);
            });
            cx.spawn_in(window, async move |view, cx| {
                loop {
                    cx.background_executor()
                        .timer(Duration::from_millis(20))
                        .await;
                    match receive.try_recv() {
                        Ok(result) => {
                            let _ = view.update_in(cx, |this, window, cx| {
                                this.settings_result(request, result, window, cx)
                            });
                            break;
                        }
                        Err(mpsc::TryRecvError::Empty) => (),
                        Err(mpsc::TryRecvError::Disconnected) => break,
                    }
                }
            })
            .detach();
            cx.notify();
        }
    }

    fn poll_settings_write(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(request) = self.shell.settings_state.pending.clone().filter(|request| request.client_id.is_some()) else { return false; };
        match self.db.take_shell_result(request.client_id.as_deref().unwrap()) {
            Ok(Some(result)) => {
                let result = if let Some(error) = result["error"].as_str() { Err(error.to_owned()) } else { Ok(result["value"].clone()) };
                self.settings_result(request, result, window, cx);
                false
            }
            Ok(None) => true,
            Err(error) => { self.settings_result(request, Err(error.to_string()), window, cx); false }
        }
    }

    fn settings_result(
        &mut self,
        request: Request,
        result: Result<Value, String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let s = &mut self.shell.settings_state;
        // Late reads must not populate a different section/board or replace a newer request.
        if !s
            .pending
            .as_ref()
            .is_some_and(|r| r.serial == request.serial)
        {
            return;
        }
        s.pending = None;
        if s.board != request.board || s.section != request.section {
            return;
        }
        match result {
            Err(error) => {
                s.error = Some(error);
            }
            Ok(value) if request.method == "GET" => {
                if request.path == "/user/context" && let Some(id) = value["user"]["id"].as_i64().filter(|id| *id > 0) {
                    let _ = self.db.kv_set("current-user-id", &id.to_string());
                }
                if request.section == "general" && !s.draft.focus_handle(cx).is_focused(window) {
                    s.draft.update(cx, |input, cx| {
                        input.set_content(
                            value["user"]["displayName"]
                                .as_str()
                                .unwrap_or("")
                                .to_owned(),
                            cx,
                        )
                    });
                } else if request.section == "board-ai"
                    && !s.draft.focus_handle(cx).is_focused(window)
                {
                    s.draft.update(cx, |input, cx| {
                        input.set_content(
                            value["instruction"]["customInstruction"]
                                .as_str()
                                .unwrap_or("")
                                .to_owned(),
                            cx,
                        )
                    });
                }
                s.data.insert(request.path, value);
            }
            Ok(value)
                if request.path == "/skills/import"
                    && request.body.as_ref().is_some_and(|v| v["dry_run"] == true) =>
            {
                s.preview = value["skills"].as_array().cloned().unwrap_or_default();
                s.selected_slugs = s
                    .preview
                    .iter()
                    .filter_map(|v| v["slug"].as_str().map(str::to_owned))
                    .collect();
            }
            Ok(value) => {
                if request.invite {
                    self.invite_next(window, cx);
                    return;
                }
                if request.path == "/projects/archive" {
                    if let Some(project) = self
                        .db
                        .projects()
                        .unwrap_or_default()
                        .into_iter()
                        .find(|p| p["id"] == request.board)
                    {
                        let mut project = project;
                        project["status"] = request.body.as_ref().unwrap()["status"].clone();
                        let _ = self.db.apply_response(
                            &serde_json::json!({"success": true, "projects": [project]}),
                        );
                    }
                    if let Some(next) = self
                        .db
                        .projects()
                        .unwrap_or_default()
                        .into_iter()
                        .find(|p| p["status"] == "Normal")
                        .and_then(|p| p["id"].as_i64())
                    {
                        self.options.board = next;
                        let _ = self.db.kv_set("last_board", &next.to_string());
                    } else {
                        let _ = self.db.kv_set("last_board", "");
                    }
                    self.shell.settings_state.lifecycle = None;
                    self.close_settings(window, cx);
                    self.switch(Screen::Inbox, cx);
                    return;
                }
                if request.section == "webhooks" {
                    if let Some(secret) = value["secret"].as_str() {
                        s.webhooks.secret = Some(secret.to_owned());
                    }
                }
                s.editing = None;
                s.delete = None;
                s.preview.clear();
                if matches!(request.section, "skills" | "board-skills") {
                    s.draft
                        .update(cx, |input, cx| input.set_content(String::new(), cx));
                }
                let message = if request.section == "webhooks" {
                    if request.path.ends_with("/test") {
                        Some("Signed test queued")
                    } else if request.method == "DELETE" {
                        self.shell.settings_state.webhooks.secret = None;
                        Some("Webhook endpoint deleted")
                    } else if request.path == "/webhooks" {
                        Some("Webhook endpoint added")
                    } else {
                        None
                    }
                } else if request.section == "board-ai" {
                    Some("Custom instructions updated")
                } else {
                    None
                };
                self.load_settings(window, cx);
                self.shell.settings_state.message = message.map(str::to_owned);
            }
        }
        cx.notify();
    }

    fn settings_value(&self) -> Value {
        self.settings_path()
            .and_then(|path| self.shell.settings_state.data.get(&path).cloned())
            .unwrap_or(Value::Null)
    }

    fn settings_action(&self, id: &'static str, label: &'static str) -> gpui::Stateful<gpui::Div> {
        div()
            .id(id)
            .px(px(12.))
            .py(px(8.))
            .rounded(px(5.))
            .border_1()
            .border_color(color(self.colors.border))
            .cursor_pointer()
            .child(label)
    }

    fn profile_settings(&self, cx: &mut Context<Self>) -> AnyElement {
        let value = self.settings_value();
        div()
            .flex()
            .flex_col()
            .gap(px(12.))
            .child("PROFILE")
            .child(
                div()
                    .id("profile-name-row")
                    .on_mouse_down_out(cx.listener(|this, _, window, cx| {
                        if this
                            .shell
                            .settings_state
                            .draft
                            .focus_handle(cx)
                            .is_focused(window)
                        {
                            this.save_profile(window, cx);
                        }
                    }))
                    .flex()
                    .items_center()
                    .gap(px(16.))
                    .border_b_1()
                    .border_color(color(self.colors.border))
                    .p(px(8.))
                    .child("Display name")
                    .child(
                        div()
                            .flex_1()
                            .child(self.shell.settings_state.draft.clone())
                            .layout_key(&self.layout, "settings.profile.name", None),
                    ),
            )
            .child(div().p(px(8.)).child(format!(
                "Email    {}",
                value["user"]["email"].as_str().unwrap_or("")
            )))
            .into_any_element()
    }

    fn save_profile(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = self
            .shell
            .settings_state
            .draft
            .read(cx)
            .content
            .trim()
            .to_string();
        if self.settings_value()["user"]["displayName"].as_str() != Some(name.as_str()) {
            self.settings_request(
                "PATCH",
                "/user/profile".into(),
                Some(serde_json::json!({"displayName": name})),
                window,
                cx,
            );
        }
    }

    fn instruction_settings(&self, cx: &mut Context<Self>) -> AnyElement {
        let board = self
            .db
            .projects()
            .unwrap_or_default()
            .into_iter()
            .find(|b| b["id"] == self.shell.settings_state.board);
        let title = board
            .as_ref()
            .and_then(|b| b["title"].as_str())
            .unwrap_or("");
        div()
            .flex()
            .flex_col()
            .gap(px(12.))
            .child(format!("Guide AI behavior for {title}."))
            .child(
                div()
                    .id("instructions-textarea")
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _, window, cx| {
                            window.focus(&this.shell.settings_state.draft.focus_handle(cx), cx)
                        }),
                    )
                    .min_h(px(220.))
                    .p(px(8.))
                    .border_1()
                    .border_color(color(self.colors.border))
                    .child(self.shell.settings_state.draft.clone())
                    .layout_key(&self.layout, "settings.instructions", None),
            )
            .child(
                self.settings_action("save-instructions", "Save instructions")
                    .on_click(cx.listener(|this, _, window, cx| {
                        let path =
                            format!("/projects/{}/instructions", this.shell.settings_state.board);
                        let text = this.shell.settings_state.draft.read(cx).content.to_string();
                        this.settings_request(
                            "PUT",
                            path,
                            Some(serde_json::json!({"custom_instruction": text})),
                            window,
                            cx,
                        );
                    }))
                    .layout_key(&self.layout, "settings.instructions.save", None),
            )
            .into_any_element()
    }

    fn member_settings(&self) -> AnyElement {
        let members = self.settings_value()["members"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter(|member| member["id"].is_i64() || !member["user"].is_null())
            .collect::<Vec<_>>();
        div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .child("People with access to the selected board.")
            .child(format!("{} PEOPLE", members.len()))
            .when(self.shell.settings_state.pending.is_some(), |d| {
                d.child("Loading members")
            })
            .when(
                members.is_empty() && self.shell.settings_state.pending.is_none(),
                |d| d.child("No members found"),
            )
            .children(members.into_iter().enumerate().map(|(i, member)| {
                let name = member["displayName"]
                    .as_str()
                    .or(member["user"]["displayName"].as_str())
                    .or(member["email"].as_str())
                    .unwrap_or("");
                let owner_id = self
                    .db
                    .projects()
                    .unwrap_or_default()
                    .into_iter()
                    .find(|p| p["id"] == self.shell.settings_state.board)
                    .and_then(|p| p["ownerId"].as_i64());
                let role = member["role"]
                    .as_str()
                    .or_else(|| {
                        (member["id"].as_i64().is_some() && member["id"].as_i64() == owner_id)
                            .then_some("Admin")
                    })
                    .unwrap_or("");
                div()
                    .p(px(8.))
                    .child(format!("{name}    {role}"))
                    .layout_key(
                        &self.layout,
                        format!("settings.member.{i}"),
                        Some(name.into()),
                    )
            }))
            .into_any_element()
    }

    fn file_settings(&self) -> AnyElement {
        let files = self.settings_value()["instruction"]["attachments"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .children(files.into_iter().map(|file| {
                let name = file["name"]
                    .as_str()
                    .or(file["fileName"].as_str())
                    .unwrap_or("")
                    .to_owned();
                let url = file["fileSource"]
                    .as_str()
                    .or(file["url"].as_str())
                    .unwrap_or("")
                    .to_owned();
                div().child(name).cursor_pointer().on_mouse_down(
                    MouseButton::Left,
                    move |_, _, cx| {
                        if url.starts_with("https://") {
                            cx.open_url(&url);
                        }
                    },
                )
            }))
            .into_any_element()
    }

    fn settings_board_picker(&self, cx: &mut Context<Self>) -> AnyElement {
        let boards: Vec<_> = self
            .db
            .projects()
            .unwrap_or_default()
            .into_iter()
            .filter(|b| b["status"].as_str().is_none_or(|status| status == "Normal"))
            .collect();
        let title = boards
            .iter()
            .find(|b| b["id"] == self.shell.settings_state.board)
            .and_then(|b| b["title"].as_str())
            .unwrap_or("Select board")
            .to_owned();
        div()
            .flex()
            .flex_col()
            .child(
                div()
                    .id("settings-board-picker")
                    .p(px(8.))
                    .rounded(px(5.))
                    .bg(color(self.colors.active))
                    .cursor_pointer()
                    .child(title)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.shell.settings_state.board_picker =
                            !this.shell.settings_state.board_picker;
                        cx.notify();
                    }))
                    .layout_key(&self.layout, "settings.board-picker", None),
            )
            .when(self.shell.settings_state.board_picker, |d| {
                d.children(boards.into_iter().enumerate().map(|(i, board)| {
                    let id = board["id"].as_i64().unwrap_or(0);
                    let name = board["title"].as_str().unwrap_or("").to_owned();
                    div()
                        .id(("settings-board", i))
                        .p(px(8.))
                        .cursor_pointer()
                        .child(name.clone())
                        .on_click(cx.listener(move |this, _, window, cx| {
                            if this
                                .shell
                                .settings_state
                                .pending
                                .as_ref()
                                .is_some_and(|r| r.method != "GET")
                            {
                                return;
                            }
                            if let Err(error) = this.db.kv_set("last_board", &id.to_string()) {
                                this.shell.settings_state.error = Some(error.to_string());
                                cx.notify();
                                return;
                            }
                            this.shell.settings_state.board = id;
                            this.shell.settings_state.board_picker = false;
                            this.select_settings(this.shell.settings_state.section, window, cx);
                        }))
                        .layout_key(&self.layout, format!("settings.board.{i}"), Some(name))
                }))
            })
            .into_any_element()
    }
}

impl Desktop {
    fn skill_settings(&self, cx: &mut Context<Self>) -> AnyElement {
        let s = &self.shell.settings_state;
        let board = (s.section == "board-skills").then_some(s.board);
        let skills: Vec<_> = self.settings_value()["skills"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter(|skill| {
                if let Some(board) = board {
                    skill["projectId"] == board
                } else {
                    !skill["userId"].is_null()
                }
            })
            .collect();
        div()
            .flex()
            .flex_col()
            .gap(px(24.))
            .child("Type /slug in AI chat, or @hyperai /slug in a comment.")
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(12.))
                    .child("LIBRARY")
                    .when(s.pending.as_ref().is_some_and(|r| r.method == "GET"), |d| {
                        d.child("Loading skills")
                    })
                    .when(skills.is_empty() && s.pending.is_none(), |d| {
                        d.child("No skills yet.")
                    })
                    .children(skills.into_iter().map(|skill| {
                        let id = skill["id"].as_i64().unwrap_or(0);
                        let enabled = skill["enabled"] == true;
                        let name = skill["name"].as_str().unwrap_or("").to_owned();
                        let markdown = skill_markdown(&skill);
                        div()
                            .id(("skill", id as u64))
                            .py(px(12.))
                            .border_b_1()
                            .border_color(color(self.colors.border))
                            .flex()
                            .flex_col()
                            .gap(px(8.))
                            .child(
                                div()
                                    .flex()
                                    .justify_between()
                                    .child(format!(
                                        "{}  /{}",
                                        name,
                                        skill["slug"].as_str().unwrap_or("")
                                    ))
                                    .child(
                                        self.settings_action(
                                            "skill-enabled",
                                            if enabled { "Enabled  ✓" } else { "Enabled" },
                                        )
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            this.settings_request(
                                                "PATCH",
                                                format!("/skills/{id}"),
                                                Some(serde_json::json!({"enabled": !enabled})),
                                                window,
                                                cx,
                                            )
                                        }))
                                        .layout_key(
                                            &self.layout,
                                            format!("settings.skill.{id}.enabled"),
                                            Some(
                                                if enabled { "Enabled  ✓" } else { "Enabled" }
                                                    .into(),
                                            ),
                                        ),
                                    ),
                            )
                            .when_some(
                                skill["description"].as_str().map(str::to_owned),
                                |d, desc| d.child(desc),
                            )
                            .child(
                                div()
                                    .flex()
                                    .gap(px(8.))
                                    .child(
                                        self.settings_action("edit-skill", "Edit")
                                            .on_click(cx.listener(move |this, _, window, cx| {
                                                this.shell.settings_state.editing = Some(id);
                                                this.shell.settings_state.draft.update(
                                                    cx,
                                                    |input, cx| {
                                                        input.set_content(markdown.clone(), cx)
                                                    },
                                                );
                                                window.focus(
                                                    &this
                                                        .shell
                                                        .settings_state
                                                        .draft
                                                        .focus_handle(cx),
                                                    cx,
                                                );
                                                cx.notify();
                                            }))
                                            .layout_key(
                                                &self.layout,
                                                format!("settings.skill.{id}.edit"),
                                                None,
                                            ),
                                    )
                                    .child(
                                        self.settings_action("delete-skill", "Delete")
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                this.shell.settings_state.delete = Some(id);
                                                cx.notify();
                                            }))
                                            .layout_key(
                                                &self.layout,
                                                format!("settings.skill.{id}.delete"),
                                                None,
                                            ),
                                    ),
                            )
                            .when(s.delete == Some(id), |d| {
                                d.child(
                                    div()
                                        .flex()
                                        .gap(px(8.))
                                        .child(format!("Delete {name}?"))
                                        .child(
                                            self.settings_action("confirm-delete-skill", "Delete")
                                                .on_click(cx.listener(
                                                    move |this, _, window, cx| {
                                                        this.settings_request(
                                                            "DELETE",
                                                            format!("/skills/{id}"),
                                                            None,
                                                            window,
                                                            cx,
                                                        )
                                                    },
                                                ))
                                                .layout_key(
                                                    &self.layout,
                                                    "settings.skill.confirm-delete",
                                                    None,
                                                ),
                                        )
                                        .child(
                                            self.settings_action("cancel-delete-skill", "Cancel")
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.shell.settings_state.delete = None;
                                                    cx.notify();
                                                }))
                                                .layout_key(
                                                    &self.layout,
                                                    "settings.skill.cancel-delete",
                                                    None,
                                                ),
                                        ),
                                )
                            })
                            .layout_key(&self.layout, format!("settings.skill.{id}"), Some(name))
                    })),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(12.))
                    .child("IMPORT FROM GITHUB")
                    .child(
                        div()
                            .flex()
                            .gap(px(8.))
                            .child(div().flex_1().child(s.import_url.clone()).layout_key(
                                &self.layout,
                                "settings.skill.import-url",
                                None,
                            ))
                            .child(
                                self.settings_action("preview-skills", "Preview")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.import_skills(true, window, cx)
                                    }))
                                    .layout_key(&self.layout, "settings.skill.preview", None),
                            ),
                    )
                    .children(s.preview.iter().map(|skill| {
                        let slug = skill["slug"].as_str().unwrap_or("").to_owned();
                        let checked = s.selected_slugs.contains(&slug);
                        let text = format!(
                            "{} {}  /{}",
                            if checked { "☑" } else { "☐" },
                            skill["name"].as_str().unwrap_or(""),
                            slug
                        );
                        div()
                            .id(SharedString::from(slug.clone()))
                            .cursor_pointer()
                            .child(text.clone())
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if !this.shell.settings_state.selected_slugs.remove(&slug) {
                                    this.shell
                                        .settings_state
                                        .selected_slugs
                                        .insert(slug.clone());
                                }
                                cx.notify();
                            }))
                            .layout_key(
                                &self.layout,
                                format!(
                                    "settings.skill.preview.{}",
                                    skill["slug"].as_str().unwrap_or("")
                                ),
                                Some(text),
                            )
                    }))
                    .when(!s.preview.is_empty(), |d| {
                        d.child(
                            self.settings_action("import-skills", "Import selected")
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.import_skills(false, window, cx)
                                }))
                                .layout_key(&self.layout, "settings.skill.import", None),
                        )
                    }),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(12.))
                    .child(if s.editing.is_some() {
                        "EDIT SKILL"
                    } else {
                        "NEW SKILL"
                    })
                    .child(
                        div()
                            .id("skill-textarea")
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, window, cx| {
                                    window.focus(
                                        &this.shell.settings_state.draft.focus_handle(cx),
                                        cx,
                                    )
                                }),
                            )
                            .min_h(px(220.))
                            .p(px(12.))
                            .border_1()
                            .border_color(color(self.colors.border))
                            .rounded(px(5.))
                            .child(s.draft.clone())
                            .layout_key(&self.layout, "settings.skill.markdown", None),
                    )
                    .child(
                        div()
                            .flex()
                            .gap(px(8.))
                            .child(
                                self.settings_action(
                                    "save-skill",
                                    if s.editing.is_some() {
                                        "Save changes"
                                    } else {
                                        "Create skill"
                                    },
                                )
                                .on_click(cx.listener(|this, _, window, cx| {
                                    let s = &this.shell.settings_state;
                                    let markdown = s.draft.read(cx).content.to_string();
                                    if markdown.trim().is_empty() {
                                        return;
                                    }
                                    let (method, path) =
                                        s.editing.map_or(("POST", "/skills".into()), |id| {
                                            ("PATCH", format!("/skills/{id}"))
                                        });
                                    let mut body = serde_json::json!({"markdown": markdown});
                                    if s.editing.is_none() {
                                        body["scope"] = if s.section == "board-skills" {
                                            "project"
                                        } else {
                                            "user"
                                        }
                                        .into();
                                        if s.section == "board-skills" {
                                            body["project_id"] = s.board.into();
                                        }
                                    }
                                    this.settings_request(method, path, Some(body), window, cx);
                                }))
                                .layout_key(
                                    &self.layout,
                                    "settings.skill.save",
                                    None,
                                ),
                            )
                            .when(s.editing.is_some(), |d| {
                                d.child(
                                    self.settings_action("cancel-skill", "Cancel")
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.shell.settings_state.editing = None;
                                            this.shell
                                                .settings_state
                                                .draft
                                                .update(cx, |input, cx| {
                                                    input.set_content(String::new(), cx)
                                                });
                                            cx.notify();
                                        }))
                                        .layout_key(&self.layout, "settings.skill.cancel", None),
                                )
                            }),
                    ),
            )
            .into_any_element()
    }

    fn import_skills(&mut self, preview: bool, window: &mut Window, cx: &mut Context<Self>) {
        let s = &self.shell.settings_state;
        let url = s.import_url.read(cx).content.trim().to_owned();
        if url.is_empty() || (!preview && s.selected_slugs.is_empty()) {
            return;
        }
        let mut body = serde_json::json!({"url": url, "dry_run": preview, "scope": if s.section == "board-skills" { "project" } else { "user" }});
        if s.section == "board-skills" {
            body["project_id"] = s.board.into();
        }
        if !preview {
            body["slugs"] = serde_json::json!(s.selected_slugs);
        }
        self.settings_request("POST", "/skills/import".into(), Some(body), window, cx);
    }
}

fn skill_markdown(skill: &Value) -> String {
    let mut lines = vec!["---".to_owned(), format!("name: {}", skill["name"])];
    if !skill["description"].is_null() {
        lines.push(format!("description: {}", skill["description"]));
    }
    if !skill["argumentHint"].is_null() {
        lines.push(format!("argument-hint: {}", skill["argumentHint"]));
    }
    lines.push("---".into());
    lines.push(skill["body"].as_str().unwrap_or("").into());
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::shell::tests::{click, draw, measured, setup};
    use gpui::TestAppContext;

    #[track_caller]
    pub(super) fn reply(
        view: gpui::WindowHandle<Desktop>,
        cx: &mut gpui::VisualTestContext,
        result: Value,
    ) -> Request {
        let request = view
            .read_with(cx, |this, _| this.shell.settings_state.pending.clone())
            .unwrap()
            .expect("input must issue a request");
        view.update(cx, |this, window, cx| {
            if request.method == "GET" {
                this.settings_result(request.clone(), Ok(result), window, cx);
            } else {
                struct Server { method: &'static str, path: String, body: Option<Value>, reply: Value }
                impl ht_core::Transport for Server {
                    fn get(&mut self, path: &str) -> Result<ht_core::http::Response, ht_core::http::TransportError> { assert_eq!(path, "/user/context"); Ok(ht_core::http::Response { status:200, body:serde_json::json!({"success":true,"user":{"id":1}}) }) }
                    fn post(&mut self, path: &str, body: &Value, key: &str) -> Result<ht_core::http::Response, ht_core::http::TransportError> { self.mutate("POST", path, body, key) }
                    fn mutate(&mut self, method: &str, path: &str, body: &Value, key: &str) -> Result<ht_core::http::Response, ht_core::http::TransportError> {
                        assert_eq!(method, self.method); assert_eq!(path, self.path); assert_eq!(body, self.body.as_ref().unwrap_or(&Value::Null)); assert!(!key.is_empty());
                        Ok(ht_core::http::Response { status:200, body:self.reply.clone() })
                    }
                }
                let mut server = Server { method:request.method, path:request.path.clone(), body:request.body.clone(), reply:result };
                let drained = this.db.drain_once(&mut server, chrono::Utc::now().timestamp()).unwrap();
                assert!(matches!(drained, ht_core::Drain::Applied(_)), "{drained:?}");
                assert!(!this.poll_settings_write(window, cx));
            }
        }).unwrap();
        request
    }

    #[gpui::test]
    fn profile_key_saves_bearer_patch_and_reopen_reads_saved_name(cx: &mut TestAppContext) {
        let (view, mut cx) = setup(cx);
        draw(view, &mut cx);
        cx.simulate_keystrokes("\\");
        reply(
            view,
            &mut cx,
            serde_json::json!({"user": {"displayName": "Original", "email": "user@example.com"}}),
        );
        click(view, &mut cx, "settings.profile.name");
        cx.simulate_keystrokes("ctrl-a");
        cx.simulate_input("Updated");
        cx.simulate_keystrokes("enter");
        let request = reply(view, &mut cx, serde_json::json!({"success": true}));
        assert_eq!(request.method, "PATCH");
        assert_eq!(request.path, "/user/profile");
        assert_eq!(request.body.unwrap()["displayName"], "Updated");
        reply(
            view,
            &mut cx,
            serde_json::json!({"user": {"displayName": "Updated", "email": "user@example.com"}}),
        );
        cx.simulate_keystrokes("escape \\");
        reply(
            view,
            &mut cx,
            serde_json::json!({"user": {"displayName": "Updated", "email": "user@example.com"}}),
        );
        view.read_with(&cx, |this, cx| {
            assert_eq!(
                this.shell.settings_state.draft.read(cx).content.as_ref(),
                "Updated"
            )
        })
        .unwrap();
    }

    #[gpui::test]
    fn settings_board_picker_instructions_save_and_late_response_isolation(
        cx: &mut TestAppContext,
    ) {
        let (view, mut cx) = setup(cx);
        click(view, &mut cx, "rail.icon.12");
        click(view, &mut cx, "settings.tab.board");
        reply(
            view,
            &mut cx,
            serde_json::json!({"members": [{"id": 1, "displayName": "Alice", "role": "Admin"}]}),
        );
        assert_eq!(
            measured(view, &mut cx, "settings.member.0")["text"],
            "Alice"
        );
        click(view, &mut cx, "settings.nav.board-ai");
        let stale = view
            .read_with(&cx, |this, _| {
                this.shell.settings_state.pending.clone().unwrap()
            })
            .unwrap();
        click(view, &mut cx, "settings.board-picker");
        click(view, &mut cx, "settings.board.1");
        view.read_with(&cx, |this, _| {
            assert_eq!(this.db.kv_get("last_board").unwrap().as_deref(), Some("16"))
        })
        .unwrap();
        reply(
            view,
            &mut cx,
            serde_json::json!({"instruction": {"customInstruction": "Beta guidance"}}),
        );
        view.update(&mut cx, |this, window, cx| {
            this.settings_result(
                stale,
                Ok(serde_json::json!({"instruction": {"customInstruction": "WRONG BOARD"}})),
                window,
                cx,
            )
        })
        .unwrap();
        draw(view, &mut cx);
        let bounds = view.read_with(&cx, |this, cx| {
            this.shell.settings_state.draft.read(cx).input_bounds().unwrap()
        }).unwrap();
        cx.simulate_click(bounds.center(), Default::default());
        cx.simulate_keystrokes("ctrl-a");
        cx.simulate_input("New Beta guidance");
        view.read_with(&cx, |this, cx| {
            assert_eq!(this.shell.settings_state.draft.read(cx).content.as_ref(), "New Beta guidance");
        }).unwrap();
        click(view, &mut cx, "settings.instructions.save");
        let request = reply(view, &mut cx, serde_json::json!({"success": true}));
        assert_eq!(request.method, "PUT");
        assert_eq!(request.path, "/projects/16/instructions");
        assert_eq!(
            request.body.unwrap()["custom_instruction"],
            "New Beta guidance"
        );
        reply(
            view,
            &mut cx,
            serde_json::json!({"instruction": {"customInstruction": "New Beta guidance"}}),
        );
        view.read_with(&cx, |this, cx| {
            assert_eq!(
                this.shell.settings_state.draft.read(cx).content.as_ref(),
                "New Beta guidance"
            )
        })
        .unwrap();
    }

    #[gpui::test]
    fn settings_skills_toggle_edit_save_delete_cancel_and_confirm(cx: &mut TestAppContext) {
        let (view, mut cx) = setup(cx);
        click(view, &mut cx, "rail.icon.12");
        click(view, &mut cx, "settings.tab.team");
        let mut skill = serde_json::json!({"id": 42, "name": "Review", "slug": "review", "description": "Check", "body": "Be thorough", "enabled": true, "userId": 1, "projectId": null});
        reply(
            view,
            &mut cx,
            serde_json::json!({"skills": [skill.clone()]}),
        );
        click(view, &mut cx, "settings.skill.42.enabled");
        let request = reply(view, &mut cx, serde_json::json!({"success": true}));
        assert_eq!(request.method, "PATCH");
        assert_eq!(request.body.unwrap()["enabled"], false);
        skill["enabled"] = false.into();
        reply(
            view,
            &mut cx,
            serde_json::json!({"skills": [skill.clone()]}),
        );
        assert_eq!(
            measured(view, &mut cx, "settings.skill.42.enabled")["text"],
            "Enabled"
        );
        click(view, &mut cx, "settings.skill.42.edit");
        cx.simulate_input("---\nname: Updated Review\n---\nNew review guidance");
        // Scroll the real settings container to the editor's measured bounds before clicking Save.
        view.update(&mut cx, |this, _, _| {
            this.shell.settings_state.scroll.scroll_to_bottom()
        })
        .unwrap();
        click(view, &mut cx, "settings.skill.save");
        let request = reply(view, &mut cx, serde_json::json!({"success": true}));
        assert_eq!(request.path, "/skills/42");
        assert!(
            request.body.unwrap()["markdown"]
                .as_str()
                .unwrap()
                .contains("New review guidance")
        );
        skill["name"] = "Updated Review".into();
        skill["body"] = "New review guidance".into();
        reply(
            view,
            &mut cx,
            serde_json::json!({"skills": [skill.clone()]}),
        );
        assert_eq!(
            measured(view, &mut cx, "settings.skill.42")["text"],
            "Updated Review"
        );
        view.update(&mut cx, |this, _, _| {
            this.shell
                .settings_state
                .scroll
                .set_offset(point(px(0.), px(0.)))
        })
        .unwrap();
        click(view, &mut cx, "settings.skill.42.delete");
        click(view, &mut cx, "settings.skill.cancel-delete");
        assert_eq!(
            measured(view, &mut cx, "settings.skill.42")["text"],
            "Updated Review"
        );
        click(view, &mut cx, "settings.skill.42.delete");
        click(view, &mut cx, "settings.skill.confirm-delete");
        let request = reply(view, &mut cx, serde_json::json!({"success": true}));
        assert_eq!(request.method, "DELETE");
        reply(view, &mut cx, serde_json::json!({"skills": []}));
        view.read_with(&cx, |this, _| {
            assert!(
                this.settings_value()["skills"]
                    .as_array()
                    .unwrap()
                    .is_empty()
            )
        })
        .unwrap();
    }
}

impl Desktop {
    pub(in super::super) fn shortcut_settings(&self, cx: &mut Context<Self>) -> AnyElement {
        let query = self
            .shell
            .settings_state
            .import_url
            .read(cx)
            .content
            .to_lowercase();
        let rows: Vec<Value> = serde_json::from_str(include_str!("shell_shortcuts.json"))
            .expect("web shortcut registry");
        let mut group = String::new();
        let mut elements = vec![];
        for (i, row) in rows.into_iter().enumerate() {
            let heading = row["group"].as_str().unwrap_or("");
            let label = row["label"].as_str().unwrap_or("");
            let keys = row[if cfg!(target_os = "macos") {
                "macKeys"
            } else {
                "keys"
            }]
            .as_array()
            .unwrap()
            .iter()
            .map(|key| key.as_str().unwrap_or("then"))
            .collect::<Vec<_>>()
            .join("  ");
            if !query.trim().is_empty()
                && !format!("{heading} {label} {keys}")
                    .to_lowercase()
                    .contains(query.trim())
            {
                continue;
            }
            if group != heading {
                group = heading.to_owned();
                elements.push(
                    div()
                        .mt(px(24.))
                        .mb(px(12.))
                        .text_color(color(self.colors.muted))
                        .child(group.to_uppercase())
                        .layout_key(&self.layout, format!("settings.shortcut-group.{i}"), Some(group.clone())),
                );
            }
            elements.push(
                div()
                    .p(px(8.))
                    .flex()
                    .justify_between()
                    .child(div().child(label.to_owned()).layout_key(&self.layout, format!("settings.shortcut-label.{i}"), Some(label.into())))
                    .child(div().child(div().child(keys).layout_key(&self.layout, format!("settings.shortcut-key.{i}.0"), None)).layout_key(&self.layout, format!("settings.shortcut-keys.{i}"), None))
                    .layout_key(
                        &self.layout,
                        format!("settings.shortcut.{i}"),
                        Some(label.into()),
                    ),
            );
        }
        let empty = elements.is_empty();
        div()
            .flex()
            .flex_col()
            .gap(px(4.))
            .child(
                div()
                    .h(px(36.))
                    .p(px(8.))
                    .border_1()
                    .border_color(color(self.colors.border))
                    .rounded(px(8.))
                    .child(self.shell.settings_state.import_url.clone())
                    .layout_key(&self.layout, "settings.shortcuts.search", None),
            )
            .when(empty, |d| d.child("No shortcuts found"))
            .children(elements)
            .into_any_element()
    }
}

impl Desktop {
    pub(in super::super) fn profile_blur(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let handle = self.shell.settings_state.draft.focus_handle(cx);
        cx.on_blur(&handle, window, |this, window, cx| {
            if this.shell.settings && this.shell.settings_state.section == "general" {
                this.save_profile(window, cx);
            }
        })
        .detach();
    }
}

impl Desktop {
    pub(in super::super) fn shortcut_search_focus(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.shell
            .settings_state
            .import_url
            .update(cx, |input, cx| {
                input.set_content(String::new(), cx);
                input.configure("Search shortcuts", false, cx);
            });
        window.focus(&self.shell.settings_state.import_url.focus_handle(cx), cx);
    }
}

#[path = "shell_webhooks.rs"]
mod webhooks;

impl Desktop {
    fn learn_settings(&self, cx: &mut Context<Self>) -> AnyElement {
        div().flex().flex_col().gap(px(12.)).child("GUIDES")
            .children([
                ("Tutorial Videos", "Watch short walkthroughs of core Hypertask workflows.", "https://www.youtube.com/playlist?list=PLb7ZrOwcC7l2Du0sq2-fj4uJG7N0vjxmQ"),
                ("MCP Guide", "Connect AI assistants to your workspace with MCP.", "https://help.hypertask.ai/help/how-to-connect-ai-agents-to-hypertask-with-mcp"),
                ("Book Coaching", "Book a one-to-one session for hands-on product help.", "https://calendar.superhuman.com/book/11SzDGi12vkuEu8gf0/icPzJ"),
                ("Help Center", "Find answers to common setup and workflow questions.", "https://help.hypertask.ai/"),
                ("Docs", "Browse technical guides for Hypertask integrations and APIs.", "https://docs.hypertask.ai/"),
                ("Custom fields", "Add board-specific properties like ICE score, channel, or launch date.", "https://docs.hypertask.ai/features/custom-fields/"),
            ].map(|(title, description, url)| div().flex().flex_col().gap(px(4.)).child(self.settings_link(title, url)).child(description)))
            .child("TIPS")
            .child(div().id("quick-tips").p(px(8.)).cursor_pointer().child(if self.footer { "Quick Tips    ✓" } else { "Quick Tips" })
                .on_click(cx.listener(|this, _, _, cx| {
                    this.footer = !this.footer;
                    if let Err(error) = this.db.kv_set("showQuickTips", if this.footer { "true" } else { "false" }) { this.shell.settings_state.error = Some(error.to_string()); }
                    cx.notify();
                })).layout_key(&self.layout, "settings.quick-tips", Some("Quick Tips".into())))
            .into_any_element()
    }
}

#[cfg(test)]
mod more_tests {
    use super::*;
    use crate::ui::shell::tests::{click, draw, measured, setup};
    use gpui::TestAppContext;

    #[gpui::test]
    fn settings_appearance_and_follow_system_change_visible_theme_and_persist(
        cx: &mut TestAppContext,
    ) {
        let (view, mut cx) = setup(cx);
        click(view, &mut cx, "rail.icon.12");
        click(view, &mut cx, "settings.nav.appearance");
        for (i, theme) in [
            (1, ThemeName::Porcelain),
            (2, ThemeName::Graphite),
            (3, ThemeName::Amoled),
            (4, ThemeName::Dia),
        ] {
            click(view, &mut cx, &format!("settings.theme.{i}"));
            let title = measured(view, &mut cx, "settings.title");
            assert_eq!(title["color"], format!("#{:06x}", theme.colors().text));
            view.read_with(&cx, |this, _| assert_eq!(this.options.theme, theme))
                .unwrap();
        }
        click(view, &mut cx, "settings.theme.0");
        view.update(&mut cx, |this, window, _| {
            assert_eq!(this.options.theme, system_theme(window.appearance()));
            assert_eq!(this.db.kv_get("theme").unwrap().as_deref(), Some("system"));
        })
        .unwrap();
        assert!(!measured(view, &mut cx, "settings.theme.0")["bg"].is_null());
    }

    #[gpui::test]
    fn settings_calendar_local_preferences_click_persist_and_reload(cx: &mut TestAppContext) {
        let (view, mut cx) = setup(cx);
        click(view, &mut cx, "rail.icon.12");
        click(view, &mut cx, "settings.nav.calendar");
        assert_eq!(
            measured(view, &mut cx, "settings.title")["text"],
            "Calendar"
        );
        assert!(!measured(view, &mut cx, "settings.calendar.Monday")["bg"].is_null());
        assert_eq!(
            measured(view, &mut cx, "settings.calendar.weekends")["text"],
            "Show weekends"
        );
        click(view, &mut cx, "settings.calendar.Sunday");
        assert!(!measured(view, &mut cx, "settings.calendar.Sunday")["bg"].is_null());
        click(view, &mut cx, "settings.calendar.weekends");
        assert_eq!(
            measured(view, &mut cx, "settings.calendar.weekends")["text"],
            "Show weekends    ✓"
        );
        view.update(&mut cx, |this, _, cx| {
            let value: Value =
                serde_json::from_str(&this.db.kv_get("calendarSettingsAtom").unwrap().unwrap())
                    .unwrap();
            assert_eq!(
                value,
                serde_json::json!({"weekStartsOn": "sunday", "showWeekends": true})
            );
            this.shell.settings_state = Settings::new(&this.db, cx);
            cx.notify();
        })
        .unwrap();
        click(view, &mut cx, "settings.nav.calendar");
        assert!(!measured(view, &mut cx, "settings.calendar.Sunday")["bg"].is_null());
        assert_eq!(
            measured(view, &mut cx, "settings.calendar.weekends")["text"],
            "Show weekends    ✓"
        );
        click(view, &mut cx, "settings.calendar.Monday");
        click(view, &mut cx, "settings.calendar.weekends");
        click(view, &mut cx, "settings.back");
        click(view, &mut cx, "rail.icon.12");
        assert!(!measured(view, &mut cx, "settings.calendar.Monday")["bg"].is_null());
        assert_eq!(
            measured(view, &mut cx, "settings.calendar.weekends")["text"],
            "Show weekends"
        );
        view.read_with(&cx, |this, _| {
            let value: Value =
                serde_json::from_str(&this.db.kv_get("calendarSettingsAtom").unwrap().unwrap())
                    .unwrap();
            assert_eq!(
                value,
                serde_json::json!({"weekStartsOn": "monday", "showWeekends": false})
            );
        })
        .unwrap();
    }

    #[gpui::test]
    fn settings_help_links_and_cli_api_copy_use_web_targets(cx: &mut TestAppContext) {
        let (view, mut cx) = setup(cx);
        click(view, &mut cx, "rail.icon.12");
        click(view, &mut cx, "settings.tab.help");
        for &(label, url) in LINKS {
            click(view, &mut cx, &format!("settings.link.{label}"));
            assert_eq!(cx.opened_url().as_deref(), Some(url));
        }
        click(view, &mut cx, "settings.link.Contact us");
        assert_eq!(cx.opened_url().as_deref(), Some("mailto:help@hypertask.ai"));
        click(view, &mut cx, "settings.tab.connect");
        click(view, &mut cx, "settings.copy.mcp-server");
        assert_eq!(
            cx.read_from_clipboard().unwrap().text().unwrap(),
            "https://mcp.hypertask.ai/mcp"
        );
        click(view, &mut cx, "settings.nav.cli");
        for (id, value) in [("install", INSTALL), ("login", LOGIN)] {
            click(view, &mut cx, &format!("settings.copy.{id}"));
            assert_eq!(cx.read_from_clipboard().unwrap().text().unwrap(), value);
        }
        click(view, &mut cx, "settings.nav.api");
        for (id, value) in [("base-url", API), ("example", EXAMPLE)] {
            click(view, &mut cx, &format!("settings.copy.{id}"));
            assert_eq!(cx.read_from_clipboard().unwrap().text().unwrap(), value);
        }
    }

    #[gpui::test]
    fn settings_learning_links_and_quick_tips_are_reachable(cx: &mut TestAppContext) {
        let (view, mut cx) = setup(cx);
        click(view, &mut cx, "rail.icon.12");
        click(view, &mut cx, "settings.tab.help");
        click(view, &mut cx, "settings.nav.learn");
        click(view, &mut cx, "settings.link.Help Center");
        assert_eq!(
            cx.opened_url().as_deref(),
            Some("https://help.hypertask.ai/")
        );
        view.update(&mut cx, |this, _, _| {
            this.shell.settings_state.scroll.scroll_to_bottom()
        })
        .unwrap();
        click(view, &mut cx, "settings.quick-tips");
        click(view, &mut cx, "settings.back");
        draw(view, &mut cx);
        view.read_with(&cx, |this, _| {
            assert!(!this.footer);
            assert_eq!(
                this.db.kv_get("showQuickTips").unwrap().as_deref(),
                Some("false")
            );
            assert!(
                !this
                    .layout
                    .as_ref()
                    .unwrap()
                    .borrow()
                    .interaction_elements()
                    .iter()
                    .any(|e| e["key"] == "footer")
            );
        })
        .unwrap();
    }
}

impl Desktop {
    fn lifecycle_settings(&self, cx: &mut Context<Self>) -> AnyElement {
        let board = self
            .db
            .projects()
            .unwrap_or_default()
            .into_iter()
            .find(|p| p["id"] == self.shell.settings_state.board);
        let Some(board) = board else {
            return div().into_any_element();
        };
        let title = board["title"].as_str().unwrap_or("");
        let owner = self.settings_value()["user"]["id"]
            .as_i64()
            .is_some_and(|id| board["ownerId"] == id);
        div().flex().flex_col().gap(px(12.)).child("BOARD MANAGEMENT")
            .when(owner, |d| d.child(self.settings_action("archive-board", "Archive board")
                .on_click(cx.listener(|this, _, _, cx| { this.shell.settings_state.lifecycle = Some("Archive"); cx.notify(); }))
                .layout_key(&self.layout, "settings.board.archive", Some("Archive board".into())))
                .child("Hide this board and its tasks until you restore it."))
            .when(self.shell.settings_state.lifecycle == Some("Archive"), |d| d.child(div().flex().flex_col().gap(px(8.))
                .child(format!("Archive \"{title}\"? It moves to Archived and its favorite slot is cleared."))
                .child(self.settings_action("confirm-archive-board", "Archive board").on_click(cx.listener(|this, _, window, cx| {
                    this.settings_request("POST", "/projects/archive".into(), Some(serde_json::json!({"project_id": this.shell.settings_state.board, "status": "Archive"})), window, cx);
                })).layout_key(&self.layout, "settings.board.confirm-archive", None))
                .child(self.settings_action("cancel-archive-board", "Cancel").on_click(cx.listener(|this, _, _, cx| { this.shell.settings_state.lifecycle = None; cx.notify(); }))
                    .layout_key(&self.layout, "settings.board.cancel-archive", None))))
            .into_any_element()
    }
}

impl Desktop {
    pub(in super::super) fn clear_settings_identity(&mut self, cx: &mut Context<Self>) {
        let s = &mut self.shell.settings_state;
        s.data.clear();
        s.pending = None;
        s.error = None;
        s.message = None;
        s.preview.clear();
        s.webhooks.secret = None;
        s.draft
            .update(cx, |input, cx| input.set_content(String::new(), cx));
        s.import_url
            .update(cx, |input, cx| input.set_content(String::new(), cx));
    }
}

#[cfg(test)]
mod mutation_tests {
    use super::tests::reply;
    use super::*;
    use crate::ui::shell::tests::{click, measured, setup};
    use gpui::TestAppContext;

    #[gpui::test]
    fn settings_create_skill_and_import_only_selected_preview(cx: &mut TestAppContext) {
        let (view, mut cx) = setup(cx);
        click(view, &mut cx, "rail.icon.12");
        click(view, &mut cx, "settings.tab.team");
        reply(view, &mut cx, serde_json::json!({"skills": []}));
        click(view, &mut cx, "settings.skill.markdown");
        cx.simulate_input("---\nname: New skill\n---\nReview carefully");
        view.update(&mut cx, |this, _, _| {
            this.shell.settings_state.scroll.scroll_to_bottom()
        })
        .unwrap();
        click(view, &mut cx, "settings.skill.save");
        let request = reply(view, &mut cx, serde_json::json!({"success": true}));
        assert_eq!(request.method, "POST");
        assert_eq!(request.body.as_ref().unwrap()["scope"], "user");
        assert!(
            request.body.unwrap()["markdown"]
                .as_str()
                .unwrap()
                .contains("Review carefully")
        );
        reply(
            view,
            &mut cx,
            serde_json::json!({"skills": [{"id": 43, "userId": 1, "projectId": null, "name": "New skill", "slug": "new-skill", "body": "Review carefully", "enabled": true}]}),
        );
        view.update(&mut cx, |this, _, _| {
            this.shell
                .settings_state
                .scroll
                .set_offset(point(px(0.), px(0.)))
        })
        .unwrap();
        click(view, &mut cx, "settings.skill.import-url");
        cx.simulate_input("https://github.com/example/skills");
        click(view, &mut cx, "settings.skill.preview");
        reply(
            view,
            &mut cx,
            serde_json::json!({"success": true, "skills": [{"slug": "keep", "name": "Keep"}, {"slug": "skip", "name": "Skip"}]}),
        );
        click(view, &mut cx, "settings.skill.preview.skip");
        click(view, &mut cx, "settings.skill.import");
        let request = reply(view, &mut cx, serde_json::json!({"success": true}));
        assert_eq!(request.path, "/skills/import");
        assert_eq!(request.body.unwrap()["slugs"], serde_json::json!(["keep"]));
    }

    #[gpui::test]
    fn settings_webhooks_create_secret_copy_test_and_delete_confirmation(cx: &mut TestAppContext) {
        let (view, mut cx) = setup(cx);
        click(view, &mut cx, "rail.icon.12");
        click(view, &mut cx, "settings.tab.connect");
        click(view, &mut cx, "settings.nav.webhooks");
        reply(view, &mut cx, serde_json::json!({"subscriptions": []}));
        click(view, &mut cx, "settings.webhook.url");
        cx.simulate_input("https://example.com/hypertask");
        view.update(&mut cx, |this, _, _| {
            this.shell.settings_state.scroll.scroll_to_bottom()
        })
        .unwrap();
        click(view, &mut cx, "settings.webhook.event.comment.mention");
        click(view, &mut cx, "settings.webhook.add");
        let request = reply(
            view,
            &mut cx,
            serde_json::json!({"success": true, "secret": "whsec_fixture"}),
        );
        assert_eq!(request.path, "/webhooks");
        let body = request.body.unwrap();
        assert_eq!(body["project_id"], 15);
        assert!(
            !body["events"]
                .as_array()
                .unwrap()
                .contains(&serde_json::json!("comment.mention"))
        );
        let endpoint = serde_json::json!({"id": "wh1", "url": "https://example.com/hypertask", "events": ["task.created"], "secretHint": "...ture"});
        reply(
            view,
            &mut cx,
            serde_json::json!({"subscriptions": [endpoint.clone()]}),
        );
        view.update(&mut cx, |this, _, _| {
            this.shell
                .settings_state
                .scroll
                .set_offset(point(px(0.), px(0.)))
        })
        .unwrap();
        click(view, &mut cx, "settings.copy.webhook-secret");
        assert_eq!(
            cx.read_from_clipboard().unwrap().text().unwrap(),
            "whsec_fixture"
        );
        click(view, &mut cx, "settings.webhook.wh1.test");
        let request = reply(
            view,
            &mut cx,
            serde_json::json!({"success": true, "delivered": true}),
        );
        assert_eq!(request.path, "/webhooks/wh1/test");
        assert_eq!(
            measured(view, &mut cx, "settings.message")["text"],
            "Signed test queued"
        );
        reply(
            view,
            &mut cx,
            serde_json::json!({"subscriptions": [endpoint.clone()]}),
        );
        click(view, &mut cx, "settings.webhook.wh1.delete");
        click(view, &mut cx, "settings.webhook.cancel-delete");
        measured(view, &mut cx, "settings.webhook.wh1");
        click(view, &mut cx, "settings.webhook.wh1.delete");
        click(view, &mut cx, "settings.webhook.confirm-delete");
        let request = reply(view, &mut cx, serde_json::json!({"success": true}));
        assert_eq!(request.method, "DELETE");
        assert_eq!(request.path, "/webhooks?id=wh1");
    }

    #[gpui::test]
    fn settings_archive_board_requires_confirmation_and_updates_cache_after_success(
        cx: &mut TestAppContext,
    ) {
        let (view, mut cx) = setup(cx);
        click(view, &mut cx, "rail.icon.12");
        click(view, &mut cx, "settings.tab.board");
        click(view, &mut cx, "settings.nav.board-general");
        reply(view, &mut cx, serde_json::json!({"user": {"id": 1}}));
        click(view, &mut cx, "settings.board.archive");
        click(view, &mut cx, "settings.board.cancel-archive");
        view.read_with(&cx, |this, _| {
            assert!(this.shell.settings_state.pending.is_none())
        })
        .unwrap();
        click(view, &mut cx, "settings.board.archive");
        click(view, &mut cx, "settings.board.confirm-archive");
        let request = reply(view, &mut cx, serde_json::json!({"success": true, "project":{"id":15,"status":"Archive"}}));
        assert_eq!(request.body.unwrap()["status"], "Archive");
        measured(view, &mut cx, "inbox.tabs");
        view.read_with(&cx, |this, _| {
            assert_eq!(
                this.db
                    .projects()
                    .unwrap()
                    .iter()
                    .find(|p| p["id"] == 15)
                    .unwrap()["status"],
                "Archive"
            )
        })
        .unwrap();
    }
}

#[path = "shell_invite.rs"]
mod invite;

impl Desktop {
    fn mcp_settings(&self, cx: &mut Context<Self>) -> AnyElement {
        let registry: Value =
            serde_json::from_str(include_str!("shell_mcp.json")).expect("web MCP registry");
        let integrations = registry["integrations"].as_array().unwrap();
        let selected = &integrations[self.shell.settings_state.integration];
        let bearer = self.shell.settings_state.bearer_config;
        let config = if bearer {
            self.shell.token.as_ref().map_or_else(|| "Please generate a token first".into(), |token| {
                match selected["id"].as_str().unwrap() {
                    "chatgpt" | "lovable" => token.clone(),
                    "codex" => format!("export HYPERTASK_TOKEN={}\ncodex mcp add hypertask --url https://mcp.hypertask.ai/mcp --bearer-token-env-var HYPERTASK_TOKEN", serde_json::json!(token)),
                    "vscode" => serde_json::to_string_pretty(&serde_json::json!({"servers": {"hypertask-mcp": {"type": "http", "url": "https://mcp.hypertask.ai/mcp", "headers": {"Authorization": format!("Bearer {token}")}}}})).unwrap(),
                    _ => serde_json::to_string_pretty(&serde_json::json!({"mcpServers": {"hypertask-mcp": {"url": "https://mcp.hypertask.ai/mcp", "headers": {"Authorization": format!("Bearer {token}")}}}})).unwrap(),
                }
            })
        } else {
            selected["command"].as_str().unwrap().to_owned()
        };
        div()
            .flex()
            .flex_col()
            .gap(px(24.))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(12.))
                    .child("SERVER")
                    .child("Use this URL from any MCP client.")
                    .child(self.code_row("mcp-server", "https://mcp.hypertask.ai/mcp", cx)),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(12.))
                    .child("CONNECT")
                    .child(div().flex().flex_wrap().gap(px(8.)).children(
                        integrations.iter().enumerate().map(|(i, item)| {
                            let label = item["label"].as_str().unwrap().to_owned();
                            div()
                                .id(("mcp-integration", i))
                                .p(px(8.))
                                .rounded(px(5.))
                                .cursor_pointer()
                                .when(i == self.shell.settings_state.integration, |d| {
                                    d.bg(color(self.colors.active))
                                })
                                .child(label.clone())
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.shell.settings_state.integration = i;
                                    cx.notify();
                                }))
                                .layout_key(
                                    &self.layout,
                                    format!("settings.mcp.integration.{i}"),
                                    Some(label),
                                )
                        }),
                    ))
                    .child(format!(
                        "Connect with {}",
                        selected["label"].as_str().unwrap()
                    ))
                    .child(
                        selected[if bearer { "bearerHint" } else { "hint" }]
                            .as_str()
                            .unwrap()
                            .to_owned(),
                    )
                    .child(self.code_row("mcp-config", config, cx))
                    .child(
                        self.settings_action(
                            "mcp-auth-mode",
                            if bearer {
                                "Use client command"
                            } else {
                                "Use bearer token"
                            },
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.shell.settings_state.bearer_config =
                                !this.shell.settings_state.bearer_config;
                            cx.notify();
                        }))
                        .layout_key(
                            &self.layout,
                            "settings.mcp.auth-mode",
                            None,
                        ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(4.))
                    .child("AVAILABLE TOOLS")
                    .children(registry["tools"].as_array().unwrap().iter().map(|tool| {
                        div()
                            .p(px(8.))
                            .flex()
                            .justify_between()
                            .child(tool["name"].as_str().unwrap().to_owned())
                            .child(tool["description"].as_str().unwrap().to_owned())
                    })),
            )
            .child(self.settings_link("Read the MCP docs", "https://docs.hypertask.ai/mcp"))
            .into_any_element()
    }
}

#[cfg(test)]
mod mcp_tests {
    use super::*;
    use crate::ui::shell::tests::{click, measured, setup};
    #[gpui::test]
    fn settings_mcp_clients_copy_source_commands_and_bearer_unavailable_state(
        cx: &mut gpui::TestAppContext,
    ) {
        let (view, mut cx) = setup(cx);
        click(view, &mut cx, "rail.icon.12");
        click(view, &mut cx, "settings.tab.connect");
        click(view, &mut cx, "settings.mcp.integration.2");
        click(view, &mut cx, "settings.copy.mcp-config");
        assert_eq!(
            cx.read_from_clipboard().unwrap().text().unwrap(),
            "codex mcp add hypertask --url https://mcp.hypertask.ai/mcp"
        );
        click(view, &mut cx, "settings.mcp.auth-mode");
        click(view, &mut cx, "settings.copy.mcp-config");
        assert_eq!(
            cx.read_from_clipboard().unwrap().text().unwrap(),
            "Please generate a token first"
        );
        measured(view, &mut cx, "settings.title");
    }
}

#[cfg(test)]
impl Desktop {
    pub(in crate::ui) fn reveal_settings_target(&mut self, measured: &Value) {
        let y = measured["y"].as_f64().unwrap() as f32;
        let bottom = y + measured["h"].as_f64().unwrap() as f32;
        let delta = if y < 60. {
            y - 60.
        } else if bottom > 900. {
            bottom - 900.
        } else {
            0.
        };
        let mut offset = self.shell.settings_state.scroll.offset();
        offset.y -= px(delta);
        self.shell.settings_state.scroll.set_offset(offset);
    }
}

#[cfg(test)]
mod result_tests {
    use super::*;
    use crate::ui::shell::tests::{click, measured, setup};
    #[gpui::test]
    fn settings_copy_button_shows_copied_then_resets_without_dumping_content(
        cx: &mut gpui::TestAppContext,
    ) {
        let (view, mut cx) = setup(cx);
        click(view, &mut cx, "rail.icon.12");
        click(view, &mut cx, "settings.tab.connect");
        click(view, &mut cx, "settings.copy.mcp-server");
        assert_eq!(
            measured(view, &mut cx, "settings.copy.mcp-server")["text"],
            "Copied"
        );
        cx.executor().advance_clock(Duration::from_secs(2));
        cx.run_until_parked();
        assert_eq!(
            measured(view, &mut cx, "settings.copy.mcp-server")["text"],
            "Copy"
        );
    }
}

#[cfg(test)]
mod palette_theme_tests {
    use super::*;
    use crate::ui::shell::tests::{click, measured, setup};
    #[gpui::test]
    fn palette_theme_updates_settings_selection_and_persistence(cx: &mut gpui::TestAppContext) {
        let (view, mut cx) = setup(cx);
        click(view, &mut cx, "rail.icon.12");
        click(view, &mut cx, "settings.nav.appearance");
        click(view, &mut cx, "settings.theme.0");
        click(view, &mut cx, "settings.back");
        cx.simulate_keystrokes("ctrl-k");
        cx.simulate_input("Graphite");
        cx.simulate_keystrokes("enter");
        cx.simulate_keystrokes("\\");
        assert!(!measured(view, &mut cx, "settings.theme.2")["bg"].is_null());
        assert!(measured(view, &mut cx, "settings.theme.0")["bg"].is_null());
        view.read_with(&cx, |this, _| {
            assert_eq!(
                this.db.kv_get("theme").unwrap().as_deref(),
                Some("graphite")
            )
        })
        .unwrap();
    }
}

#[path = "shell_commands.rs"]
mod commands;

#[cfg(test)]
mod return_tests {
    use super::*;
    use crate::ui::shell::tests::{click, measured, setup};

    #[gpui::test]
    fn settings_close_preserves_search_query_table_and_task_context(cx: &mut gpui::TestAppContext) {
        let (view, mut cx) = setup(cx);
        for destination in [0, 1, 2] {
            view.update(&mut cx, |this, window, cx| {
                match destination {
                    0 => {
                        this.navigate_search(window, cx);
                        this.shell.search_input.update(cx, |input, cx| input.set_content("Alpha".into(), cx));
                    }
                    1 => this.navigate_table(cx),
                    _ => this.open_task(101, cx),
                }
                this.open_settings(window, cx);
                this.select_settings("learn", window, cx);
            }).unwrap();
            click(view, &mut cx, "settings.back");
            view.read_with(&cx, |this, cx| {
                assert!(!this.shell.settings);
                match destination {
                    0 => {
                        assert!(this.shell.destination == navigation::Destination::Search);
                        assert_eq!(this.shell.search_input.read(cx).content.as_ref(), "Alpha");
                    }
                    1 => assert!(this.shell.destination == navigation::Destination::Table),
                    _ => { assert_eq!(this.screen, Screen::Task); assert_eq!(this.options.task, 101); }
                }
            }).unwrap();
        }
    }

    #[gpui::test]
    fn direct_settings_boot_returns_to_remembered_board_or_home(cx: &mut gpui::TestAppContext) {
        cx.update(crate::input::init);
        for remembered in [None, Some("16"), Some("invalid"), Some("999")] {
            let view = cx.open_window(gpui::size(px(1440.), px(900.)), |window, cx| {
                let mut db = Db::memory().unwrap();
                db.apply_response(&serde_json::json!({"success":true,"projects":[{"id":16,"title":"Beta","status":"Normal"}]})).unwrap();
                if let Some(id) = remembered { db.kv_set("last_board", id).unwrap(); }
                Desktop::new(db, Options::parse(["--offline", "--screen", "settings"].map(str::to_owned)).unwrap(), window, cx)
            });
            let mut visual = gpui::VisualTestContext::from_window(view.into(), cx);
            view.read_with(&visual, |this, _| assert!(this.shell.settings)).unwrap();
            crate::ui::shell::tests::draw(view, &mut visual);
            visual.simulate_keystrokes("escape");
            view.read_with(&visual, |this, _| {
                assert!(!this.shell.settings);
                assert_eq!(this.screen, if remembered == Some("16") { Screen::Board } else { Screen::Inbox });
                if remembered == Some("16") { assert_eq!(this.options.board, 16); }
            }).unwrap();
        }
    }
}
