use crate::{
    Options,
    ai_sidebar::{AiSidebar, Scope},
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
    Description,
    ExistingComment,
}

#[path = "ai_workspace.rs"]
mod ai_workspace;

#[path = "comment_selection.rs"]
mod comment_selection;

#[path = "comment_actions.rs"]
mod comment_actions;
#[path = "editor_actions.rs"]
mod editor_actions;
#[path = "shell.rs"]
mod shell;

#[path = "comment_drafts.rs"]
mod comment_drafts;

#[path = "inbox_review.rs"]
mod inbox_review;

#[path = "inbox.rs"]
mod inbox;

#[path = "comments.rs"]
mod comments;
#[path = "task_fields.rs"]
mod task_fields;
#[path = "board_mutations.rs"]
mod board_mutations;
#[path = "board.rs"]
mod board;

#[path = "interactions.rs"]
mod interactions;

pub struct Desktop {
    pub focus: FocusHandle,
    shell: shell::Shell,
    db: Db,
    options: Options,
    colors: Theme,
    screen: Screen,
    previous: Screen,
    inbox: Vec<InboxItem>,
    inbox_actions: inbox::InboxActions,
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
    interaction: interactions::Interaction,
    task_fields: task_fields::TaskFields,
    board_actions: board::BoardActions,
    board_forms: board_mutations::Forms,
    comment_actions: comment_actions::CommentActions,
    comment_drafts: comment_drafts::CommentDrafts,
    moving: bool,
    footer: bool,
    ready_scheduled: bool,
    layout: Capture,
    error: Option<String>,
    worker: Option<Worker>,
    signin: SignIn,
    auto_board: bool,
    ai_sidebar: Option<Entity<AiSidebar>>,
    _ai_sidebar_subscription: Option<gpui::Subscription>,
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
            ).comment_editor()
        });
        cx.observe(&comment_input, |this, _, cx| { this.observe_comment_draft(cx); cx.notify(); }).detach();
        if options.background {
            cx.observe(&title_input, |_, _, cx| cx.notify()).detach();
        }
        let focus = cx.focus_handle();
        let (mut signin, secret) = SignIn::start(options.offline);
        let ai_sidebar = (AI_CHAT_PANEL_SHOWN || options.ai_sidebar).then(|| {
            cx.new(|cx| {
                let mut sidebar = AiSidebar::new(colors, secret.clone(), focus.clone(), window, cx);
                sidebar.restore_presentation(&db);
                sidebar
            })
        });
        let ai_sidebar_subscription = ai_sidebar
            .as_ref()
            .map(|sidebar| cx.observe(sidebar, |this, sidebar, cx| {
                let sidebar = sidebar.read(cx);
                this.shell.ai_open = sidebar.is_open();
                if let Err(error) = sidebar.persist_presentation(&this.db) { this.error = Some(error.to_string()); }
                cx.notify();
            }));
        let worker = secret.clone().and_then(|secret| Self::start_worker(&options, &mut signin, secret));
        let auto_board = !options.offline && !options.board_given && signin.is_interactive();
        let mut options = options;
        if auto_board {
            Self::resolve_board(&db, &mut options);
        }
        let detail_scroll = ScrollHandle::new();
        let mut shell = shell::Shell::new(&db, cx);
        shell.set_token(secret.clone());
        if let Some(sidebar) = &ai_sidebar { shell.ai_open = sidebar.read(cx).is_open(); }
        let mut view = Self {
            shell,
            focus,
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
            inbox_actions: inbox::InboxActions::new(cx),
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
            interaction: interactions::Interaction::new(cx),
            task_fields: task_fields::TaskFields::new(cx),
            board_actions: board::BoardActions::new(cx),
            board_forms: board_mutations::Forms::new(cx),
            comment_actions: comment_actions::CommentActions::new(cx),
            comment_drafts: comment_drafts::CommentDrafts::default(),
            moving: false,
            footer: true,
            ready_scheduled: false,
            error: None,
            worker,
            signin,
            auto_board,
            ai_sidebar,
            _ai_sidebar_subscription: ai_sidebar_subscription,
        };
        view.bind_editors(cx);
        view.reload();
        view.init_shell(window, cx);
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
                            this.poll_editor_results(cx);
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
                self.set_shell_token(None, cx);
                if let Some(sidebar) = &self.ai_sidebar {
                    sidebar.update(cx, |sidebar, cx| sidebar.set_token(None, cx));
                }
                if let Some(worker) = self.worker.take() {
                    std::thread::spawn(move || drop(worker));
                }
            }
            Event::SignedIn(secret) => {
                self.set_shell_token(Some(secret.clone()), cx);
                if let Some(sidebar) = &self.ai_sidebar {
                    sidebar.update(cx, |sidebar, cx| {
                        sidebar.set_token(Some(secret.clone()), cx)
                    });
                }
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
                    self.reload_inbox_tabs();
                }
                Screen::Board => self.load_columns(self.options.board)?,
                Screen::Task => {
                    self.detail = self.db.task_detail(self.options.task)?;
                    if let Some(detail) = &mut self.detail {
                        detail.comments.sort_by(|a,b| a.created_at.cmp(&b.created_at).then(a.id.cmp(&b.id)));
                    }
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
        let mut columns = self.db.board(project)?;
        if self.screen == Screen::Board && self.board_actions.project == project {
            for column in &mut columns { column.cards.retain(|task| board::search_matches(task, &self.board_actions.search_text)); }
        }
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
            .filter(|(_, n)| self.inbox_split_matches(n, self.split) && inbox::search_matches(n, &self.inbox_actions.search_text))
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
        self.exit_ai_fullscreen(cx);
        if self.auto_board && screen == Screen::Board {
            Self::resolve_board(&self.db, &mut self.options);
            let _ = self
                .db
                .kv_set("last_board", &self.options.board.to_string());
        }
        self.shell.reset_destination();
        self.screen = screen;
        if screen == Screen::Board && self.board_actions.project != self.options.board {
            self.board_actions.project = self.options.board;
            self.board_actions.searching = false;
            self.board_actions.search_text.clear();
            self.board_actions.search.update(cx, |input,cx|input.set_content(String::new(),cx));
        }
        self.selected = 0;
        self.editing = Editing::None;
        self.moving = false;
        self.reload();
        self.request_sync();
        cx.notify();
    }

    fn open_task(&mut self, id: i64, cx: &mut Context<Self>) {
        self.remember_task_origin();
        self.mark_opened_notification(id, cx);
        self.remember_playlist();
        if self.screen != Screen::Task {
            self.previous = self.screen;
        }
        if self.options.task != id {
            self.reset_comment_actions();
            self.comment_input
                .update(cx, |input, cx| input.set_content(String::new(), cx));
        }
        self.task_fields.summary_open = false;
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
        if let Some(task) = self
            .selected_task()
            .and_then(|id| self.db.task(id).ok().flatten())
        {
            self.title_input
                .update(cx, |input, cx| input.set_content(task.title, cx));
            self.editing = Editing::Title;
            window.focus(&self.title_input.focus_handle(cx), cx);
            cx.notify();
        }
    }

    fn edit_comment(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.comment_input.read(cx).is_empty() {self.restore_editor(self.comment_input.clone(),Some(ht_core::DraftType::Comment),cx);}
        self.editing = Editing::Comment;
        self.composer_anchor.scroll_to(window, cx);
        window.focus(&self.comment_input.focus_handle(cx), cx);
        cx.notify();
    }

    fn save_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.editing == Editing::ExistingComment { return self.save_existing_comment(window, cx); }
        let Some(task_id) = self.selected_task() else {
            return false;
        };
        let comment = self.editing == Editing::Comment;
        if comment && self.comment_input.read(cx).is_empty() {
            return false;
        }
        let write = match self.editing {
            Editing::Title => Write::TaskTitle {
                task_id,
                title: self.title_input.read(cx).content.to_string(),
            },
            Editing::Comment => Write::TaskComment {
                task_id,
                text: self.comment_body_html(cx),
            },
            Editing::Description => Write::TaskPatch {
                task_id,
                patch: ht_core::TaskPatch::Description(self.interaction.description.read(cx).html()),
            },
            Editing::None | Editing::ExistingComment => return false,
        };
        if self.enqueue(write, cx) {
            self.finish_editor_job(cx);
            self.editing = Editing::None;
            if comment {
                self.comment_draft_sent();
                self.comment_input
                    .update(cx, |input, cx| input.reset(window, cx));
            }
            window.focus(&self.focus, cx);
            true
        } else {
            false
        }
    }

    fn key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if self.unread_review_key(event,window,cx) {return;}
        if self.comment_reader_key(event, window, cx) { return; }
        if self.board_form_key(event, window, cx) || self.task_fields_key(event, window, cx) || self.board_key(event, window, cx) { return; }
        if self.inbox_key(event, window, cx) { return; }
        if self.shell_key(event, window, cx) {
            return;
        }
        if let Some(sidebar) = &self.ai_sidebar {
            if event.keystroke.modifiers.control && event.keystroke.key == "q" {
                self.shell.ai_open = true;
                sidebar.update(cx, |sidebar, cx| sidebar.focus_composer(window, cx));
                return;
            }
            let commands = (event.keystroke.modifiers.control || event.keystroke.modifiers.platform)
                && event.keystroke.key == "k" && !event.keystroke.modifiers.shift && !event.keystroke.modifiers.alt;
            if (sidebar.read(cx).has_focus(window, cx)
                || sidebar.read(cx).mode() == crate::ai_sidebar::DisplayMode::Fullscreen) && !commands {
                return;
            }
        }
        if !self.comment_key(event, window, cx) { self.interaction_key(event, window, cx); }
    }

    fn footer(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let items = crate::footer::hints(self.screen);
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

    fn board_screen(&self, window: &Window, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let t = self.colors;
        let title = self.db.projects().unwrap_or_default().into_iter()
            .find(|project| project["id"] == self.options.board)
            .and_then(|project| project["title"].as_str().or(project["name"].as_str()).map(str::to_owned))
            .or_else(|| self.columns.iter().flat_map(|c| &c.cards).find(|task| !task.board_title.is_empty()).map(|task|task.board_title.clone()))
            .unwrap_or_else(|| "Board".into());
        let count: usize = self.columns.iter().map(|c| c.cards.len()).sum();
        let sidebar_width = self
            .ai_sidebar
            .as_ref()
            .filter(|_| self.shell.ai_open)
            .map_or(0., |sidebar| sidebar.read(cx).width());
        let board_width = (f32::from(window.viewport_size().width) - self.shell.width() - sidebar_width).max(0.);
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
            .when(self.board_actions.searching, |d| d.child(self.board_search(cx)))
            .when(!self.interaction.bulk.is_empty(), |d| d.child(self.board_selection_bar(cx)))
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
        let active = (self.column == column && self.selected == row)
            || self.interaction.bulk.contains(&task.id);
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
                            .child(self.card_controls(id, column, row, cx))
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
                                        .id(("card-inbox", id as u64))
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            cx.stop_propagation();
                                            this.column = column; this.selected = row;
                                            this.run_shortcut(crate::shortcuts::Action::ArchiveNotification, window, cx);
                                        }))
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
                                .when(
                                    self.editing == Editing::Title
                                        && self.column == column
                                        && self.selected == row,
                                    |d| d.w_full(),
                                )
                                .child(
                                    if self.editing == Editing::Title
                                        && self.column == column
                                        && self.selected == row
                                    {
                                        self.focused_editor(Editing::Title, cx)
                                    } else {
                                        div().child(title.clone()).into_any_element()
                                    },
                                )
                                .layout_key(
                                    &self.layout,
                                    format!("board.card.{index}.title"),
                                    Some(title),
                                ),
                        ),
                    )
                    .when(
                        priority_name(&task.extra) != "No Priority" || task.due_date.is_some(),
                        |d| {
                            d.child(
                                div().text_size(px(11.)).text_color(color(t.muted)).child(
                                    [
                                        Some(priority_name(&task.extra))
                                            .filter(|p| *p != "No Priority")
                                            .map(str::to_owned),
                                        task.due_date.as_deref().map(short_date),
                                    ]
                                    .into_iter()
                                    .flatten()
                                    .collect::<Vec<_>>()
                                    .join(" · "),
                                ),
                            )
                        },
                    )
                    .when(
                        task.extra.get("hasBoardDraft").and_then(|v| v.as_bool()) == Some(true),
                        |d| {
                            d.child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(color(t.accent))
                                    .child("Draft"),
                            )
                        },
                    )
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
                .child(self.focused_editor(Editing::Title, cx))
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
            .child(div().id("task-back").absolute().left(px(23.)).top(px(51.)).cursor_pointer().on_click(cx.listener(|this, _, window, cx| this.run_shortcut(crate::shortcuts::Action::Back, window, cx))).child(icon("arrow-left", t.muted, 18.)).layout_key(&self.layout,"task.back",None))
            .child(div().absolute().left(px(12.)).top(px(88.)).w(px(40.)).h(px(68.)).rounded_full().bg(color(navigation_color)).shadow(shadow()).flex().flex_col().justify_center().items_center().gap(px(10.)).text_color(color(t.muted)).children(["chevron-up", "chevron-down"].into_iter().map(|name| div().opacity(0.4).child(icon(name, t.muted, 18.)))))
            .child(div().w_full().max_w(px(1136.)).min_h_full().bg(color(t.detail)).flex().flex_col()
            .child(div().flex_shrink_0().pt(px(36.)).pl(px(82.)).pr(px(64.)).pb(px(8.)).relative()

                .child(div().w_full().flex().items_start().gap(px(8.)).font_weight(FontWeight::BOLD).text_size(px(24.)).line_height(px(36.)).min_h(px(37.)).child(title)
                    .child(div().flex_shrink_0().flex().items_center().gap(px(16.)).pt(px(10.25)).text_size(px(14.)).line_height(px(21.)).text_color(color(option_color))
                        .child(div().px(px(4.)).py(px(2.)).when(self.options.theme == ThemeName::Dia, |d| d.font_weight(FontWeight::MEDIUM)).child("Share")).child(icon("archive", option_color, 20.)).child(icon("circle-check", option_color, 20.)).child(icon("clock", option_color, 18.)).child(icon("layers", 0x696b6e, 16.))))
                .when(!self.summary.is_empty(), |d| d.child(self.task_summary_view(cx))).layout_key(&self.layout, "task.header", None))
            .child(div().relative().flex().items_start().gap(px(16.)).mt(px(16.)).pl(px(64.)).pr(px(64.)).pb(px(32.)).flex_shrink_0()

                .child(div().flex_1().min_w(px(500.)).max_w(px(680.)).flex().flex_col().gap(px(16.))
                    .child(div().id("task-description").anchor_scroll(Some(self.description_anchor.clone())).min_h(px(207.)).pt(px(19.)).pb(px(23.)).pl(px(19.)).pr(px(15.)).bg(color(t.comment)).border_1().border_color(color(if self.interaction.task_part == 0 { t.accent } else { t.border })).rounded(px(if self.options.theme == ThemeName::Dia { 10. } else { 5. })).shadow_md().flex().flex_col()
                        .child(div().h(px(21.)).flex().items_start().gap(px(8.)).text_color(color(0x8e9093)).text_size(px(14.)).line_height(px(21.))
                            .child(div().size(px(18.)).rounded_full().bg(color(avatar_color)).text_color(color(t.text)).text_size(px(11.)).line_height(px(16.5)).font_weight(FontWeight::SEMIBOLD).flex().items_center().justify_center().child(self.creator_initials.clone()))
                            .child(self.creator.clone()).when(d.task.extra.get("hasDescriptionDraft").and_then(|v| v.as_bool()) == Some(true), |d| d.child("Draft")).child(div().flex_1()).child(div().text_size(px(12.)).line_height(px(18.)).mt(px(1.5)).text_color(color(t.muted)).child(short_date(d.task.extra.get("createdAt").and_then(|v| v.as_str()).unwrap_or("")))))
                        .child(div().pt(px(8.)).min_h(px(88.)).text_size(px(16.)).line_height(px(24.)).when(self.editing != Editing::Description && self.description.text.trim().is_empty(), |d| d.child(div().text_size(px(14.)).line_height(px(21.)).text_color(color(placeholder)).child(div().font_weight(FontWeight::SEMIBOLD).text_size(px(18.)).line_height(px(21.6)).when(self.options.theme == ThemeName::Dia, |d| d.font_family("Newsreader").italic()).child("Add Description")).child("Tip: Hit ENTER to edit mode or CTRL+D to jump here"))).when(self.editing != Editing::Description && !self.description.text.trim().is_empty(), |d| d.child(self.description.element())).when(self.editing == Editing::Description, |d| d.child(self.focused_editor(Editing::Description, cx))).layout_key(&self.layout, "task.description", Some(self.description.text.to_string())))
                        .child(div().flex_1().min_h_0())
                        .child(div().pl(px(10.)).flex().items_center().text_size(px(12.)).line_height(px(18.)).text_color(color(t.muted))
                            .child(div().flex().items_center().gap(px(4.)).child(icon("plus", t.muted, 8.)).child("Add sub-tasks"))
                            .child(div().ml(px(16.)).flex().items_center().gap(px(4.)).child(icon("plus", t.muted, 8.)).child("Add page"))
                            .child(div().flex_1()).child(icon("ai-sparkles", 0xc668ff, 12.)).child(div().ml(px(8.)).mr(px(10.)).child("Edit with AI"))))
                    .children(d.comments.iter().zip(&self.comments).enumerate().map(|(i, (comment, body))| self.comment_row(i, comment, body, cx)))
                    .child(self.comment_composer(cx)))
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
            ("Priority", priority_name(&task.extra).into()),
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
                    .when(matches!(label, "Assignees" | "Agents" | "Priority" | "Due date" | "Tags" | "Task size" | "ID"), |d| {
                        d.cursor_pointer().on_click(cx.listener(move |this, _, window, cx| {
                            cx.stop_propagation();
                            this.open_task_field(label, window, cx);
                        }))
                    })
                    .when(label == "ID", |d| d.tooltip(|_,cx|cx.new(|_|task_fields::TicketTooltip).into()))
                    .when(label == "Status", |d| {
                        d.cursor_pointer().on_click(cx.listener(|this, _, window, cx| {
                            cx.stop_propagation();
                            this.run_shortcut(crate::shortcuts::Action::MoveColumn, window, cx);
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
                            .when(label == "Status" && self.has_next_status(task), |d| {
                                d.child(self.next_status_button(cx))
                            })
                            .when(label == "Project", |d| {
                                d.child(
                                    self.task_board_button(cx),
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

const AI_CHAT_PANEL_SHOWN: bool = true;

impl Render for Desktop {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(layout) = &self.layout {
            layout.borrow_mut().begin(self.split);
        }
        if self.options.background
            && let Some((path, capture)) = self.options.dump_layout.clone().zip(self.layout.clone())
        {
            let task = self
                .selected_task()
                .and_then(|id| self.db.task(id).ok().flatten());
            let count = match self.screen {
                Screen::Inbox => self.filtered.len(),
                Screen::Board => self.columns.get(self.column).map_or(0, |c| c.cards.len()),
                Screen::Task => 0,
            };
            crate::behavior::snapshot(
                window,
                path,
                capture,
                self.screen,
                self.options.theme,
                serde_json::json!({
                    "error": self.error,
                    "theme": format!("{:?}", self.options.theme).to_lowercase(),
                    "selection_count": count, "last_index": count.saturating_sub(1),
                    "board_count": self.columns.iter().map(|c| c.cards.len()).sum::<usize>(),
                    "inbox_count": self.inbox.len(), "composer_index": self.comments.len() + 1,
                    "choice_count": self.choices(cx).len(),
                    "key_events": self.interaction.key_events, "last_key": self.interaction.last_key,
                    "last_ctrl": self.interaction.last_ctrl, "last_shift": self.interaction.last_shift,
                    "column_ids": self.columns.iter().map(|c| c.id).collect::<Vec<_>>(),
                    "task_title": task.as_ref().map(|t| &t.title),
                    "ticket": task.as_ref().and_then(|t| t.ticket_number.as_ref()),
                    "project_id": task.as_ref().map(|t| t.project_id),
                    "board_id": self.options.board,
                    "unique_index": task.as_ref().and_then(|t| t.extra.get("uniqueIndex")),
                    "task_assignees": task.as_ref().and_then(|t| t.extra.get("assignees")),
                    "task_labels": task.as_ref().and_then(|t| t.extra.get("labels")),
                    "first_assignee": task.as_ref().and_then(|t| self.db.project_members(t.project_id).ok()).and_then(|m| m.first().cloned()).and_then(|m| m["id"].as_i64()),
                    "first_label": task.as_ref().and_then(|t| self.db.project_labels(t.project_id).ok()).and_then(|l| l.first().cloned()).and_then(|l| l["id"].as_str().map(str::to_owned)),
                    "screen": format!("{:?}", self.screen).to_lowercase(),
                    "selected": self.selected, "column": self.column, "split": self.split,
                    "task_id": self.selected_task(),
                    "composer_focused": self.comment_input.focus_handle(cx).is_focused(window),
                    "root_focused": self.focus.is_focused(window),
                    "comment_text": self.comment_input.read(cx).content.as_ref(),
                    "title_text": self.title_input.read(cx).content.as_ref(),
                    "description_text": self.interaction.description.read(cx).content.as_ref(),
                    "editing": match self.editing { Editing::None => "none", Editing::Title => "title", Editing::Comment => "comment", Editing::Description => "description", Editing::ExistingComment => "edit-comment" },
                    "footer": crate::footer::hints(self.screen).iter().map(|(k, v)| format!("{k} {v}")).collect::<Vec<_>>(),
                    "command_open": self.interaction.modal == Some(interactions::Modal::Commands),
                    "modal": self.interaction.modal.map(|m| format!("{m:?}").to_lowercase()),
                    "command_selected": self.interaction.choice,
                    "query": self.interaction.query.read(cx).content.as_ref(),
                    "bulk_count": self.interaction.bulk.len(),
                    "task_part": self.interaction.task_part,
                }),
            );
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
                    .filter(|_| !self.options.background)
                    .map(|(path, capture)| (path, capture, self.screen, self.options.theme)),
            );
        }
        if self.signin.phase != Phase::Ready {
            return self.sign_in_panel(cx).into_any_element();
        }
        self.prepare_comment_readers(window,cx);
        self.restore_comment_draft(cx);
        if self.inbox_actions.review.is_some() {return self.unread_review_screen(cx).into_any_element();}
        if let Some(sidebar) = &self.ai_sidebar {
            let task = (self.screen != Screen::Board)
                .then(|| self.selected_task())
                .flatten();
            let scope = Scope::load(&self.db, self.screen, self.options.board, task);
            sidebar.update(cx, |sidebar, cx| { sidebar.capture = self.layout.clone(); sidebar.fit_viewport(f32::from(window.viewport_size().width), cx); sidebar.set_scope(scope, cx); });
        }
        if self.shell.settings {
            return self.settings_screen(window, cx).into_any_element();
        }
        let body = match self.screen {
            Screen::Inbox => self.inbox_screen(cx).into_any_element(),
            Screen::Board => self.board_screen(window, cx).into_any_element(),
            Screen::Task => self.task_screen(cx),
        };
        let body = self.shell_body(body, cx);
        let t = self.colors;
        div()
            .id("desktop-workspace")
            .size_full()
            .flex()
            .font_family(self.options.theme.font())
            .text_size(px(16.))
            .line_height(gpui::relative(1.5))
            .text_color(color(t.text))
            .bg(color(t.page))
            .track_focus(&self.focus)
            .relative()
            .capture_any_mouse_down(cx.listener(|this, _, window, cx| {
                if this.interaction.modal.is_none() {
                    this.editing = Editing::None;
                    window.focus(&this.focus, cx);
                }
            }))
            .capture_key_down(cx.listener(Self::key_down))
            .on_click(cx.listener(|this,_,_,cx|{if this.task_fields.summary_open {this.task_fields.summary_open=false;cx.notify();}}))
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
            .on_mouse_move(cx.listener(|this, event, window, cx| {
                if let Some(sidebar) = &this.ai_sidebar {
                    sidebar.update(cx, |sidebar, cx| sidebar.resize_move(event, window, cx));
                }
            }))
            .on_mouse_up(gpui::MouseButton::Left, cx.listener(|this, event, window, cx| {
                if let Some(sidebar) = &this.ai_sidebar {
                    sidebar.update(cx, |sidebar, cx| sidebar.resize_end(event, window, cx));
                }
            }))
            .child(self.ai_panel(window, cx))
            .child(self.shell_reopen(cx))
            .child(self.shell_overlay(window, cx))
            .child(self.interaction_overlay(window, cx))
            .child(self.comment_overlay(cx))
            .child(self.inbox_reminder_overlay(cx))
            .child(self.due_date_overlay(cx))
            .child(self.board_form_overlay(cx))
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
        .find_map(|key| {
            extra.get(key).and_then(|v| {
                v["displayName"]
                    .as_str()
                    .filter(|s| !s.is_empty())
                    .or_else(|| v["email"].as_str())
            })
        })
        .or_else(|| extra.get("creatorName").and_then(|v| v.as_str()))
        .unwrap_or("Unknown author")
        .to_owned()
}

fn priority_name(extra: &serde_json::Map<String, serde_json::Value>) -> &str {
    extra
        .get("priority")
        .and_then(|v| v.as_str().or_else(|| v["Priority_Value"].as_str()))
        .unwrap_or("No Priority")
}

fn inbox_metadata(task: &serde_json::Value) -> String {
    let mut parts = Vec::new();
    if let Some(date) = task["dueDate"].as_str() {
        parts.push(short_date(date));
    }
    if let Some(priority) = task["priority"]
        .as_str()
        .or_else(|| task["priority"]["Priority_Value"].as_str())
    {
        parts.push(priority.to_owned());
    }
    if let Some(estimate) = task["estimate"]["estimate_value"].as_str() {
        parts.push(estimate.to_owned());
    }
    let labels: Vec<_> = task["taskLabels"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|label| label["label"]["value"].as_str())
        .filter(|name| !name.is_empty())
        .collect();
    parts.extend(labels.iter().take(3).map(|name| (*name).to_owned()));
    if labels.len() > 3 {
        parts.push(format!("+{}", labels.len() - 3));
    }
    parts.join(" · ")
}

fn task_summary(extra: &serde_json::Map<String, serde_json::Value>) -> String {
    let markdown = if let Some(summary) = extra.get("ai_summary") {
        summary["content"].as_str().unwrap_or("")
    } else {
        extra
            .get("Task_Summary")
            .and_then(|v| v.as_array())
            .and_then(|a| a.first())
            .and_then(|v| v["content"].as_str())
            .or_else(|| extra.get("summary").and_then(|v| v.as_str()))
            .unwrap_or("")
    };
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
    extra
        .get("agent_display_name")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .or_else(|| {
            ["agent", "creator", "user"].into_iter().find_map(|key| {
                extra.get(key).and_then(|v| {
                    v["displayName"]
                        .as_str()
                        .filter(|s| !s.is_empty())
                        .or_else(|| v["email"].as_str())
                })
            })
        })
        .unwrap_or(
            if extra.get("pending").and_then(|v| v.as_bool()) == Some(true) {
                "You · Pending"
            } else {
                "Unknown author"
            },
        )
        .to_owned()
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
    fn optional_mcp_ai_summary_takes_precedence_and_missing_or_null_is_hidden() {
        for (extra, expected) in [
            (
                json!({"ai_summary":{"content":"- **Live** summary", "updated_at":"2026-10-23"}, "summary":"Old"}),
                "Live summary",
            ),
            (json!({"ai_summary":null, "summary":"Old summary"}), ""),
            (json!({}), ""),
        ] {
            assert_eq!(task_summary(extra.as_object().unwrap()), expected);
        }
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
            (json!({"Task_Summary": [{"content": " \n "}]}), ""),
            (json!({}), ""),
        ] {
            assert_eq!(task_summary(extra.as_object().unwrap()), expected);
        }
    }

    #[test]
    fn fidelity_fixture_preserves_priority_creator_summary_and_durable_comment_author() {
        let f: serde_json::Value =
            serde_json::from_str(include_str!("../../../fixtures/data-fidelity.json")).unwrap();
        for key in ["list", "detail"] {
            assert_eq!(
                priority_name(f[key]["tasks"][0].as_object().unwrap()),
                "High"
            );
        }
        assert_eq!(
            creator_name(f["detail"]["tasks"][0].as_object().unwrap()),
            "Example Creator"
        );
        assert_eq!(
            creator_name(f["list"]["tasks"][0].as_object().unwrap()),
            "Unknown author"
        );
        assert_eq!(
            task_summary(f["webSummary"].as_object().unwrap()),
            "Example saved summary."
        );
        assert_eq!(
            task_summary(f["detail"]["tasks"][0].as_object().unwrap()),
            ""
        );
        assert_eq!(
            comment_author(f["comments"]["comments"][0].as_object().unwrap()),
            "Retired Example Agent"
        );
    }

    #[test]
    fn comment_author_does_not_shadow_people_with_null_or_empty_agents() {
        for extra in [
            json!({"agent": null, "creator": null, "user": {"displayName": "Example Person"}}),
            json!({"agent": {"displayName": ""}, "creator": {"displayName": "Example Person"}}),
        ] {
            assert_eq!(comment_author(extra.as_object().unwrap()), "Example Person");
        }
        assert_eq!(
            comment_author(json!({"pending": true}).as_object().unwrap()),
            "You · Pending"
        );
        assert_eq!(
            creator_name(
                json!({"createdBy": {"email": "creator@example.com"}})
                    .as_object()
                    .unwrap()
            ),
            "creator@example.com"
        );
        assert_eq!(
            priority_name(json!({"priority": null}).as_object().unwrap()),
            "No Priority"
        );
    }

    #[test]
    fn inbox_metadata_uses_server_shapes_and_web_label_limit() {
        let task = json!({"priority": {"Priority_Value": "High"}, "estimate": {"estimate_value": "S"},
            "dueDate": "2026-10-20T12:00:00Z", "taskLabels": [
                {"label": {"value": "One"}}, {"label": {"value": "Two"}},
                {"label": {"value": "Three"}}, {"label": {"value": "Four"}}]});
        assert_eq!(
            inbox_metadata(&task),
            "20 Oct · High · S · One · Two · Three · +1"
        );
        assert_eq!(
            inbox_metadata(&json!({"priority": null, "dueDate": null})),
            ""
        );
    }
}
