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
    ai_chat::{self, Client, DefaultContext, Event, MODELS, Message, TurnIds},
};
use std::{
    sync::mpsc::{self, Receiver},
    thread,
    time::Duration,
};

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
            db.board(project).unwrap_or_default()
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
                project_id: (project > 0).then_some(project),
                task_id: task.as_ref().map(|t| t.id),
                surface: surface.into(),
                surface_path: path,
                view_name: Some(board_name),
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
}

impl Chat {
    fn new() -> Self {
        Self {
            id: ai_chat::session_id(),
            title: "New AI Chat".into(),
            messages: vec![],
            status: String::new(),
            error: None,
        }
    }
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
    input: Entity<TextInput>,
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
    more_menu: bool,
    collapsed: bool,
    scroll: ScrollHandle,
    active: Option<ActiveTurn>,
}

impl AiSidebar {
    pub fn new(
        colors: Theme,
        token: Option<String>,
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
            .multiline()
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
        Self {
            colors,
            input,
            focus: cx.focus_handle(),
            previous_focus: None,
            client: token.and_then(|t| Client::new(t).ok()),
            scope: None,
            include_context: true,
            chats: vec![Chat::new()],
            selected: 0,
            model: 0,
            model_menu: false,
            history_menu: false,
            more_menu: false,
            collapsed: false,
            scroll: ScrollHandle::new(),
            active: None,
        }
    }

    pub fn set_token(&mut self, token: Option<String>, cx: &mut Context<Self>) {
        self.client = token.and_then(|t| Client::new(t).ok());
        cx.notify();
    }

    pub fn set_scope(&mut self, scope: Scope, cx: &mut Context<Self>) {
        if self.scope.as_ref() != Some(&scope) {
            self.scope = Some(scope);
            self.include_context = true;
            cx.notify();
        }
    }

    pub fn focus_composer(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let focus = self.input.focus_handle(cx);
        if focus.is_focused(window) {
            if let Some(previous) = &self.previous_focus {
                window.focus(previous, cx);
            }
        } else {
            self.previous_focus = window.focused(cx);
            self.collapsed = false;
            window.focus(&focus, cx);
        }
        cx.notify();
    }

    pub fn has_focus(&self, window: &Window, cx: &App) -> bool {
        self.focus.contains_focused(window, cx)
    }

    fn new_chat(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.chats.push(Chat::new());
        self.selected = self.chats.len() - 1;
        self.include_context = true;
        self.history_menu = false;
        self.more_menu = false;
        self.input.update(cx, |input, cx| input.reset(window, cx));
        window.focus(&self.input.focus_handle(cx), cx);
        cx.notify();
    }

    fn send(&mut self, text: String, window: &mut Window, cx: &mut Context<Self>) {
        let text = text.trim().to_owned();
        if text.is_empty() || self.active.is_some() {
            return;
        }
        let chat = &mut self.chats[self.selected];
        chat.error = None;
        let Some(client) = self.client.clone() else {
            chat.error =
                Some("AI chat is unavailable offline. Sign in and reconnect to send.".into());
            cx.notify();
            return;
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
            &text,
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
            return;
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
        self.active = Some(ActiveTurn {
            ids,
            chat: self.selected,
            assistant,
            updates,
            cancel_reply: None,
            stopping: false,
            client,
        });
        self.input.update(cx, |input, cx| input.reset(window, cx));
        cx.notify();
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

    fn poll(&mut self, cx: &mut Context<Self>) {
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
                    chat.error =
                        Some("AI reply interrupted before done; partial text is preserved".into());
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
                Update::Event(Event::Error(error)) => chat.error = Some(error),
                Update::Event(Event::Done(done)) => {
                    if done.status == "error" && chat.error.is_none() {
                        chat.error = Some("AI reply failed".into());
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
            if active.chat == self.selected {
                self.scroll.scroll_to_bottom();
            }
        }
        if terminal {
            if chat.error.is_some() {
                chat.status.clear();
            }
            self.active = None;
        }
        if changed {
            cx.notify();
        }
    }

    fn key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let key = event.keystroke.key.as_str();
        let modifiers = event.keystroke.modifiers;
        if modifiers.control && key == "q" {
            self.focus_composer(window, cx);
        } else if modifiers.control && modifiers.shift && matches!(key, "o" | "j") {
            self.new_chat(window, cx);
        } else if key == "escape" {
            if self.active.is_some() {
                self.stop(cx);
            } else if let Some(previous) = &self.previous_focus {
                window.focus(previous, cx);
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
        }
        // Workspace J/K/E/M shortcuts must never consume composer typing.
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
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = self.colors;
        if self.collapsed {
            return div()
                .id("ai-collapsed")
                .w(px(28.))
                .h_full()
                .border_l_1()
                .border_color(color(t.border))
                .bg(color(t.container))
                .cursor_pointer()
                .on_click(cx.listener(|this, _, window, cx| this.focus_composer(window, cx)))
                .child(
                    div()
                        .mt(px(25.))
                        .ml(px(6.))
                        .child(icon("chevron-left", t.muted)),
                )
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
            .px(px(24.))
            .py(px(16.));
        if empty {
            let mut welcome = div()
                .size_full()
                .flex()
                .flex_col()
                .justify_center()
                .px(px(20.))
                .gap(px(8.));
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
                        .rounded(px(6.))
                        .bg(color(t.active))
                        .text_size(px(12.))
                        .cursor_pointer()
                        .hover(|d| d.bg(color(t.hover)))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.send(prompt.clone(), window, cx)
                        }))
                        .child(label),
                );
            }
            transcript = transcript.child(welcome);
        } else {
            for (index, message) in chat.messages.iter().enumerate() {
                if message.message.role == "human" {
                    transcript = transcript.child(
                        div()
                            .id(("ai-message", index))
                            .mb(px(16.))
                            .p(px(12.))
                            .rounded(px(8.))
                            .bg(color(t.card))
                            .child(message.message.content.clone()),
                    );
                } else {
                    transcript = transcript.child(
                        div()
                            .id(("ai-message", index))
                            .mb(px(20.))
                            .child(message.rich.element()),
                    );
                }
            }
        }
        let error = chat.error.clone();
        let status = if self
            .active
            .as_ref()
            .is_some_and(|a| a.chat != self.selected)
        {
            "Replying in another chat…".into()
        } else {
            chat.status.clone()
        };
        let mut header = div()
            .h(px(72.))
            .flex_shrink_0()
            .flex()
            .items_center()
            .px(px(16.))
            .gap(px(8.))
            .child(
                div()
                    .id("ai-history-toggle")
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.history_menu = !this.history_menu;
                        this.model_menu = false;
                        this.more_menu = false;
                        cx.notify();
                    }))
                    .child(
                        div()
                            .truncate()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(title),
                    )
                    .child(icon("chevron-down", t.muted)),
            )
            .child(
                div()
                    .id("ai-new")
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, window, cx| this.new_chat(window, cx)))
                    .child(icon("square-pen", t.muted)),
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
                    .child(icon("ellipsis", t.muted)),
            )
            .child(
                div()
                    .id("ai-collapse")
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.collapsed = true;
                        if let Some(previous) = &this.previous_focus {
                            window.focus(previous, cx);
                        }
                        cx.notify();
                    }))
                    .child(icon("chevron-right", t.muted)),
            );
        if self.history_menu {
            header = header.bg(color(t.container));
        }
        let mut panel = div()
            .id("ai-sidebar")
            .w(px(420.))
            .flex_shrink_0()
            .h_full()
            .min_h_0()
            .border_l_1()
            .border_color(color(t.border))
            .bg(color(t.container))
            .flex()
            .flex_col()
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::key_down))
            .child(header);
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
                            this.selected = index;
                            this.history_menu = false;
                            cx.notify();
                        }))
                        .child(chat.title.clone()),
                );
            }
            panel = panel.child(
                menu.child(
                    div()
                        .text_size(px(11.))
                        .text_color(color(t.muted))
                        .child("This window only · server history awaits bearer support"),
                ),
            );
        }
        if self.more_menu {
            panel = panel.child(
                div()
                    .mx(px(16.))
                    .p(px(8.))
                    .bg(color(t.card))
                    .rounded(px(6.))
                    .child(
                        div()
                            .id("ai-new-menu")
                            .p(px(6.))
                            .cursor_pointer()
                            .hover(|d| d.bg(color(t.hover)))
                            .on_click(cx.listener(|this, _, window, cx| this.new_chat(window, cx)))
                            .child("New AI Chat"),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(color(t.muted))
                            .child("Ordinary AI · provider access is checked by the server"),
                    ),
            );
        }
        panel = panel
            .child(transcript)
            .when(!status.is_empty(), |d| {
                d.child(
                    div()
                        .px(px(20.))
                        .pb(px(8.))
                        .text_size(px(12.))
                        .text_color(color(t.muted))
                        .child(status),
                )
            })
            .when_some(error, |d, error| {
                d.child(
                    div()
                        .px(px(20.))
                        .pb(px(8.))
                        .text_size(px(12.))
                        .text_color(color(0xe07171))
                        .child(error),
                )
            });
        let mut composer = div()
            .mx(px(12.))
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
                    .min_h(px(40.))
                    .max_h(px(160.))
                    .overflow_y_scroll()
                    .text_size(px(14.))
                    .child(self.input.clone()),
            );
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
            .child(
                div()
                    .id("ai-send-stop")
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, window, cx| {
                        if this.active.is_some() {
                            this.stop(cx);
                        } else {
                            this.send(this.input.read(cx).content.to_string(), window, cx);
                        }
                    }))
                    .child(if self.active.is_some() {
                        div().child("Stop")
                    } else {
                        div().child(icon("send", t.muted))
                    }),
            );
        composer = composer.child(controls);
        panel.child(composer).into_any_element()
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
}
