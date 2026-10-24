use super::*;
use crate::shortcuts::Action;
use gpui::MouseButton;

pub(super) struct ReturnTo {
    screen: Screen,
    board: i64,
    column: usize,
    selected: usize,
    split: usize,
    notification: Option<i64>,
}

pub(super) struct BoardActions {
    pub search: Entity<TextInput>,
    pub search_text: String,
    pub searching: bool,
    pub project: i64,
}
impl BoardActions {
    pub fn new(cx: &mut Context<Desktop>) -> Self {
        let search =
            cx.new(|cx| TextInput::new(String::new(), "Find in view...", color(0x8e9093), cx));
        cx.observe(&search, |this, input, cx| {
            this.board_actions.search_text = input.read(cx).content.trim().to_lowercase();
            this.selected = 0;
            if this.screen == Screen::Board {
                this.reload();
            }
            cx.notify();
        })
        .detach();
        Self {
            search,
            search_text: String::new(),
            searching: false,
            project: 0,
        }
    }
}
pub(super) fn search_matches(task: &TaskView, query: &str) -> bool {
    query.is_empty()
        || task.title.to_lowercase().contains(query)
        || task
            .ticket_number
            .as_deref()
            .unwrap_or("")
            .to_lowercase()
            .contains(query)
}
impl Desktop {
    pub(super) fn remember_task_origin(&mut self) {
        if self.screen == Screen::Task {
            return;
        }
        self.interaction.return_to = Some(ReturnTo {
            screen: self.screen,
            board: self.options.board,
            column: self.column,
            selected: self.selected,
            split: self.split,
            notification: if self.screen == Screen::Inbox {
                self.selection_id()
            } else {
                None
            },
        });
    }
    pub(super) fn return_from_task(&mut self, cx: &mut Context<Self>) {
        if let Some(origin) = self.interaction.return_to.take() {
            self.options.board = origin.board;
            self.column = origin.column;
            self.split = origin.split;
            self.switch(origin.screen, cx);
            self.split = origin.split.min(self.splits.len().saturating_sub(1));
            if origin.screen == Screen::Inbox {
                self.filter_inbox();
            }
            self.selected = origin
                .notification
                .and_then(|id| self.filtered.iter().position(|&i| self.inbox[i].id == id))
                .unwrap_or(origin.selected);
            self.clamp_selection();
            cx.notify();
        } else {
            self.switch(self.previous, cx);
        }
    }
    pub(super) fn board_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.screen != Screen::Board || self.interaction.modal.is_some() {
            return false;
        }
        let key = &event.keystroke;
        let ctrl = key.modifiers.control || key.modifiers.platform;
        if ctrl && key.key == "f" && !key.modifiers.shift && !key.modifiers.alt {
            if self.board_actions.project != self.options.board {
                self.board_actions.project = self.options.board;
                self.board_actions.search_text.clear();
                self.board_actions
                    .search
                    .update(cx, |input, cx| input.set_content(String::new(), cx));
            }
            self.board_actions.searching = true;
            window.focus(&self.board_actions.search.focus_handle(cx), cx);
        } else if key.key == "escape" && self.board_actions.searching {
            self.board_actions.searching = false;
            self.board_actions.search_text.clear();
            self.board_actions
                .search
                .update(cx, |input, cx| input.set_content(String::new(), cx));
            self.reload();
            window.focus(&self.focus, cx);
        } else {
            return self
                .board_actions
                .search
                .focus_handle(cx)
                .is_focused(window);
        }
        cx.notify();
        cx.stop_propagation();
        true
    }
    pub(super) fn board_search(&self, cx: &mut Context<Self>) -> AnyElement {
        div()
            .px(px(24.))
            .py(px(8.))
            .child(self.board_actions.search.clone())
            .layout_key(&self.layout, "board.search", None)
    }
    pub(super) fn card_controls(
        &self,
        task: i64,
        column: usize,
        row: usize,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        div()
            .flex()
            .gap(px(6.))
            .child(
                div()
                    .id(("card-select", task as u64))
                    .cursor_pointer()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|_, _, _, cx| cx.stop_propagation()),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        cx.stop_propagation();
                        this.column = column;
                        this.selected = row;
                        if !this.interaction.bulk.remove(&task) {
                            this.interaction.bulk.insert(task);
                        }
                        cx.notify();
                    }))
                    .child(if self.interaction.bulk.contains(&task) {
                        "☑"
                    } else {
                        "☐"
                    })
                    .layout_key(&self.layout, format!("board.card.{task}.select"), None),
            )
            .child(
                div()
                    .id(("card-commands", task as u64))
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, window, cx| {
                        cx.stop_propagation();
                        this.column = column;
                        this.selected = row;
                        this.run_shortcut(Action::Commands, window, cx);
                    }))
                    .child(gpui::img("htc-logo").size(px(14.)))
                    .layout_key(&self.layout, format!("board.card.{task}.commands"), None),
            )
            .into_any_element()
    }
    pub(super) fn board_selection_bar(&self, cx: &mut Context<Self>) -> AnyElement {
        div()
            .px(px(24.))
            .py(px(8.))
            .flex()
            .gap(px(16.))
            .items_center()
            .text_size(px(13.))
            .child(format!("{} selected", self.interaction.bulk.len()))
            .child(
                div()
                    .id("board-bulk-actions")
                    .cursor_pointer()
                    .child("Open actions")
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.run_shortcut(Action::Commands, window, cx)
                    }))
                    .layout_key(&self.layout, "board.bulk.actions", None),
            )
            .child("Esc to clear")
            .layout_key(&self.layout, "board.bulk", None)
    }
    pub(super) fn add_board_commands(&self, rows: &mut Vec<crate::command_center::Command>) {
        if self.screen == Screen::Board {
            for (key, name, action) in [
                (
                    "addColumn",
                    "Add board column",
                    crate::command_center::BoardAction::AddColumn,
                ),
                (
                    "renameColumn",
                    "Rename board column",
                    crate::command_center::BoardAction::RenameColumn,
                ),
                (
                    "deleteColumn",
                    "Delete board column",
                    crate::command_center::BoardAction::DeleteColumn,
                ),
                (
                    "renameBoard",
                    "Rename board",
                    crate::command_center::BoardAction::RenameBoard,
                ),
            ] {
                rows.push(crate::command_center::Command {
                    key: key.into(),
                    name: name.into(),
                    group: "Board",
                    keys: String::new(),
                    keywords: "board column",
                    action: crate::command_center::CommandAction::BoardForm(action),
                });
            }
        }
        if self.selected_task().is_some() {
            rows.push(crate::command_center::Command {
                key: "moveTaskToBoard".into(),
                name: "Move to board".into(),
                group: "Task",
                keys: "SHIFT M".into(),
                keywords: "move task another board",
                action: crate::command_center::CommandAction::BoardForm(
                    crate::command_center::BoardAction::MoveTask,
                ),
            });
        }
        if self.screen != Screen::Board || self.interaction.bulk.is_empty() {
            return;
        }
        let commands = [
            (
                "bulkMoveToColumn",
                "Move selected tasks to column",
                "M",
                Action::MoveColumn,
            ),
            (
                "bulkAssignUser",
                "Assign selected tasks",
                "A",
                Action::Assign,
            ),
            (
                "bulkSetLabels",
                "Set tags on selected tasks",
                "T",
                Action::Tags,
            ),
            (
                "bulkArchive",
                "Archive selected tasks",
                "CTRL E",
                Action::Archive,
            ),
        ]
        .into_iter()
        .map(|(key, name, keys, action)| crate::command_center::Command {
            key: key.into(),
            name: name.into(),
            keys: keys.into(),
            group: "Selected tasks",
            keywords: "bulk selected tasks",
            action: crate::command_center::CommandAction::Shortcut(action),
        });
        rows.splice(0..0, commands);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::shell::tests::{click, draw};
    use crate::ui::task_fields::tests::desktop;
    use gpui::{VisualTestContext, WindowHandle};
    use serde_json::json;

    fn visual(cx: &mut gpui::TestAppContext) -> (WindowHandle<Desktop>, VisualTestContext) {
        cx.update(crate::input::init);
        let view = cx.open_window(gpui::size(px(1440.), px(900.)), |window, cx| {
            let mut view = desktop(window, cx);
            view.screen = Screen::Board;
            view.reload();
            view
        });
        let mut cx = VisualTestContext::from_window(view.into(), cx);
        draw(view, &mut cx);
        (view, cx)
    }
    #[gpui::test]
    fn board_checkbox_selects_without_opening_and_card_click_preserves_selection(
        cx: &mut gpui::TestAppContext,
    ) {
        let (view, mut cx) = visual(cx);
        click(view, &mut cx, "board.card.1.select");
        view.read_with(&mut cx, |this, _| {
            assert_eq!(this.screen, Screen::Board);
            assert_eq!(
                this.interaction.bulk.iter().copied().collect::<Vec<_>>(),
                vec![1]
            );
            assert!(this.db.mutations().unwrap().is_empty());
        })
        .unwrap();
        click(view, &mut cx, "board.card.0.title");
        view.read_with(&mut cx, |this, _| {
            assert_eq!(this.screen, Screen::Task);
            assert_eq!(this.options.task, 1);
        })
        .unwrap();
    }
    #[gpui::test]
    fn board_bulk_open_actions_has_selected_context_and_moves_only_selected_cards(
        cx: &mut gpui::TestAppContext,
    ) {
        let (view, mut cx) = visual(cx);
        click(view, &mut cx, "board.card.1.select");
        click(view, &mut cx, "board.card.2.select");
        click(view, &mut cx, "board.bulk.actions");
        view.read_with(&mut cx, |this, cx| {
            assert_eq!(this.choices(cx)[0].name, "Move selected tasks to column");
        })
        .unwrap();
        click(view, &mut cx, "command.choice.0");
        click(view, &mut cx, "command.choice.1");
        view.read_with(&mut cx, |this, _| {
            for id in [1, 2] {
                assert_eq!(this.db.task(id).unwrap().unwrap().section_id, Some(11));
            }
            assert_eq!(
                this.db
                    .mutations()
                    .unwrap()
                    .iter()
                    .map(|m| m.payload_json["task_id"].as_i64().unwrap())
                    .collect::<Vec<_>>(),
                vec![1, 2]
            );
        })
        .unwrap();
    }
    #[gpui::test]
    fn board_card_command_logo_focuses_clicked_task_and_options_shortcut_opens_palette(
        cx: &mut gpui::TestAppContext,
    ) {
        let (view, mut cx) = visual(cx);
        click(view, &mut cx, "board.card.2.commands");
        view.read_with(&mut cx, |this, _| {
            assert_eq!(this.selected_task(), Some(2));
            assert_eq!(this.screen, Screen::Board);
            assert_eq!(this.interaction.modal, Some(interactions::Modal::Commands));
        })
        .unwrap();
        cx.simulate_keystrokes("escape ctrl-shift-=");
        view.read_with(&mut cx, |this, _| {
            assert_eq!(this.interaction.modal, Some(interactions::Modal::Commands))
        })
        .unwrap();
    }
    #[gpui::test]
    fn board_search_is_inline_title_ticket_only_and_escape_restores_board(
        cx: &mut gpui::TestAppContext,
    ) {
        let (view, mut cx) = visual(cx);
        cx.simulate_keystrokes("ctrl-f");
        cx.simulate_input("hT-2");
        cx.run_until_parked();
        view.read_with(&mut cx, |this, _| {
            assert!(this.interaction.modal.is_none());
            assert_eq!(this.columns[0].cards.len(), 1);
            assert_eq!(this.columns[0].cards[0].id, 2);
        })
        .unwrap();
        cx.simulate_keystrokes("escape ctrl-f");
        cx.simulate_input("Hidden");
        cx.run_until_parked();
        view.read_with(&mut cx, |this, _| {
            assert!(this.columns.iter().all(|c| c.cards.is_empty()))
        })
        .unwrap();
        cx.simulate_keystrokes("escape");
        view.read_with(&mut cx, |this, _| {
            assert_eq!(this.columns.iter().map(|c| c.cards.len()).sum::<usize>(), 3);
            assert_eq!(this.screen, Screen::Board);
            assert!(this.db.mutations().unwrap().is_empty());
        })
        .unwrap();
    }
    #[gpui::test]
    fn board_e_archives_focused_notification_not_title_or_stale_task_option(
        cx: &mut gpui::TestAppContext,
    ) {
        let (view, mut cx) = visual(cx);
        view.update(&mut cx, |this, window, cx| {
            this.db
                .apply_response(&json!({"success":true,"user_notifications":[
                    {"id":101,"taskId":1,"projectId":15,"status":"Normal","createdAt":"2026-01-01T00:00:00Z"},
                    {"id":102,"taskId":2,"projectId":15,"status":"Normal","createdAt":"2026-01-02T00:00:00Z"}
                ]}))
                .unwrap();
            this.reload();
            this.selected = 1;
            window.focus(&this.focus, cx);
        })
        .unwrap();
        cx.simulate_keystrokes("e");
        view.read_with(&mut cx, |this, _| {
            assert_eq!(
                this.db.mutations().unwrap()[0].payload_json["notification_ids"],
                json!([102])
            );
            assert!(this.editing == Editing::None);
            assert_eq!(this.db.task(2).unwrap().unwrap().title, "Other");
        })
        .unwrap();
    }
}

#[cfg(test)]
mod navigation_tests {
    use super::*;
    use crate::ui::task_fields::tests::desktop;
    use serde_json::json;
    #[gpui::test]
    fn escape_returns_to_same_searched_board_column_and_focused_card(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(crate::input::init);
        let (view, cx) = cx.add_window_view(|window, cx| {
            let mut v = desktop(window, cx);
            v.screen = Screen::Board;
            v.reload();
            v.column = 1;
            v
        });
        cx.simulate_keystrokes("enter escape");
        view.read_with(cx, |this, _| {
            assert_eq!(this.screen, Screen::Board);
            assert_eq!(this.column, 1);
            assert_eq!(this.selected_task(), Some(3));
            assert!(this.db.mutations().unwrap().is_empty());
        });
    }
    #[gpui::test]
    fn escape_returns_to_same_inbox_notification_not_first_row(cx: &mut gpui::TestAppContext) {
        cx.update(crate::input::init);
        let (view,cx)=cx.add_window_view(|window,cx|{
            let mut v=desktop(window,cx);
            v.db.apply_response(&json!({"success":true,"user_notifications":[
                {"id":101,"taskId":1,"projectId":15,"status":"Normal","seen":true,"createdAt":"2026-01-01T00:00:00Z"},
                {"id":102,"taskId":2,"projectId":15,"status":"Normal","seen":true,"createdAt":"2026-01-02T00:00:00Z"}
            ]})).unwrap();v.screen=Screen::Inbox;v.reload();v.split=v.splits.iter().position(|s|s=="All").unwrap();v.filter_inbox();v.selected=1;v
        });
        cx.simulate_keystrokes("enter escape");
        view.read_with(cx, |this, _| {
            assert_eq!(this.screen, Screen::Inbox);
            assert_eq!(this.splits[this.split], "All");
            assert_eq!(this.selection_id(), Some(101));
            assert!(this.db.mutations().unwrap().is_empty());
        });
    }
}
