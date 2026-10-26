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
    BlockedBy,
    Tags,
    Size,
    DueDate,
    Delete,
    RemoveSubtask,
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
    WaitingOn(Option<i64>),
    Agent(usize),
    Label(usize),
    Size(u8),
    DueDate(i64),
    Task(i64),
    Board(i64),
    Delete,
    CreateLabel,
    RemoveSubtask(i64),
}
pub(super) struct Interaction {
    pub modal: Option<Modal>,
    pub agents_only: bool,
    pub command_task: bool,
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
            let mut input = TextInput::new(
                String::new(),
                "Type a command or search...",
                color(0x8e9093),
                cx,
            );
            input.line_height = 21.;
            input
        });
        cx.observe(&query, |this, input, cx| {
            if this.interaction.modal == Some(Modal::Search) {
                this.request_search(input.read(cx).content.to_string());
            }
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
            command_task: false,
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
        if matches!(self.interaction.modal, Some(Modal::Priority | Modal::Size))
            && !ctrl && !key.modifiers.alt && !key.modifiers.shift
            && key.key.len() == 1 && key.key.as_bytes()[0].is_ascii_digit()
        {
            let index = (key.key.as_bytes()[0] - b'0') as usize;
            let action = match self.interaction.modal {
                Some(Modal::Priority) if index < 5 => Some(ChoiceAction::Priority(index as u8)),
                // Web digits index the catalog, not estimate_index (1 selects XS/2).
                Some(Modal::Size) => [0, 2, 3, 4, 5, 6].get(index).copied().map(ChoiceAction::Size),
                _ => None,
            };
            if let Some(action) = action {
                self.apply_choice(Choice { name: String::new(), action }, window, cx);
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
                let shift = if self.screen == Screen::Task { shortcuts::shifted(key) } else { shift };
                let action = shortcuts::sequence(&key.key, shift, self.screen);
                if let Some(action) = action {
                    if action == Action::Inbox {
                        self.split = 0;
                    }
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
            CreateTask => {
                if self.screen == Screen::Board && self.columns.is_empty() {
                    self.open_board_form(crate::command_center::BoardAction::AddColumn, window, cx);
                }
            }
            EditTaskPart => {
                if self.interaction.task_part == 0 {
                    self.edit_description(window, cx);
                } else if self.interaction.task_part <= self.comments.len() {
                    self.run_comment_action(EditComment, window, cx);
                } else {
                    self.edit_comment(window, cx);
                }
            }
            BlockedBy => self.show_modal(Modal::BlockedBy, window, cx),
            ToggleTimer => self.toggle_task_timer(window, cx),
            StarTask => {
                if let Some(task) = self.selected_task() {
                    match self.db.task(task) {
                        Ok(Some(row)) if row.extra.get("savedContent").is_some_and(|saved| saved.is_array()) => {
                            let active = row.extra["savedContent"].as_array().unwrap().is_empty();
                            {
                                let project = row.project_id;
                                self.enqueue(Write::Board { change: ht_core::BoardWrite::StarTask { task, project, active } }, cx);
                            }
                        }
                        _ => self.error = Some("Sync this task before changing its star".into()),
                    }
                }
            }
            SummarizeTask => {
                if let Some(sidebar) = &self.ai_sidebar {
                    sidebar.update(cx, |sidebar, cx| sidebar.command_prompt("Summarize this ticket, its description and the key points from the comments, concisely.", None, window, cx));
                    self.shell.ai_open = true;
                }
            }
            ExpandSummary => {
                if self.detail.as_ref().is_some_and(|d| !full_task_summary(&d.task.extra).is_empty()) {
                    self.task_fields.summary_open = !self.task_fields.summary_open;
                }
            }
            PinAiChat | FullscreenAiChat => self.ai_command(action, cx),
            StarComment | PinComment | CopyCommentContent | CopyCommentLink | CopyCommentToAiChat | BranchComment | SummarizeComment | ReplyComment | EditComment | DeleteComment | ReactComment | LikeComment
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
                if self.editing == Editing::Description {
                    self.save_edit(window, cx);
                } else if self.editing != Editing::None {
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
            First | Last if self.screen == Screen::Task => {
                if action == First {
                    self.interaction.task_part = 0;
                    self.description_anchor.scroll_to(window, cx);
                    window.focus(&self.focus, cx);
                } else {
                    self.interaction.task_part = self.comments.len() + 1;
                    self.composer_anchor.scroll_to(window, cx);
                }
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
        self.interaction.command_task = false;
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
        if matches!(modal, Modal::Assign | Modal::Tags | Modal::BlockedBy) {
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
        if modal == Modal::Search { self.request_search(String::new()); }
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
        let context = if self.interaction.command_task { Screen::Task } else { self.screen };
        let mut rows = command_center::commands(context, self.selected_task().is_some());
        self.add_comment_commands(&mut rows);
        self.add_board_commands(&mut rows);
        if self.ai_sidebar.is_some() {
            for (key, name, group, keys, keywords, action) in [
                ("appShellAIChat", "App shell: Toggle AI chat", "App shell surfaces", "5", "surface switch toggle ai chat", CommandAction::Shell(command_center::ShellCommand::ToggleAi)),
                ("appShellAIChatAlternative", "App shell: Toggle AI chat (alternative)", "App shell surfaces", "]", "surface switch toggle ai chat bracket alternative", CommandAction::Shell(command_center::ShellCommand::ToggleAi)),
                ("aiChatInterface", "Chat with AI", "AI", "", "chat ask ai assistant bot conversation answer help discuss", CommandAction::Shell(command_center::ShellCommand::ToggleAi)),
                ("pinAiChatOpen", "Pin AI chat open", "AI", "", "pin keep chat open always", CommandAction::Shortcut(Action::PinAiChat)),
                ("fullScreenAiChat", "Open AI chat full screen", "AI", "", "full screen chat fullscreen expand ai big large page", CommandAction::Shortcut(Action::FullscreenAiChat)),
                ("toggleAIChatView", "Switch AI chat mode", "AI", "", "switch toggle ai chat mode sidebar floating view layout", CommandAction::Shell(command_center::ShellCommand::ToggleAiMode)),
            ] {
                rows.push(command_center::Command { key: key.into(), name: name.into(), group, keys: keys.into(), keywords, action });
            }
        }
        if self.screen == Screen::Task && !self.task_inbox_ids().is_empty() {
            rows.push(command_center::Command { key: "removeTaskNotification".into(), name: "Remove task notification".into(), group: "Task", keys: "E".into(), keywords: "remove notification mute", action: CommandAction::Shortcut(Action::ArchiveNotification) });
        }
        if self.selected_task().and_then(|id| self.db.task(id).ok().flatten())
            .is_some_and(|t| t.status == Status::Archive)
        {
            rows.retain(|row| !matches!(row.action, CommandAction::Shortcut(Action::Assign | Action::MoveColumn)));
            if let Some(row) = rows.iter_mut().find(|r| r.key == "archiveTask") {
                row.name = "Unarchive task".into();
            }
        }
        if context == Screen::Task && self.selected_task().and_then(|id| self.db.task(id).ok().flatten())
            .is_some_and(|task| !task.title.is_empty() && task.ticket_number.as_deref().is_some_and(|ticket| !ticket.is_empty()))
        {
            rows.push(command_center::Command { key: "copyBranchName".into(), name: "Copy branch name".into(), group: "Task", keys: String::new(), keywords: "git branch name copy github", action: CommandAction::Shell(command_center::ShellCommand::CopyBranchName) });
        }
        if context == Screen::Task && self.interaction.bulk.is_empty()
            && let Some(task) = self.selected_task().and_then(|id| self.db.task(id).ok().flatten())
            && task.status == Status::Normal
            && let Some(user) = self.db.current_user_id()
            && self.db.project_members(task.project_id).unwrap_or_default().iter().any(|member| member["id"] == user)
        {
            rows.push(command_center::Command { key: "assignToMe".into(), name: "Assign to me".into(), group: "Task", keys: String::new(), keywords: "assign me self take claim mine my task i'll do it", action: CommandAction::Shell(command_center::ShellCommand::AssignToMe) });
        }
        if context == Screen::Task && self.interaction.bulk.is_empty()
            && self.db.current_user_id().is_some()
            && let Some(task) = self.selected_task().and_then(|id| self.db.task(id).ok().flatten())
        {
            rows.push(command_center::Command { key: "showInInbox".into(), name: "Move task to inbox".into(), group: "Task", keys: String::new(), keywords: "inbox activity move notifications", action: CommandAction::Shell(command_center::ShellCommand::MoveToInbox) });
            rows.push(command_center::Command { key: "blockedByPerson".into(), name: "Blocked by person…".into(), group: "Task", keys: "SHIFT B".into(), keywords: "blocked waiting blocker block person stuck", action: CommandAction::Shortcut(Action::BlockedBy) });
            if task.status == Status::Normal {
                rows.push(command_center::Command { key: "acceptTask".into(), name: "Accept task".into(), group: "Task", keys: String::new(), keywords: "accept task triage approve backlog ready next column intake", action: CommandAction::Shell(command_center::ShellCommand::AcceptTask) });
                rows.push(command_center::Command { key: "declineTask".into(), name: "Decline task".into(), group: "Task", keys: String::new(), keywords: "decline task triage reject dismiss archive intake", action: CommandAction::Shortcut(Action::Archive) });
            }
            if let Some(saved) = task.extra.get("savedContent").and_then(serde_json::Value::as_array) {
                rows.push(command_center::Command { key: "starTask".into(), name: if saved.is_empty() { "Star task" } else { "Unstar task" }.into(), group: "Task", keys: "ALT S".into(), keywords: "star favorite bookmark", action: CommandAction::Shortcut(Action::StarTask) });
            }
            if task.extra.get("parentTaskId").and_then(serde_json::Value::as_i64).is_some_and(|id| id > 0) {
                rows.push(command_center::Command { key: "removeParenttask".into(), name: "Remove as sub-task".into(), group: "Task", keys: String::new(), keywords: "remove parent unlink", action: CommandAction::Shell(command_center::ShellCommand::RemoveParent) });
            }
            if self.screen == Screen::Task && self.ai_sidebar.is_some() {
                rows.push(command_center::Command { key: "summarizeTicket".into(), name: "Summarize ticket".into(), group: "Task", keys: String::new(), keywords: "summarize summary tldr task ticket concise recap", action: CommandAction::Shortcut(Action::SummarizeTask) });
            }
            let tracking = task.extra.get("project").and_then(|project| project.get("timeTrackingEnabled")).and_then(serde_json::Value::as_bool)
                .or_else(|| self.db.projects().ok()?.into_iter().find(|project| project["id"] == task.project_id)?["timeTrackingEnabled"].as_bool())
                .or_else(|| self.db.kv_get(&format!("timer-state:{}", task.id)).ok().flatten()
                    .and_then(|state| serde_json::from_str::<serde_json::Value>(&state).ok())?["timer_enabled"].as_bool());
            if tracking == Some(true) {
                rows.push(command_center::Command { key: "toggleTimeTracking".into(), name: "Start or stop timer".into(), group: "Time", keys: "W".into(), keywords: "start stop toggle time tracking timer work log", action: CommandAction::Shortcut(Action::ToggleTimer) });
            }
        }
        if context == Screen::Task && self.removable_subtasks().is_some_and(|rows| !rows.is_empty()) {
            rows.push(command_center::Command { key: "removeSubtask".into(), name: "Remove a sub-task".into(), group: "Task", keys: String::new(), keywords: "remove delete subtask", action: CommandAction::Shell(command_center::ShellCommand::RemoveSubtask) });
        }
        command_center::order(&mut rows, context);
        let context_groups = if rows.first().is_some_and(|row| row.group == "Comment") { 2 } else { 1 };
        let mut groups = 0;
        let insert = rows.windows(2).position(|pair| {
            if pair[0].group != pair[1].group { groups += 1; }
            groups == context_groups
        }).map_or(rows.len(), |i| i + 1);
        let boards = self.db.projects().unwrap_or_default().into_iter()
            .filter(|p| p["id"].as_i64() != Some(self.options.board))
            .take(50)
            .filter_map(|p| {
                let id = p["id"].as_i64()?;
                Some(command_center::Command {
                    key: format!("gotoBoard-{id}"),
                    name: format!("Go to board: {}", p["title"].as_str().or(p["name"].as_str())?),
                    group: "Boards",
                    keys: String::new(),
                    keywords: "board project go open switch",
                    action: CommandAction::Board(id),
                })
            });
        rows.splice(insert..insert, boards);
        let usage = self.command_usage();
        let now = chrono::Utc::now().timestamp_millis();
        let score = |r: &command_center::Command| {
            command_center::frecency(
                usage[&r.key]["frequency"].as_u64().unwrap_or(0),
                usage[&r.key]["lastUsedAt"].as_i64().unwrap_or(now),
                now,
            )
        };
        let mut start = 0;
        while start < rows.len() {
            let end = start + rows[start..].iter().take_while(|r| r.group == rows[start].group).count();
            if !matches!(rows[start].group, "Comment" | "Selected tasks") {
                rows[start..end].sort_by(|a, b| score(b).total_cmp(&score(a)));
            }
            start = end;
        }
        if context == Screen::Task || !self.interaction.bulk.is_empty() {
            return rows;
        }
        let mut frequent: Vec<_> = rows.iter().filter(|r| usage[&r.key]["frequency"].as_u64().unwrap_or(0) > 0).cloned().collect();
        frequent.sort_by(|a, b| score(b).total_cmp(&score(a)));
        frequent.truncate(5);
        for row in &mut frequent { row.group = "Frequently used"; }
        if frequent.is_empty() {
            for key in ["createTask", "createTaskWithAiWriter", "inviteBoard", "createBoard", "addColumn"] {
                if let Some(mut row) = rows.iter().find(|row| row.key == key).cloned() {
                    row.group = "Get started";
                    frequent.push(row);
                }
            }
        }
        rows.splice(0..0, frequent);
        rows
    }
    fn removable_subtasks(&self) -> Option<Vec<Choice>> {
        self.db.current_user_id()?;
        let task = self.selected_task().and_then(|id| self.db.task(id).ok().flatten())?;
        let children = task.extra.get("sub_tasks").or_else(|| task.extra.get("subTasks"))?.as_array()?;
        let mut choices = Vec::new();
        for child in children {
            let id = child["id"].as_i64().filter(|id| *id > 0)?;
            let cached = self.db.task(id).ok().flatten()?;
            if cached.extra.get("parentTaskId").and_then(serde_json::Value::as_i64) != Some(task.id) { continue; }
            choices.push(Choice {
                name: format!("{} {}", child["ticketNumber"].as_str().unwrap_or(""), child["title"].as_str()?).trim().into(),
                action: ChoiceAction::RemoveSubtask(id),
            });
        }
        Some(choices)
    }
    fn filtered_commands(&self, cx: &App) -> Vec<command_center::Command> {
        let commands = self.command_rows();
        let query: &str = self.interaction.query.read(cx).content.as_ref();
        let mut rows = command_center::ranked(&commands, query, &self.command_usage(), chrono::Utc::now().timestamp_millis())
            .into_iter().cloned().collect::<Vec<_>>();
        if !query.trim().is_empty() {
            rows.push(command_center::Command { key: "searchTasksHandover".into(), name: format!("Search tasks: \"{query}\""), group: "Results", keys: String::new(), keywords: "", action: CommandAction::Shell(command_center::ShellCommand::SearchTasks) });
        }
        rows
    }
    pub(super) fn choices(&self, cx: &App) -> Vec<Choice> {
        let query: &str = self.interaction.query.read(cx).content.as_ref();
        match self.interaction.modal {
            Some(Modal::Commands) => {
                self.filtered_commands(cx).into_iter()
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
                self.searched_tasks(query, project)
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
            Some(Modal::RemoveSubtask) => self.removable_subtasks().unwrap_or_default(),
            Some(Modal::BlockedBy) => std::iter::once(Choice { name: "Nobody (clear)".into(), action: ChoiceAction::WaitingOn(None) })
                .chain(self.db.project_members(self.selected_task().and_then(|id| self.db.task(id).ok().flatten()).map_or(self.options.board, |t| t.project_id)).unwrap_or_default().iter()
                    .filter(|member| member["agent"].is_null() && member["id"].is_i64())
                    .filter_map(|member| Some(Choice { name: member["displayName"].as_str().or(member["email"].as_str())?.into(), action: ChoiceAction::WaitingOn(member["id"].as_i64()) })))
                .collect(),
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
        self.apply_choice(choice, window, cx);
    }
    fn apply_choice(&mut self, choice: Choice, window: &mut Window, cx: &mut Context<Self>) {
        if self.interaction.modal == Some(Modal::Commands) {
            let rows = self.filtered_commands(cx);
            let mut usage = self.command_usage();
            let now = chrono::Utc::now().timestamp_millis();
            if let Some(command) = rows.get(self.interaction.choice) {
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
            ChoiceAction::WaitingOn(user) => {
                if let Some(task) = self.selected_task() { self.enqueue(Write::Board { change: ht_core::BoardWrite::WaitingOn { task, user } }, cx); }
            }
            ChoiceAction::RemoveSubtask(task_id) => {
                if self.removable_subtasks().is_some_and(|rows| rows.iter().any(|row| matches!(row.action, ChoiceAction::RemoveSubtask(id) if id == task_id))) {
                    self.enqueue(Write::TaskPatch { task_id, patch: TaskPatch::Parent(None) }, cx);
                }
            }
            ChoiceAction::Command(CommandAction::Shortcut(Action::Search)) => self.navigate_search(window, cx),
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
                        | ChoiceAction::Board(_) | ChoiceAction::WaitingOn(_) | ChoiceAction::RemoveSubtask(_) => {
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
        let filtered = self.filtered_commands(cx);
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
                    Modal::Search => "Search all tasks",
                    Modal::LocalSearch => "Search this view",
                    Modal::Boards => "Switch boards",
                    Modal::Column => "Move task to column",
                    Modal::Assign => "Assign task",
                    Modal::BlockedBy => "Blocked by person",
                    Modal::Tags => "Set tags",
                    Modal::Priority => "Set priority",
                    Modal::Size => "Set task size",
                    Modal::DueDate => "Set due date",
                    Modal::Delete => "Delete task?",
                    Modal::RemoveSubtask => "Remove a sub-task",
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
                        .line_height(px(16.5))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(color(t.muted))
                        .child(group.to_uppercase())
                        .layout_key(&self.layout, format!("command.group.{i}"), Some(group.into())),
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
                    .line_height(px(19.5))
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
                    .child(div().flex_1().flex().items_center().child(choice.name.clone())
                        .when(modal == Modal::Commands && (matches!(filtered[i].key.as_str(), "summarizeTicket" | "summarizeComment" | "copyCommentToAiChat" | "assignToMe" | "cliInstall") || (filtered[i].key == "branchInNewChat" && filtered[i].group == "Comment")), |d| d.child(
                            div().ml(px(8.)).px(px(5.)).py(px(2.)).rounded(px(2.))
                                .bg(color(0x4455bb)).text_color(color(0xffffff)).text_size(px(11.))
                                .font_weight(FontWeight::MEDIUM).line_height(px(11.)).child("New")
                                .layout_key(&self.layout, format!("command.new.{i}"), Some("New".into())))))
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
                                            .line_height(px(11.))
                                            .text_color(color(t.text))
                                            .child(if cfg!(target_os = "macos") && key == "CTRL" {
                                                "CMD".to_owned()
                                            } else if cfg!(target_os = "macos") && key == "ALT" {
                                                "OPT".to_owned()
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
                    .left(px(((f32::from(window.viewport_size().width) - if self.shell.ai_open {
                        self.ai_sidebar.as_ref().filter(|sidebar| sidebar.read(cx).mode() == crate::ai_sidebar::DisplayMode::Sidebar)
                            .map_or(0., |sidebar| sidebar.read(cx).width())
                    } else { 0. } - 560.) / 2.).max(0.)))
                    .w(px(560.))
                    .font_family(self.options.theme.font())
                    .text_color(color(t.text))
                    .text_size(px(14.))
                    .line_height(px(21.))
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
                            .child(icon("command-search", t.muted, 13.).layout_key(&self.layout, "command.search.icon", None))
                            .child(
                                div()
                                    .flex_1()
                                    .child(self.interaction.query.clone())
                                    .layout_key(&self.layout, "command.input", None),
                            ),
                    )
                    .when(self.interaction.modal == Some(Modal::Search) && self.search_load.is_some(), |d| d.child(
                        div().px(px(16.)).py(px(8.)).child("Loading tasks…").layout_key(&self.layout, "command.loading", Some("Loading tasks…".into()))))
                    .child(list.layout_key(&self.layout, "command.results", None))
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
                            .child(crate::version()),
                    )
                    .layout_key(&self.layout, "command.center", None),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod cmdk_parity_tests {
    use super::*;
    use crate::ui::task_fields::tests::desktop;

    #[gpui::test]
    fn cmdk_task_commands_match_web_order_labels_and_groups(cx: &mut gpui::TestAppContext) {
        cx.update(crate::input::init);
        let view = cx.open_window(gpui::size(px(1440.), px(900.)), desktop);
        view.update(cx, |this, _, _| {
            assert!(this.selected_task().is_some());
            let rows = this.command_rows();
            let task: Vec<_> = rows.iter().filter(|row| row.group == "Task")
                .map(|row| (row.key.as_str(), row.name.as_str(), row.keys.as_str())).collect();
            assert_eq!(task, vec![
                ("acceptTask", "Accept task", ""),
                ("declineTask", "Decline task", ""),
                ("assignUser", "Assign task", "A"),
                ("blockedByPerson", "Blocked by person…", "SHIFT B"),
                ("setDueDate", "Set due date", "D"),
                ("priorityModal", "Set priority", "P"),
                ("labelmodal", "Set tags", "T"),
                ("estimateModal", "Set task size", "S"),
                ("moveToColumn", "Move task to column", "M"),
                ("movetasktodifferentboard", "Move task to board", "SHIFT M"),
                ("archiveTask", "Archive task", "CTRL E"),
                ("copyformattedTaskLink", "Copy private task link (title + URL)", "CTRL :"),
                ("copyLink", "Copy private task URL", "CTRL SHIFT :"),
                ("copyTaskId", "Copy task ID", "CTRL SHIFT I"),
                ("copyTaskTitleId", "Copy task title and ID", "CTRL I"),
                ("copyBranchName", "Copy branch name", ""),
                ("toggleAllComments", "Expand / collapse all comments", "SHIFT O"),
                ("renameTask", "Rename task", ""),
                ("showInInbox", "Move task to inbox", ""),
                ("deleteTask", "Delete task", "SHIFT 3"),
                ("summarizeTicket", "Summarize ticket", ""),
            ]);
        }).unwrap();
    }

    #[gpui::test]
    fn cmdk_board_and_inbox_groups_have_complete_web_sequences(cx: &mut gpui::TestAppContext) {
        cx.update(crate::input::init);
        let view = cx.open_window(gpui::size(px(1440.), px(900.)), desktop);
        view.update(cx, |this, _, _| {
            for screen in [Screen::Inbox, Screen::Board] {
                this.screen = screen;
                this.reload();
                let rows = this.command_rows();
                let keys = |group| rows.iter().filter(|row| row.group == group)
                    .map(|row| row.key.as_str()).collect::<Vec<_>>();
                assert_eq!(keys("Navigate"), ["copyCurrentPageUrl", "GotoInbox", "searchTask"]);
                assert_eq!(keys("App shell surfaces"), ["appShellInbox", "appShellBoard", "appShellAIChat", "appShellAIChatAlternative"]);
                assert_eq!(keys("AI"), ["aiChatInterface", "pinAiChatOpen", "fullScreenAiChat", "toggleAIChatView"]);
                assert_eq!(keys("Appearance"), ["systemTheme", "lightTheme", "darkTheme", "amoledTheme", "paperTheme"]);
                assert_eq!(keys("Settings"), ["settingsCalendarShowWeekends", "settingsLearnQuickTips", "settingsGeneral", "settingsAppearance", "settingsSkills", "settingsBoardMembers", "settingsBoardAi", "settingsBoardSkills", "settingsBoardFiles", "settingsCli", "settingsShortcuts", "settingsLearn"]);
                assert_eq!(keys("Help"), ["shortcuts", "settings", "boardSettings", "helpCenter", "quickTips", "cliInstall"]);
                assert!(keys("Task").is_empty());
                if screen == Screen::Board {
                    assert_eq!(keys("Board"), ["createTaskWithAiWriter", "sendFeedback", "renameColumn", "addColumn", "deleteColumn", "inviteBoard", "manageMembers", "renameBoard", "archiveBoards", "toggleRailExpanded"]);
                    assert_eq!(keys("Get started"), ["createTaskWithAiWriter", "inviteBoard", "addColumn"]);
                } else {
                    assert_eq!(keys("Inbox"), ["swipeThroughUnread"]);
                    assert_eq!(keys("Board"), ["createTaskWithAiWriter", "sendFeedback", "renameColumn", "addColumn", "deleteColumn", "inviteBoard", "manageMembers", "renameBoard", "archiveBoards", "toggleRailExpanded"]);
                    assert_eq!(keys("Get started"), ["createTaskWithAiWriter", "inviteBoard", "addColumn"]);
                }
                let mut groups = rows.iter().map(|row| row.group).collect::<Vec<_>>();
                groups.dedup();
                assert_eq!(groups, if screen == Screen::Board {
                    vec!["Get started", "Board", "Navigate", "Inbox", "App shell surfaces", "AI", "Appearance", "Settings", "Help"]
                } else {
                    vec!["Get started", "Navigate", "Inbox", "App shell surfaces", "Board", "AI", "Appearance", "Settings", "Help"]
                });
            }
        }).unwrap();
    }

    #[gpui::test]
    fn cmdk_comment_context_has_web_group_order_and_owner_actions(cx: &mut gpui::TestAppContext) {
        cx.update(crate::input::init);
        let view = cx.open_window(gpui::size(px(1440.), px(900.)), desktop);
        view.update(cx, |this, _, _| {
            this.db.apply_comments(1, &serde_json::json!({"success":true,"comments":[
                {"id":7,"text":"One","creatorId":42,"creator":{"id":42},"createdAt":"2026-01-01T00:00:00Z"},
                {"id":8,"text":"Two","creatorId":43,"creator":{"id":43},"createdAt":"2026-01-02T00:00:00Z"}
            ]})).unwrap();
            this.db.apply_task(&serde_json::json!({"id":1,"projectId":15,"title":"Needle","status":"Normal","project":{"id":15,"timeTrackingEnabled":true}})).unwrap();
            this.reload();
            let task_rows = this.command_rows();
            assert!(task_rows.iter().position(|row| row.group == "Time").unwrap() < task_rows.iter().position(|row| row.group == "Navigate").unwrap());
            for (part, own) in [(1, true), (2, false)] {
                this.interaction.task_part = part;
                let rows = this.command_rows();
                let comments = rows.iter().filter(|row| row.group == "Comment")
                    .map(|row| (row.key.as_str(), row.name.as_str(), row.keys.as_str())).collect::<Vec<_>>();
                let mut expected = vec![
                    ("summarizeComment", "Summarize comment", ""),
                    ("fastLikeComment", "Fast like", ""),
                    ("replyToComment", "Reply to comment", "ENTER"),
                    ("reactToComment", "React to comment", "R"),
                    ("pinComment", "Pin comment", "CTRL SHIFT P"),
                    ("copyCommentContent", "Copy comment content", ""),
                    ("starComment", "Star comment", "CTRL SHIFT S"),
                    ("branchInNewChat", "Branch in new chat", ""),
                    ("copyCommentToAiChat", "Copy comment to AI chat", ""),
                ];
                if own { expected.insert(0, ("editcomment", "Edit comment", "")); expected.push(("deletemessage", "Delete comment", "")); }
                assert_eq!(comments, expected);
                let mut groups = rows.iter().map(|row| row.group).collect::<Vec<_>>();
                groups.dedup();
                assert_eq!(groups, ["Comment", "Task", "Navigate", "Inbox", "Time", "App shell surfaces", "Board", "AI", "Appearance", "Settings", "Help"]);
            }
        }).unwrap();
    }

    #[gpui::test]
    fn cmdk_usage_and_bulk_recommendations_follow_web_context(cx: &mut gpui::TestAppContext) {
        cx.update(crate::input::init);
        let view = cx.open_window(gpui::size(px(1440.), px(900.)), desktop);
        view.update(cx, |this, _, _| {
            let now = chrono::Utc::now().timestamp_millis();
            this.db.kv_set("command-usage", &serde_json::json!({"copyTaskId":{"frequency":10,"lastUsedAt":now},"searchTask":{"frequency":20,"lastUsedAt":now}}).to_string()).unwrap();
            let rows = this.command_rows();
            assert_eq!(rows[0].key, "copyTaskId");
            assert!(!rows.iter().any(|row| row.group == "Frequently used"));
            let navigate = rows.iter().filter(|row| row.group == "Navigate").map(|row|row.key.as_str()).collect::<Vec<_>>();
            assert_eq!(navigate, ["searchTask", "copyCurrentPageUrl", "GotoInbox"]);
            this.screen = Screen::Board;
            this.reload();
            assert_eq!(this.command_rows()[0].group, "Frequently used");
            this.interaction.bulk.insert(1);
            let rows = this.command_rows();
            assert_eq!(rows.iter().take(4).map(|row|row.key.as_str()).collect::<Vec<_>>(), ["bulkMoveToColumn", "bulkAssignUser", "bulkSetLabels", "bulkArchive"]);
            assert!(!rows.iter().any(|row| matches!(row.group, "Get started" | "Frequently used")));
        }).unwrap();
    }

    #[gpui::test]
    fn cmdk_ai_and_sidebar_actions_run_from_keyboard_and_persist_usage(cx: &mut gpui::TestAppContext) {
        use crate::ui::shell::tests::{draw, measured, setup};
        use crate::ai_sidebar::DisplayMode;
        let (view, mut cx) = setup(cx);
        for (name, key) in [
            ("Switch AI chat mode", "toggleAIChatView"),
            ("Chat with AI", "aiChatInterface"),
            ("Pin AI chat open", "pinAiChatOpen"),
            ("Collapse / expand sidebar", "toggleRailExpanded"),
        ] {
            cx.simulate_keystrokes("ctrl-k");
            cx.simulate_input(name);
            draw(view, &mut cx);
            assert_eq!(measured(view, &mut cx, "command.choice.0")["text"], name);
            cx.simulate_keystrokes("enter");
            draw(view, &mut cx);
            view.read_with(&mut cx, |this, cx| {
                assert_eq!(this.command_usage()[key]["frequency"], 1);
                assert!(this.interaction.modal.is_none());
                let sidebar = this.ai_sidebar.as_ref().unwrap().read(cx);
                match key {
                    "toggleAIChatView" => assert_eq!(sidebar.mode(), DisplayMode::Floating),
                    "aiChatInterface" => assert!(!this.shell.ai_open),
                    "pinAiChatOpen" => assert!(this.shell.ai_open && sidebar.is_open()),
                    "toggleRailExpanded" => assert_eq!(this.shell.width(), 130.),
                    _ => unreachable!(),
                }
            }).unwrap();
        }
    }


    #[gpui::test]
    fn cmdk_notification_row_and_archived_edits_follow_native_capabilities(cx: &mut gpui::TestAppContext) {
        cx.update(crate::input::init);
        let view = cx.open_window(gpui::size(px(1440.), px(900.)), desktop);
        view.update(cx, |this, _, _| {
            this.db.apply_response(&serde_json::json!({"success":true,"user_notifications":[{"id":20,"taskId":1,"projectId":15,"status":"Normal","createdAt":"2026-01-01T00:00:00Z"}]})).unwrap();
            this.reload();
            let rows = this.command_rows();
            let position = |key| rows.iter().position(|row|row.key == key).unwrap();
            assert!(position("toggleAllComments") < position("removeTaskNotification"));
            assert!(position("removeTaskNotification") < position("renameTask"));
            this.db.apply_task(&serde_json::json!({"id":1,"projectId":15,"title":"Archived","status":"Archive"})).unwrap();
            this.reload();
            let rows = this.command_rows();
            assert_eq!(rows.iter().find(|row| row.key == "archiveTask").unwrap().name, "Unarchive task");
            for key in ["assignUser", "moveToColumn", "removeTaskNotification", "setReminder"] {
                assert!(!rows.iter().any(|row|row.key == key), "unsupported archived action {key}");
            }
        }).unwrap();
    }

    #[gpui::test]
    fn cmdk_boards_use_web_keys_labels_and_context_insertion(cx: &mut gpui::TestAppContext) {
        let (view, mut cx) = crate::ui::shell::tests::setup(cx);
        view.update(&mut cx, |this, _, _| {
            let rows = this.command_rows();
            let boards = rows.iter().filter(|row|row.group == "Boards").map(|row|(row.key.as_str(),row.name.as_str())).collect::<Vec<_>>();
            assert_eq!(boards, [("gotoBoard-16", "Go to board: Beta")]);
            let index = rows.iter().position(|row|row.group == "Boards").unwrap();
            assert_eq!(rows[index-1].group, "Navigate");
            assert_eq!(rows[index+1].group, "Inbox");
        }).unwrap();
    }

    #[gpui::test]
    fn cmdk_paints_web_modal_rows_header_and_new_badge(cx: &mut gpui::TestAppContext) {
        use crate::ui::shell::tests::{draw, measured, setup};
        let (view, mut cx) = setup(cx);
        cx.simulate_keystrokes("ctrl-k");
        cx.simulate_input("CLI");
        draw(view, &mut cx);
        let modal = measured(view, &mut cx, "command.center");
        assert_eq!(modal["w"], 560.);
        assert!((modal["y"].as_f64().unwrap() - 216.).abs() < 1.);
        assert!((modal["x"].as_f64().unwrap() - 230.).abs() < 1.);
        assert_eq!(measured(view, &mut cx, "command.choice.0")["h"], 36.);
        assert_eq!(measured(view, &mut cx, "command.choice.0")["text"], "CLI");
        assert_eq!(measured(view, &mut cx, "command.search.icon")["w"], 13.);
        assert_eq!(measured(view, &mut cx, "command.new.0")["text"], "New");
        assert!(measured(view, &mut cx, "command.results")["h"].as_f64().unwrap() <= 364.);
    }


    #[gpui::test]
    fn cmdk_saved_comment_labels_match_cached_web_state(cx: &mut gpui::TestAppContext) {
        cx.update(crate::input::init);
        let view = cx.open_window(gpui::size(px(1440.), px(900.)), desktop);
        view.update(cx, |this, _, _| {
            this.db.apply_comments(1, &serde_json::json!({"success":true,"comments":[{"id":7,"text":"Saved","creatorId":42,"createdAt":"2026-01-01T00:00:00Z","savedContent":[{"type":"Public"},{"type":"Private"}]}]})).unwrap();
            this.reload();
            this.interaction.task_part = 1;
            let rows = this.command_rows();
            assert_eq!(rows.iter().find(|row|row.key == "pinComment").unwrap().name, "Unpin comment");
            assert_eq!(rows.iter().find(|row|row.key == "starComment").unwrap().name, "Unstar comment");
            assert!(!rows.iter().any(|row|row.key == "copyCommentToAiChat"));
        }).unwrap();
    }

    #[gpui::test]
    fn cmdk_copy_branch_name_uses_web_slug_and_only_loaded_ticket_data(cx: &mut gpui::TestAppContext) {
        use crate::ui::shell::tests::{draw, measured};
        cx.update(crate::input::init);
        let view = cx.open_window(gpui::size(px(1440.), px(900.)), desktop);
        let mut cx = gpui::VisualTestContext::from_window(view.into(), cx);
        for (title, expected) in [
            ("  Fix é BUG / edge___case!  ".to_owned(), "ht-1-fix-bug-edge-case".to_owned()),
            (format!("{} tail", "A".repeat(49)), format!("ht-1-{}", "a".repeat(49))),
            ("🙂".to_owned(), "ht-1".to_owned()),
        ] {
            view.update(&mut cx, |this, _, cx| {
                this.db.apply_task(&serde_json::json!({"id":1,"projectId":15,"title":title,"ticketNumber":" HT-1 ","status":"Normal"})).unwrap();
                this.reload();
                cx.notify();
            }).unwrap();
            cx.simulate_keystrokes("ctrl-k");
            cx.simulate_input("Copy branch name");
            draw(view, &mut cx);
            assert_eq!(measured(view, &mut cx, "command.choice.0")["text"], "Copy branch name");
            cx.simulate_keystrokes("enter");
            view.read_with(&mut cx, |_, cx| assert_eq!(cx.read_from_clipboard().unwrap().text().unwrap(), expected)).unwrap();
        }
        view.update(&mut cx, |this, _, _| {
            this.db.apply_task(&serde_json::json!({"id":1,"projectId":15,"title":"No ticket","ticketNumber":null,"status":"Normal"})).unwrap();
            this.reload();
            assert!(!this.command_rows().iter().any(|row|row.key == "copyBranchName"));
        }).unwrap();
    }

}

#[cfg(test)]
mod cmdk_round3_tests {
    use super::*;
    use crate::ui::{shell::tests::{draw, measured}, task_fields::tests::visual};

    #[gpui::test]
    fn cmdk3_search_handover_follows_ranked_rows_and_keeps_query(cx: &mut gpui::TestAppContext) {
        let (view, mut cx) = visual(cx);
        for (index, screen) in [Screen::Inbox, Screen::Board, Screen::Task].into_iter().enumerate() {
            view.update(&mut cx, |this, window, cx| {
                this.screen = screen;
                this.reload();
                window.focus(&this.focus, cx);
            }).unwrap();
            cx.simulate_keystrokes("ctrl-k");
            let query = " Needle \"quoted\" ";
            cx.simulate_input(query);
            draw(view, &mut cx);
            assert_eq!(measured(view, &mut cx, "command.choice.0")["text"], format!("Search tasks: \"{query}\""));
            view.read_with(&mut cx, |this, cx| {
                let rows = this.filtered_commands(cx);
                assert_eq!(rows.len(), 1);
                assert_eq!(rows[0].key, "searchTasksHandover");
                assert_eq!(rows[0].keys, "");
            }).unwrap();
            cx.simulate_keystrokes("enter");
            view.read_with(&mut cx, |this, _| {
                assert!(this.interaction.modal.is_none());
                assert_eq!(this.search_query, query.trim());
                assert_eq!(this.command_usage()["searchTasksHandover"]["frequency"], index + 1);
                assert!(this.db.mutations().unwrap().is_empty());
            }).unwrap();
            draw(view, &mut cx);
            measured(view, &mut cx, "shell.search.input");
            cx.simulate_keystrokes("ctrl-a ctrl-c");
            cx.update(|_, cx| assert_eq!(cx.read_from_clipboard().and_then(|item| item.text()), Some(query.into())));
            cx.simulate_keystrokes("escape");
        }
    }

    #[gpui::test]
    fn cmdk3_search_handover_is_last_not_an_empty_query_recommendation(cx: &mut gpui::TestAppContext) {
        let (view, mut cx) = visual(cx);
        cx.simulate_keystrokes("ctrl-k");
        view.read_with(&mut cx, |this, cx| assert!(!this.filtered_commands(cx).iter().any(|r| r.key == "searchTasksHandover"))).unwrap();
        cx.simulate_input("copy");
        view.read_with(&mut cx, |this, cx| {
            let rows = this.filtered_commands(cx);
            assert!(rows.len() > 1);
            assert_eq!(rows[0].key, "copyformattedTaskLink");
            assert_eq!(rows.last().unwrap().key, "searchTasksHandover");
            assert!(rows[..rows.len()-1].iter().all(|r| r.key != "searchTasksHandover"));
        }).unwrap();
        cx.simulate_keystrokes("escape ctrl-k");
        cx.simulate_input("Search");
        cx.simulate_keystrokes("enter");
        view.read_with(&mut cx, |this, _| {
            assert!(this.interaction.modal.is_none());
            assert!(this.search_query.is_empty());
        }).unwrap();
        draw(view, &mut cx);
        measured(view, &mut cx, "shell.search.input");
        cx.simulate_input("Needle");
        view.read_with(&mut cx, |this, _| assert_eq!(this.search_query, "Needle")).unwrap();
    }

    #[gpui::test]
    fn cmdk3_follow_system_changes_theme_and_persists_preference(cx: &mut gpui::TestAppContext) {
        let (view, mut cx) = visual(cx);
        cx.simulate_keystrokes("ctrl-k");
        cx.simulate_input("Follow system");
        cx.simulate_keystrokes("enter");
        view.update(&mut cx, |this, window, _| {
            let expected = match window.appearance() {
                gpui::WindowAppearance::Dark | gpui::WindowAppearance::VibrantDark => ThemeName::Amoled,
                _ => ThemeName::Porcelain,
            };
            assert_eq!(this.options.theme, expected);
            assert_eq!(this.db.kv_get("theme").unwrap().as_deref(), Some("system"));
            assert_eq!(this.command_usage()["systemTheme"]["frequency"], 1);
            assert!(this.interaction.modal.is_none());
        }).unwrap();
    }

    #[gpui::test]
    fn cmdk3_remove_child_picker_unlinks_selected_child_not_parent(cx: &mut gpui::TestAppContext) {
        let (view, mut cx) = visual(cx);
        view.update(&mut cx, |this, _, _| {
            this.db.apply_task(&serde_json::json!({"id":1,"projectId":15,"title":"Needle","status":"Normal","sub_tasks":[
                {"id":2,"ticketNumber":"HT-2","title":"Other","uniqueIndex":2},
                {"id":3,"ticketNumber":"HT-3","title":"Third","uniqueIndex":3}
            ]})).unwrap();
            for id in [2, 3] { this.db.apply_task(&serde_json::json!({"id":id,"projectId":15,"title":if id == 2 {"Other"} else {"Third"},"status":"Normal","parent_id":1})).unwrap(); }
            this.reload();
        }).unwrap();
        cx.simulate_keystrokes("ctrl-k");
        cx.simulate_input("Remove a sub-task");
        cx.simulate_keystrokes("enter");
        draw(view, &mut cx);
        assert_eq!(measured(view, &mut cx, "command.choice.0")["text"], "HT-2 Other");
        assert_eq!(measured(view, &mut cx, "command.choice.1")["text"], "HT-3 Third");
        cx.simulate_input("Third");
        cx.simulate_keystrokes("enter");
        view.read_with(&mut cx, |this, _| {
            let rows = this.db.mutations().unwrap();
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].kind, "task_patch");
            assert_eq!(rows[0].payload_json, serde_json::json!({"task_id":3,"parent_task_id":null}));
            assert_eq!(this.options.task, 1);
            assert_eq!(this.db.task(2).unwrap().unwrap().extra["parentTaskId"], 1);
            assert!(this.db.task(3).unwrap().unwrap().extra["parentTaskId"].is_null());
            assert_eq!(this.removable_subtasks().unwrap().len(), 1);
        }).unwrap();
    }

    #[gpui::test]
    fn cmdk3_child_removal_requires_complete_authoritative_cached_children(cx: &mut gpui::TestAppContext) {
        let (view, mut cx) = visual(cx);
        view.update(&mut cx, |this, _, _| {
            for children in [serde_json::json!([]), serde_json::json!([{"id":99,"title":"Not cached"}]), serde_json::json!([{"id":2,"title":"Other"}])] {
                this.db.apply_task(&serde_json::json!({"id":1,"projectId":15,"title":"Needle","status":"Normal","sub_tasks":children})).unwrap();
                assert!(!this.command_rows().iter().any(|row| row.key == "removeSubtask"));
            }
            assert!(this.db.mutations().unwrap().is_empty());
        }).unwrap();
    }

    #[gpui::test]
    fn cmdk3_palette_typography_and_tokens_match_all_web_themes(cx: &mut gpui::TestAppContext) {
        let glyph = gpui::AssetSource::load(&crate::Assets, "command-search").unwrap().unwrap();
        assert!(std::str::from_utf8(&glyph).unwrap().contains("stroke-width=\"1.75\""));
        let (view, mut cx) = visual(cx);
        for (theme, background, selected) in [
            (ThemeName::Porcelain, "#ffffff", "#e2e2e6"), (ThemeName::Graphite, "#2c2f36", "#3a3d45"),
            (ThemeName::Amoled, "#141414", "#303239"), (ThemeName::Dia, "#ffffff", "#c9e3d2"),
        ] {
            view.update(&mut cx, |this, _, cx| this.apply_named_theme(theme, cx)).unwrap();
            cx.simulate_keystrokes("ctrl-k");
            cx.simulate_input("Follow system");
            draw(view, &mut cx);
            let modal = measured(view, &mut cx, "command.center");
            let row = measured(view, &mut cx, "command.choice.0");
            let group = measured(view, &mut cx, "command.group.0");
            assert_eq!(modal["w"], 560.);
            assert_eq!(modal["bg"], background);
            assert_eq!(row["h"], 36.);
            assert_eq!(row["bg"], selected);
            assert_eq!(row["fontSize"], 13.);
            assert_eq!(row["lineHeight"], 19.5);
            assert_eq!(group["fontSize"], 11.);
            assert_eq!(group["lineHeight"], 16.5);
            let input = measured(view, &mut cx, "command.input");
            assert_eq!(input["fontSize"], 14.);
            assert_eq!(input["h"], 21.);
            assert_eq!(measured(view, &mut cx, "command.search.icon")["w"], 13.);
            cx.simulate_keystrokes("escape");
        }
    }
}

#[cfg(test)]
mod cmdk_round3_board_tests {
    use super::*;
    use crate::ui::task_fields::tests::desktop;

    #[gpui::test]
    fn cmdk3_board_forms_work_from_inbox_board_and_task_without_changing_surface(cx: &mut gpui::TestAppContext) {
        cx.update(crate::input::init);
        for screen in [Screen::Inbox, Screen::Board, Screen::Task] {
            for (label, variant, title, section) in [
                ("Add board column", "AddColumn", Some("Review"), None),
                ("Rename board column", "RenameColumn", Some("Backlog"), Some(10)),
                ("Delete board column", "DeleteColumn", None, Some(10)),
                ("Rename board", "RenameBoard", Some("New board name"), None),
                ("Archive board", "ArchiveBoard", None, None),
            ] {
                let (view, cx) = cx.add_window_view(|window, cx| {
                    let mut this = desktop(window, cx);
                    this.db.apply_response(&serde_json::json!({"success":true,"projects":[
                        {"id":15,"title":"Board","ownerId":42,"status":"Normal"},
                        {"id":16,"title":"Other board","ownerId":7,"status":"Normal"}
                    ]})).unwrap();
                    this.screen = screen;
                    if screen == Screen::Task { this.options.board = 16; }
                    this.reload();
                    this
                });
                cx.simulate_keystrokes("ctrl-k");
                cx.simulate_input(label);
                let index = view.read_with(cx, |this, cx| this.choices(cx).iter().position(|row| row.name == label).expect(label));
                for _ in 0..index { cx.simulate_keystrokes("down"); }
                cx.simulate_keystrokes("enter");
                if let Some(title) = title { cx.simulate_input(title); }
                cx.simulate_keystrokes("enter");
                view.read_with(cx, |this, _| {
                    let rows = this.db.mutations().unwrap();
                    assert_eq!(rows.len(), 1, "{screen:?} {label}");
                    let change = &rows[0].payload_json["change"][variant];
                    assert_eq!(change["project"], 15, "{screen:?} {label}");
                    if let Some(section) = section { assert_eq!(change["section"], section); }
                    if let Some(title) = title { assert_eq!(change["title"], title); }
                    assert_eq!(this.screen, screen);
                    assert_eq!(this.db.task(1).unwrap().unwrap().title, "Needle");
                });
            }
        }
    }

    #[gpui::test]
    fn cmdk3_board_forms_omit_missing_catalog_and_never_use_foreign_columns(cx: &mut gpui::TestAppContext) {
        cx.update(crate::input::init);
        let (view, cx) = cx.add_window_view(desktop);
        view.update_in(cx, |this, _, _| {
            this.screen = Screen::Inbox;
            this.options.board = 999;
            for key in ["addColumn", "renameColumn", "deleteColumn", "renameBoard", "archiveBoards"] {
                assert!(!this.command_rows().iter().any(|row| row.key == key), "{key}");
            }
            this.db.apply_response(&serde_json::json!({"success":true,"projects":[{"id":999,"title":"Unloaded board"}]})).unwrap();
            let rows = this.command_rows();
            assert!(rows.iter().any(|row| row.key == "addColumn"));
            assert!(rows.iter().any(|row| row.key == "renameBoard"));
            for key in ["renameColumn", "deleteColumn"] { assert!(!rows.iter().any(|row| row.key == key)); }
            assert!(this.db.mutations().unwrap().is_empty());
        });
    }
}
