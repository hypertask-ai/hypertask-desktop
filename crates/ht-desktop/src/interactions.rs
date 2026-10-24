use super::*;
use crate::{
    command_center::{self, CommandAction},
    shortcuts::{self, Action},
};
use gpui::{ClipboardItem, MouseButton};
use ht_core::{TaskPatch, generated::Status};
use std::{collections::BTreeSet, time::Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Modal {
    Commands,
    Search,
    LocalSearch,
    Boards,
    Column,
    Priority,
    Assign,
    Tags,
    Size,
    DueDate,
    Delete,
}
#[derive(Clone)]
pub(super) struct Choice {
    pub(super) name: String,
    action: ChoiceAction,
}
#[derive(Clone, Copy)]
enum ChoiceAction {
    Command(CommandAction),
    Column(i64),
    Priority(u8),
    Assignee(i64),
    Agent(usize),
    Label(usize),
    Size(u8),
    DueDate(i64),
    Task(i64),
    Board(i64),
    Delete,
    CreateLabel,
}
pub(super) struct Interaction {
    pub modal: Option<Modal>,
    pub agents_only: bool,
    pub query: Entity<TextInput>,
    pub description: Entity<TextInput>,
    pub choice: usize,
    pub bulk: BTreeSet<i64>,
    pub playlist: Vec<i64>,
    pub return_to: Option<super::board::ReturnTo>,
    pub task_part: usize,
    pub key_events: u64,
    pub last_key: String,
    pub last_ctrl: bool,
    pub last_shift: bool,
    last_g: Option<(Instant, bool)>,
    modal_scroll: ScrollHandle,
}
impl Interaction {
    pub fn new(cx: &mut Context<Desktop>) -> Self {
        let query = cx.new(|cx| {
            TextInput::new(
                String::new(),
                "Type a command or search...",
                color(0x8e9093),
                cx,
            )
        });
        cx.observe(&query, |this, _, cx| {
            this.interaction.choice = 0;
            cx.notify();
        })
        .detach();
        let description = cx.new(|cx| {
            TextInput::new(String::new(), "Add Description", color(0x8e9093), cx).multiline_editor()
        });
        cx.observe(&description, |_, _, cx| cx.notify()).detach();
        Self {
            modal: None,
            agents_only: false,
            query,
            description,
            choice: 0,
            bulk: BTreeSet::new(),
            playlist: vec![],
            return_to: None,
            task_part: 0,
            key_events: 0,
            last_key: String::new(),
            last_ctrl: false,
            last_shift: false,
            last_g: None,
            modal_scroll: ScrollHandle::new(),
        }
    }
}
impl Desktop {
    pub(super) fn interaction_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let key = &event.keystroke;
        let ctrl = key.modifiers.control || key.modifiers.platform;
        if self.options.background {
            self.interaction.key_events += 1;
            self.interaction.last_key = key.key.clone();
            self.interaction.last_ctrl = ctrl;
            self.interaction.last_shift = shortcuts::shifted(key);
            cx.notify();
        }
        if self.board_actions.searching && self.board_actions.search.focus_handle(cx).is_focused(window) && key.key != "escape" { return; }
        if key.key == "k" && ctrl && !key.modifiers.shift && !key.modifiers.alt {
            if self.interaction.modal == Some(Modal::Commands) {
                self.close_modal(window, cx);
            } else {
                self.show_modal(Modal::Commands, window, cx);
            }
            cx.stop_propagation();
            return;
        }
        if self.interaction.modal.is_some() {
            let count = self.choices(cx).len();
            match key.key.as_str() {
                "escape" => self.close_modal(window, cx),
                "down" | "j" if !ctrl && !key.modifiers.alt => {
                    self.interaction.choice =
                        (self.interaction.choice + 1).min(count.saturating_sub(1));
                    self.interaction
                        .modal_scroll
                        .scroll_to_item(self.interaction.choice);
                    cx.notify();
                }
                "up" | "k" if !ctrl && !key.modifiers.alt => {
                    self.interaction.choice = self.interaction.choice.saturating_sub(1);
                    self.interaction
                        .modal_scroll
                        .scroll_to_item(self.interaction.choice);
                    cx.notify();
                }
                "enter" => self.run_choice(window, cx),
                "tab" => (),
                _ => return,
            }
            cx.stop_propagation();
            return;
        }
        // Focus, rather than a stale editing flag, is authoritative after mouse navigation.
        let editor = if self.comment_input.focus_handle(cx).is_focused(window) {
            Editing::Comment
        } else if self.title_input.focus_handle(cx).is_focused(window) {
            Editing::Title
        } else if self
            .interaction
            .description
            .focus_handle(cx)
            .is_focused(window)
        {
            Editing::Description
        } else if self
            .comment_actions
            .input
            .focus_handle(cx)
            .is_focused(window)
        {
            Editing::ExistingComment
        } else {
            Editing::None
        };
        self.editing = editor;
        if editor == Editing::None && !ctrl && !key.modifiers.alt {
            if let Some((_at, shift)) = self
                .interaction
                .last_g
                .take()
                .filter(|(at, _)| at.elapsed() < Duration::from_secs(1))
            {
                let action = shortcuts::sequence(&key.key, shift, self.screen);
                if let Some(action) = action {
                    self.run_shortcut(action, window, cx);
                    cx.stop_propagation();
                    return;
                }
            }
            if key.key == "g" {
                self.interaction.last_g = Some((Instant::now(), key.modifiers.shift));
                cx.stop_propagation();
                return;
            }
        }
        let action = if editor == Editing::Title && key.key == "enter" && !key.modifiers.alt {
            Some(Action::Save {
                stay: true,
                complete: false,
            })
        } else {
            shortcuts::resolve(key, self.screen, editor != Editing::None)
        };
        if let Some(action) = action {
            self.run_shortcut(action, window, cx);
            cx.stop_propagation();
        }
    }

    pub(super) fn remember_playlist(&mut self) {
        if self.screen != Screen::Task {
            self.interaction.playlist = match self.screen {
                Screen::Inbox => self
                    .filtered
                    .iter()
                    .filter_map(|&i| self.inbox[i].task_id)
                    .collect(),
                Screen::Board => self
                    .columns
                    .iter()
                    .flat_map(|c| c.cards.iter().map(|t| t.id))
                    .collect(),
                Screen::Task => vec![],
            };
            self.interaction.playlist.dedup();
        }
        self.interaction.task_part = 0;
    }

    pub(super) fn run_shortcut(
        &mut self,
        action: Action,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        use Action::*;
        match action {
            Compose => self.open_compose(window, cx),
            ExpandSummary => {
                if !self.summary.is_empty() { self.task_fields.summary_open = !self.task_fields.summary_open; }
            }
            PinAiChat | FullscreenAiChat | DeleteAllAiChats => self.ai_command(action, cx),
            StarComment | PinComment | CopyCommentContent | CopyCommentLink | CopyCommentToAiChat | BranchCommentToAiChat | ReplyComment | EditComment | DeleteComment | ReactComment | LikeComment
            | ExpandComment | ExpandComments => self.run_comment_action(action, window, cx),
            Commands => self.show_modal(Modal::Commands, window, cx),
            Inbox | Board => {
                if action == Board
                    && let Some(id) = self.selected_task()
                    && let Ok(Some(task)) = self.db.task(id)
                {
                    self.options.board = task.project_id;
                }
                self.switch(
                    if action == Inbox {
                        Screen::Inbox
                    } else {
                        Screen::Board
                    },
                    cx,
                );
                window.focus(&self.focus, cx);
            }
            Back => {
                if self.editing != Editing::None {
                    self.comment_actions.edit_id = None;
                    self.editing = Editing::None;
                    window.focus(&self.focus, cx);
                } else if !self.interaction.bulk.is_empty() {
                    self.interaction.bulk.clear();
                } else if self.screen == Screen::Task {
                    self.return_from_task(cx);
                    window.focus(&self.focus, cx);
                }
                self.moving = false;
            }
            Save { stay, complete } => {
                let comment = self.editing == Editing::Comment;
                let saved = self.save_edit(window, cx);
                if saved && comment && self.previous == Screen::Inbox && (!stay || complete) {
                    if !self.archive_notification(cx) {
                        return;
                    }
                    if complete && let Some(task_id) = self.selected_task() {
                        if !self.enqueue(
                            Write::TaskPatch {
                                task_id,
                                patch: TaskPatch::Status(Status::Archive),
                            },
                            cx,
                        ) {
                            return;
                        }
                    }
                    self.playlist_step(1, cx);
                }
            }
            DiscardDraft => {
                if self.editing == Editing::None {
                    if !self.discard_comment_draft(cx) { return; }
                } else if !self.discard_editor_draft(cx) { return; }
                match self.editing {
                    Editing::Description => self.interaction.description.update(cx, |input, cx| {
                        input.set_html(self.detail.as_ref().map_or("", |d| d.task.description.as_str()), cx)
                    }),
                    Editing::ExistingComment => self.comment_actions.edit_id = None,
                    _ => {
                        self.comment_input.update(cx, |input, cx| input.reset(window, cx));
                    }
                }
                self.editing = Editing::None;
                window.focus(&self.focus, cx);
            }
            Comment => {
                if self.screen != Screen::Task
                    && let Some(id) = self.selected_task()
                {
                    self.open_task(id, cx);
                }
                if self.screen == Screen::Task {
                    self.edit_comment(window, cx);
                }
            }
            Rename => {
                if self.screen == Screen::Inbox
                    && let Some(id) = self.selected_task()
                {
                    self.open_task(id, cx);
                }
                self.edit_title(window, cx);
            }
            Description => self.edit_description(window, cx),
            Open if self.screen == Screen::Task => {
                if self.interaction.task_part == 0 {
                    self.edit_description(window, cx);
                } else if self.interaction.task_part <= self.comments.len() {
                    self.run_comment_action(ReplyComment, window, cx);
                } else {
                    self.edit_comment(window, cx);
                }
            }
            Open => {
                if let Some(id) = self.selected_task() {
                    self.open_task(id, cx);
                }
            }
            Next | Previous if self.screen == Screen::Task => {
                self.playlist_step(if action == Next { 1 } else { -1 }, cx);
            }
            ExtendNext | ExtendPrevious => {
                if let Some(id) = self.selection_id() {
                    self.interaction.bulk.insert(id);
                }
                self.selected = if action == ExtendNext {
                    self.selected.saturating_add(1)
                } else {
                    self.selected.saturating_sub(1)
                };
                self.clamp_selection();
                if let Some(id) = self.selection_id() {
                    self.interaction.bulk.insert(id);
                }
            }
            Next | Previous => {
                self.selected = if action == Next {
                    self.selected.saturating_add(1)
                } else {
                    self.selected.saturating_sub(1)
                };
                self.clamp_selection();
            }
            First | Last => {
                self.selected = if action == First { 0 } else { usize::MAX };
                self.clamp_selection();
            }
            NextColumn | PreviousColumn => match self.screen {
                Screen::Inbox => {
                    let count = self.splits.len();
                    if count > 0 {
                        self.split =
                            (self.split + if action == NextColumn { 1 } else { count - 1 }) % count;
                        self.selected = 0;
                        self.filter_inbox();
                    }
                }
                Screen::Board => self.focus_column(action == NextColumn),
                Screen::Task => {
                    let count = self.comments.len() + 2;
                    self.interaction.task_part = (self.interaction.task_part
                        + if action == NextColumn { 1 } else { count - 1 })
                        % count;
                    match self.interaction.task_part {
                        0 => self.description_anchor.scroll_to(window, cx),
                        n if n == count - 1 => self.composer_anchor.scroll_to(window, cx),
                        n => {
                            if let Some(anchor) = self.comment_anchors.get(n - 1) {
                                anchor.scroll_to(window, cx);
                            }
                        }
                    }
                }
            },
            MoveLeft | MoveRight => self.move_task(action == MoveRight, cx),
            Archive => {
                if self.screen == Screen::Board && !self.interaction.bulk.is_empty() {
                    self.bulk_archive(true, cx);
                } else {
                    self.archive_task(cx);
                }
            }
            ArchiveNotification => {
                let archived = self.archive_notification(cx);
                if archived && self.screen == Screen::Task && self.previous == Screen::Inbox {
                    self.playlist_step(1, cx);
                }
            }
            Select => {
                if let Some(id) = self.selection_id()
                    && !self.interaction.bulk.remove(&id)
                {
                    self.interaction.bulk.insert(id);
                }
            }
            SelectAll { all_splits } => {
                let ids: Vec<_> = match self.screen {
                    Screen::Inbox if all_splits => self.inbox.iter().map(|n| n.id).collect(),
                    Screen::Inbox => self.filtered.iter().map(|&i| self.inbox[i].id).collect(),
                    Screen::Board => self
                        .columns
                        .iter()
                        .flat_map(|c| c.cards.iter().map(|t| t.id))
                        .collect(),
                    Screen::Task => vec![],
                };
                self.interaction.bulk.extend(ids);
            }
            ReviewUnread => self.start_unread_review(window,cx),
            BulkArchive => self.bulk_archive(false, cx),
            Remind | BulkRemind => {
                let ids = if action == BulkRemind { self.interaction.bulk.iter().copied().collect() }
                    else if self.screen == Screen::Inbox { self.selection_id().into_iter().collect() }
                    else { self.task_inbox_ids() };
                self.show_inbox_reminder(ids,window,cx);
            }
            MoveColumn => self.show_modal(Modal::Column, window, cx),
            MoveBoard => self.open_board_form(crate::command_center::BoardAction::MoveTask, window, cx),
            Assign => self.show_modal(Modal::Assign, window, cx),
            Tags => self.show_modal(Modal::Tags, window, cx),
            Priority => self.show_modal(Modal::Priority, window, cx),
            Size => self.show_modal(Modal::Size, window, cx),
            DueDate => self.show_due_date(window, cx),
            Delete => self.show_modal(Modal::Delete, window, cx),
            CopyId | CopyTitleId | CopyUrl | CopyTitleUrl => self.copy_task(action, cx),
            Search | LocalSearch | Boards => self.show_modal(
                match action {
                    Search => Modal::Search,
                    LocalSearch => Modal::LocalSearch,
                    _ => Modal::Boards,
                },
                window,
                cx,
            ),
            CopyPageUrl => {
                if self.screen == Screen::Task {
                    self.copy_task(CopyUrl, cx);
                } else {
                    cx.write_to_clipboard(ClipboardItem::new_string(
                        if self.screen == Screen::Inbox {
                            "https://app.hypertask.ai/inbox".into()
                        } else {
                            format!(
                                "https://app.hypertask.ai/project/project-{}",
                                self.options.board
                            )
                        },
                    ));
                }
            }
        }
        if self.screen == Screen::Inbox {
            self.inbox_scroll
                .scroll_to_item(self.selected, ScrollStrategy::Nearest);
        }
        if self.screen == Screen::Board
            && let Some(list) = self.column_lists.get(self.column)
        {
            list.scroll_to_reveal_item(self.selected);
        }
        cx.notify();
    }
    pub(super) fn selection_id(&self) -> Option<i64> {
        if self.screen == Screen::Inbox {
            self.filtered.get(self.selected).map(|&i| self.inbox[i].id)
        } else {
            self.selected_task()
        }
    }
    fn focus_column(&mut self, next: bool) {
        let count = self.columns.len();
        if count > 0 {
            self.column = (self.column + if next { 1 } else { count - 1 }) % count;
            self.selected = 0;
            self.clamp_selection();
            self.board_scroll.scroll_to_item(self.column);
        }
    }
    fn move_task(&mut self, right: bool, cx: &mut Context<Self>) {
        let Some(task_id) = self.selected_task() else {
            return;
        };
        let index = if self.screen == Screen::Task {
            self.detail
                .as_ref()
                .and_then(|d| {
                    self.columns
                        .iter()
                        .position(|c| Some(c.id) == d.task.section_id)
                })
                .unwrap_or(self.column)
        } else {
            self.column
        };
        let next = if right {
            index.saturating_add(1)
        } else {
            index.saturating_sub(1)
        };
        if next == index {
            return;
        }
        if let Some(column) = self.columns.get(next) {
            let section_id = column.id;
            if self.enqueue(
                Write::TaskSection {
                    task_id,
                    section_id,
                },
                cx,
            ) {
                self.column = next;
                self.selected = self.columns[next]
                    .cards
                    .iter()
                    .position(|t| t.id == task_id)
                    .unwrap_or(0);
                self.clamp_selection();
            }
        }
    }
    fn archive_task(&mut self, cx: &mut Context<Self>) {
        if let Some(task_id) = self.selected_task() {
            let status = if self
                .db
                .task(task_id)
                .ok()
                .flatten()
                .is_some_and(|t| t.status == Status::Archive)
            {
                Status::Normal
            } else {
                Status::Archive
            };
            self.enqueue(
                Write::TaskPatch {
                    task_id,
                    patch: TaskPatch::Status(status),
                },
                cx,
            );
        }
    }
    fn archive_notification(&mut self, cx: &mut Context<Self>) -> bool {
        let id = if self.screen == Screen::Inbox {
            self.selection_id()
        } else {
            self.selected_task().and_then(|task| self.db.inbox().ok()?.iter().find(|n| n.task_id == Some(task)).map(|n| n.id))
        };
        if let Some(notification_id) = id {
            return self.enqueue(Write::InboxArchive { notification_id }, cx);
        }
        if self.screen == Screen::Board { self.error = Some("This task is not in inbox".into()); cx.notify(); }
        true
    }
    fn bulk_archive(&mut self, tasks: bool, cx: &mut Context<Self>) {
        for id in self.interaction.bulk.iter().copied().collect::<Vec<_>>() {
            let write = if tasks {
                Write::TaskPatch {
                    task_id: id,
                    patch: TaskPatch::Status(Status::Archive),
                }
            } else {
                Write::InboxArchive {
                    notification_id: id,
                }
            };
            if !self.enqueue(write, cx) {
                return;
            }
        }
        self.interaction.bulk.clear();
    }
    pub(super) fn playlist_step(&mut self, delta: isize, cx: &mut Context<Self>) {
        let Some(index) = self
            .interaction
            .playlist
            .iter()
            .position(|&id| id == self.options.task)
        else {
            return;
        };
        let next = index.saturating_add_signed(delta);
        if let Some(&id) = self.interaction.playlist.get(next) {
            if id != self.options.task {
                self.open_task(id, cx);
            }
        } else if self.previous == Screen::Inbox {
            self.switch(Screen::Inbox, cx);
        }
    }
    fn copy_task(&mut self, action: Action, cx: &mut Context<Self>) {
        let task = self
            .selected_task()
            .and_then(|id| self.db.task(id).ok().flatten());
        let Some(task) = task else {
            return;
        };
        let id = task
            .ticket_number
            .clone()
            .unwrap_or_else(|| task.id.to_string());
        let url = if matches!(action, Action::CopyId | Action::CopyTitleId) {
            String::new()
        } else if let Some(index) = task.extra.get("uniqueIndex").and_then(|v| v.as_i64()) {
            format!(
                "https://app.hypertask.ai/detail/project-{}/{index}",
                task.project_id
            )
        } else {
            self.error = Some("The cached task has no private URL; sync the task first".into());
            return;
        };
        let text = match action {
            Action::CopyId => id.to_uppercase(),
            Action::CopyTitleId => format!("{} | {}", id.to_uppercase(), task.title),
            Action::CopyTitleUrl => format!("{} | {} | {url}", id.to_uppercase(), task.title),
            _ => url,
        };
        cx.write_to_clipboard(ClipboardItem::new_string(text));
    }
    pub(super) fn focused_editor(&self, editing: Editing, cx: &mut Context<Self>) -> AnyElement {
        let input = match editing {
            Editing::Title => self.title_input.clone(),
            Editing::Description => self.interaction.description.clone(),
            _ => unreachable!(),
        };
        div()
            .w_full()
            .id(if editing == Editing::Title {
                "title-editor"
            } else {
                "description-editor"
            })
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, _, _| this.editing = editing),
            )
            .on_click(cx.listener(|_, _, _, cx| cx.stop_propagation()))
            .child(input)
            .layout_key(
                &self.layout,
                if editing == Editing::Title {
                    "editor.title"
                } else {
                    "editor.description"
                },
                None,
            )
    }
    pub(super) fn edit_description(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.screen != Screen::Task {
            return;
        }
        self.interaction.description.update(cx, |input, cx| {
            input.set_html(self.detail.as_ref().map_or("", |d| d.task.description.as_str()), cx)
        });
        self.restore_editor(self.interaction.description.clone(),Some(ht_core::DraftType::Description),cx);
        self.editing = Editing::Description;
        self.description_anchor.scroll_to(window, cx);
        window.focus(&self.interaction.description.focus_handle(cx), cx);
        cx.notify();
    }

    pub(super) fn show_modal(&mut self, modal: Modal, window: &mut Window, cx: &mut Context<Self>) {
        self.task_fields.summary_open = false;
        self.interaction.agents_only = false;
        if !matches!(
            modal,
            Modal::Commands | Modal::Search | Modal::LocalSearch | Modal::Boards
        ) && self.selected_task().is_none()
        {
            return;
        }
        if matches!(modal, Modal::Assign | Modal::Column)
            && self
                .selected_task()
                .and_then(|id| self.db.task(id).ok().flatten())
                .is_some_and(|t| t.status != Status::Normal)
        {
            self.error =
                Some("Unarchive this task before assigning it or changing its column".into());
            cx.notify();
            return;
        }
        if matches!(modal, Modal::Assign | Modal::Tags) {
            if let Some(worker) = &self.worker {
                let project = self
                    .selected_task()
                    .and_then(|id| self.db.task(id).ok().flatten())
                    .map_or(self.options.board, |t| t.project_id);
                let _ = worker.handle().sync_catalog(project);
            }
        }
        self.interaction
            .query
            .update(cx, |input, cx| input.set_content(String::new(), cx));
        self.interaction.modal = Some(modal);
        self.interaction.choice = 0;
        self.interaction
            .modal_scroll
            .set_offset(point(px(0.), px(0.)));
        window.focus(&self.interaction.query.focus_handle(cx), cx);
        cx.notify();
    }
    fn close_modal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.dismiss_compose_search();
        self.interaction.modal = None;
        let focus = match self.editing {
            Editing::Comment => self.comment_input.focus_handle(cx),
            Editing::Title => self.title_input.focus_handle(cx),
            Editing::Description => self.interaction.description.focus_handle(cx),
            Editing::ExistingComment => self.comment_actions.input.focus_handle(cx),
            Editing::None => self.focus.clone(),
        };
        window.focus(&focus, cx);
        cx.notify();
    }
    fn command_usage(&self) -> serde_json::Value {
        self.db
            .kv_get("command-usage")
            .ok()
            .flatten()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_else(|| serde_json::json!({}))
    }
    fn command_rows(&self) -> Vec<command_center::Command> {
        let mut rows = command_center::commands(self.screen, self.selected_task().is_some());
        self.add_comment_commands(&mut rows);
        self.add_board_commands(&mut rows);
        if self.ai_sidebar.is_some() {
            for (key, name, action) in [
                ("pinAiChatOpen", "Pin AI chat open", Action::PinAiChat),
                ("fullScreenAiChat", "Open AI chat full screen", Action::FullscreenAiChat),
                ("deleteAllChats", "Delete all chat sessions", Action::DeleteAllAiChats),
            ] {
                rows.push(command_center::Command { key: key.into(), name: name.into(), group: "AI", keys: String::new(), keywords: "ai chat", action: CommandAction::Shortcut(action) });
            }
        }
        if self
            .selected_task()
            .and_then(|id| self.db.task(id).ok().flatten())
            .is_some_and(|t| t.status == Status::Archive)
        {
            rows.retain(|r| {
                !matches!(
                    r.action,
                    CommandAction::Shortcut(Action::Assign | Action::MoveColumn)
                )
            });
            if let Some(row) = rows.iter_mut().find(|r| r.key == "archiveTask") {
                row.name = "Unarchive task".into();
            }
        }
        let insert = rows.first().map_or(0, |first| {
            rows.iter()
                .position(|r| r.group != first.group)
                .unwrap_or(rows.len())
        });
        let boards = self
            .db
            .projects()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|p| {
                let id = p["id"].as_i64()?;
                Some(command_center::Command {
                    key: format!("board-{id}"),
                    name: p["title"].as_str().or(p["name"].as_str())?.into(),
                    group: "Boards",
                    keys: String::new(),
                    keywords: "board project kanban navigate open",
                    action: CommandAction::Board(id),
                })
            });
        rows.splice(insert..insert, boards);
        let usage = self.command_usage();
        let now = chrono::Utc::now().timestamp_millis();
        let mut frequent: Vec<_> = rows
            .iter()
            .filter(|r| usage[&r.key]["frequency"].as_u64().unwrap_or(0) > 0)
            .cloned()
            .collect();
        frequent.sort_by(|a, b| {
            let score = |r: &command_center::Command| {
                command_center::frecency(
                    usage[&r.key]["frequency"].as_u64().unwrap_or(0),
                    usage[&r.key]["lastUsedAt"].as_i64().unwrap_or(now),
                    now,
                )
            };
            score(b).total_cmp(&score(a))
        });
        frequent.truncate(5);
        for row in &mut frequent {
            row.group = "Frequently used";
        }
        rows.splice(0..0, frequent);
        rows
    }
    pub(super) fn choices(&self, cx: &App) -> Vec<Choice> {
        let query: &str = self.interaction.query.read(cx).content.as_ref();
        match self.interaction.modal {
            Some(Modal::Commands) => {
                let commands = self.command_rows();
                command_center::ranked(
                    &commands,
                    query,
                    &self.command_usage(),
                    chrono::Utc::now().timestamp_millis(),
                )
                .into_iter()
                .map(|c| Choice {
                    name: c.name.clone(),
                    action: ChoiceAction::Command(c.action),
                })
                .collect()
            }
            Some(Modal::Search | Modal::LocalSearch) => {
                let project = (self.interaction.modal == Some(Modal::LocalSearch)
                    && self.screen == Screen::Board)
                    .then_some(self.options.board);
                self.db
                    .search_tasks(query, project)
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|t| {
                        self.interaction.modal != Some(Modal::LocalSearch)
                            || self.screen != Screen::Inbox
                            || self.inbox.iter().any(|n| n.task_id == Some(t.id))
                    })
                    .map(|t| Choice {
                        name: format!("{} {}", t.ticket_number.as_deref().unwrap_or(""), t.title),
                        action: ChoiceAction::Task(t.id),
                    })
                    .collect()
            }
            Some(Modal::Boards) => self
                .db
                .projects()
                .unwrap_or_default()
                .into_iter()
                .filter_map(|p| {
                    Some(Choice {
                        name: p["title"].as_str().or(p["name"].as_str())?.into(),
                        action: ChoiceAction::Board(p["id"].as_i64()?),
                    })
                })
                .collect(),
            Some(Modal::Column) => self
                .columns
                .iter()
                .map(|c| Choice {
                    name: c.title.clone(),
                    action: ChoiceAction::Column(c.id),
                })
                .collect(),
            Some(Modal::Assign | Modal::Tags) => {
                let Some(task) = self
                    .selected_task()
                    .and_then(|id| self.db.task(id).ok().flatten())
                else {
                    return vec![];
                };
                if self.interaction.modal == Some(Modal::Assign) {
                    self.db
                        .project_members(task.project_id)
                        .unwrap_or_default()
                        .into_iter()
                        .enumerate()
                        .filter(|(_,member)| !self.interaction.agents_only || member["id"].is_string())
                        .filter_map(|(i,m)| {
                            Some(Choice {
                                name: m["displayName"]
                                    .as_str()
                                    .filter(|s| !s.is_empty())
                                    .or(m["email"].as_str())?
                                    .into(),
                                action: if let Some(id) = m["id"].as_i64() { ChoiceAction::Assignee(id) } else { m["id"].as_str()?; ChoiceAction::Agent(i) },
                            })
                        })
                        .collect()
                } else {
                    self.db
                        .project_labels(task.project_id)
                        .unwrap_or_default()
                        .into_iter()
                        .enumerate()
                        .filter_map(|(i, l)| {
                            Some(Choice {
                                name: l["name"].as_str().or(l["value"].as_str())?.into(),
                                action: ChoiceAction::Label(i),
                            })
                        })
                        .collect()
                }
            }
            Some(Modal::Priority) => ["No Priority", "Urgent", "High", "Medium", "Low"]
                .iter()
                .enumerate()
                .map(|(i, name)| Choice {
                    name: (*name).into(),
                    action: ChoiceAction::Priority(i as u8),
                })
                .collect(),
            Some(Modal::Size) => [
                (0, "No size"),
                (2, "XS"),
                (3, "S"),
                (4, "M"),
                (5, "L"),
                (6, "XL"),
            ]
            .iter()
            .map(|&(i, name)| Choice {
                name: name.into(),
                action: ChoiceAction::Size(i),
            })
            .collect(),
            Some(Modal::DueDate) => [
                ("Today", 0),
                ("Tomorrow", 1),
                ("Next week", 7),
                ("No due date", -1),
            ]
            .iter()
            .map(|&(name, days)| Choice {
                name: name.into(),
                action: ChoiceAction::DueDate(days),
            })
            .collect(),
            Some(Modal::Delete) => vec![Choice {
                name: "Delete task".into(),
                action: ChoiceAction::Delete,
            }],
            None => vec![],
        }
        .into_iter()
        .chain((self.interaction.modal == Some(Modal::Tags) && !query.trim().is_empty()
            && self.selected_task().and_then(|id| self.db.task(id).ok().flatten()).is_some_and(|task| {
                !self.db.project_labels(task.project_id).unwrap_or_default().iter().any(|label| label["value"].as_str().or(label["name"].as_str()).is_some_and(|name|name.eq_ignore_ascii_case(query.trim())))
            })).then(|| Choice { name:format!("Create tag \"{}\"", query.trim()), action:ChoiceAction::CreateLabel }))
        .filter(|c| {
            matches!(
                self.interaction.modal,
                Some(Modal::Commands | Modal::Search | Modal::LocalSearch)
            ) || c.name.to_lowercase().contains(&query.to_lowercase())
        })
        .collect()
    }
    fn run_choice(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let choice = self.choices(cx).get(self.interaction.choice).cloned();
        let Some(choice) = choice else {
            return;
        };
        if self.interaction.modal == Some(Modal::Commands) {
            let rows = self.command_rows();
            let query: &str = self.interaction.query.read(cx).content.as_ref();
            let mut usage = self.command_usage();
            let now = chrono::Utc::now().timestamp_millis();
            if let Some(command) =
                command_center::ranked(&rows, query, &usage, now).get(self.interaction.choice)
            {
                let frequency = usage[&command.key]["frequency"].as_u64().unwrap_or(0) + 1;
                usage[&command.key] = serde_json::json!({"frequency":frequency,"lastUsedAt":now});
                let _ = self.db.kv_set("command-usage", &usage.to_string());
            }
        }
        let query = self.interaction.query.read(cx).content.trim().to_owned();
        let keep_open = matches!(self.interaction.modal,Some(Modal::Assign|Modal::Tags))
            && matches!(choice.action,ChoiceAction::Assignee(_)|ChoiceAction::Agent(_)|ChoiceAction::Label(_))
            && !(self.screen == Screen::Board && !self.interaction.bulk.is_empty());
        if !keep_open { self.close_modal(window, cx); }
        match choice.action {
            ChoiceAction::Command(CommandAction::Shortcut(action)) => {
                self.run_shortcut(action, window, cx)
            }
            ChoiceAction::Task(id) => {
                self.open_task(id, cx);
                window.focus(&self.focus, cx);
            }
            ChoiceAction::Board(id) => {
                self.options.board = id;
                self.auto_board = false;
                let _ = self.db.kv_set("last_board", &id.to_string());
                self.switch(Screen::Board, cx);
                window.focus(&self.focus, cx);
            }
            ChoiceAction::Command(CommandAction::Board(id)) => {
                self.options.board = id;
                self.auto_board = false;
                let _ = self.db.kv_set("last_board", &id.to_string());
                self.switch(Screen::Board, cx);
                window.focus(&self.focus, cx);
            }
            ChoiceAction::Command(CommandAction::Shell(action)) => {
                self.run_shell_command(action, window, cx);
            }
            ChoiceAction::Command(CommandAction::BoardForm(action)) => self.open_board_form(action, window, cx),
            ChoiceAction::CreateLabel => {
                if let Some(task) = self.selected_task().and_then(|id| self.db.task(id).ok().flatten()) {
                    self.enqueue(Write::Board { change: ht_core::BoardWrite::CreateTaskLabel { project:task.project_id, task:task.id, name:query } }, cx);
                }
            }
            ChoiceAction::Command(CommandAction::Theme(theme)) => {
                self.apply_named_theme(theme, cx);
            }
            _ => {
                let targets: Vec<_> = if self.screen == Screen::Board
                    && !self.interaction.bulk.is_empty()
                    && matches!(
                        choice.action,
                        ChoiceAction::Column(_)
                            | ChoiceAction::Assignee(_) | ChoiceAction::Agent(_)
                            | ChoiceAction::Label(_)
                    ) {
                    self.interaction.bulk.iter().copied().collect()
                } else {
                    self.selected_task().into_iter().collect()
                };
                for task_id in targets {
                    let write = match choice.action {
                        ChoiceAction::Column(section_id) => Write::TaskSection {
                            task_id,
                            section_id,
                        },
                        ChoiceAction::Agent(index) => {
                            let Some(task) = self.db.task(task_id).ok().flatten() else { return; };
                            let members = self.db.project_members(task.project_id).unwrap_or_default();
                            let Some(agent) = members.get(index).and_then(|member|member["id"].as_str()) else { return; };
                            let active = !task.extra.get("assignees").and_then(serde_json::Value::as_array).into_iter().flatten().any(|row|row["agent"]["id"] == agent);
                            Write::Board { change: ht_core::BoardWrite::AssignAgent { task:task_id, agent:agent.into(), active } }
                        }
                        ChoiceAction::Assignee(id) => {
                            let Some(task) = self.db.task(task_id).ok().flatten() else {
                                return;
                            };
                            let mut ids: Vec<_> = task
                                .extra
                                .get("assignees")
                                .unwrap_or(&serde_json::Value::Null)
                                .as_array()
                                .into_iter()
                                .flatten()
                                .filter(|m|m["agent"].is_null())
                                .filter_map(|m| m["id"].as_i64())
                                .collect();
                            if ids.contains(&id) {
                                ids.retain(|v| *v != id);
                            } else {
                                ids.push(id);
                            }
                            Write::TaskPatch {
                                task_id,
                                patch: TaskPatch::Assignees(ids),
                            }
                        }
                        ChoiceAction::Label(index) => {
                            let Some(task) = self.db.task(task_id).ok().flatten() else {
                                return;
                            };
                            let labels =
                                self.db.project_labels(task.project_id).unwrap_or_default();
                            let Some(id) = labels.get(index).and_then(|l| l["id"].as_str()) else {
                                return;
                            };
                            let mut ids: Vec<_> = task
                                .extra
                                .get("labels")
                                .unwrap_or(&serde_json::Value::Null)
                                .as_array()
                                .into_iter()
                                .flatten()
                                .filter_map(|l| l["id"].as_str().map(str::to_owned))
                                .collect();
                            if ids.iter().any(|v| v == id) {
                                ids.retain(|v| v != id);
                            } else {
                                ids.push(id.to_owned());
                            }
                            Write::TaskPatch {
                                task_id,
                                patch: TaskPatch::Labels(ids),
                            }
                        }
                        ChoiceAction::Priority(i) => Write::TaskPatch {
                            task_id,
                            patch: TaskPatch::Priority(i),
                        },
                        ChoiceAction::Size(i) => Write::TaskPatch {
                            task_id,
                            patch: TaskPatch::Estimate(i),
                        },
                        ChoiceAction::DueDate(days) => {
                            let date = (days >= 0).then(|| {
                                (chrono::Utc::now().date_naive() + chrono::Duration::days(days))
                                    .and_hms_opt(0, 0, 0)
                                    .unwrap()
                                    .and_utc()
                                    .to_rfc3339()
                            });
                            Write::TaskPatch {
                                task_id,
                                patch: TaskPatch::DueDate(date),
                            }
                        }
                        ChoiceAction::Delete => Write::TaskPatch {
                            task_id,
                            patch: TaskPatch::Status(Status::Deleted),
                        },
                        ChoiceAction::Command(_) | ChoiceAction::CreateLabel
                        | ChoiceAction::Task(_)
                        | ChoiceAction::Board(_) => {
                            unreachable!()
                        }
                    };
                    if !self.enqueue(write, cx) {
                        return;
                    }
                    if matches!(choice.action, ChoiceAction::Delete) {
                        self.interaction.bulk.remove(&task_id);
                    }
                    if matches!(choice.action, ChoiceAction::Column(_))
                        && self.screen == Screen::Board
                        && self.interaction.bulk.is_empty()
                    {
                        if let Some((column, row)) =
                            self.columns.iter().enumerate().find_map(|(c, column)| {
                                column
                                    .cards
                                    .iter()
                                    .position(|t| t.id == task_id)
                                    .map(|row| (c, row))
                            })
                        {
                            self.column = column;
                            self.selected = row;
                            self.board_scroll.scroll_to_item(column);
                        }
                    }
                }
                if matches!(choice.action, ChoiceAction::Delete) && self.screen == Screen::Task {
                    self.switch(self.previous, cx);
                    window.focus(&self.focus, cx);
                }
            }
        }
    }

    pub(super) fn interaction_overlay(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(modal) = self.interaction.modal else {
            return div().into_any_element();
        };
        let t = self.colors;
        let (background, selected, keycap) = match self.options.theme {
            ThemeName::Porcelain => (0xffffff, 0xe2e2e6, 0xececef),
            ThemeName::Graphite => (0x2c2f36, 0x3a3d45, 0x363b44),
            ThemeName::Amoled => (0x141414, 0x303239, 0x2d2d2d),
            ThemeName::Dia => (0xffffff, 0xc9e3d2, 0xf1eee7),
        };
        let query: &str = self.interaction.query.read(cx).content.as_ref();
        let commands = self.command_rows();
        let filtered = command_center::ranked(
            &commands,
            query,
            &self.command_usage(),
            chrono::Utc::now().timestamp_millis(),
        );
        let choices = self.choices(cx);
        let mut last_group = String::new();
        let mut list = div()
            .id("command-results")
            .max_h(px(364.))
            .overflow_y_scroll()
            .track_scroll(&self.interaction.modal_scroll)
            .pb(px(6.));
        for (i, choice) in choices.into_iter().enumerate() {
            let group = if modal == Modal::Commands && !query.trim().is_empty() {
                "Results"
            } else if modal == Modal::Commands {
                filtered[i].group
            } else {
                match modal {
                    Modal::Search => "Search cached tasks",
                    Modal::LocalSearch => "Search this view",
                    Modal::Boards => "Switch boards",
                    Modal::Column => "Move task to column",
                    Modal::Assign => "Assign task",
                    Modal::Tags => "Set tags",
                    Modal::Priority => "Set priority",
                    Modal::Size => "Set task size",
                    Modal::DueDate => "Set due date",
                    Modal::Delete => "Delete task?",
                    Modal::Commands => unreachable!(),
                }
            };
            if group != last_group {
                last_group = group.into();
                list = list.child(
                    div()
                        .px(px(16.))
                        .pt(px(10.))
                        .pb(px(4.))
                        .text_size(px(11.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(color(t.muted))
                        .child(group.to_uppercase()),
                );
            }
            let keys = if modal == Modal::Commands {
                filtered[i].keys.as_str()
            } else {
                ""
            };
            let checked = self.selected_task().and_then(|id|self.db.task(id).ok().flatten()).is_some_and(|task| match choice.action {
                ChoiceAction::Column(id) => task.section_id == Some(id),
                ChoiceAction::Priority(index) => task.extra.get("priority").is_some_and(|p|p["priority_index"] == index),
                ChoiceAction::Size(index) => task.extra.get("estimate").is_some_and(|p|p["estimate_index"] == index),
                ChoiceAction::Assignee(id) => task.extra.get("assignees").and_then(serde_json::Value::as_array).into_iter().flatten().any(|row| row["agent"].is_null() && row["id"] == id),
                ChoiceAction::Agent(index) => self.db.project_members(task.project_id).ok().and_then(|rows|rows.get(index).cloned()).is_some_and(|member|task.extra.get("assignees").and_then(serde_json::Value::as_array).into_iter().flatten().any(|row|row["agent"]["id"] == member["id"])),
                ChoiceAction::Label(index) => self.db.project_labels(task.project_id).ok().and_then(|rows|rows.get(index).cloned()).is_some_and(|label|task.extra.get("labels").and_then(serde_json::Value::as_array).into_iter().flatten().any(|row|row["id"] == label["id"])),
                _ => false,
            });
            list = list.child(
                div()
                    .id(("command-choice", i))
                    .h(px(36.))
                    .mx(px(6.))
                    .px(px(12.))
                    .rounded(px(2.))
                    .flex()
                    .items_center()
                    .gap(px(12.))
                    .cursor_pointer()
                    .text_size(px(13.))
                    .when(i == self.interaction.choice, |d| d.bg(color(selected)))
                    .hover(|d| d.bg(color(selected)))
                    .on_mouse_move(cx.listener(move |this, _, _, cx| {
                        if this.interaction.choice != i {
                            this.interaction.choice = i;
                            cx.notify();
                        }
                    }))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        cx.stop_propagation();
                        this.interaction.choice = i;
                        this.run_choice(window, cx);
                    }))
                    .child(div().flex_1().child(choice.name.clone()))
                    .when(checked, |d| d.child("✓"))
                    .child(
                        div().flex().items_center().gap(px(3.)).children(
                            keys.split(' ')
                                .filter(|key| {
                                    !keys.is_empty() && (!key.is_empty() || keys.contains("  "))
                                })
                                .map(|key| {
                                    if key.is_empty() {
                                        div()
                                            .text_size(px(11.))
                                            .text_color(color(t.muted))
                                            .child("then")
                                            .into_any_element()
                                    } else {
                                        div()
                                            .px(px(5.))
                                            .py(px(2.))
                                            .rounded(px(2.))
                                            .bg(color(keycap))
                                            .text_size(px(11.))
                                            .font_weight(FontWeight::MEDIUM)
                                            .child(if cfg!(target_os = "macos") && key == "CTRL" {
                                                "CMD".to_owned()
                                            } else {
                                                key.to_owned()
                                            })
                                            .into_any_element()
                                    }
                                }),
                        ),
                    )
                    .layout_key(
                        &self.layout,
                        format!("command.choice.{i}"),
                        Some(choice.name),
                    ),
            );
        }
        div()
            .id("interaction-backdrop")
            .absolute()
            .size_full()
            .top_0()
            .left_0()
            .bg(rgba(0x00000080))
            .flex()
            .justify_center()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|_, _, _, cx| cx.stop_propagation()),
            )
            .on_click(cx.listener(|this, _, window, cx| this.close_modal(window, cx)))
            .child(
                div()
                    .id("command-center")
                    .absolute()
                    .top(px(f32::from(window.viewport_size().height) * 0.24))
                    .w(px(560.))
                    .rounded(px(5.))
                    .bg(color(background))
                    .shadow(shadow())
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .on_click(cx.listener(|_, _, _, cx| cx.stop_propagation()))
                    .when(self.compose_open(), |d| d.child(self.compose_tabs(cx)))
                    .child(
                        div()
                            .h(px(44.))
                            .px(px(16.))
                            .border_b_1()
                            .border_color(rgba(0x0000001a))
                            .flex()
                            .items_center()
                            .gap(px(10.))
                            .child(icon("search", t.muted, 13.))
                            .child(
                                div()
                                    .flex_1()
                                    .child(self.interaction.query.clone())
                                    .layout_key(&self.layout, "command.input", None),
                            ),
                    )
                    .child(list)
                    .child(
                        div()
                            .px(px(16.))
                            .py(px(8.))
                            .border_t_1()
                            .border_color(rgba(0x0000001a))
                            .text_size(px(11.))
                            .text_color(color(t.muted))
                            .flex()
                            .items_center()
                            .gap(px(16.))
                            .children(
                                [("↑↓", "navigate"), ("↵", "select"), ("esc", "close")]
                                    .into_iter()
                                    .map(|(key, label)| {
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(4.))
                                            .child(
                                                div()
                                                    .px(px(5.))
                                                    .py(px(2.))
                                                    .rounded(px(2.))
                                                    .bg(color(keycap))
                                                    .text_color(color(t.text))
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .line_height(px(11.))
                                                    .child(key),
                                            )
                                            .child(label)
                                    }),
                            ),
                    )
                    .child(
                        div()
                            .px(px(16.))
                            .pb(px(8.))
                            .text_size(px(11.))
                            .text_color(color(t.muted))
                            .child(concat!("v ", env!("CARGO_PKG_VERSION"))),
                    )
                    .layout_key(&self.layout, "command.center", None),
            )
            .into_any_element()
    }
}
