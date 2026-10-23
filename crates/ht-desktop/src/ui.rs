use crate::{
    Options,
    input::TextInput,
    layout::{Capture, LayoutDump, LayoutKey},
    rich_text::RichText,
    signin::{Event, Phase, SignIn},
    theme::{Theme, ThemeName, color},
};
use gpui::{
    AnyElement, App, Context, Entity, FocusHandle, Focusable, FontWeight, KeyDownEvent, Keystroke,
    ListAlignment, ListState, PlatformInput, ScrollAnchor, ScrollHandle, ScrollStrategy,
    SharedString, UniformListScrollHandle, Window, div, list, point, prelude::*, px, rgba, svg,
    uniform_list,
};
use ht_core::{BoardColumn, Db, InboxItem, TaskDetail, TaskView, Worker, Write};
use std::{
    cell::RefCell,
    io::Write as _,
    path::PathBuf,
    rc::Rc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Inbox,
    Board,
    Task,
}
impl Screen {
    pub fn parse(s: &str) -> anyhow::Result<Self> {
        match s {
            "inbox" => Ok(Self::Inbox),
            "board" => Ok(Self::Board),
            "task" => Ok(Self::Task),
            _ => anyhow::bail!("unknown screen: {s}"),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Editing {
    None,
    Title,
    Comment,
}

pub struct Desktop {
    pub focus: FocusHandle,
    db: Db,
    options: Options,
    colors: Theme,
    screen: Screen,
    previous: Screen,
    inbox: Vec<InboxItem>,
    filtered: Vec<usize>,
    splits: Vec<String>,
    split: usize,
    columns: Vec<BoardColumn>,
    column_lists: Vec<ListState>,
    detail: Option<TaskDetail>,
    description: RichText,
    summary: SharedString,
    creator: SharedString,
    creator_initials: SharedString,
    comments: Vec<RichText>,
    selected: usize,
    column: usize,
    inbox_scroll: UniformListScrollHandle,
    board_scroll: ScrollHandle,
    detail_scroll: ScrollHandle,
    composer_anchor: ScrollAnchor,
    description_anchor: ScrollAnchor,
    comment_anchors: Vec<ScrollAnchor>,
    editing: Editing,
    title_input: Entity<TextInput>,
    comment_input: Entity<TextInput>,
    moving: bool,
    footer: bool,
    ready_scheduled: bool,
    layout: Capture,
    error: Option<String>,
    worker: Option<Worker>,
    signin: SignIn,
    auto_board: bool,
}

fn icon(name: &'static str, tint: u32, width: f32) -> impl IntoElement + Styled + use<> {
    svg()
        .path(name)
        .size(px(width))
        .text_color(color(tint))
        .flex_shrink_0()
}

fn shadow() -> Vec<gpui::BoxShadow> {
    vec![gpui::BoxShadow {
        inset: false,
        color: rgba(0x00000018).into(),
        offset: point(px(0.), px(2.)),
        blur_radius: px(4.),
        spread_radius: px(0.),
    }]
}

impl Desktop {
    pub fn new(db: Db, options: Options, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let colors = options.theme.colors();
        let title_input =
            cx.new(|cx| TextInput::new(String::new(), "Task title", color(colors.muted), cx));
        let comment_input = cx.new(|cx| {
            TextInput::new(
                String::new(),
                "Tip: Hit CTRL+M to comment",
                color(0xadb5bd),
                cx,
            )
        });
        let (mut signin, secret) = SignIn::start(options.offline);
        let worker = secret.and_then(|secret| Self::start_worker(&options, &mut signin, secret));
        let auto_board = !options.offline && !options.board_given && signin.is_interactive();
        let mut options = options;
        if auto_board {
            Self::resolve_board(&db, &mut options);
        }
        let detail_scroll = ScrollHandle::new();
        let mut view = Self {
            focus: cx.focus_handle(),
            screen: options.screen,
            previous: Screen::Board,
            layout: options
                .dump_layout
                .as_ref()
                .map(|_| Rc::new(RefCell::new(LayoutDump::default()))),
            options,
            db,
            colors,
            inbox: vec![],
            filtered: vec![],
            splits: vec![],
            split: 0,
            columns: vec![],
            column_lists: vec![],
            detail: None,
            description: RichText::default(),
            summary: SharedString::default(),
            creator: SharedString::default(),
            creator_initials: SharedString::default(),
            comments: vec![],
            selected: 0,
            column: 0,
            inbox_scroll: UniformListScrollHandle::new(),
            board_scroll: ScrollHandle::new(),
            composer_anchor: ScrollAnchor::for_handle(detail_scroll.clone()),
            description_anchor: ScrollAnchor::for_handle(detail_scroll.clone()),
            comment_anchors: vec![],
            detail_scroll,
            editing: Editing::None,
            title_input,
            comment_input,
            moving: false,
            footer: true,
            ready_scheduled: false,
            error: None,
            worker,
            signin,
            auto_board,
        };
        view.reload();
        view.request_sync();
        if !view.options.offline && view.signin.is_interactive() {
            cx.spawn_in(window, async move |view, cx| {
                loop {
                    cx.background_executor()
                        .timer(Duration::from_millis(300))
                        .await;
                    if view.update(cx, |this, cx| this.poll_signin(cx)).is_err() {
                        break;
                    }
                }
            })
            .detach();
        }
        if view.worker.is_some() || (!view.options.offline && view.signin.is_interactive()) {
            cx.spawn_in(window, async move |view, cx| {
                loop {
                    cx.background_executor().timer(Duration::from_secs(1)).await;
                    if view
                        .update(cx, |this, cx| {
                            this.reload();
                            if let Some(worker) = &this.worker {
                                if worker.errors().try_iter().next().is_some() {
                                    this.error =
                                        Some("Sync unavailable — edits are saved locally".into());
                                }
                            }
                            cx.notify();
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            })
            .detach();
        }
        view
    }

    fn start_worker(options: &Options, signin: &mut SignIn, secret: String) -> Option<Worker> {
        let http = ht_core::Http::new(secret).ok()?;
        signin.watch(http.unauthorized_flag());
        Worker::start(&options.db, http).ok()
    }

    fn resolve_board(db: &Db, options: &mut Options) {
        let remembered = db
            .kv_get("last_board")
            .ok()
            .flatten()
            .and_then(|id| id.parse().ok());
        if let Some(id) = remembered.or(db.first_project_id().ok().flatten()) {
            options.board = id;
        }
    }

    fn poll_signin(&mut self, cx: &mut Context<Self>) {
        match self.signin.poll() {
            Event::Idle => return,
            Event::Expired => {
                if let Some(worker) = self.worker.take() {
                    std::thread::spawn(move || drop(worker));
                }
            }
            Event::SignedIn(secret) => {
                self.worker = Self::start_worker(&self.options, &mut self.signin, secret);
                self.screen = Screen::Inbox;
                self.selected = 0;
                self.error = None;
                self.reload();
                self.request_sync();
            }
        }
        cx.notify();
    }

    fn request_sync(&self) {
        if let Some(worker) = &self.worker {
            let handle = worker.handle();
            let _ = match self.screen {
                Screen::Inbox => handle.sync_inbox(),
                Screen::Board => handle.sync_board(self.options.board),
                Screen::Task => handle.sync_task(self.options.task),
            };
        }
    }

    fn reload(&mut self) {
        let result: anyhow::Result<()> = (|| {
            self.inbox = self.db.inbox()?;
            match self.screen {
                Screen::Inbox => {
                    self.splits = vec!["Important".into(), "Agents".into()];
                    for n in &self.inbox {
                        if let Some(title) = n
                            .raw_json
                            .pointer("/project/title")
                            .and_then(|v| v.as_str())
                        {
                            if !self.splits.iter().any(|s| s == title) {
                                self.splits.push(title.to_owned());
                            }
                        }
                    }
                    self.splits[2..].sort_unstable();
                    self.splits.push("All".into());
                    self.split = self.split.min(self.splits.len().saturating_sub(1));
                    self.filter_inbox();
                }
                Screen::Board => self.load_columns(self.options.board)?,
                Screen::Task => {
                    self.detail = self.db.task_detail(self.options.task)?;
                    if let Some(detail) = &self.detail {
                        self.description = RichText::parse(&detail.task.description, self.colors);
                        self.summary = task_summary(&detail.task.extra).into();
                        let creator = creator_name(&detail.task.extra);
                        self.creator_initials = creator
                            .chars()
                            .take(2)
                            .collect::<String>()
                            .to_uppercase()
                            .into();
                        self.creator = creator.into();
                        self.comments = detail
                            .comments
                            .iter()
                            .map(|c| RichText::parse(&c.text, self.colors))
                            .collect();
                        self.comment_anchors.resize_with(self.comments.len(), || {
                            ScrollAnchor::for_handle(self.detail_scroll.clone())
                        });
                        let project = detail.task.project_id;
                        self.load_columns(project)?;
                    }
                }
            }
            Ok(())
        })();
        if let Err(error) = result {
            self.error = Some(error.to_string());
        }
    }

    fn load_columns(&mut self, project: i64) -> anyhow::Result<()> {
        let columns = self.db.board(project)?;
        let lists = columns
            .iter()
            .map(|col| {
                self.columns
                    .iter()
                    .position(|old| old.id == col.id)
                    .filter(|&i| self.columns[i].cards == col.cards)
                    .map(|i| self.column_lists[i].clone())
                    .unwrap_or_else(|| {
                        ListState::new(col.cards.len(), ListAlignment::Top, px(160.))
                    })
            })
            .collect();
        self.columns = columns;
        self.column_lists = lists;
        self.column = self.column.min(self.columns.len().saturating_sub(1));
        self.clamp_selection();
        Ok(())
    }

    fn split_matches(n: &InboxItem, name: &str) -> bool {
        name == "All"
            || n.raw_json["computedSplit"].as_str() == Some(name)
            || n.raw_json
                .pointer("/project/title")
                .and_then(|v| v.as_str())
                == Some(name)
            || (name == "Important"
                && (n.raw_json["directReply"].as_bool() == Some(true)
                    || matches!(
                        n.raw_json["type"].as_str(),
                        Some("TaskReminder" | "TaskMention" | "CommentMention" | "TaskAssign")
                    )))
    }

    fn filter_inbox(&mut self) {
        self.filtered = self
            .inbox
            .iter()
            .enumerate()
            .filter(|(_, n)| Self::split_matches(n, &self.splits[self.split]))
            .map(|(i, _)| i)
            .collect();
        self.clamp_selection();
    }

    fn clamp_selection(&mut self) {
        let count = match self.screen {
            Screen::Inbox => self.filtered.len(),
            Screen::Board => self.columns.get(self.column).map_or(0, |c| c.cards.len()),
            Screen::Task => self.comments.len(),
        };
        self.selected = self.selected.min(count.saturating_sub(1));
    }

    fn enqueue(&mut self, write: Write, cx: &mut Context<Self>) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        match self.db.enqueue(write, now) {
            Ok(id) => {
                println!("QUEUED {id}");
                let _ = std::io::stdout().flush();
                self.error = None;
                self.reload();
                if let Some(worker) = &self.worker {
                    let _ = worker.handle().wake();
                }
                cx.notify();
                true
            }
            Err(error) => {
                self.error = Some(error.to_string());
                cx.notify();
                false
            }
        }
    }

    fn switch(&mut self, screen: Screen, cx: &mut Context<Self>) {
        if self.auto_board && screen == Screen::Board {
            Self::resolve_board(&self.db, &mut self.options);
            let _ = self
                .db
                .kv_set("last_board", &self.options.board.to_string());
        }
        self.screen = screen;
        self.selected = 0;
        self.editing = Editing::None;
        self.moving = false;
        self.reload();
        self.request_sync();
        cx.notify();
    }

    fn open_task(&mut self, id: i64, cx: &mut Context<Self>) {
        self.previous = self.screen;
        if self.options.task != id {
            self.comment_input
                .update(cx, |input, cx| input.set_content(String::new(), cx));
        }
        self.options.task = id;
        self.detail_scroll.set_offset(point(px(0.), px(0.)));
        self.switch(Screen::Task, cx);
    }

    fn selected_task(&self) -> Option<i64> {
        match self.screen {
            Screen::Inbox => self
                .filtered
                .get(self.selected)
                .and_then(|&i| self.inbox[i].task_id),
            Screen::Board => self
                .columns
                .get(self.column)?
                .cards
                .get(self.selected)
                .map(|t| t.id),
            Screen::Task => self.detail.as_ref().map(|d| d.task.id),
        }
    }

    fn edit_title(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(d) = &self.detail {
            self.title_input
                .update(cx, |input, cx| input.set_content(d.task.title.clone(), cx));
            self.editing = Editing::Title;
            window.focus(&self.title_input.focus_handle(cx), cx);
            cx.notify();
        }
    }

    fn edit_comment(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.editing = Editing::Comment;
        self.composer_anchor.scroll_to(window, cx);
        window.focus(&self.comment_input.focus_handle(cx), cx);
        cx.notify();
    }

    fn save_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(task_id) = self.selected_task() else {
            return;
        };
        let write = match self.editing {
            Editing::Title => Write::TaskTitle {
                task_id,
                title: self.title_input.read(cx).content.to_string(),
            },
            Editing::Comment => Write::TaskComment {
                task_id,
                text: self
                    .comment_input
                    .read(cx)
                    .content
                    .replace('&', "&amp;")
                    .replace('<', "&lt;")
                    .replace('>', "&gt;"),
            },
            Editing::None => return,
        };
        if self.enqueue(write, cx) {
            self.editing = Editing::None;
            self.comment_input
                .update(cx, |input, cx| input.reset(window, cx));
            window.focus(&self.focus, cx);
        }
    }

    fn key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let key = event.keystroke.key.as_str();
        let ctrl = event.keystroke.modifiers.control || event.keystroke.modifiers.platform;
        if self.editing != Editing::None {
            if key == "escape" {
                self.editing = Editing::None;
                window.focus(&self.focus, cx);
                cx.notify();
            } else if key == "enter" && (self.editing == Editing::Title || ctrl) {
                self.save_edit(window, cx);
            }
            return;
        }
        match key {
            "j" | "down" => {
                self.selected = self.selected.saturating_add(1);
                self.clamp_selection();
            }
            "k" | "up" => self.selected = self.selected.saturating_sub(1),
            "tab" => match self.screen {
                Screen::Inbox => {
                    let count = self.splits.len();
                    if count > 0 {
                        self.split = (self.split
                            + if event.keystroke.modifiers.shift {
                                count - 1
                            } else {
                                1
                            })
                            % count;
                        self.selected = 0;
                        self.filter_inbox();
                    }
                }
                Screen::Board => {
                    let count = self.columns.len();
                    if count > 0 {
                        self.column = (self.column
                            + if event.keystroke.modifiers.shift {
                                count - 1
                            } else {
                                1
                            })
                            % count;
                        self.selected = 0;
                        self.clamp_selection();
                        self.board_scroll.scroll_to_item(self.column);
                    }
                }
                Screen::Task => self.moving = !self.moving,
            },
            "enter" => {
                if self.screen != Screen::Task {
                    if let Some(id) = self.selected_task() {
                        self.open_task(id, cx);
                    }
                } else {
                    self.edit_title(window, cx);
                }
            }
            "escape" => {
                if self.moving {
                    self.moving = false;
                } else if self.screen == Screen::Task {
                    self.switch(self.previous, cx);
                }
            }
            "e" if self.screen == Screen::Inbox && !ctrl => {
                if let Some(&i) = self.filtered.get(self.selected) {
                    self.enqueue(
                        Write::InboxArchive {
                            notification_id: self.inbox[i].id,
                        },
                        cx,
                    );
                }
            }
            "m" if self.screen == Screen::Task => {
                if ctrl {
                    self.edit_comment(window, cx);
                } else {
                    self.moving = !self.moving;
                }
            }
            "d" if ctrl && self.screen == Screen::Task => {
                self.description_anchor.scroll_to(window, cx)
            }
            "h" | "l" if event.keystroke.modifiers.shift && self.screen == Screen::Board => {
                if let Some(task_id) = self.selected_task() {
                    let index = if key == "h" {
                        self.column.saturating_sub(1)
                    } else {
                        (self.column + 1).min(self.columns.len().saturating_sub(1))
                    };
                    if let Some(c) = self.columns.get(index) {
                        let id = c.id;
                        if self.enqueue(
                            Write::TaskSection {
                                task_id,
                                section_id: id,
                            },
                            cx,
                        ) {
                            self.column = index;
                        }
                    }
                }
            }
            _ => return,
        }
        if self.screen == Screen::Task && matches!(key, "j" | "k" | "up" | "down") {
            if let Some(anchor) = self.comment_anchors.get(self.selected) {
                anchor.scroll_to(window, cx);
            }
        }
        if self.screen == Screen::Inbox {
            self.inbox_scroll
                .scroll_to_item(self.selected, ScrollStrategy::Nearest);
        }
        if self.screen == Screen::Board {
            if let Some(list) = self.column_lists.get(self.column) {
                list.scroll_to_reveal_item(self.selected);
            }
        }
        cx.stop_propagation();
        cx.notify();
    }

    fn rail(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let t = self.colors;
        let button = |i: usize, name: &'static str, selected: bool| {
            div()
                .w(px(36.))
                .h(px(34.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(5.))
                .text_color(color(if selected { t.text } else { t.muted }))
                .when(selected, |d| d.bg(color(t.hover)))
                .child(icon(name, if selected { t.text } else { t.muted }, 18.))
                .layout_key(&self.layout, format!("rail.icon.{i}"), None)
        };
        div()
            .w(px(48.))
            .h_full()
            .flex_shrink_0()
            .bg(color(t.rail))
            .border_r_1()
            .border_color(color(t.rail_border))
            .flex()
            .flex_col()
            .items_center()
            .pt(px(8.))
            .pb(px(8.))
            .child(
                div()
                    .w(px(16.))
                    .h(px(20.))
                    .mb(px(4.))
                    .flex()
                    .items_center()
                    .text_color(color(t.muted))
                    .child(icon("chevron-right", t.muted, 16.))
                    .layout_key(&self.layout, "rail.icon.0", None),
            )
            .child(button(1, "panel-left", false))
            .child(div().h(px(16.)))
            .child(
                div()
                    .id("rail-inbox")
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, _, cx| this.switch(Screen::Inbox, cx)))
                    .child(button(2, "inbox", self.screen == Screen::Inbox)),
            )
            .child(div().h(px(4.)))
            .child(
                div()
                    .id("rail-board")
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, _, cx| this.switch(Screen::Board, cx)))
                    .child(button(3, "columns-2", self.screen == Screen::Board)),
            )
            .children(
                ["rows-3", "calendar", "sparkles", "search"]
                    .into_iter()
                    .enumerate()
                    .map(|(i, name)| div().mt(px(4.)).child(button(i + 4, name, false))),
            )
            .child(div().flex_1())
            .children(
                ["bot", "user-plus", "rocket", "message-circle", "settings"]
                    .into_iter()
                    .enumerate()
                    .map(|(i, name)| div().mb(px(4.)).child(button(i + 8, name, false))),
            )
            .layout_key(&self.layout, "rail", None)
    }

    fn footer(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let items: &[(&'static str, &'static str)] = match self.screen {
            Screen::Inbox => &[
                ("CTRL K", "Command center"),
                ("J", "Down"),
                ("K", "Up"),
                ("Enter", "Open task"),
                ("TAB", "Cycle inbox splits"),
                ("E", "Archive"),
                ("CTRL E", "Mark task as done"),
            ],
            Screen::Board => &[
                ("CTRL K", "Command center"),
                ("J", "Down"),
                ("K", "Up"),
                ("TAB", "Next column"),
                ("C", "Add task"),
                ("N / Shift+C", "Quick add"),
                ("CTRL E", "Archive"),
                ("CTRL B", "Change Board"),
                ("/", "Search"),
            ],
            Screen::Task => &[
                ("CTRL K", "Command center"),
                ("A", "Assign"),
                ("M", "Change status(move)"),
                ("P", "Priority"),
                ("S", "Task size"),
                ("T", "Tags"),
                ("CTRL E", "Archive"),
                ("ESC", "Back"),
                ("TAB", "Cycle"),
            ],
        };
        div()
            .text_size(px(14.))
            .line_height(px(21.))
            .h(px(37.))
            .w_full()
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(color(self.colors.footer))
            .relative()
            .overflow_hidden()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(if self.screen == Screen::Inbox {
                        16.
                    } else {
                        12.
                    }))
                    .mr(px(18.))
                    .whitespace_nowrap()
                    .flex_shrink_0()
                    .children(items.iter().enumerate().map(|(i, &(key, label))| {
                        div()
                            .flex()
                            .flex_shrink_0()
                            .items_center()
                            .gap(px(8.))
                            .relative()
                            .when(self.screen == Screen::Inbox && i > 0, |d| {
                                d.child(
                                    div()
                                        .absolute()
                                        .left(px(-10.))
                                        .top(px(8.5))
                                        .size(px(4.))
                                        .rounded_full()
                                        .bg(color(0x8e9093)),
                                )
                            })
                            .child(
                                div().flex().items_center().gap(px(4.)).children(
                                    key.split(if key.starts_with("CTRL ") { " " } else { "\0" })
                                        .map(|key| {
                                            div()
                                                .px(px(6.25))
                                                .h(px(20.))
                                                .flex()
                                                .items_center()
                                                .justify_center()
                                                .rounded(px(2.))
                                                .font_family("Liberation Mono")
                                                .text_size(px(12.))
                                                .line_height(px(14.))
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .bg(color(
                                                    if matches!(
                                                        self.options.theme,
                                                        ThemeName::Graphite | ThemeName::Amoled
                                                    ) {
                                                        0x95999e
                                                    } else {
                                                        0x555b64
                                                    },
                                                ))
                                                .text_color(color(0xffffff))
                                                .child(key)
                                        }),
                                ),
                            )
                            .child(label)
                    }))
                    .layout_key(
                        &self.layout,
                        "footer.text",
                        Some(
                            items
                                .iter()
                                .map(|(key, label)| format!("{key} {label}"))
                                .collect::<Vec<_>>()
                                .join(" "),
                        ),
                    ),
            )
            .child(
                div()
                    .absolute()
                    .left_0()
                    .top_0()
                    .h_full()
                    .w(px(8.))
                    .bg(color(self.colors.footer)),
            )
            .child(
                div()
                    .absolute()
                    .left(px(8.))
                    .top_0()
                    .h_full()
                    .w(px(32.))
                    .bg(gpui::linear_gradient(
                        90.,
                        gpui::linear_color_stop(color(self.colors.footer), 0.),
                        gpui::linear_color_stop(color(self.colors.footer).opacity(0.), 1.),
                    )),
            )
            .child(
                div()
                    .absolute()
                    .right(px(26.))
                    .top_0()
                    .h_full()
                    .w(px(32.))
                    .bg(gpui::linear_gradient(
                        90.,
                        gpui::linear_color_stop(color(self.colors.footer).opacity(0.), 0.),
                        gpui::linear_color_stop(color(self.colors.footer), 1.),
                    )),
            )
            .child(
                div()
                    .absolute()
                    .right_0()
                    .top_0()
                    .h_full()
                    .w(px(26.))
                    .bg(color(self.colors.footer)),
            )
            .child(
                div()
                    .id("close-tips")
                    .absolute()
                    .right(px(8.))
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.footer = false;
                        cx.notify();
                    }))
                    .child(icon("x", self.colors.text, 18.)),
            )
            .layout_key(&self.layout, "footer", None)
    }

    fn inbox_screen(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let t = self.colors;
        let tabs = self.splits.iter().enumerate().map(|(i, name)| {
            let count = self
                .inbox
                .iter()
                .filter(|n| Self::split_matches(n, name))
                .count();
            div()
                .id(("split", i))
                .cursor_pointer()
                .flex()
                .h(px(32.))
                .text_size(px(14.))
                .pr(px(15.))
                .items_center()
                .gap(px(4.))
                .font_weight(FontWeight::NORMAL)
                .text_color(color(t.text))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.split = i;
                    this.selected = 0;
                    this.filter_inbox();
                    cx.notify();
                }))
                .child(
                    div()
                        .flex()
                        .items_baseline()
                        .gap(px(4.))
                        .font_weight(if i == self.split {
                            FontWeight::SEMIBOLD
                        } else {
                            FontWeight::NORMAL
                        })
                        .text_color(color(if i == self.split { 0xf3f4f6 } else { t.muted }))
                        .child(name.clone())
                        .child(
                            div()
                                .text_size(px(11.))
                                .font_weight(FontWeight::NORMAL)
                                .text_color(color(
                                    if matches!(
                                        self.options.theme,
                                        ThemeName::Graphite | ThemeName::Amoled
                                    ) {
                                        0xd1d5dc
                                    } else {
                                        0x4b5563
                                    },
                                ))
                                .child(count.to_string()),
                        ),
                )
                .when(i + 1 == self.splits.len(), |d| {
                    d.child(div().px(px(4.)).child(icon("settings", t.muted, 14.)))
                })
                .layout_key(
                    &self.layout,
                    format!("inbox.tab.{i}"),
                    Some(format!("{name} {count}")),
                )
        });
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .bg(color(t.inbox_page))
            .items_center()
            .child(
                div()
                    .w_full()
                    .max_w(px(1440.))
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .h(px(80.))
                            .flex_shrink_0()
                            .mx(px(12.))
                            .pl(px(52.))
                            .pr(px(20.))
                            .pt(px(24.))
                            .pb(px(24.))
                            .flex()
                            .gap(px(9.))
                            .child(
                                div()
                                    .flex_1()
                                    .h(px(32.))
                                    .flex()
                                    .items_center()
                                    .gap(px(9.))
                                    .children(tabs)
                                    .layout_key(&self.layout, "inbox.tabs", None),
                            )
                            .child(div()),
                    )
                    .child(
                        div()
                            .mx(px(12.))
                            .mb(px(12.))
                            .flex_1()
                            .min_h_0()
                            .bg(color(t.inbox))
                            .rounded(px(6.))
                            .when(self.options.theme != ThemeName::Amoled, |d| {
                                d.shadow(vec![
                                    gpui::BoxShadow {
                                        inset: false,
                                        color: rgba(0x0000001a).into(),
                                        offset: point(px(0.), px(4.)),
                                        blur_radius: px(6.),
                                        spread_radius: px(-1.),
                                    },
                                    gpui::BoxShadow {
                                        inset: false,
                                        color: rgba(0x0000001a).into(),
                                        offset: point(px(0.), px(2.)),
                                        blur_radius: px(4.),
                                        spread_radius: px(-2.),
                                    },
                                ])
                            })
                            .flex()
                            .flex_col()
                            .overflow_hidden()
                            .child(
                                div()
                                    .h(px(46.))
                                    .flex_shrink_0()
                                    .pl(px(52.))
                                    .pt(px(24.))
                                    .text_size(px(12.))
                                    .line_height(px(14.4))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(color(0x95999e))
                                    .when(self.options.theme == ThemeName::Dia, |d| {
                                        d.font_family("Newsreader").italic()
                                    })
                                    .child(
                                        div().flex().child(
                                            div()
                                                .child(inbox_date(
                                                    self.filtered.first().map(|&i| {
                                                        self.inbox[i].created_at.as_str()
                                                    }),
                                                ))
                                                .layout_key(
                                                    &self.layout,
                                                    "inbox.section",
                                                    Some(inbox_date(self.filtered.first().map(
                                                        |&i| self.inbox[i].created_at.as_str(),
                                                    ))),
                                                ),
                                        ),
                                    ),
                            )
                            .when(self.filtered.is_empty(), |d| {
                                d.child(
                                    div()
                                        .flex_1()
                                        .flex()
                                        .flex_col()
                                        .items_center()
                                        .justify_center()
                                        .gap(px(12.))
                                        .text_color(color(t.muted))
                                        .child(icon("inbox", t.muted, 32.))
                                        .child("You're all caught up"),
                                )
                            })
                            .child(
                                uniform_list(
                                    "notifications",
                                    self.filtered.len(),
                                    cx.processor(|this, range: std::ops::Range<usize>, _, cx| {
                                        range.map(|i| this.inbox_row(i, cx)).collect::<Vec<_>>()
                                    }),
                                )
                                .track_scroll(&self.inbox_scroll)
                                .flex_1()
                                .min_h_0(),
                            ),
                    ),
            )
    }

    fn inbox_row(&self, row: usize, cx: &mut Context<Self>) -> AnyElement {
        let n = &self.inbox[self.filtered[row]];
        let t = self.colors;
        let id = n.id;
        let kind = n.raw_json.pointer("/comment/text").and_then(|v| v.as_str())
            .map(|html| RichText::parse(html, t).text.to_string())
            .filter(|text| !text.trim().is_empty())
            .unwrap_or_else(|| notification_label(n.raw_json["type"].as_str().unwrap_or("")));
        let source = n.source.clone().unwrap_or_default();
        let source = match n.raw_json["unreadCount"].as_u64().filter(|count| *count > 0) {
            Some(count) => format!("{source} ({count})"),
            None => source,
        };
        div()
            .id(("notification", id as u64))
            .w_full()
            .h(px(36.))
            .flex()
            .items_center()
            .border_l(px(4.))
            .border_color(gpui::transparent_black())
            .pl(px(48.))
            .pr(px(48.))
            .cursor_pointer()
            .when(row == self.selected, |d| {
                d.border_color(color(if self.options.theme == ThemeName::Porcelain {
                    0x9a9daa
                } else {
                    t.accent
                }))
                .bg(color(if self.options.theme == ThemeName::Porcelain {
                    0xe2e2e6
                } else if self.options.theme == ThemeName::Dia {
                    t.active
                } else {
                    t.footer
                }))
            })
            .hover(|d| d.bg(color(t.hover)))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.selected = row;
                if let Some(id) = this.selected_task() {
                    this.open_task(id, cx);
                }
            }))
            .child(
                div()
                    .w_full()
                    .h(px(36.))
                    .flex()
                    .items_center()
                    .gap(px(32.))
                    .child(
                        div()
                            .text_size(px(13.))
                            .line_height(px(19.5))
                            .w(gpui::relative(0.15))
                            .min_w_0()
                            .flex_shrink_0()
                            .flex()
                            .items_center()
                            .relative()
                            .child(
                                div()
                                    .absolute()
                                    .left(px(-8.))
                                    .size(px(7.))
                                    .rounded_full()
                                    .when(!n.seen, |d| d.bg(color(t.accent))),
                            )
                            .child(
                                div()
                                    .truncate()
                                    .child(source.clone())
                                    .layout_key(
                                        &self.layout,
                                        format!("inbox.row.{row}.author"),
                                        Some(
                                            source.clone(),
                                        ),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .line_height(px(19.5))
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .overflow_hidden()
                            .child(
                                div()
                                    .flex()
                                    .items_baseline()
                                    .gap(px(4.))
                                    .min_w(px(180.))
                                    .child(
                                        div()
                                            .flex()
                                            .flex_shrink_0()
                                            .text_color(color(0x999a9d))
                                            .text_size(px(11.))
                                            .line_height(px(16.5))
                                            .child(n.ticket_number.clone().unwrap_or_default())
                                            .child(" ")
                                            .layout_key(
                                                &self.layout,
                                                format!("inbox.row.{row}.taskid"),
                                                n.ticket_number.clone(),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .min_w_0()
                                            .truncate()
                                            .child(n.title.clone())
                                            .layout_key(
                                                &self.layout,
                                                format!("inbox.row.{row}.title"),
                                                Some(n.title.clone()),
                                            ),
                                    ),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w(px(90.))
                                    .text_color(color(0x8e9093))
                                    .truncate()
                                    .child(kind.clone())
                                    .layout_key(
                                        &self.layout,
                                        format!("inbox.row.{row}.kind"),
                                        Some(kind),
                                    ),
                            ),
                    )
                    .child(div().text_size(px(11.)).text_color(color(t.muted)).child(inbox_metadata(&n.raw_json["task"])))
                    .child(
                        div()
                            .w(px(62.))
                            .flex_shrink_0()
                            .flex()
                            .items_center()
                            .justify_end()
                            .gap(px(10.))
                            .pr(px(4.))
                            .child(
                                div()
                                    .id(("archive-notification", id as u64))
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        cx.stop_propagation();
                                        this.enqueue(
                                            Write::InboxArchive {
                                                notification_id: id,
                                            },
                                            cx,
                                        );
                                    }))
                                    .child(icon("archive-x", 0x696b6e, 18.)),
                            )
                            .child(icon("clock", 0x696b6e, 18.))
                            .layout_key(&self.layout, format!("inbox.row.{row}.actions"), None),
                    )
                    .layout_key(&self.layout, format!("inbox.row.{row}"), None),
            )
            .into_any_element()
    }

    fn board_screen(&self, window: &Window, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let t = self.colors;
        let title = self
            .columns
            .iter()
            .flat_map(|c| &c.cards)
            .find(|t| !t.board_title.is_empty())
            .map(|t| t.board_title.as_str())
            .unwrap_or("Board");
        let count: usize = self.columns.iter().map(|c| c.cards.len()).sum();
        let board_width =
            (f32::from(window.viewport_size().width) - 48. - ai_chat_panel_width()).max(0.);
        let column_width = ((board_width * 0.97
            - 16. * self.columns.len().saturating_sub(1) as f32)
            / self.columns.len().max(1) as f32)
            .clamp(280., 535.);
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .bg(color(t.page))
            .child(
                div()
                    .h(px(56.))
                    .pt(px(24.))
                    .flex_shrink_0()
                    .pl(px(board_width * 0.015 + 10.))
                    .pr(px(board_width * 0.015))
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .child(icon("house", t.text, 13.))
                    .child(
                        div()
                            .text_size(px(14.))
                            .line_height(px(21.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(title.to_owned())
                            .layout_key(&self.layout, "board.title", Some(title.to_owned())),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(color(
                                if matches!(
                                    self.options.theme,
                                    ThemeName::Graphite | ThemeName::Amoled
                                ) {
                                    0xd1d5dc
                                } else {
                                    0x4b5563
                                },
                            ))
                            .child(count.to_string())
                            .layout_key(&self.layout, "board.count", Some(count.to_string())),
                    )
                    .child(div().flex_1())
                    .child(
                        div()
                            .flex()
                            .gap(px(12.))
                            .children(["filter", "arrow-up-down", "ellipsis"].into_iter().map(
                                |name| {
                                    div()
                                        .size(px(32.))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .child(icon(name, t.muted, 18.))
                                },
                            ))
                            .layout_key(&self.layout, "board.toolbar", None),
                    )
                    .layout_key(&self.layout, "board.header", None),
            )
            .child(
                div()
                    .id("board-columns")
                    .track_scroll(&self.board_scroll)
                    .overflow_x_scroll()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .gap(px(16.))
                    .px(px(board_width * 0.015))
                    .pt(px(16.))
                    .pb(px(35.))
                    .children(self.columns.iter().enumerate().map(|(i, c)| {
                        div()
                            .w(px(column_width))
                            .h_full()
                            .flex_shrink_0()
                            .rounded(px(6.))
                            .bg(color(t.container))
                            .border_1()
                            .border_color(color(t.column_border))
                            .shadow(vec![
                                gpui::BoxShadow {
                                    inset: false,
                                    color: rgba(0x0000001a).into(),
                                    offset: point(px(0.), px(4.)),
                                    blur_radius: px(3.),
                                    spread_radius: px(-1.),
                                },
                                gpui::BoxShadow {
                                    inset: false,
                                    color: rgba(0x0000001a).into(),
                                    offset: point(px(0.), px(2.)),
                                    blur_radius: px(2.),
                                    spread_radius: px(-2.),
                                },
                            ])
                            .flex()
                            .flex_col()
                            .overflow_hidden()
                            .child(
                                div()
                                    .h(px(if self.options.theme == ThemeName::Dia {
                                        37.
                                    } else {
                                        36.
                                    }))
                                    .flex_shrink_0()
                                    .px(px(8.))
                                    .flex()
                                    .items_center()
                                    .gap(px(6.))
                                    .text_size(px(13.))
                                    .line_height(px(19.5))
                                    .child(
                                        div()
                                            .when(self.options.theme == ThemeName::Dia, |d| {
                                                d.font_family("Newsreader")
                                                    .italic()
                                                    .text_size(px(14.))
                                                    .line_height(px(21.))
                                            })
                                            .child(c.title.clone())
                                            .layout_key(
                                                &self.layout,
                                                format!("board.column.{i}.title"),
                                                Some(c.title.clone()),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .text_color(color(
                                                if self.options.theme == ThemeName::Dia {
                                                    t.accent
                                                } else {
                                                    t.muted
                                                },
                                            ))
                                            .text_size(px(12.))
                                            .line_height(px(18.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .mt(px(2.))
                                            .when(!c.cards.is_empty(), |d| {
                                                d.child(c.cards.len().to_string())
                                            })
                                            .layout_key(
                                                &self.layout,
                                                format!("board.column.{i}.count"),
                                                Some(if c.cards.is_empty() {
                                                    String::new()
                                                } else {
                                                    c.cards.len().to_string()
                                                }),
                                            ),
                                    )
                                    .child(div().flex_1())
                                    .child(
                                        div()
                                            .size(px(10.))
                                            .flex_shrink_0()
                                            .relative()
                                            .child(
                                                div()
                                                    .absolute()
                                                    .left(px(1.))
                                                    .top(px(4.))
                                                    .w(px(8.))
                                                    .h(px(2.))
                                                    .bg(color(t.text)),
                                            )
                                            .child(
                                                div()
                                                    .absolute()
                                                    .left(px(4.))
                                                    .top(px(1.))
                                                    .w(px(2.))
                                                    .h(px(8.))
                                                    .bg(color(t.text)),
                                            )
                                            .layout_key(
                                                &self.layout,
                                                format!("board.column.{i}.add"),
                                                None,
                                            ),
                                    ),
                            )
                            .child(
                                list(
                                    self.column_lists[i].clone(),
                                    cx.processor(move |this, row: usize, _, cx| {
                                        this.board_card(i, row, cx).into_any_element()
                                    }),
                                )
                                .flex_1()
                                .min_h_0(),
                            )
                            .layout_key(&self.layout, format!("board.column.{i}"), None)
                    })),
            )
    }

    fn board_card(&self, column: usize, row: usize, cx: &mut Context<Self>) -> AnyElement {
        let index = self.columns[..column]
            .iter()
            .map(|c| c.cards.len())
            .sum::<usize>()
            + row;
        let task = &self.columns[column].cards[row];
        let t = self.colors;
        let active = self.column == column && self.selected == row;
        let title = task.title.clone();
        let id = task.id;
        let labels = task
            .extra
            .get("labels")
            .and_then(|v| v.as_array())
            .filter(|v| !v.is_empty());
        div()
            .id(("task-card", id as u64))
            .px(px(8.))
            .pb(px(16.))
            .child(
                div()
                    .id("card")
                    .bg(color(if active { t.active } else { t.card }))
                    .rounded(px(if self.options.theme == ThemeName::Dia {
                        10.
                    } else {
                        5.
                    }))
                    .border_1()
                    .border_color(color(if active && self.options.theme == ThemeName::Dia {
                        t.accent
                    } else if matches!(self.options.theme, ThemeName::Porcelain | ThemeName::Dia) {
                        t.border
                    } else {
                        if active { t.text } else { t.card }
                    }))
                    .when(
                        !active
                            && matches!(
                                self.options.theme,
                                ThemeName::Graphite | ThemeName::Amoled
                            ),
                        |d| d.border_color(rgba(0x00000000)),
                    )
                    .when(
                        matches!(self.options.theme, ThemeName::Graphite | ThemeName::Amoled),
                        |d| d.border_0().border_l_4(),
                    )
                    .shadow(if self.options.theme == ThemeName::Dia {
                        vec![gpui::BoxShadow {
                            inset: false,
                            color: rgba(0x1f1d1a0d).into(),
                            offset: point(px(0.), px(1.)),
                            blur_radius: px(1.5),
                            spread_radius: px(0.),
                        }]
                    } else {
                        vec![
                            gpui::BoxShadow {
                                inset: false,
                                color: rgba(0x0000001a).into(),
                                offset: point(px(0.), px(4.)),
                                blur_radius: px(3.),
                                spread_radius: px(-1.),
                            },
                            gpui::BoxShadow {
                                inset: false,
                                color: rgba(0x0000001a).into(),
                                offset: point(px(0.), px(2.)),
                                blur_radius: px(2.),
                                spread_radius: px(-2.),
                            },
                        ]
                    })
                    .p(px(8.))
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .cursor_pointer()
                    .hover(|d| {
                        d.bg(color(if active {
                            t.hover
                        } else {
                            match self.options.theme {
                                ThemeName::Porcelain => 0xe2e2e6,
                                ThemeName::Graphite => 0x2c3037,
                                ThemeName::Amoled => 0x232323,
                                ThemeName::Dia => 0xdeede2,
                            }
                        }))
                    })
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.column = column;
                        this.selected = row;
                        this.open_task(id, cx);
                    }))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(4.))
                            .min_h(px(16.))
                            .text_size(px(if self.options.theme == ThemeName::Dia {
                                12.
                            } else {
                                11.
                            }))
                            .line_height(px(if self.options.theme == ThemeName::Dia {
                                18.
                            } else {
                                16.5
                            }))
                            .font_weight(if self.options.theme == ThemeName::Dia {
                                FontWeight::MEDIUM
                            } else {
                                FontWeight::NORMAL
                            })
                            .text_color(color(if self.options.theme == ThemeName::Dia {
                                t.accent
                            } else {
                                t.muted
                            }))
                            .child(
                                div()
                                    .child(task.ticket_number.clone().unwrap_or_default())
                                    .layout_key(
                                        &self.layout,
                                        format!("board.card.{index}.id"),
                                        task.ticket_number.clone(),
                                    ),
                            )
                            .child(div().flex_1())
                            .when(self.inbox.iter().any(|n| n.task_id == Some(id)), |d| {
                                d.child(
                                    div()
                                        .px(px(6.))
                                        .py(px(1.))
                                        .h(px(22.))
                                        .border_1()
                                        .border_color(rgba(0x0000001a))
                                        .rounded(px(4.))
                                        .bg(color(match self.options.theme {
                                            ThemeName::Graphite => 0x262a30,
                                            ThemeName::Amoled => 0x0c0c0c,
                                            _ => t.page,
                                        }))
                                        .text_size(px(12.))
                                        .line_height(px(18.))
                                        .text_color(color(
                                            if self.options.theme == ThemeName::Amoled {
                                                0xffffff
                                            } else {
                                                t.text
                                            },
                                        ))
                                        .font_weight(FontWeight::BOLD)
                                        .child("In inbox")
                                        .layout_key(
                                            &self.layout,
                                            format!("board.card.{index}.badge"),
                                            Some("In inbox".into()),
                                        ),
                                )
                            }),
                    )
                    .child(
                        div().flex().child(
                            div()
                                .text_size(px(13.))
                                .line_height(px(19.5))
                                .child(title.clone())
                                .layout_key(
                                    &self.layout,
                                    format!("board.card.{index}.title"),
                                    Some(title),
                                ),
                        ),
                    )
                    .when(priority_name(&task.extra) != "No Priority" || task.due_date.is_some(), |d| {
                        d.child(div().text_size(px(11.)).text_color(color(t.muted)).child(
                            [Some(priority_name(&task.extra)).filter(|p| *p != "No Priority").map(str::to_owned),
                             task.due_date.as_deref().map(short_date)]
                                .into_iter().flatten().collect::<Vec<_>>().join(" · ")
                        ))
                    })
                    .when(task.extra.get("hasBoardDraft").and_then(|v| v.as_bool()) == Some(true), |d| {
                        d.child(div().text_size(px(11.)).text_color(color(t.accent)).child("Draft"))
                    })
                    .when(task.comment_count > 0 || task.assignee_count > 0, |d| {
                        d.child(
                            div()
                                .flex()
                                .gap(px(12.))
                                .text_color(color(t.muted))
                                .text_size(px(11.))
                                .when(task.comment_count > 0, |d| {
                                    d.child(
                                        div()
                                            .flex()
                                            .gap(px(4.))
                                            .items_center()
                                            .child(icon("message-square", t.muted, 12.))
                                            .child(task.comment_count.to_string()),
                                    )
                                })
                                .when(task.assignee_count > 0, |d| {
                                    d.child(
                                        div()
                                            .flex()
                                            .gap(px(4.))
                                            .items_center()
                                            .child(icon("user-round", t.muted, 12.))
                                            .child(task.assignee_count.to_string()),
                                    )
                                })
                                .layout_key(
                                    &self.layout,
                                    format!("board.card.{index}.meta"),
                                    Some(
                                        [task.comment_count, task.assignee_count]
                                            .into_iter()
                                            .filter(|count| *count > 0)
                                            .map(|count| count.to_string())
                                            .collect::<Vec<_>>()
                                            .join(" "),
                                    ),
                                ),
                        )
                    })
                    .when_some(labels, |d, labels| {
                        d.child(
                            div().flex().flex_wrap().gap(px(4.)).children(
                                labels
                                    .iter()
                                    .filter_map(|label| {
                                        label["name"].as_str().or_else(|| label["value"].as_str())
                                    })
                                    .enumerate()
                                    .map(|(i, label)| {
                                        div()
                                            .px(px(4.))
                                            .rounded(px(3.))
                                            .bg(color(t.footer))
                                            .text_size(px(11.))
                                            .text_color(color(t.muted))
                                            .child(label.to_owned())
                                            .layout_key(
                                                &self.layout,
                                                if i == 0 {
                                                    format!("board.card.{index}.label")
                                                } else {
                                                    format!("board.card.{index}.label.{i}")
                                                },
                                                Some(label.to_owned()),
                                            )
                                    }),
                            ),
                        )
                    })
                    .layout_key(&self.layout, format!("board.card.{index}"), None),
            )
            .into_any_element()
    }

    fn task_screen(&self, cx: &mut Context<Self>) -> AnyElement {
        let t = self.colors;
        let Some(d) = &self.detail else {
            return div()
                .flex_1()
                .p(px(64.))
                .child("Task is not available offline. Sync this task first.")
                .into_any_element();
        };
        let title = if self.editing == Editing::Title {
            div()
                .flex_1()
                .min_w_0()
                .child(self.title_input.clone())
                .into_any_element()
        } else {
            div()
                .id("task-title")
                .flex_1()
                .min_w_0()
                .whitespace_normal()
                .cursor_text()
                .on_click(cx.listener(|this, _, window, cx| this.edit_title(window, cx)))
                .child(d.task.title.clone())
                .layout_key(&self.layout, "task.title", Some(d.task.title.clone()))
        };
        let properties = self.properties(&d.task, cx);
        let placeholder = 0xaeb4bc;
        let option_color = if self.options.theme == ThemeName::Dia {
            0x6f6b63
        } else {
            0x696b6e
        };
        let avatar_color = match self.options.theme {
            ThemeName::Porcelain => 0xe2e2e6,
            ThemeName::Graphite => 0x3a3d45,
            ThemeName::Amoled => 0x303239,
            ThemeName::Dia => 0xc9e3d2,
        };
        let navigation_color = match self.options.theme {
            ThemeName::Graphite => 0x21242a,
            ThemeName::Amoled => 0x0c0c0c,
            _ => 0xffffff,
        };
        div().id("task-scroll").track_scroll(&self.detail_scroll).overflow_y_scroll().flex_1().min_h_0().bg(color(t.page)).flex().flex_col().items_center().relative()
            .child(div().id("task-back").absolute().left(px(23.)).top(px(51.)).cursor_pointer().on_click(cx.listener(|this, _, _, cx| this.switch(this.previous, cx))).child(icon("arrow-left", t.muted, 18.)))
            .child(div().absolute().left(px(12.)).top(px(88.)).w(px(40.)).h(px(68.)).rounded_full().bg(color(navigation_color)).shadow(shadow()).flex().flex_col().justify_center().items_center().gap(px(10.)).text_color(color(t.muted)).children(["chevron-up", "chevron-down"].into_iter().map(|name| div().opacity(0.4).child(icon(name, t.muted, 18.)))))
            .child(div().w_full().max_w(px(1136.)).min_h_full().bg(color(t.detail)).flex().flex_col()
            .child(div().flex_shrink_0().pt(px(36.)).pl(px(82.)).pr(px(64.)).pb(px(8.)).relative()

                .child(div().w_full().flex().items_start().gap(px(8.)).font_weight(FontWeight::BOLD).text_size(px(24.)).line_height(px(36.)).min_h(px(37.)).child(title)
                    .child(div().flex_shrink_0().flex().items_center().gap(px(16.)).pt(px(10.25)).text_size(px(14.)).line_height(px(21.)).text_color(color(option_color))
                        .child(div().px(px(4.)).py(px(2.)).when(self.options.theme == ThemeName::Dia, |d| d.font_weight(FontWeight::MEDIUM)).child("Share")).child(icon("archive", option_color, 20.)).child(icon("circle-check", option_color, 20.)).child(icon("clock", option_color, 18.)).child(icon("layers", 0x696b6e, 16.))))
                .when(!self.summary.is_empty(), |d| d.child(div().pt(px(8.)).text_size(px(13.)).line_height(px(19.5)).font_weight(FontWeight::MEDIUM).text_color(color(t.muted)).child(self.summary.clone()))).layout_key(&self.layout, "task.header", None))
            .child(div().relative().flex().items_start().gap(px(16.)).mt(px(16.)).pl(px(64.)).pr(px(64.)).pb(px(32.)).flex_shrink_0()

                .child(div().flex_1().min_w(px(500.)).max_w(px(680.)).flex().flex_col().gap(px(16.))
                    .child(div().id("task-description").anchor_scroll(Some(self.description_anchor.clone())).min_h(px(207.)).pt(px(19.)).pb(px(23.)).pl(px(19.)).pr(px(15.)).bg(color(t.comment)).border_1().border_color(color(t.border)).rounded(px(if self.options.theme == ThemeName::Dia { 10. } else { 5. })).shadow_md().flex().flex_col()
                        .child(div().h(px(21.)).flex().items_start().gap(px(8.)).text_color(color(0x8e9093)).text_size(px(14.)).line_height(px(21.))
                            .child(div().size(px(18.)).rounded_full().bg(color(avatar_color)).text_color(color(t.text)).text_size(px(11.)).line_height(px(16.5)).font_weight(FontWeight::SEMIBOLD).flex().items_center().justify_center().child(self.creator_initials.clone()))
                            .child(self.creator.clone()).when(d.task.extra.get("hasDescriptionDraft").and_then(|v| v.as_bool()) == Some(true), |d| d.child("Draft")).child(div().flex_1()).child(div().text_size(px(12.)).line_height(px(18.)).mt(px(1.5)).text_color(color(t.muted)).child(short_date(d.task.extra.get("createdAt").and_then(|v| v.as_str()).unwrap_or("")))))
                        .child(div().pt(px(8.)).h(px(88.)).text_size(px(16.)).line_height(px(24.)).when(self.description.text.trim().is_empty(), |d| d.child(div().text_size(px(14.)).line_height(px(21.)).text_color(color(placeholder)).child(div().font_weight(FontWeight::SEMIBOLD).text_size(px(18.)).line_height(px(21.6)).when(self.options.theme == ThemeName::Dia, |d| d.font_family("Newsreader").italic()).child("Add Description")).child("Tip: Hit ENTER to edit mode or CTRL+D to jump here"))).when(!self.description.text.trim().is_empty(), |d| d.child(self.description.element())).layout_key(&self.layout, "task.description", Some(self.description.text.to_string())))
                        .child(div().flex_1().min_h_0())
                        .child(div().pl(px(10.)).flex().items_center().text_size(px(12.)).line_height(px(18.)).text_color(color(t.muted))
                            .child(div().flex().items_center().gap(px(4.)).child(icon("plus", t.muted, 8.)).child("Add sub-tasks"))
                            .child(div().ml(px(16.)).flex().items_center().gap(px(4.)).child(icon("plus", t.muted, 8.)).child("Add page"))
                            .child(div().flex_1()).child(icon("ai-sparkles", 0xc668ff, 12.)).child(div().ml(px(8.)).mr(px(10.)).child("Edit with AI"))))
                    .child(div().id("comment-composer").anchor_scroll(Some(self.composer_anchor.clone())).h(px(if self.options.theme == ThemeName::Porcelain { 152. } else { 154. })).pt(px(19.)).pb(px(if matches!(self.options.theme, ThemeName::Graphite | ThemeName::Amoled) { 27. } else { 25. })).pl(px(19.)).pr(px(16.)).rounded(px(if self.options.theme == ThemeName::Dia { 10. } else { 5. })).bg(color(t.comment)).border_1().border_color(color(t.border)).shadow_md().flex().flex_col()
                        .on_click(cx.listener(|this, _, window, cx| this.edit_comment(window, cx)))
                        .when(d.task.extra.get("hasCommentDraft").and_then(|v| v.as_bool()) == Some(true), |d| d.child(div().text_size(px(12.)).text_color(color(t.muted)).child("Draft")))
                        .child(div().text_size(px(14.)).line_height(px(21.)).child(self.comment_input.clone()))
                        .child(div().flex_1())
                        .child(div().flex().items_center().text_size(px(14.)).line_height(px(21.)).text_color(color(0x95999e)).font_weight(FontWeight::SEMIBOLD)
                            .child(div().flex().items_center().child("Send + ").child(icon("send-archive", 0x696b6e, 14.)))
                            .child(div().id("send-comment").ml(px(24.)).cursor_pointer().on_click(cx.listener(|this, _, window, cx| { cx.stop_propagation(); this.editing = Editing::Comment; this.save_edit(window, cx); })).child("Send"))
                            .child(div().flex_1()).child(div().text_size(px(16.)).font_weight(FontWeight::NORMAL).text_color(color(0xc668ff)).child("ai"))
                            .child(div().ml(px(8.)).child(icon("paperclip", 0x999a9d, 16.))).child(div().ml(px(10.)).child(icon("mic", 0x999a9d, 18.))).child(div().ml(px(3.)).child(icon("chevron-down", 0x999a9d, 14.)))).layout_key(&self.layout, "task.composer", None))
                    .children(d.comments.iter().zip(&self.comments).enumerate().map(|(i, (comment, body))| {
                        div().id(("comment", i)).anchor_scroll(Some(self.comment_anchors[i].clone())).p(px(20.)).bg(color(t.comment)).border_1().border_color(color(t.border)).rounded(px(if self.options.theme == ThemeName::Dia { 10. } else { 5. })).shadow(shadow()).flex().flex_col().gap(px(12.))
                            .child(div().flex().text_color(color(t.muted)).child(div().child(comment_author(&comment.extra)).layout_key(&self.layout, format!("task.comment.{i}.author"), Some(comment_author(&comment.extra)))).child(div().flex_1()).child(short_date(&comment.created_at)))
                            .child(div().line_height(px(21.)).child(body.element()).layout_key(&self.layout, format!("task.comment.{i}.body"), Some(body.text.to_string())))
                            .layout_key(&self.layout, format!("task.comment.{i}"), None)
                    })))
                .child(properties)))
            .into_any_element()
    }

    fn properties(&self, task: &TaskView, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let t = self.colors;
        let assignees = task
            .extra
            .get("assignees")
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter(|v| v["agent"].is_null())
                    .filter_map(|v| v["displayName"].as_str().or_else(|| v["email"].as_str()))
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "No assignees".into());
        let rows = [
            ("Assignees", assignees),
            (
                "Agents",
                task.extra
                    .get("assignees")
                    .and_then(|v| v.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| {
                                v.pointer("/agent/displayName").and_then(|v| v.as_str())
                            })
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| "No agents".into()),
            ),
            (
                "ID",
                task.ticket_number
                    .clone()
                    .unwrap_or_else(|| task.id.to_string()),
            ),
            ("Status", task.section.clone()),
            (
                "Due date",
                task.due_date
                    .as_deref()
                    .map(short_date)
                    .unwrap_or_else(|| "-".into()),
            ),
            ("Project", task.board_title.clone()),
            (
                "Priority",
                priority_name(&task.extra).into(),
            ),
            (
                "Task size",
                task.extra
                    .get("estimate")
                    .and_then(|v| v.get("estimate_value"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("-")
                    .into(),
            ),
            (
                "Tags",
                task.extra
                    .get("labels")
                    .and_then(|v| v.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v["name"].as_str().or_else(|| v["value"].as_str()))
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| "No tags".into()),
            ),
        ];
        let row_count = rows.len();
        div()
            .w(px(312.))
            .flex_shrink_0()
            .rounded(px(if self.options.theme == ThemeName::Dia {
                10.
            } else {
                5.
            }))
            .bg(color(t.comment))
            .border_1()
            .border_color(color(t.border))
            .map(|d| {
                if self.options.theme == ThemeName::Dia {
                    d.shadow(vec![gpui::BoxShadow {
                        inset: false,
                        color: rgba(0x1f1d1a0a).into(),
                        offset: point(px(0.), px(1.)),
                        blur_radius: px(3.),
                        spread_radius: px(0.),
                    }])
                } else {
                    d.shadow_md()
                }
            })
            .px(px(16.))
            .py(px(10.))
            .text_size(px(14.))
            .text_color(color(t.muted))
            .font_weight(FontWeight::MEDIUM)
            .line_height(px(21.))
            .flex()
            .flex_col()
            .children(rows.into_iter().enumerate().map(|(i, (label, value))| {
                div()
                    .id(SharedString::from(format!("property-{label}")))
                    .min_h(px(
                        if matches!(label, "Agents" | "ID" | "Task size" | "Tags") {
                            27.
                        } else {
                            24.
                        },
                    ))
                    .mb(px(if i + 1 < row_count { 24. } else { 0. }))
                    .flex()
                    .items_start()
                    .gap(px(20.))
                    .when(label == "Status", |d| {
                        d.cursor_pointer().on_click(cx.listener(|this, _, _, cx| {
                            this.moving = !this.moving;
                            cx.notify();
                        }))
                    })
                    .child(
                        div()
                            .w(px(260. / 3.))
                            .flex_shrink_0()
                            .text_color(color(0x8e9093))
                            .child(label.to_owned()),
                    )
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .items_center()
                            .gap(px(if label == "Project" { 6. } else { 10. }))
                            .when(label == "Assignees" && value == "No assignees", |d| {
                                d.text_color(color(0x8e9093))
                            })
                            .when(label == "ID" && self.options.theme == ThemeName::Dia, |d| {
                                d.text_color(color(0x3e6b4f))
                            })
                            .child(div().child(value.clone()).layout_key(
                                &self.layout,
                                if label == "ID" {
                                    "task.id".to_owned()
                                } else {
                                    format!("task.property.{i}.value")
                                },
                                Some(value),
                            ))
                            .when(label == "Status", |d| {
                                d.child(
                                    svg()
                                        .path("arrow-right")
                                        .size(px(11.))
                                        .text_color(color(0x8e9093))
                                        .flex_shrink_0()
                                        .mb(px(2.)),
                                )
                            })
                            .when(label == "Project", |d| {
                                d.child(
                                    svg()
                                        .path("corner-left-up")
                                        .size(px(11.))
                                        .text_color(color(0x8e9093))
                                        .flex_shrink_0()
                                        .mb(px(2.)),
                                )
                            }),
                    )
                    .layout_key(&self.layout, format!("task.meta.{i}"), None)
            }))
            .when(self.moving, |d| {
                d.child(
                    div()
                        .border_t_1()
                        .border_color(color(t.border))
                        .p(px(8.))
                        .flex()
                        .flex_col()
                        .children(self.columns.iter().map(|column| {
                            let section_id = column.id;
                            let task_id = task.id;
                            div()
                                .id(("move-to", section_id as u64))
                                .p(px(8.))
                                .cursor_pointer()
                                .hover(|d| d.bg(color(t.hover)))
                                .child(column.title.clone())
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    if this.enqueue(
                                        Write::TaskSection {
                                            task_id,
                                            section_id,
                                        },
                                        cx,
                                    ) {
                                        this.moving = false;
                                    }
                                }))
                        })),
                )
            })
    }
}

impl Desktop {
    fn sign_in_panel(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let t = self.colors;
        let waiting = self.signin.phase == Phase::Waiting;
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .font_family(self.options.theme.font())
            .text_size(px(14.))
            .text_color(color(t.text))
            .bg(color(t.page))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(16.))
                    .child(
                        div()
                            .text_size(px(20.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Sign in to Hypertask"),
                    )
                    .child(
                        div()
                            .id("sign-in")
                            .px(px(16.))
                            .h(px(34.))
                            .flex()
                            .items_center()
                            .rounded(px(5.))
                            .border_1()
                            .border_color(color(t.border))
                            .bg(color(t.card))
                            .cursor_pointer()
                            .hover(|d| d.bg(color(t.hover)))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.signin.begin();
                                cx.notify();
                            }))
                            .child(if waiting {
                                "Waiting for your browser..."
                            } else {
                                "Sign in in your browser"
                            }),
                    )
                    .when_some(self.signin.message.clone(), |d, message| {
                        d.child(div().text_color(color(t.muted)).child(message))
                    }),
            )
    }
}

const AI_CHAT_PANEL_SHOWN: bool = false;
const AI_CHAT_PANEL_WIDTH: f32 = 420.;

fn ai_chat_panel_width() -> f32 {
    if AI_CHAT_PANEL_SHOWN { AI_CHAT_PANEL_WIDTH } else { 0. }
}

impl Render for Desktop {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(layout) = &self.layout {
            layout.borrow_mut().begin(self.split);
        }
        if !self.ready_scheduled {
            self.ready_scheduled = true;
            let exit = self.options.exit_after_ready;
            let bench = self.options.bench_input;
            ready_after_presentation(
                window,
                exit,
                bench,
                self.options.print_memory,
                self.options
                    .dump_layout
                    .clone()
                    .zip(self.layout.clone())
                    .map(|(path, capture)| (path, capture, self.screen, self.options.theme)),
            );
        }
        if self.signin.phase != Phase::Ready {
            return self.sign_in_panel(cx).into_any_element();
        }
        let body = match self.screen {
            Screen::Inbox => self.inbox_screen(cx).into_any_element(),
            Screen::Board => self.board_screen(window, cx).into_any_element(),
            Screen::Task => self.task_screen(cx),
        };
        let t = self.colors;
        div()
            .size_full()
            .flex()
            .font_family(self.options.theme.font())
            .text_size(px(16.))
            .line_height(gpui::relative(1.5))
            .text_color(color(t.text))
            .bg(color(t.page))
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::key_down))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .flex()
                    .child(self.rail(cx))
                    .child(
                        div()
                            .flex_1()
                            .h_full()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .relative()
                            .when_some(self.error.clone(), |d, error| {
                                d.child(div().p(px(8.)).text_color(color(0xcf222e)).child(error))
                            })
                            .child(body)
                            .when(self.footer, |d| d.child(self.footer(cx))),
                    )
                    .layout_key(&self.layout, "main", None),
            )
            .when(AI_CHAT_PANEL_SHOWN, |d| d.child(
                div()
                    .w(px(AI_CHAT_PANEL_WIDTH))
                    .flex_shrink_0()
                    .h_full()
                    .bg(color(t.detail))
                    .child(
                        div()
                            .size_full()
                            .border_l_1()
                            .border_color(color(t.rail_border)),
                    )
                    .layout_key(&self.layout, "rightpanel", None),
            ))
            .into_any_element()
    }
}

fn ready_after_presentation(
    window: &Window,
    exit: bool,
    bench: usize,
    print_memory: bool,
    dump: Option<(PathBuf, Rc<RefCell<LayoutDump>>, Screen, ThemeName)>,
) {
    window.on_next_frame(move |window, cx| {
        let presented = window
            .frame_duration_snapshot()
            .by_refresh_interval
            .iter()
            .any(|frames| {
                frames.frames_on_time + frames.frames_late + frames.frames_unmeasured > 0
            });
        if !presented {
            ready_after_presentation(window, exit, bench, print_memory, dump);
            return;
        }
        if let Some((path, capture, screen, theme)) = dump {
            if let Err(error) = capture
                .borrow()
                .write(&path, screen, theme, window.viewport_size())
            {
                eprintln!("Hypertask: layout dump failed: {error:#}");
                std::process::exit(1);
            }
        }
        println!("READY");
        if print_memory {
            if let Err(error) = crate::memory::print() {
                eprintln!("Hypertask: memory measurement failed: {error:#}");
            }
        }
        let _ = std::io::stdout().flush();
        if exit {
            cx.quit();
        } else if bench > 0 {
            bench_step(window, cx, bench, vec![]);
        }
    });
}

fn bench_step(window: &mut Window, cx: &mut App, total: usize, samples: Vec<f64>) {
    if samples.len() == total {
        let mut sorted = samples;
        sorted.sort_by(f64::total_cmp);
        let percentile =
            |p: f64| sorted[((sorted.len() as f64 * p).ceil() as usize).saturating_sub(1)];
        println!(
            "INPUT_TO_FRAME n={} p50={:.3} p95={:.3} max={:.3}",
            total,
            percentile(0.5),
            percentile(0.95),
            sorted[sorted.len() - 1]
        );
        let _ = std::io::stdout().flush();
        cx.quit();
        return;
    }
    let previous = window.input_latency_snapshot();
    let key = match samples.len() % 4 {
        0 => "j",
        2 => "k",
        _ => "tab",
    };
    window.dispatch_event(
        PlatformInput::KeyDown(KeyDownEvent {
            keystroke: Keystroke::parse(key).expect("benchmark key"),
            is_held: false,
            prefer_character_input: false,
        }),
        cx,
    );
    await_input_frame(window, previous, total, samples, 0);
}

fn await_input_frame(
    window: &Window,
    previous: gpui::profiler::InputLatencySnapshot,
    total: usize,
    mut samples: Vec<f64>,
    attempts: usize,
) {
    window.on_next_frame(move |window, cx| {
        let mut delta = window.input_latency_snapshot().latency_histogram;
        delta
            .subtract(&previous.latency_histogram)
            .expect("monotonic input samples");
        if delta.is_empty() && attempts < 120 {
            await_input_frame(window, previous, total, samples, attempts + 1);
        } else if delta.len() != 1 {
            eprintln!(
                "Benchmark expected one presented frame per input; observed {}",
                delta.len()
            );
            std::process::exit(1);
        } else {
            samples.push(delta.max() as f64 / 1_000_000.);
            bench_step(window, cx, total, samples);
        }
    });
}

fn notification_label(kind: &str) -> String {
    let mut out = String::new();
    for ch in kind.chars() {
        if ch.is_uppercase() && !out.is_empty() {
            out.push(' ');
        }
        out.push(ch);
    }
    out
}

fn short_date(value: &str) -> String {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|d| d.format("%-d %b").to_string())
        .unwrap_or_else(|_| value.to_owned())
}

fn inbox_date(value: Option<&str>) -> String {
    let Some(date) = value.and_then(|v| chrono::DateTime::parse_from_rfc3339(v).ok()) else {
        return String::new();
    };
    use chrono::Datelike;
    let today = chrono::Local::now().date_naive();
    let days = (today - date.with_timezone(&chrono::Local).date_naive()).num_days();
    let monday = today.weekday().num_days_from_monday() as i64;
    match days {
        ..=0 => "TODAY".into(),
        1 => "YESTERDAY".into(),
        _ if days <= monday => "THIS WEEK".into(),
        _ if days <= monday + 7 => "LAST WEEK".into(),
        _ => date.format("%B %Y").to_string().to_uppercase(),
    }
}

fn creator_name(extra: &serde_json::Map<String, serde_json::Value>) -> String {
    ["createdBy", "user", "creator"]
        .into_iter()
        .find_map(|key| extra.get(key).and_then(|v| v["displayName"].as_str().filter(|s| !s.is_empty()).or_else(|| v["email"].as_str())))
        .or_else(|| extra.get("creatorName").and_then(|v| v.as_str()))
        .unwrap_or("Unknown author")
        .to_owned()
}

fn priority_name(extra: &serde_json::Map<String, serde_json::Value>) -> &str {
    extra.get("priority").and_then(|v| v.as_str().or_else(|| v["Priority_Value"].as_str()))
        .unwrap_or("No Priority")
}

fn inbox_metadata(task: &serde_json::Value) -> String {
    let mut parts = Vec::new();
    if let Some(date) = task["dueDate"].as_str() {
        parts.push(short_date(date));
    }
    if let Some(priority) = task["priority"].as_str().or_else(|| task["priority"]["Priority_Value"].as_str()) {
        parts.push(priority.to_owned());
    }
    if let Some(estimate) = task["estimate"]["estimate_value"].as_str() {
        parts.push(estimate.to_owned());
    }
    let labels: Vec<_> = task["taskLabels"].as_array().into_iter().flatten()
        .filter_map(|label| label["label"]["value"].as_str()).filter(|name| !name.is_empty()).collect();
    parts.extend(labels.iter().take(3).map(|name| (*name).to_owned()));
    if labels.len() > 3 {
        parts.push(format!("+{}", labels.len() - 3));
    }
    parts.join(" · ")
}

fn task_summary(extra: &serde_json::Map<String, serde_json::Value>) -> String {
    let markdown = extra
        .get("Task_Summary")
        .and_then(|v| v.as_array())
        .and_then(|a| a.first())
        .and_then(|v| v["content"].as_str())
        .or_else(|| extra.get("summary").and_then(|v| v.as_str()))
        .unwrap_or("");
    let first = markdown
        .lines()
        .map(str::trim)
        .find_map(|line| line.strip_prefix("- "))
        .or_else(|| {
            markdown
                .lines()
                .map(str::trim)
                .find(|line| !line.is_empty() && !line.starts_with('#'))
        })
        .unwrap_or(markdown.trim());
    let mut teaser = first
        .trim_start_matches('#')
        .trim_start()
        .replace(['*', '`'], "");
    if let Some((prefix, reference)) = teaser.rsplit_once('[') {
        if reference
            .strip_suffix(']')
            .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
        {
            teaser = prefix.trim_end().to_owned();
        }
    }
    teaser.trim().to_owned()
}

fn comment_author(extra: &serde_json::Map<String, serde_json::Value>) -> String {
    extra.get("agent_display_name").and_then(|v| v.as_str()).filter(|s| !s.is_empty())
        .or_else(|| ["agent", "creator", "user"].into_iter().find_map(|key| {
            extra.get(key).and_then(|v| v["displayName"].as_str().filter(|s| !s.is_empty())
                .or_else(|| v["email"].as_str()))
        }))
        .unwrap_or(if extra.get("pending").and_then(|v| v.as_bool()) == Some(true) {
            "You · Pending"
        } else {
            "Unknown author"
        }).to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn creator_uses_live_user_and_mcp_created_by_without_null_shadowing() {
        for extra in [
            json!({"user": {"displayName": "alex+owner"}}),
            json!({"creator": null, "createdBy": {"displayName": "alex+owner"}}),
            json!({"user": null, "createdBy": null, "creator": {"displayName": "alex+owner"}}),
        ] {
            assert_eq!(creator_name(extra.as_object().unwrap()), "alex+owner");
        }
    }

    #[test]
    fn summary_uses_saved_live_content_and_the_first_bullet_teaser() {
        let extra = json!({"Task_Summary": [{"content": "## What this is\n- **QA reminder** check task, marked safe to delete; no real work scope. [1]\n- Description is empty."}], "summary": "Older summary"});
        assert_eq!(
            task_summary(extra.as_object().unwrap()),
            "QA reminder check task, marked safe to delete; no real work scope."
        );
    }

    #[test]
    fn summary_handles_plain_text_and_only_falls_back_for_empty_content() {
        for (extra, expected) in [
            (json!({"summary": "A saved summary"}), "A saved summary"),
            (json!({"summary": "# A saved heading"}), "A saved heading"),
            (
                json!({"Task_Summary": [{"content": "# Heading\n\nPlain `saved` content"}]}),
                "Plain saved content",
            ),
            (
                json!({"Task_Summary": [{"content": " \n "}]}),
                "",
            ),
            (json!({}), ""),
        ] {
            assert_eq!(task_summary(extra.as_object().unwrap()), expected);
        }
    }

    #[test]
    fn fidelity_fixture_preserves_priority_creator_summary_and_durable_comment_author() {
        let f: serde_json::Value = serde_json::from_str(include_str!("../../../fixtures/data-fidelity.json")).unwrap();
        for key in ["list", "detail"] {
            assert_eq!(priority_name(f[key]["tasks"][0].as_object().unwrap()), "High");
        }
        assert_eq!(creator_name(f["detail"]["tasks"][0].as_object().unwrap()), "Example Creator");
        assert_eq!(creator_name(f["list"]["tasks"][0].as_object().unwrap()), "Unknown author");
        assert_eq!(task_summary(f["webSummary"].as_object().unwrap()), "Example saved summary.");
        assert_eq!(task_summary(f["detail"]["tasks"][0].as_object().unwrap()), "");
        assert_eq!(comment_author(f["comments"]["comments"][0].as_object().unwrap()), "Retired Example Agent");
    }

    #[test]
    fn comment_author_does_not_shadow_people_with_null_or_empty_agents() {
        for extra in [
            json!({"agent": null, "creator": null, "user": {"displayName": "Example Person"}}),
            json!({"agent": {"displayName": ""}, "creator": {"displayName": "Example Person"}}),
        ] {
            assert_eq!(comment_author(extra.as_object().unwrap()), "Example Person");
        }
        assert_eq!(comment_author(json!({"pending": true}).as_object().unwrap()), "You · Pending");
        assert_eq!(creator_name(json!({"createdBy": {"email": "creator@example.com"}}).as_object().unwrap()), "creator@example.com");
        assert_eq!(priority_name(json!({"priority": null}).as_object().unwrap()), "No Priority");
    }


    #[test]
    fn inbox_metadata_uses_server_shapes_and_web_label_limit() {
        let task = json!({"priority": {"Priority_Value": "High"}, "estimate": {"estimate_value": "S"},
            "dueDate": "2026-10-20T12:00:00Z", "taskLabels": [
                {"label": {"value": "One"}}, {"label": {"value": "Two"}},
                {"label": {"value": "Three"}}, {"label": {"value": "Four"}}]});
        assert_eq!(inbox_metadata(&task), "20 Oct · High · S · One · Two · Three · +1");
        assert_eq!(inbox_metadata(&json!({"priority": null, "dueDate": null})), "");
    }

}
