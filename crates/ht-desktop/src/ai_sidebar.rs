use crate::layout::LayoutKey;
use crate::{
    input::TextInput,
    rich_text::RichText,
    theme::{Theme, color},
    ui::Screen,
};
use gpui::{
    App, Context, Entity, FocusHandle, Focusable, FontWeight, KeyDownEvent, ScrollHandle, Window,
    div, prelude::*, px, svg,
};
use ht_core::{
    Db,
    ai_chat::{self, Client, DefaultContext, Event, MODELS, Message, Session, TurnIds},
};
use std::{
    collections::VecDeque,
    sync::{
        Arc,
        mpsc::{self, Receiver},
    },
    thread,
    time::Duration,
};

#[path = "ai_memory.rs"]
mod memory;

#[path = "ai_skills.rs"]
mod skills;

#[path = "ai_presentation.rs"]
mod presentation;
pub use presentation::DisplayMode;
use presentation::Presentation;

#[derive(Clone, PartialEq)]
pub struct Scope {
    context: DefaultContext,
    team_id: Option<String>,
    label: String,
    suggestions: Vec<String>,
}

impl Scope {
    pub fn load(db: &Db, screen: Screen, project: i64, task_id: Option<i64>) -> Self {
        let task = task_id.and_then(|id| db.task(id).ok().flatten());
        let project = task.as_ref().map_or(project, |t| t.project_id);
        let (board_name, team_id) = ai_chat::project_metadata(db, project);
        let columns = if screen == Screen::Board {
            db.board_cards(project).unwrap_or_default()
        } else {
            vec![]
        };
        let surface = match screen {
            Screen::Board => "board",
            Screen::Task => "task_detail",
            Screen::Inbox => "inbox",
        };
        let path = match screen {
            Screen::Board => format!("/board/{project}"),
            Screen::Task => task
                .as_ref()
                .map(|t| {
                    format!(
                        "/detail/project-{project}/{}",
                        t.extra
                            .get("uniqueIndex")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(t.id)
                    )
                })
                .unwrap_or_default(),
            Screen::Inbox => "/inbox".into(),
        };
        let label = task
            .as_ref()
            .map(|t| {
                format!(
                    "{} {}",
                    t.ticket_number.as_deref().unwrap_or("Current task"),
                    t.title
                )
            })
            .unwrap_or_else(|| board_name.clone());
        let suggestions = ai_chat::suggestions(
            task.as_ref().filter(|_| screen == Screen::Task),
            (screen == Screen::Board).then_some((board_name.as_str(), columns.as_slice())),
            chrono::Utc::now().timestamp(),
        );
        Self {
            context: DefaultContext {
                project_id: (screen != Screen::Inbox && project > 0).then_some(project),
                task_id: task.as_ref().map(|t| t.id),
                surface: surface.into(),
                surface_path: path,
                view_name: (screen != Screen::Inbox).then_some(board_name),
            },
            team_id,
            label,
            suggestions,
        }
    }
}

struct ChatMessage {
    message: Message,
    rich: RichText,
}
struct Chat {
    id: String,
    title: String,
    messages: Vec<ChatMessage>,
    status: String,
    error: Option<String>,
    scroll: ScrollHandle,
}

impl Chat {
    fn new() -> Self {
        Self {
            id: ai_chat::session_id(),
            title: "New AI Chat".into(),
            messages: vec![],
            status: String::new(),
            error: None,
            scroll: ScrollHandle::new(),
        }
    }
}

struct QueuedPrompt {
    text: String,
    prompt: String,
}

enum Update {
    Event(Event),
    Failed(String),
}
struct ActiveTurn {
    ids: TurnIds,
    chat: usize,
    assistant: usize,
    updates: Receiver<Update>,
    cancel_reply: Option<Receiver<Result<(), String>>>,
    stopping: bool,
    client: Client,
}

pub struct AiSidebar {
    colors: Theme,
    skills: skills::Skills,
    presentation: Presentation,
    pub capture: crate::layout::Capture,
    input: Entity<TextInput>,
    draft_quotes: Vec<(String, String)>,
    input_scroll: ScrollHandle,
    _input_subscription: gpui::Subscription,
    focus: FocusHandle,
    previous_focus: Option<FocusHandle>,
    client: Option<Client>,
    scope: Option<Scope>,
    include_context: bool,
    chats: Vec<Chat>,
    selected: usize,
    model: usize,
    model_menu: bool,
    history_menu: bool,
    history: Vec<Arc<Session>>,
    history_pending: Option<Receiver<Result<Vec<Arc<Session>>, String>>>,
    history_error: Option<String>,
    history_limit: usize,
    chat_pending: Option<Receiver<(String, String, Vec<ChatMessage>)>>,
    more_menu: bool,
    collapsed: bool,
    scroll: ScrollHandle,
    active: Option<ActiveTurn>,
    queued: VecDeque<QueuedPrompt>,
}

impl AiSidebar {
    pub fn expand(&mut self, cx: &mut Context<Self>) {
        self.set_open(true, cx);
    }

    pub fn set_theme(&mut self, colors: Theme, cx: &mut Context<Self>) {
        self.colors = colors;
        for chat in &mut self.chats {
            for message in &mut chat.messages {
                message.rich = markdown(&message.message.content, colors);
            }
        }
        cx.notify();
    }

    pub fn new(
        colors: Theme,
        token: Option<String>,
        workspace_focus: FocusHandle,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let input = cx.new(|cx| {
            TextInput::new(
                String::new(),
                "Focus with CTRL + Q, @ for context",
                color(colors.muted),
                cx,
            )
            .ai_composer()
        });
        let input_scroll = ScrollHandle::new();
        let scroll = input_scroll.clone();
        let mut draft = String::new();
        let input_subscription = cx.observe(&input, move |_, input, cx| {
            let content = input.read(cx).content.to_string();
            if content != draft {
                draft = content;
                scroll.scroll_to_bottom();
                cx.notify();
            }
        });
        cx.spawn_in(window, async move |view, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(33))
                    .await;
                if view.update(cx, |this, cx| this.poll(cx)).is_err() {
                    break;
                }
            }
        })
        .detach();
        let chat = Chat::new();
        let scroll = chat.scroll.clone();
        let mut sidebar = Self {
            colors,
            presentation: Presentation::default(),
            capture: None,
            input,
            draft_quotes: vec![],
            input_scroll,
            _input_subscription: input_subscription,
            focus: cx.focus_handle(),
            previous_focus: Some(workspace_focus),
            skills: skills::Skills::new(token.clone()),
            client: token.and_then(|t| Client::new(t).ok()),
            scope: None,
            include_context: true,
            chats: vec![chat],
            selected: 0,
            model: 0,
            model_menu: false,
            history_menu: false,
            history: vec![],
            history_pending: None,
            history_error: None,
            history_limit: 30,
            chat_pending: None,
            more_menu: false,
            collapsed: false,
            scroll,
            active: None,
            queued: VecDeque::new(),
        };
        sidebar.load_memory_history(cx);
        sidebar
    }

    pub fn set_token(&mut self, token: Option<String>, cx: &mut Context<Self>) {
        self.stop(cx);
        self.active = None;
        self.chats = vec![Chat::new()];
        self.scroll = self.chats[0].scroll.clone();
        self.queued.clear();
        self.selected = 0;
        self.history.clear();
        self.draft_quotes.clear();
        self.history_pending = None;
        self.history_error = None;
        self.chat_pending = None;
        self.history_menu = false;
        self.input
            .update(cx, |input, cx| input.set_content(String::new(), cx));
        self.skills = skills::Skills::new(token.clone());
        self.client = token.and_then(|t| Client::new(t).ok());
        cx.notify();
    }

    pub fn set_scope(&mut self, scope: Scope, cx: &mut Context<Self>) {
        if self.scope.as_ref() != Some(&scope) {
            let changed_context = self.scope.as_ref().is_none_or(|old| {
                old.context.project_id != scope.context.project_id
                    || old.context.task_id != scope.context.task_id
                    || old.context.surface != scope.context.surface
            });
            self.scope = Some(scope);
            if changed_context {
                self.include_context = true;
            }
            cx.notify();
        }
    }

    pub fn focus_composer(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let focus = self.input.focus_handle(cx);
        if focus.is_focused(window) && self.mode() == DisplayMode::Sidebar && !self.collapsed {
            if let Some(previous) = &self.previous_focus {
                window.focus(previous, cx);
            }
        } else {
            if !self.focus.contains_focused(window, cx) {
                if let Some(previous) = window.focused(cx) {
                    self.previous_focus = Some(previous);
                }
            }
            self.set_open(true, cx);
            window.focus(&focus, cx);
        }
        cx.notify();
    }

    #[cfg(test)]
    pub(crate) fn draft_text(&self, cx: &App) -> String {
        self.input.read(cx).content.to_string()
    }

    pub fn has_focus(&self, window: &Window, cx: &App) -> bool {
        self.focus.contains_focused(window, cx)
    }

    pub fn width(&self) -> f32 {
        if self.collapsed && self.mode() == DisplayMode::Sidebar {
            28.
        } else {
            self.presentation.width
        }
    }

    fn toggle_history(&mut self, cx: &mut Context<Self>) {
        self.history_menu = !self.history_menu;
        self.model_menu = false;
        self.more_menu = false;
        if self.history_menu && self.history_pending.is_none() {
            self.history_error = None;
            self.history_limit = 30;
            if let Some(client) = self.client.clone() {
                let (send, receive) = mpsc::channel();
                match thread::Builder::new()
                    .name("ht-ai-history".into())
                    .spawn(move || {
                        let result = client
                            .sessions()
                            .map(|sessions| sessions.into_iter().map(Arc::new).collect())
                            .map_err(|e| e.to_string());
                        let _ = send.send(result);
                    }) {
                    Ok(_) => self.history_pending = Some(receive),
                    Err(error) => self.history_error = Some(error.to_string()),
                }
            } else {
                self.history_error = Some("Sign in to load server history".into());
            }
        }
        cx.notify();
    }

    fn open_history_chat(&mut self, index: usize, cx: &mut Context<Self>) {
        let session = Arc::clone(&self.history[index]);
        let colors = self.colors;
        let (send, receive) = mpsc::channel();
        match thread::Builder::new()
            .name("ht-ai-transcript".into())
            .spawn(move || {
                let id = session.id.clone();
                let title = session.title.clone().unwrap_or_else(|| "AI Chat".into());
                let messages = session
                    .messages
                    .iter()
                    .cloned()
                    .map(|message| {
                        let rich = if message.role == "human" {
                            RichText::default()
                        } else {
                            markdown(&message.content, colors)
                        };
                        ChatMessage { message, rich }
                    })
                    .collect();
                let _ = send.send((id, title, messages));
            }) {
            Ok(_) => self.chat_pending = Some(receive),
            Err(error) => self.history_error = Some(error.to_string()),
        }
        cx.notify();
    }

    fn poll_history(&mut self, cx: &mut Context<Self>) {
        if let Some(pending) = &self.history_pending {
            match pending.try_recv() {
                Ok(result) => {
                    self.history_pending = None;
                    match result {
                        Ok(sessions) => self.history = sessions,
                        Err(error) => self.history_error = Some(error),
                    }
                    cx.notify();
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.history_pending = None;
                    self.history_error = Some("AI history load interrupted".into());
                    cx.notify();
                }
                Err(mpsc::TryRecvError::Empty) => (),
            }
        }
        if let Some(pending) = &self.chat_pending {
            match pending.try_recv() {
                Ok((id, title, messages)) => {
                    self.chat_pending = None;
                    self.chats.push(Chat { id, title, messages, status: String::new(), error: None, scroll: ScrollHandle::new() });
                    self.selected = self.chats.len() - 1;
                    self.scroll = self.chats[self.selected].scroll.clone();
                    self.queued.clear();
                    self.history_menu = false;
                    cx.notify();
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.chat_pending = None;
                    self.history_error = Some("AI transcript load interrupted".into());
                    cx.notify();
                }
                Err(mpsc::TryRecvError::Empty) => (),
            }
        }
    }

    pub(crate) fn new_chat(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.chat_pending = None;
        self.draft_quotes.clear();
        self.chats.push(Chat::new());
        self.selected = self.chats.len() - 1;
        self.scroll = self.chats[self.selected].scroll.clone();
        self.queued.clear();
        self.include_context = true;
        self.history_menu = false;
        self.more_menu = false;
        self.input.update(cx, |input, cx| input.reset(window, cx));
        window.focus(&self.input.focus_handle(cx), cx);
        cx.notify();
    }

    pub fn insert_comment_quote(
        &mut self,
        html: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        use gpui::EntityInputHandler;
        let plain = RichText::parse(&html, self.colors).text;
        self.input.update(cx, |input, cx| {
            input.replace_text_in_range(None, &plain, window, cx)
        });
        self.draft_quotes.push((plain, html));
        self.set_open(true, cx);
        window.focus(&self.input.focus_handle(cx), cx);
    }

    fn prompt_body(&self, text: &str) -> String {
        let mut message = text.to_owned();
        if !self.draft_quotes.is_empty() {
            message = quote_text_html(text);
            for (plain, html) in &self.draft_quotes {
                let escaped = quote_text_html(plain);
                let escaped = &escaped[3..escaped.len() - 4];
                message = message.replacen(escaped, html, 1);
            }
        }
        message
    }

    fn send(&mut self, text: String, window: &mut Window, cx: &mut Context<Self>) {
        let message = self.prompt_body(if self.draft_quotes.is_empty() {
            text.trim()
        } else {
            &text
        });
        let text = text.trim().to_owned();
        if text.is_empty() { return; }
        if self.active.is_some() {
            self.queued.push_back(QueuedPrompt { text, prompt: message });
            self.draft_quotes.clear();
            self.input.update(cx, |input, cx| input.reset(window, cx));
            cx.notify();
            return;
        }
        if self.start_turn(text, message, cx) {
            self.draft_quotes.clear();
            self.input.update(cx, |input, cx| input.reset(window, cx));
        }
    }

    fn start_turn(&mut self, text: String, message: String, cx: &mut Context<Self>) -> bool {
        let chat = &mut self.chats[self.selected];
        chat.error = None;
        let Some(client) = self.client.clone() else {
            chat.error =
                Some("AI chat is unavailable offline. Sign in and reconnect to send.".into());
            cx.notify();
            return false;
        };
        let ids = TurnIds::new(chat.id.clone());
        let mut context = self
            .scope
            .as_ref()
            .map(|s| s.context.clone())
            .unwrap_or_default();
        if !self.include_context {
            context.project_id = None;
            context.task_id = None;
            context.view_name = None;
        }
        let history: Vec<Message> = chat.messages.iter().map(|m| m.message.clone()).collect();
        let body = ai_chat::request(
            &message,
            &ids,
            &ai_chat::session_id(),
            MODELS[self.model],
            &context,
            self.scope.as_ref().and_then(|s| s.team_id.as_deref()),
            &history,
        );
        let (send, updates) = mpsc::sync_channel(64);
        let worker_client = client.clone();
        if let Err(error) = thread::Builder::new()
            .name("ht-ai-stream".into())
            .spawn(move || {
                if let Err(error) =
                    worker_client.stream(&body, |event| send.send(Update::Event(event)).is_ok())
                {
                    let _ = send.send(Update::Failed(error.to_string()));
                }
            })
        {
            chat.error = Some(error.to_string());
            cx.notify();
            return false;
        }
        chat.messages.push(ChatMessage {
            message: Message {
                role: "human".into(),
                content: text,
            },
            rich: RichText::default(),
        });
        let assistant = chat.messages.len();
        chat.messages.push(ChatMessage {
            message: Message {
                role: "assistant".into(),
                content: String::new(),
            },
            rich: RichText::default(),
        });
        chat.status = "Working…".into();
        self.scroll.scroll_to_bottom();
        self.active = Some(ActiveTurn {
            ids,
            chat: self.selected,
            assistant,
            updates,
            cancel_reply: None,
            stopping: false,
            client,
        });
        cx.notify();
        true
    }

    fn stop(&mut self, cx: &mut Context<Self>) {
        let Some(active) = &mut self.active else {
            return;
        };
        if active.stopping {
            return;
        }
        let (send, receive) = mpsc::channel();
        let client = active.client.clone();
        let ids = active.ids.clone();
        match thread::Builder::new()
            .name("ht-ai-cancel".into())
            .spawn(move || {
                let _ = send.send(client.cancel(&ids).map_err(|e| e.to_string()));
            }) {
            Ok(_) => {
                active.stopping = true;
                active.cancel_reply = Some(receive);
                self.chats[active.chat].status = "Stopping…".into();
            }
            Err(error) => self.chats[active.chat].error = Some(error.to_string()),
        }
        cx.notify();
    }

    fn away_from_latest(&self) -> bool {
        f32::from(self.scroll.max_offset().y + self.scroll.offset().y) > 4.
    }

    fn poll(&mut self, cx: &mut Context<Self>) {
        self.poll_skills(cx);
        self.poll_history(cx);
        let follow_latest = f32::from(self.scroll.max_offset().y + self.scroll.offset().y) <= 48.;
        let Some(active) = &mut self.active else {
            return;
        };
        let chat = &mut self.chats[active.chat];
        let mut changed = false;
        let mut terminal = false;
        let mut content_changed = false;
        if let Some(reply) = &active.cancel_reply {
            if let Ok(result) = reply.try_recv() {
                active.cancel_reply = None;
                if let Err(error) = result {
                    active.stopping = false;
                    chat.error = Some(error);
                    chat.status = "Reply still running — Stop was refused".into();
                }
                changed = true;
            }
        }
        for _ in 0..64 {
            let update = match active.updates.try_recv() {
                Ok(update) => update,
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    if chat.error.is_none() {
                        chat.error = Some("AI reply interrupted before done; partial text is preserved".into());
                    }
                    terminal = true;
                    changed = true;
                    break;
                }
            };
            changed = true;
            match update {
                Update::Event(Event::Content(delta)) => {
                    chat.messages[active.assistant]
                        .message
                        .content
                        .push_str(&delta);
                    content_changed = true;
                }
                Update::Event(Event::Status(status)) => {
                    if !active.stopping {
                        chat.status = status;
                    }
                }
                Update::Event(Event::Title(title)) => chat.title = title,
                Update::Event(Event::Error(error)) => {
                    chat.error = Some(stream_error(&error));
                    chat.status.clear();
                }
                Update::Event(Event::Done(done)) => {
                    if done.status == "error" && !done.cancelled && chat.error.is_none() {
                        chat.error = Some(stream_error(""));
                    }
                    if done.cancelled && chat.messages[active.assistant].message.content.is_empty() {
                        chat.messages[active.assistant].message.content = "Stream cancelled.".into();
                        content_changed = true;
                    }
                    // Stream owns persistence. Never call add-message for a persisted turn.
                    chat.status = if done.cancelled {
                        "Stopped".into()
                    } else {
                        String::new()
                    };
                    terminal = true;
                    break;
                }
                Update::Failed(error) => {
                    chat.error = Some(error);
                    terminal = true;
                    break;
                }
            }
        }
        if content_changed {
            chat.messages[active.assistant].rich = markdown(
                &chat.messages[active.assistant].message.content,
                self.colors,
            );
            if active.chat == self.selected && follow_latest {
                self.scroll.scroll_to_bottom();
            }
        }
        if terminal {
            if chat.error.is_some() {
                chat.status.clear();
            }
            self.active = None;
            if let Some(next) = self.queued.pop_front() {
                if !self.start_turn(next.text.clone(), next.prompt.clone(), cx) {
                    self.queued.push_front(next);
                }
            }
        }
        if changed {
            cx.notify();
        }
    }

    fn key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let key = event.keystroke.key.as_str();
        let modifiers = event.keystroke.modifiers;
        if modifiers.control && !modifiers.platform && !modifiers.shift && !modifiers.alt
            && key == "q" && !event.is_held
        {
            self.focus_composer(window, cx);
        } else if (modifiers.control || modifiers.platform) && modifiers.shift && !modifiers.alt
            && matches!(key, "o" | "j")
        {
            self.new_chat(window, cx);
        } else if (modifiers.control || modifiers.platform) && !modifiers.alt
            && ((modifiers.shift && matches!(key, "/" | "?")) || key == "?")
        {
            self.set_open(false, cx);
            if let Some(previous) = &self.previous_focus {
                window.focus(previous, cx);
            }
        } else if key == "escape" {
            if self.model_menu || self.history_menu || self.more_menu {
                self.model_menu = false;
                self.history_menu = false;
                self.more_menu = false;
                cx.notify();
                cx.stop_propagation();
                return;
            }
            if self.active.is_some() {
                self.stop(cx);
            }
            self.model_menu = false;
            self.history_menu = false;
            self.more_menu = false;
            cx.notify();
        } else if self.input.focus_handle(cx).is_focused(window) && key == "enter" {
            if modifiers.shift {
                self.input
                    .update(cx, |input, cx| input.insert_newline(window, cx));
            } else {
                self.send(self.input.read(cx).content.to_string(), window, cx);
            }
        } else {
            return;
        }
        cx.stop_propagation();
    }
}

fn icon(name: &'static str, tint: u32) -> impl IntoElement {
    svg()
        .path(name)
        .size(px(15.))
        .text_color(color(tint))
        .flex_shrink_0()
}

fn quote_text_html(text: &str) -> String {
    format!(
        "<p>{}</p>",
        text.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('\n', "<br>")
    )
}

fn stream_error(raw: &str) -> String {
    if raw.trim().is_empty() {
        return "Sorry, an error occurred while processing your request.".into();
    }
    if let Some((_, after)) = raw.split_once("\"message\"") {
        if let Some((_, value)) = after.split_once(':') {
            if let Some(Ok(message)) = serde_json::Deserializer::from_str(value.trim()).into_iter::<String>().next() {
                return message;
            }
        }
    }
    if let Some((_, after)) = raw.split_once("'message':") {
        if let Some(value) = after.trim_start().strip_prefix('\'') {
            if let Some(end) = value.find('\'') { return value[..end].to_owned(); }
        }
    }
    let lower = raw.to_lowercase();
    if raw.contains("context_length_exceeded") || (lower.contains("exceed") && lower.contains("limit") && lower.contains("token")) {
        return "The conversation or attachments are too large for the model token limit. Try shortening your message or removing some context.".into();
    }
    if raw.encode_utf16().count() > 800 {
        let mut units = 0;
        let prefix: String = raw.chars().take_while(|c| { units += c.len_utf16(); units <= 797 }).collect();
        return format!("{prefix}…");
    }
    raw.to_owned()
}

fn markdown(text: &str, colors: Theme) -> RichText {
    let parser = pulldown_cmark::Parser::new_ext(
        text,
        pulldown_cmark::Options::ENABLE_STRIKETHROUGH | pulldown_cmark::Options::ENABLE_TABLES,
    );
    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, parser);
    RichText::parse(&html, colors)
}

impl Render for AiSidebar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = self.colors;
        self.fit_viewport(f32::from(window.viewport_size().width), cx);
        if self.collapsed {
            return div()
                .id("ai-collapsed")
                .w(px(if self.mode() == DisplayMode::Sidebar {
                    28.
                } else {
                    200.
                }))
                .h_full()
                .border_l_1()
                .border_color(color(t.border))
                .bg(color(t.container))
                .cursor_pointer()
                .on_click(cx.listener(|this, _, window, cx| this.focus_composer(window, cx)))
                .child(if self.mode() == DisplayMode::Sidebar {
                    div()
                        .mt(px(25.))
                        .ml(px(6.))
                        .child(icon("chevron-left", t.muted))
                } else {
                    div()
                        .px(px(12.))
                        .py(px(8.))
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(icon("sparkles", t.muted))
                        .child(
                            div()
                                .flex_1()
                                .truncate()
                                .child(self.chats[self.selected].title.clone()),
                        )
                        .child(
                            div()
                                .id("ai-minimized-close")
                                .child(icon("x", t.muted))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    cx.stop_propagation();
                                    this.set_open(false, cx);
                                })),
                        )
                })
                .into_any_element();
        }
        let chat = &self.chats[self.selected];
        let title = if chat.title == "New AI Chat"
            && self
                .scope
                .as_ref()
                .is_some_and(|s| s.context.surface != "task_detail")
        {
            self.scope
                .as_ref()
                .and_then(|s| s.context.view_name.clone())
                .unwrap_or_else(|| chat.title.clone())
        } else {
            chat.title.clone()
        };
        let empty = chat.messages.is_empty();
        let mut transcript = div()
            .id("ai-transcript")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .on_scroll_wheel(cx.listener(|_, _, _, cx| cx.notify()))
            .p(px(if empty { 0. } else { 8. }))
            .when(self.mode() == DisplayMode::Fullscreen, |d| {
                d.w_full().max_w(px(800.)).mx_auto()
            });
        if empty {
            let mut welcome = div()
                .size_full()
                .flex()
                .flex_col()
                .justify_center()
                .px(px(40.))
                .py(px(16.))
                .gap(px(8.));
            if self.scope.as_ref().is_some_and(|scope| scope.context.surface == "task_detail" && scope.context.task_id.is_some()) {
                welcome = welcome.child(
                    div().id("ai-task-summary").w_full().p(px(8.)).rounded(px(4.))
                        .border_1().border_color(color(t.accent)).text_size(px(12.))
                        .cursor_pointer().child("Summarize the current task")
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.send("/i Summarize the current task".into(), window, cx);
                        }))
                        .layout_key(&self.capture, "ai.task.summary", Some("Summarize the current task".into())),
                );
            }
            for (index, prompt) in self
                .scope
                .as_ref()
                .map(|s| s.suggestions.clone())
                .unwrap_or_default()
                .into_iter()
                .enumerate()
            {
                let label = prompt.clone();
                welcome = welcome.child(
                    div()
                        .id(("ai-suggestion", index))
                        .w_full()
                        .min_h(px(34.))
                        .px(px(8.))
                        .py(px(8.))
                        .rounded(px(4.))
                        .bg(color(t.active))
                        .text_size(px(12.))
                        .cursor_pointer()
                        .hover(|d| d.bg(color(t.hover)))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.send(prompt.clone(), window, cx)
                        }))
                        .child(label.clone())
                        .layout_key(&self.capture, format!("ai.suggestion.{index}"), Some(label)),
                );
            }
            transcript = transcript.child(welcome.layout_key(&self.capture, "ai.welcome", None));
        } else {
            for (index, message) in chat.messages.iter().enumerate() {
                let human = message.message.role == "human";
                let streaming = self.active.as_ref().is_some_and(|active| active.chat == self.selected && active.assistant == index);
                let mut body = div().px(px(8.)).py(px(4.)).rounded(px(4.))
                    .when(human, |d| d.bg(color(t.card)))
                    .child(if human {
                        RichText::parse(&message.message.content, t).element()
                    } else {
                        message.rich.element()
                    });
                if streaming && chat.error.is_none() {
                    body = body.child(
                        div().text_color(color(t.muted))
                            .child(if !chat.status.is_empty() {
                                chat.status.clone()
                            } else if message.message.content.trim().is_empty() {
                                "•••".into()
                            } else {
                                "▍".into()
                            })
                            .layout_key(&self.capture, "ai.stream.status", Some(chat.status.clone())),
                    );
                }
                if !human && index + 1 == chat.messages.len() {
                    if let Some(error) = &chat.error {
                        body = body.child(div().text_color(color(0xe07171)).child(error.clone())
                            .layout_key(&self.capture, "ai.error", Some(error.clone())));
                    }
                }
                let mut row = div().id(("ai-message", index)).flex().flex_col()
                    .when(index + 1 < chat.messages.len(), |d| d.mb(px(8.)))
                    .when(human, |d| d.items_end())
                    .child(body.layout_key(&self.capture, format!("ai.message.{index}.body"), None));
                if !streaming && (!message.message.content.is_empty() || (index + 1 == chat.messages.len() && chat.error.is_some())) {
                    let mut actions = div().flex().gap(px(4.)).px(px(8.)).text_size(px(11.));
                    if human {
                        actions = actions.child(
                            div().id(("ai-edit-message", index)).tab_index(0).cursor_pointer().child(icon("square-pen", t.muted))
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    let content = this.chats[this.selected].messages[index].message.content.clone();
                                    this.draft_quotes.clear();
                                    this.input.update(cx, |input, cx| input.set_html(&content, cx));
                                    window.focus(&this.input.focus_handle(cx), cx);
                                }))
                                .layout_key(&self.capture, format!("ai.message.{index}.edit"), None),
                        );
                    }
                    if human || (index + 1 == chat.messages.len() && chat.error.is_some()) {
                        actions = actions.child(
                            div().id(("ai-retry-message", index)).tab_index(0).cursor_pointer().child(icon("rotate-cw", t.muted))
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    let messages = &this.chats[this.selected].messages;
                                    let content = if messages[index].message.role == "human" {
                                        Some(messages[index].message.content.clone())
                                    } else {
                                        messages.iter().rev().find(|message| message.message.role == "human")
                                            .map(|message| message.message.content.clone())
                                    };
                                    if let Some(content) = content { this.send(content, window, cx); }
                                }))
                                .layout_key(&self.capture, format!("ai.message.{index}.retry"), None),
                        );
                    }
                    actions = actions.when(!message.message.content.is_empty(), |d| d.child(
                        div().id(("ai-copy-response", index)).tab_index(0).cursor_pointer().child(icon("copy", t.muted))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                let message = &this.chats[this.selected].messages[index];
                                let text = if message.message.role == "human" {
                                    RichText::parse(&message.message.content, this.colors).text
                                } else {
                                    message.rich.text.clone()
                                };
                                cx.write_to_clipboard(gpui::ClipboardItem::new_string(text));
                            }))
                            .layout_key(&self.capture, format!("ai.response.{index}.copy"), None),
                    ));
                    row = row.child(actions);
                }
                transcript = transcript.child(row.layout_key(&self.capture, format!("ai.message.{index}"), Some(message.message.content.clone())));
            }
        }
        let error = chat.error.clone().filter(|_| chat.messages.last().is_none_or(|message| message.message.role != "assistant"));
        let mut header = div()
            .h(px(48.))
            .flex_shrink_0()
            .flex()
            .items_center()
            .px(px(8.))
            .pt(px(16.))
            .gap(px(16.))
            .child(
                div()
                    .id("ai-history-toggle")
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_history(cx)))
                    .child(
                        div()
                            .truncate()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(title),
                    )
                    .child(icon("chevron-down", t.muted))
                    .layout_key(&self.capture, "ai.history.toggle", None),
            )
            .child(
                div()
                    .id("ai-new")
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, window, cx| this.new_chat(window, cx)))
                    .child(icon("square-pen", t.muted))
                    .layout_key(&self.capture, "ai.new", None),
            )
            .child(
                div()
                    .id("ai-more")
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.more_menu = !this.more_menu;
                        this.history_menu = false;
                        cx.notify();
                    }))
                    .child(icon("ellipsis", t.muted))
                    .layout_key(&self.capture, "ai.more", None),
            )
            .child(
                div()
                    .id("ai-collapse")
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, window, cx| {
                        if this.mode() == DisplayMode::Sidebar {
                            this.set_open(false, cx);
                        } else {
                            this.collapsed = true;
                        }
                        if let Some(previous) = &this.previous_focus {
                            window.focus(previous, cx);
                        }
                        cx.notify();
                    }))
                    .child(icon(
                        if self.mode() == DisplayMode::Sidebar {
                            "chevron-right"
                        } else {
                            "minus"
                        },
                        t.muted,
                    ))
                    .layout_key(&self.capture, "ai.close", None),
            );
        if self.mode() == DisplayMode::Fullscreen {
            header = div()
                .h(px(48.))
                .w_full()
                .flex_shrink_0()
                .px(px(16.))
                .flex()
                .items_center()
                .gap(px(8.))
                .child(icon("sparkles", t.accent))
                .child(
                    div()
                        .flex_1()
                        .truncate()
                        .child(self.chats[self.selected].title.clone()),
                )
                .child(
                    div()
                        .id("ai-exit-fullscreen")
                        .cursor_pointer()
                        .p(px(4.))
                        .on_click(cx.listener(|this, _, _, cx| this.exit_fullscreen(cx)))
                        .child(icon("minimize-2", t.muted))
                        .layout_key(&self.capture, "ai.fullscreen.exit", None),
                );
        }
        if self.history_menu {
            header = header.bg(color(t.container));
        }
        let mut header_area = div()
            .id("ai-header-area")
            .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                if this.history_menu || this.more_menu {
                    this.history_menu = false;
                    this.more_menu = false;
                    cx.notify();
                }
            }))
            .child(header.layout_key(&self.capture, "ai.header", None));
        let mut panel = div()
            .id("ai-sidebar")
            .w_full()
            .flex_shrink_0()
            .h_full()
            .min_h_0()
            .border_l_1()
            .border_color(color(t.border))
            .bg(color(t.container))
            .flex()
            .flex_col()
            .track_focus(&self.focus)
            .capture_key_down(cx.listener(Self::skill_key))
            .on_key_down(cx.listener(Self::key_down))
            .relative();
        if self.history_menu {
            let mut menu = div()
                .id("ai-history")
                .max_h(px(240.))
                .overflow_y_scroll()
                .mx(px(16.))
                .p(px(8.))
                .rounded(px(6.))
                .bg(color(t.card));
            for (index, chat) in self.chats.iter().enumerate().rev() {
                menu = menu.child(
                    div()
                        .id(("ai-chat-choice", index))
                        .p(px(6.))
                        .cursor_pointer()
                        .hover(|d| d.bg(color(t.hover)))
                        .when(index == self.selected, |d| d.bg(color(t.active)))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.chat_pending = None;
                            this.selected = index;
                            this.scroll = this.chats[index].scroll.clone();
                            this.queued.clear();
                            this.history_menu = false;
                            cx.notify();
                        }))
                        .child(chat.title.clone())
                        .layout_key(&self.capture, format!("ai.history.{index}"), None),
                );
            }
            for (index, session) in self.history.iter().enumerate().take(self.history_limit) {
                if self.chats.iter().any(|chat| chat.id == session.id) {
                    continue;
                }
                menu = menu.child(
                    div()
                        .id(("ai-server-chat", index))
                        .p(px(6.))
                        .cursor_pointer()
                        .hover(|d| d.bg(color(t.hover)))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if this.chat_pending.is_none() {
                                this.open_history_chat(index, cx);
                            }
                        }))
                        .child(session.title.clone().unwrap_or_else(|| "AI Chat".into())),
                );
            }
            if self.history.len() > self.history_limit {
                menu = menu.child(
                    div()
                        .id("ai-history-more")
                        .p(px(6.))
                        .cursor_pointer()
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.history_limit += 30;
                            cx.notify();
                        }))
                        .child("Show more"),
                );
            }
            if self.history_pending.is_some() || self.chat_pending.is_some() {
                menu = menu.child(div().text_size(px(11.)).child("Loading history…"));
            }
            if let Some(error) = &self.history_error {
                menu = menu.child(
                    div()
                        .text_size(px(11.))
                        .text_color(color(0xe07171))
                        .child(error.clone()),
                );
            }
            header_area = header_area.child(menu);
        }
        if self.more_menu {
            header_area = header_area.child(
                div()
                    .id("ai-overflow-menu")
                    .mx(px(16.))
                    .p(px(8.))
                    .bg(color(t.card))
                    .rounded(px(6.))
                    .children(
                        [
                            (
                                "ai-pin",
                                if self.presentation.pinned {
                                    "Unpin AI chat"
                                } else {
                                    "Pin AI chat open"
                                },
                            ),
                            (
                                "ai-mode",
                                if self.mode() == DisplayMode::Sidebar {
                                    "Toggle Floating View"
                                } else {
                                    "Toggle Sidebar View"
                                },
                            ),
                            ("ai-fullscreen", "Open full screen"),
                        ]
                        .into_iter()
                        .map(|(id, label)| {
                            div()
                                .id(id)
                                .p(px(6.))
                                .cursor_pointer()
                                .hover(|d| d.bg(color(t.hover)))
                                .child(label)
                                .on_click(cx.listener(move |this, _, _, cx| match id {
                                    "ai-pin" => this.toggle_pin(cx),
                                    "ai-mode" => this.set_mode(
                                        if this.mode() == DisplayMode::Sidebar {
                                            DisplayMode::Floating
                                        } else {
                                            DisplayMode::Sidebar
                                        },
                                        cx,
                                    ),
                                    _ => this.set_mode(DisplayMode::Fullscreen, cx),
                                }))
                                .layout_key(&self.capture, id.replace('-', "."), None)
                        }),
                    )
                    .child(
                        div()
                            .id("ai-new-menu")
                            .p(px(6.))
                            .cursor_pointer()
                            .hover(|d| d.bg(color(t.hover)))
                            .on_click(cx.listener(|this, _, window, cx| this.new_chat(window, cx)))
                            .child("New AI Chat"),
                    ),
            );
        }
        panel = panel
            .child(header_area)
            .child(transcript.layout_key(&self.capture, "ai.transcript", None))
            .when(self.away_from_latest() && self.active.is_none(), |d| {
                d.child(
                    div()
                        .id("ai-scroll-latest")
                        .cursor_pointer()
                        .mx(px(20.))
                        .child("↓")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.scroll.scroll_to_bottom();
                            cx.notify();
                        }))
                        .layout_key(&self.capture, "ai.latest", None),
                )
            })
            .when_some(error, |d, error| {
                d.child(
                    div()
                        .px(px(20.))
                        .pb(px(8.))
                        .text_size(px(12.))
                        .text_color(color(0xe07171))
                        .child(error.clone())
                        .layout_key(&self.capture, "ai.error", Some(error)),
                )
            });
        let mut composer = div()
            .id("ai-composer")
            .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                if this.model_menu {
                    this.model_menu = false;
                    cx.notify();
                }
            }))
            .mx(px(8.))
            .mb(px(8.))
            .p(px(8.))
            .rounded(px(8.))
            .bg(color(t.card))
            .flex()
            .flex_col()
            .gap(px(12.))
            .child(
                div()
                    .id("ai-composer-input")
                    .track_scroll(&self.input_scroll)
                    .min_h(px(40.))
                    .max_h(px(380.))
                    .overflow_y_scroll()
                    .text_size(px(14.))
                    .child(self.input.clone())
                    .layout_key(&self.capture, "ai.input", None),
            );
        if !self.queued.is_empty() {
            composer = composer.child(
                div().mb(px(8.)).p(px(8.)).rounded(px(5.)).border_1().border_color(color(t.border))
                    .child("Queued · sends when reply finishes")
                    .children(self.queued.iter().enumerate().map(|(index, item)| {
                        div().id(("ai-queued", index)).flex().gap(px(8.)).text_size(px(12.))
                            .child(div().flex_1().truncate().child(item.text.clone()))
                            .child(div().id(("ai-queue-remove", index)).cursor_pointer().child(icon("x", t.muted))
                                .on_click(cx.listener(move |this, _, _, cx| { this.queued.remove(index); cx.notify(); }))
                                .layout_key(&self.capture, format!("ai.queue.{index}.remove"), None))
                            .layout_key(&self.capture, format!("ai.queue.{index}"), Some(item.text.clone()))
                    })),
            );
        }
        if !self.skill_choices(cx).is_empty() { composer = composer.child(self.skill_menu(cx)); }
        if self.model_menu {
            let mut menu = div()
                .id("ai-model-options")
                .max_h(px(210.))
                .overflow_y_scroll()
                .flex()
                .flex_col();
            for (index, model) in MODELS.iter().enumerate() {
                menu = menu.child(
                    div()
                        .id(("ai-model", index))
                        .p(px(6.))
                        .text_size(px(12.))
                        .cursor_pointer()
                        .hover(|d| d.bg(color(t.hover)))
                        .when(index == self.model, |d| d.bg(color(t.active)))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.model = index;
                            this.model_menu = false;
                            cx.notify();
                        }))
                        .child(model.label),
                );
            }
            composer = composer.child(
                menu.child(
                    div()
                        .text_size(px(11.))
                        .text_color(color(t.muted))
                        .child("Availability is server-checked"),
                ),
            );
        }
        let label = if self.include_context {
            self.scope
                .as_ref()
                .map(|s| s.label.clone())
                .unwrap_or_else(|| "Current board".into())
        } else {
            "All boards".into()
        };
        let controls = div()
            .flex()
            .items_center()
            .gap(px(12.))
            .text_size(px(12.))
            .child(
                div()
                    .id("ai-model-toggle")
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.model_menu = !this.model_menu;
                        cx.notify();
                    }))
                    .child(MODELS[self.model].label)
                    .child(icon("chevron-down", t.muted)),
            )
            .child(
                div()
                    .id("ai-context-toggle")
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.include_context = !this.include_context;
                        cx.notify();
                    }))
                    .child(icon("columns-2", t.muted))
                    .child(div().truncate().child(label))
                    .child(icon(
                        if self.include_context {
                            "x"
                        } else {
                            "chevron-down"
                        },
                        t.muted,
                    )),
            )
            .when(self.active.is_some(), |d| d.child(
                div().id("ai-stop").cursor_pointer().child("Stop")
                    .on_click(cx.listener(|this, _, _, cx| this.stop(cx)))
                    .layout_key(&self.capture, "ai.stop", None),
            ))
            .when(self.active.is_none() || !self.input.read(cx).content.trim().is_empty(), |d| d.child(
                div().id("ai-send").cursor_pointer().child(icon("send", t.muted))
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.send(this.input.read(cx).content.to_string(), window, cx);
                    }))
                    .layout_key(&self.capture, "ai.send", None),
            ));
        composer = composer.child(controls);
        if self.mode() == DisplayMode::Fullscreen {
            composer = composer.w_full().max_w(px(800.)).mx_auto();
        }
        panel = panel.child(composer.layout_key(&self.capture, "ai.composer", None));
        if self.mode() == DisplayMode::Sidebar {
            panel = panel.child(self.resize_edge(cx));
        }
        panel.into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn markdown_replies_use_existing_rich_text_with_unicode_and_blocks() {
        let rendered = markdown(
            "## Reply\n\n**Hello** é🙂\n\n- first\n- second\n\n[link](https://hypertask.ai)",
            crate::theme::THEMES[1],
        );
        assert!(rendered.text.contains("Reply\n"));
        assert!(rendered.text.contains("Hello é🙂"));
        assert!(rendered.text.contains("• first"));
        assert!(rendered.text.contains("link"));
        assert!(!rendered.text.contains("**"));
    }

    #[test]
    fn scope_matches_web_board_task_and_inbox_rules() {
        let mut db = Db::memory().unwrap();
        db.apply_response(&serde_json::json!({"success":true,
            "projects":[{"id":123,"title":"Board","teamId":42}],
            "tasks":[{"id":456,"projectId":123,"title":"Investigate","ticketNumber":"HTPR-123","uniqueIndex":7,"status":"Normal"}]
        })).unwrap();
        let board = Scope::load(&db, Screen::Board, 123, None);
        assert_eq!(board.context.project_id, Some(123));
        assert_eq!(board.context.task_id, None);
        assert_eq!(board.team_id.as_deref(), Some("42"));
        let task = Scope::load(&db, Screen::Task, 999, Some(456));
        assert_eq!(task.context.project_id, Some(123));
        assert_eq!(task.context.task_id, Some(456));
        assert_eq!(task.context.surface_path, "/detail/project-123/7");
        assert_eq!(task.label, "HTPR-123 Investigate");
        let inbox = Scope::load(&db, Screen::Inbox, 999, Some(456));
        assert_eq!(inbox.context.project_id, None);
        assert_eq!(inbox.context.task_id, Some(456));
        assert_eq!(inbox.context.surface, "inbox");
        assert_ne!(inbox.suggestions, task.suggestions);
    }

    #[gpui::test]
    fn sidebar_enter_still_sends_and_shift_enter_is_exactly_one_newline(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(crate::input::init);
        let (view, cx) = cx.add_window_view(|window, cx| {
            let workspace = cx.focus_handle();
            let mut sidebar = AiSidebar::new(crate::theme::THEMES[0], None, workspace, window, cx);
            sidebar.focus_composer(window, cx);
            sidebar
        });
        cx.simulate_input("First");
        cx.simulate_keystrokes("shift-enter");
        cx.simulate_input("Second");
        view.read_with(cx, |sidebar, cx| {
            assert_eq!(sidebar.input.read(cx).content.as_ref(), "First\nSecond")
        });
        cx.simulate_keystrokes("enter");
        view.read_with(cx, |sidebar, cx| {
            assert!(
                sidebar.chats[sidebar.selected]
                    .error
                    .as_ref()
                    .unwrap()
                    .contains("offline")
            );
            assert_eq!(sidebar.input.read(cx).content.as_ref(), "First\nSecond");
        });
    }

    #[gpui::test]
    fn keyboard_reset_platform_aliases_create_distinct_chats_and_clear_the_draft(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(crate::input::init);
        let (view, cx) = cx.add_window_view(|window, cx| {
            let workspace = cx.focus_handle();
            let mut sidebar = AiSidebar::new(crate::theme::THEMES[0], None, workspace, window, cx);
            sidebar.focus_composer(window, cx);
            sidebar
        });
        for (index, chord) in ["ctrl-shift-o", "cmd-shift-o", "ctrl-shift-j", "cmd-shift-j"].iter().enumerate() {
            cx.simulate_input("old draft");
            let old = view.read_with(cx, |sidebar, _| sidebar.chats[sidebar.selected].id.clone());
            cx.simulate_keystrokes(chord);
            view.update_in(cx, |sidebar, window, cx| {
                assert_eq!(sidebar.chats.len(), index + 2, "{chord}");
                assert_ne!(sidebar.chats[sidebar.selected].id, old, "{chord}");
                assert!(sidebar.input.read(cx).content.is_empty(), "{chord}");
                assert!(sidebar.input.focus_handle(cx).is_focused(window), "{chord}");
            });
        }
    }

    #[gpui::test]
    fn offline_composer_focus_multiline_and_new_session(cx: &mut gpui::TestAppContext) {
        let view = cx.add_window(|window, cx| {
            let workspace = cx.focus_handle();
            window.focus(&workspace, cx);
            AiSidebar::new(crate::theme::THEMES[1], None, workspace, window, cx)
        });
        view.update(cx, |sidebar, window, cx| {
            let workspace = sidebar.previous_focus.clone().unwrap();
            sidebar.focus_composer(window, cx);
            assert!(sidebar.input.focus_handle(cx).is_focused(window));
            sidebar
                .input
                .update(cx, |input, cx| input.set_content("First".into(), cx));
            // set_content selects the string, so collapse the selection before inserting.
            sidebar.input.update(cx, |input, cx| {
                use gpui::EntityInputHandler;
                input.set_selected_text_range(5..5, window, cx);
                input.insert_newline(window, cx);
                input.replace_text_in_range(None, "Second é🙂", window, cx);
            });
            let draft = sidebar.input.read(cx).content.to_string();
            assert_eq!(draft, "First\nSecond é🙂");
            sidebar.send(draft.clone(), window, cx);
            assert!(sidebar.active.is_none());
            assert!(
                sidebar.chats[0]
                    .error
                    .as_deref()
                    .unwrap()
                    .contains("offline")
            );
            assert_eq!(sidebar.input.read(cx).content.as_ref(), draft);
            sidebar.focus_composer(window, cx);
            assert!(workspace.is_focused(window));
            let old_id = sidebar.chats[0].id.clone();
            sidebar.new_chat(window, cx);
            assert_ne!(old_id, sidebar.chats[sidebar.selected].id);
            assert!(sidebar.input.read(cx).content.is_empty());
            assert!(sidebar.chats[sidebar.selected].error.is_none());
        })
        .unwrap();
    }

    #[gpui::test]
    fn history_is_lazy_background_results_are_polled_without_an_active_turn(
        cx: &mut gpui::TestAppContext,
    ) {
        let view = cx.add_window(|window, cx| {
            let workspace = cx.focus_handle();
            AiSidebar::new(
                crate::theme::THEMES[1],
                Some("test-token".into()),
                workspace,
                window,
                cx,
            )
        });
        view.update(cx, |sidebar, window, cx| {
            assert!(sidebar.history.is_empty());
            assert!(sidebar.history_pending.is_none());
            assert_eq!(sidebar.width(), 420.);
            sidebar.collapsed = true;
            assert_eq!(sidebar.width(), 28.);
            sidebar.focus_composer(window, cx);
            assert_eq!(sidebar.width(), 420.);
            let (send, receive) = mpsc::channel();
            sidebar.history_pending = Some(receive);
            send.send(Ok(vec![Arc::new(Session {
                id: "remote-session".into(),
                title: Some("Web history".into()),
                messages: vec![Message {
                    role: "assistant".into(),
                    content: "Saved reply".into(),
                }],
            })]))
            .unwrap();
            sidebar.poll(cx);
            assert_eq!(sidebar.history.len(), 1);
            assert!(sidebar.history_pending.is_none());
            assert!(sidebar.active.is_none());
            let original = Arc::clone(&sidebar.history[0]);
            sidebar.open_history_chat(0, cx);
            assert_eq!(original.messages[0].content, "Saved reply");
            // New-chat intent invalidates a late transcript result.
            let (_send, receive) = mpsc::channel();
            sidebar.chat_pending = Some(receive);
            sidebar.new_chat(window, cx);
            assert!(sidebar.chat_pending.is_none());
            sidebar.set_token(None, cx);
            assert!(sidebar.history.is_empty());
            assert!(sidebar.history_pending.is_none());
            sidebar.toggle_history(cx);
            assert!(sidebar.history_menu);
            assert!(
                sidebar
                    .history_error
                    .as_deref()
                    .unwrap()
                    .contains("Sign in")
            );
        })
        .unwrap();
    }

    #[gpui::test]
    fn late_stream_updates_stay_with_the_original_chat_and_preserve_errors(
        cx: &mut gpui::TestAppContext,
    ) {
        let view = cx.add_window(|window, cx| {
            let workspace = cx.focus_handle();
            AiSidebar::new(crate::theme::THEMES[1], None, workspace, window, cx)
        });
        view.update(cx, |sidebar, window, cx| {
            let (send, updates) = mpsc::sync_channel(64);
            sidebar.chats[0].messages.push(ChatMessage {
                message: Message {
                    role: "assistant".into(),
                    content: String::new(),
                },
                rich: RichText::default(),
            });
            sidebar.active = Some(ActiveTurn {
                ids: TurnIds::new(sidebar.chats[0].id.clone()),
                chat: 0,
                assistant: 0,
                updates,
                cancel_reply: None,
                stopping: false,
                client: Client::new("test-token".into()).unwrap(),
            });
            sidebar.new_chat(window, cx);
            send.send(Update::Event(Event::Content("**Partial** é🙂".into())))
                .unwrap();
            send.send(Update::Event(Event::Title("Original reply".into())))
                .unwrap();
            send.send(Update::Event(Event::Error("Server refused model".into())))
                .unwrap();
            send.send(Update::Event(Event::Done(ai_chat::Done {
                status: "error".into(),
                cancelled: false,
                user_message_persisted: false,
                assistant_persisted: false,
            })))
            .unwrap();
            sidebar.poll(cx);
            assert!(sidebar.active.is_none());
            assert_eq!(sidebar.chats[0].title, "Original reply");
            assert_eq!(sidebar.chats[0].messages[0].rich.text.trim(), "Partial é🙂");
            assert_eq!(
                sidebar.chats[0].error.as_deref(),
                Some("Server refused model")
            );
            assert!(sidebar.chats[sidebar.selected].messages.is_empty());
            assert!(sidebar.chats[sidebar.selected].error.is_none());
        })
        .unwrap();
    }
}

#[cfg(test)]
#[path = "ai_message_tests.rs"]
mod message_tests;
