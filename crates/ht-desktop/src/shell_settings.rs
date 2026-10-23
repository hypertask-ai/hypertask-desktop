use super::*;
use gpui::ClipboardItem;

pub(super) struct Settings {
    section: &'static str,
    search: Entity<TextInput>,
    scroll: ScrollHandle,
}
impl Settings {
    pub fn new(cx: &mut Context<Desktop>) -> Self {
        let search = cx.new(|cx| TextInput::new(String::new(), "Search settings", color(0x8e9093), cx));
        cx.observe(&search, |_, _, cx| cx.notify()).detach();
        Self { section: "appearance", search, scroll: ScrollHandle::new() }
    }
}

const SECTIONS: &[(&str, &str, &str)] = &[
    ("profile", "appearance", "Appearance"),
    ("connect", "cli", "CLI"),
    ("connect", "api", "REST API"),
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
        self.shell.settings_state.search.update(cx, |input, cx| input.set_content(String::new(), cx));
        window.focus(&self.focus, cx);
        cx.notify();
    }

    fn close_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.shell.settings = false;
        window.focus(&self.focus, cx);
        cx.notify();
    }

    pub(in super::super) fn settings_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if event.keystroke.key == "escape" {
            self.close_settings(window, cx);
            cx.stop_propagation();
        }
        true
    }

    pub(in super::super) fn settings_screen(&self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let t = self.colors;
        let section = self.shell.settings_state.section;
        let tab = SECTIONS.iter().find(|(_, id, _)| *id == section).map_or("help", |s| s.0);
        let query = self.shell.settings_state.search.read(cx).content.to_lowercase();
        let title = SECTIONS.iter().find(|(_, id, _)| *id == section).map_or("Help", |s| s.2);
        div().size_full().flex().bg(color(t.container)).text_color(color(t.text))
            .font_family(self.options.theme.font()).text_size(px(14.)).track_focus(&self.focus)
            .capture_key_down(cx.listener(Self::key_down))
            .child(div().w(px(250.)).h_full().flex_shrink_0().border_r_1().border_color(color(t.border)).flex().flex_col()
                .child(div().p(px(16.)).border_b_1().border_color(color(t.border))
                    .child(div().id("settings-back").p(px(8.)).mb(px(16.)).cursor_pointer().child("←  Back to app")
                        .on_click(cx.listener(|this, _, window, cx| this.close_settings(window, cx)))
                        .layout_key(&self.layout, "settings.back", Some("Back to app".into())))
                    .child(div().h(px(36.)).px(px(12.)).child(self.shell.settings_state.search.clone())
                        .layout_key(&self.layout, "settings.search", None)))
                .child(div().flex_1().p(px(12.)).children(SECTIONS.iter().filter(|(scope, _, label)| {
                    (query.is_empty() && *scope == tab) || (!query.is_empty() && label.to_lowercase().contains(&query))
                }).map(|&(_, id, label)| div().id(id).px(px(8.)).py(px(8.)).rounded(px(5.)).cursor_pointer()
                    .when(id == section, |d| d.bg(color(t.active)))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.shell.settings_state.section = id;
                        window.focus(&this.focus, cx);
                        cx.notify();
                    })).child(label).layout_key(&self.layout, format!("settings.nav.{id}"), Some(label.into()))))
                    .when(tab == "help" || !query.is_empty(), |d| d.children(LINKS.iter().filter(|(label, _)| label.to_lowercase().contains(&query)).map(|&(label, url)| self.settings_link(label, url)))))
                .child(div().p(px(12.)).child(self.settings_link("Contact us", "mailto:help@hypertask.ai"))))
            .child(div().flex_1().min_w_0().h_full().flex().flex_col()
                .child(div().h(px(60.)).w_full().flex().justify_center()
                    .child(div().w_full().max_w(px(760.)).px(px(24.)).py(px(12.)).flex().gap(px(4.))
                        .children([("profile", "Profile", "appearance"), ("connect", "Connect", "cli"), ("help", "Help", "help")].map(|(id, label, default)|
                            div().id(id).px(px(12.)).py(px(6.)).rounded(px(5.)).cursor_pointer()
                                .when(tab == id, |d| d.bg(color(t.active)))
                                .on_click(cx.listener(move |this, _, _, cx| { this.shell.settings_state.section = default; cx.notify(); }))
                                .child(label).layout_key(&self.layout, format!("settings.tab.{id}"), Some(label.into()))))))
                .child(div().id("settings-content-scroll").flex_1().overflow_y_scroll().track_scroll(&self.shell.settings_state.scroll).flex().justify_center()
                    .child(div().w_full().max_w(px(760.)).px(px(24.)).pt(px(20.)).pb(px(32.)).flex().flex_col().gap(px(32.))
                        .child(div().text_size(px(24.)).font_weight(FontWeight::SEMIBOLD).child(title)
                            .layout_key(&self.layout, "settings.title", Some(title.into())))
                        .child(self.settings_content(cx)))))
            .layout_key(&self.layout, "settings", None)
    }

    fn settings_link(&self, label: &'static str, url: &'static str) -> AnyElement {
        div().id(label).px(px(8.)).py(px(8.)).cursor_pointer().child(label)
            .on_click(move |_, _, _| { let _ = open::that_detached(url); })
            .layout_key(&self.layout, format!("settings.link.{label}"), Some(label.into()))
    }

    fn code_row(&self, id: &'static str, value: &'static str, cx: &mut Context<Self>) -> AnyElement {
        div().id(id).p(px(8.)).rounded(px(5.)).bg(color(self.colors.card)).cursor_pointer()
            .on_click(cx.listener(move |_, _, _, cx| cx.write_to_clipboard(ClipboardItem::new_string(value))))
            .child(value).layout_key(&self.layout, format!("settings.copy.{id}"), Some(value.into()))
    }

    fn settings_content(&self, cx: &mut Context<Self>) -> AnyElement {
        let t = self.colors;
        match self.shell.settings_state.section {
            "appearance" => div().flex().flex_col().gap(px(4.))
                .child(div().text_color(color(t.muted)).child("APPEARANCE"))
                .children([
                    (ThemeName::Porcelain, "Light · Porcelain"),
                    (ThemeName::Graphite, "Dark · Graphite"),
                    (ThemeName::Amoled, "OLED black · AMOLED"),
                    (ThemeName::Dia, "Paper · Dia"),
                ].into_iter().enumerate().map(|(i, (theme, label))| div().id(("theme", i)).p(px(8.)).rounded(px(5.)).cursor_pointer()
                    .when(self.options.theme == theme, |d| d.bg(color(t.active)))
                    .child(if self.options.theme == theme { format!("{label}    ✓") } else { label.into() })
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.options.theme = theme;
                        this.colors = theme.colors();
                        let _ = this.db.kv_set("theme", &format!("{theme:?}").to_lowercase());
                        this.reload();
                        cx.notify();
                    })).layout_key(&self.layout, format!("settings.theme.{i}"), Some(label.into())))).into_any_element(),
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
