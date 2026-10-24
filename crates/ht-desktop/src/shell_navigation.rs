use super::*;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum Destination {
    #[default]
    Workspace,
    Table,
    Search,
}

impl Desktop {
    pub(super) fn navigate_board(&mut self, cx: &mut Context<Self>) {
        if self.screen == Screen::Task
            && let Some(task) = self.db.task(self.options.task).ok().flatten()
        {
            self.options.board = task.project_id;
            let _ = self
                .db
                .kv_set("last_board", &self.options.board.to_string());
        }
        self.switch(Screen::Board, cx);
    }

    pub(super) fn navigate_table(&mut self, cx: &mut Context<Self>) {
        self.navigate_board(cx);
        self.shell.destination = Destination::Table;
        cx.notify();
    }

    pub(super) fn navigate_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.exit_ai_fullscreen(cx);
        self.shell.agents = false;
        self.shell.search_board = (self.screen == Screen::Board).then_some(self.options.board);
        self.shell.destination = Destination::Search;
        self.shell
            .search_input
            .update(cx, |input, cx| input.set_content(String::new(), cx));
        window.focus(&self.shell.search_input.focus_handle(cx), cx);
        cx.notify();
    }

    pub(super) fn navigation_body(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        match self.shell.destination {
            Destination::Workspace => None,
            Destination::Table => {
                let board = self
                    .db
                    .projects()
                    .unwrap_or_default()
                    .into_iter()
                    .find(|b| b["id"] == self.options.board);
                let title = board
                    .as_ref()
                    .and_then(|b| b["title"].as_str())
                    .unwrap_or("")
                    .to_owned();
                let tasks = self
                    .db
                    .search_tasks("", Some(self.options.board))
                    .unwrap_or_default();
                Some(
                    div()
                        .flex_1()
                        .h_full()
                        .min_h_0()
                        .flex()
                        .flex_col()
                        .bg(color(self.colors.page))
                        .child(
                            div()
                                .h(px(56.))
                                .px(px(20.))
                                .flex()
                                .items_center()
                                .gap(px(4.))
                                .text_size(px(14.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(title.clone())
                                .child(tasks.len().to_string())
                                .layout_key(&self.layout, "shell.table.title", Some(title)),
                        )
                        .child(
                            div()
                                .px(px(20.))
                                .py(px(8.))
                                .flex()
                                .gap(px(8.))
                                .text_size(px(11.))
                                .text_color(color(self.colors.muted))
                                .child(div().w(px(90.)).child("TICKET"))
                                .child(div().flex_1().child("TITLE")),
                        )
                        .child(
                            div()
                                .id("shell-table-rows")
                                .flex_1()
                                .overflow_y_scroll()
                                .children(tasks.into_iter().enumerate().map(|(i, task)| {
                                    let id = task.id;
                                    div()
                                        .id(("shell-table-task", id as u64))
                                        .px(px(20.))
                                        .py(px(8.))
                                        .flex()
                                        .gap(px(8.))
                                        .cursor_pointer()
                                        .child(
                                            div()
                                                .w(px(90.))
                                                .font_weight(FontWeight::BOLD)
                                                .text_color(color(self.colors.muted))
                                                .child(task.ticket_number.unwrap_or_default()),
                                        )
                                        .child(div().flex_1().child(task.title.clone()))
                                        .on_click(
                                            cx.listener(move |this, _, _, cx| {
                                                this.open_task(id, cx)
                                            }),
                                        )
                                        .layout_key(
                                            &self.layout,
                                            format!("shell.table.row.{i}"),
                                            Some(task.title),
                                        )
                                })),
                        )
                        .layout_key(&self.layout, "shell.table", None),
                )
            }
            Destination::Search => {
                let query = self.shell.search_input.read(cx).content.as_ref();
                let mut tasks = if query.trim().is_empty() {
                    vec![]
                } else {
                    self.db.search_tasks(query, None).unwrap_or_default()
                };
                if let Some(board) = self.shell.search_board {
                    tasks.sort_by_key(|t| t.project_id != board);
                }
                let empty = tasks.is_empty();
                Some(
                    div()
                        .h_full()
                        .w_full()
                        .bg(color(self.colors.container))
                        .py(px(36.))
                        .flex()
                        .justify_center()
                        .child(
                            div()
                                .w_full()
                                .max_w(px(1000.))
                                .flex()
                                .flex_col()
                                .child(
                                    div()
                                        .h(px(44.))
                                        .px(px(36.))
                                        .child(self.shell.search_input.clone())
                                        .layout_key(&self.layout, "shell.search.input", None),
                                )
                                .when(!query.trim().is_empty() && empty, |d| {
                                    d.child(
                                        div()
                                            .px(px(36.))
                                            .my(px(16.))
                                            .text_color(color(self.colors.muted))
                                            .child("No results found"),
                                    )
                                })
                                .child(
                                    div()
                                        .id("shell-search-results")
                                        .flex_1()
                                        .overflow_y_scroll()
                                        .px(px(36.))
                                        .mt(px(12.))
                                        .children(tasks.into_iter().enumerate().map(
                                            |(i, task)| {
                                                let id = task.id;
                                                div()
                                                    .id(("shell-search-task", id as u64))
                                                    .py(px(8.))
                                                    .flex()
                                                    .gap(px(12.))
                                                    .cursor_pointer()
                                                    .child(task.board_title)
                                                    .child(task.ticket_number.unwrap_or_default())
                                                    .child(task.title.clone())
                                                    .on_click(cx.listener(move |this, _, _, cx| {
                                                        this.open_task(id, cx)
                                                    }))
                                                    .layout_key(
                                                        &self.layout,
                                                        format!("shell.search.result.{i}"),
                                                        Some(task.title),
                                                    )
                                            },
                                        )),
                                ),
                        )
                        .layout_key(&self.layout, "shell.search", None),
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::shell::tests::{click, draw, measured, setup};
    use gpui::TestAppContext;

    #[gpui::test]
    fn table_icon_and_three_open_table_then_board_restores_kanban(cx: &mut TestAppContext) {
        let (view, mut cx) = setup(cx);
        click(view, &mut cx, "rail.icon.4");
        assert_eq!(
            measured(view, &mut cx, "shell.table.title")["text"],
            "Alpha"
        );
        assert_eq!(
            measured(view, &mut cx, "shell.table.row.0")["text"],
            "Alpha result"
        );
        click(view, &mut cx, "rail.icon.3");
        measured(view, &mut cx, "board.header");
        cx.simulate_keystrokes("3");
        measured(view, &mut cx, "shell.table");
    }

    #[gpui::test]
    fn search_icon_has_board_context_and_results_open_matching_task(cx: &mut TestAppContext) {
        let (view, mut cx) = setup(cx);
        click(view, &mut cx, "rail.icon.3");
        click(view, &mut cx, "rail.icon.7");
        assert!(!measured(view, &mut cx, "rail.icon.7")["bg"].is_null());
        assert!(measured(view, &mut cx, "rail.icon.3")["bg"].is_null());
        cx.simulate_input("result");
        assert_eq!(
            measured(view, &mut cx, "shell.search.result.0")["text"],
            "Alpha result"
        );
        view.read_with(&cx, |this, _| assert_eq!(this.shell.search_board, Some(15)))
            .unwrap();
        click(view, &mut cx, "shell.search.result.1");
        view.read_with(&cx, |this, _| {
            assert_eq!(this.screen, Screen::Task);
            assert_eq!(this.options.task, 102);
        })
        .unwrap();
        click(view, &mut cx, "rail.icon.2");
        cx.simulate_keystrokes("6");
        draw(view, &mut cx);
        view.read_with(&cx, |this, _| assert_eq!(this.shell.search_board, None))
            .unwrap();
        cx.simulate_keystrokes("escape");
        measured(view, &mut cx, "inbox.tabs");
    }
}
