use super::*;
use gpui::ClipboardItem;
use std::{collections::BTreeMap, sync::mpsc};

#[derive(Clone, Debug)]
struct Request {
    serial: u64,
    method: &'static str,
    path: String,
    body: Option<Value>,
    section: &'static str,
    board: i64,
}

pub(super) struct Settings {
    webhooks: webhooks::Webhooks,
    section: &'static str,
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
    preview: Vec<Value>,
    selected_slugs: std::collections::BTreeSet<String>,
    #[cfg(test)]
    requests: Vec<Request>,
}
impl Settings {
    pub fn new(cx: &mut Context<Desktop>) -> Self {
        let search =
            cx.new(|cx| TextInput::new(String::new(), "Search settings", color(0x8e9093), cx));
        cx.observe(&search, |_, _, cx| cx.notify()).detach();
        let draft = cx.new(|cx| {
            TextInput::new(String::new(), "Display name", color(0x8e9093), cx).multiline()
        });
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
    ("team", "skills", "Skills"),
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

    fn close_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.shell.settings = false;
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
                            .overflow_y_scroll()
                            .track_scroll(&self.shell.settings_state.scroll)
                            .flex()
                            .justify_center()
                            .child(
                                div()
                                    .w_full()
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
        value: &'static str,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        div()
            .id(id)
            .p(px(8.))
            .rounded(px(5.))
            .bg(color(self.colors.card))
            .cursor_pointer()
            .on_click(cx.listener(move |_, _, _, cx| {
                cx.write_to_clipboard(ClipboardItem::new_string(value.to_owned()))
            }))
            .child(value)
            .layout_key(
                &self.layout,
                format!("settings.copy.{id}"),
                Some(value.into()),
            )
    }

    fn settings_content(&self, cx: &mut Context<Self>) -> AnyElement {
        let t = self.colors;
        match self.shell.settings_state.section {
            "shortcuts" => self.shortcut_settings(cx),
            "webhooks" => self.webhook_settings(cx),
            "learn" => self.learn_settings(cx),
            "mcp" => div().flex().flex_col().gap(px(12.)).child("SERVER").child("Use this URL from any MCP client.").child(self.code_row("mcp-server", "https://mcp.hypertask.ai/mcp", cx)).child(self.settings_link("Read the MCP docs", "https://docs.hypertask.ai/mcp")).into_any_element(),
            "general" => self.profile_settings(cx),
            "board-ai" => self.instruction_settings(cx),
            "board-members" => self.member_settings(),
            "board-files" => self.file_settings(),
            "skills" | "board-skills" => self.skill_settings(cx),
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
        self.load_settings(window, cx);
        cx.notify();
    }

    fn settings_path(&self) -> Option<String> {
        let s = &self.shell.settings_state;
        match s.section {
            "general" => Some("/user/context".into()),
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
        let request = Request {
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
        self.shell.settings_state.pending = Some(request.clone());
        self.shell.settings_state.error = None;
        self.shell.settings_state.message = None;
        #[cfg(test)]
        {
            self.shell.settings_state.requests.push(request);
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
                    .and_then(|http| http.json_request(job.method, &job.path, job.body.as_ref()))
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
                if request.section == "general" {
                    s.draft.update(cx, |input, cx| {
                        input.set_content(
                            value["user"]["displayName"]
                                .as_str()
                                .unwrap_or("")
                                .to_owned(),
                            cx,
                        )
                    });
                } else if request.section == "board-ai" {
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
                if request.section == "webhooks" {
                    if let Some(secret) = value["secret"].as_str() {
                        s.webhooks.secret = Some(secret.to_owned());
                    }
                }
                s.editing = None;
                s.delete = None;
                s.preview.clear();
                s.draft
                    .update(cx, |input, cx| input.set_content(String::new(), cx));
                self.load_settings(window, cx);
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
            .unwrap_or_default();
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
                let role = member["role"].as_str().unwrap_or("Member");
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
        let boards = self.db.projects().unwrap_or_default();
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
                                            None,
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

    fn reply(
        view: gpui::WindowHandle<Desktop>,
        cx: &mut gpui::VisualTestContext,
        result: Value,
    ) -> Request {
        view.update(cx, |this, window, cx| {
            let request = this
                .shell
                .settings_state
                .pending
                .clone()
                .expect("input must issue a request");
            this.settings_result(request.clone(), Ok(result), window, cx);
            request
        })
        .unwrap()
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
            serde_json::json!({"members": [{"displayName": "Alice", "role": "Admin"}]}),
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
        click(view, &mut cx, "settings.instructions");
        cx.simulate_keystrokes("ctrl-a");
        cx.simulate_input("New Beta guidance");
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
        let skill = serde_json::json!({"id": 42, "name": "Review", "slug": "review", "description": "Check", "body": "Be thorough", "enabled": true, "userId": 1, "projectId": null});
        reply(
            view,
            &mut cx,
            serde_json::json!({"skills": [skill.clone()]}),
        );
        click(view, &mut cx, "settings.skill.42.enabled");
        let request = reply(view, &mut cx, serde_json::json!({"success": true}));
        assert_eq!(request.method, "PATCH");
        assert_eq!(request.body.unwrap()["enabled"], false);
        reply(
            view,
            &mut cx,
            serde_json::json!({"skills": [skill.clone()]}),
        );
        click(view, &mut cx, "settings.skill.42.edit");
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
                .contains("Be thorough")
        );
        reply(
            view,
            &mut cx,
            serde_json::json!({"skills": [skill.clone()]}),
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
            "Review"
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
                        .into_any_element(),
                );
            }
            elements.push(
                div()
                    .p(px(8.))
                    .flex()
                    .justify_between()
                    .child(label.to_owned())
                    .child(keys)
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
            .update(cx, |input, cx| input.set_content(String::new(), cx));
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
