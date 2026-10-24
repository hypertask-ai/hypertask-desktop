use super::*;
use crate::command_center::BoardAction;
use gpui::MouseButton;
use ht_core::BoardWrite;

pub(super) struct Forms {
    input: Entity<TextInput>,
    form: Option<Form>,
}
struct Form {
    action: BoardAction,
    project: i64,
    section: i64,
    task: Option<i64>,
    destination: Option<i64>,
    pending: Option<String>,
    error: Option<String>,
    choice: usize,
}
impl Forms {
    pub fn new(cx: &mut Context<Desktop>) -> Self {
        let input = cx.new(|cx| TextInput::new(String::new(), "Name", color(0x8e9093), cx));
        cx.observe(&input, |this, _, cx| {
            if let Some(form) = &mut this.board_forms.form {
                form.choice = 0;
            }
            cx.notify();
        })
        .detach();
        Self { input, form: None }
    }
}
impl Desktop {
    pub(super) fn open_board_form(
        &mut self,
        action: BoardAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if matches!(action, BoardAction::AddColumn | BoardAction::RenameColumn | BoardAction::DeleteColumn) {
            self.views.column_auto_select = false;
            self.open_view_panel(if action == BoardAction::AddColumn { super::board_views::Panel::AddColumn } else { super::board_views::Panel::Columns }, window, cx);
            return;
        }
        self.task_fields.summary_open = false;
        if action == BoardAction::ArchiveBoard
            && !self
                .db
                .projects()
                .unwrap_or_default()
                .iter()
                .any(|project| {
                    project["id"] == self.options.board
                        && self
                            .db
                            .current_user_id()
                            .is_some_and(|user| project["ownerId"].as_i64() == Some(user))
                })
        {
            self.error = Some("Only board owner can manage this board".into());
            cx.notify();
            return;
        }
        let section = self.columns.get(self.column).map_or(0, |column| column.id);
        let task = self.selected_task();
        if action == BoardAction::MoveTask && task.is_none() {
            return;
        }
        self.board_forms.form = Some(Form {
            action,
            project: self.options.board,
            section,
            task,
            destination: None,
            pending: None,
            error: None,
            choice: 0,
        });
        self.board_forms
            .input
            .update(cx, |input, cx| input.set_content(String::new(), cx));
        if matches!(
            action,
            BoardAction::DeleteColumn | BoardAction::ArchiveBoard
        ) {
            window.focus(&self.focus, cx);
        } else {
            window.focus(&self.board_forms.input.focus_handle(cx), cx);
        }
        cx.notify();
    }
    fn close_board_form(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.board_forms.form = None;
        window.focus(&self.focus, cx);
        cx.notify();
    }
    fn board_form_choices(&self, cx: &App) -> Vec<(i64, String)> {
        let Some(form) = &self.board_forms.form else {
            return vec![];
        };
        if form.action != BoardAction::MoveTask {
            return vec![];
        }
        let query = self
            .board_forms
            .input
            .read(cx)
            .content
            .trim()
            .to_lowercase();
        let choices: Vec<_> = if let Some(project) = form.destination {
            self.db
                .board(project)
                .unwrap_or_default()
                .into_iter()
                .map(|column| (column.id, column.title))
                .collect()
        } else {
            self.db
                .projects()
                .unwrap_or_default()
                .into_iter()
                .filter_map(|project| {
                    Some((
                        project["id"].as_i64()?,
                        project["title"]
                            .as_str()
                            .or(project["name"].as_str())?
                            .to_owned(),
                    ))
                })
                .collect()
        };
        choices
            .into_iter()
            .filter(|(_, name)| name.to_lowercase().contains(&query))
            .collect()
    }
    fn submit_board_form(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(form) = &self.board_forms.form else {
            return;
        };
        if form.pending.is_some() {
            return;
        }
        let title = self.board_forms.input.read(cx).content.trim().to_owned();
        let change = match form.action {
            BoardAction::ArchiveBoard => BoardWrite::ArchiveBoard {
                project: form.project,
            },
            BoardAction::AddColumn if !title.is_empty() => BoardWrite::AddColumn {
                project: form.project,
                title,
            },
            BoardAction::RenameColumn if !title.is_empty() => BoardWrite::RenameColumn {
                project: form.project,
                section: form.section,
                title,
            },
            BoardAction::RenameBoard if !title.is_empty() => BoardWrite::RenameBoard {
                project: form.project,
                title,
            },
            BoardAction::DeleteColumn => BoardWrite::DeleteColumn {
                project: form.project,
                section: form.section,
            },
            BoardAction::MoveTask => {
                let Some((id, _)) = self.board_form_choices(cx).get(form.choice).cloned() else {
                    return;
                };
                if let Some(project) = form.destination {
                    BoardWrite::MoveTask {
                        task: form.task.unwrap(),
                        project,
                        section: id,
                    }
                } else {
                    self.board_forms.form.as_mut().unwrap().destination = Some(id);
                    self.board_forms
                        .input
                        .update(cx, |input, cx| input.set_content(String::new(), cx));
                    if let Some(worker) = &self.worker {
                        let _ = worker.handle().sync_board(id);
                    }
                    cx.notify();
                    return;
                }
            }
            _ => return,
        };
        match self
            .db
            .enqueue(Write::Board { change }, chrono::Utc::now().timestamp())
        {
            Ok(id) => {
                let key = self
                    .db
                    .mutations()
                    .ok()
                    .and_then(|rows| rows.into_iter().find(|row| row.id == id))
                    .map(|row| row.client_id)
                    .unwrap();
                self.board_forms.form.as_mut().unwrap().pending = Some(key);
                self.request_sync();
                cx.spawn_in(window, async move |view, cx| {
                    loop {
                        cx.background_executor()
                            .timer(Duration::from_millis(100))
                            .await;
                        if !view
                            .update_in(cx, |this, window, cx| this.poll_board_form(window, cx))
                            .unwrap_or(false)
                        {
                            break;
                        }
                    }
                })
                .detach();
            }
            Err(error) => self.board_forms.form.as_mut().unwrap().error = Some(error.to_string()),
        }
        cx.notify();
    }
    fn poll_board_form(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(key) = self
            .board_forms
            .form
            .as_ref()
            .and_then(|form| form.pending.clone())
        else {
            return false;
        };
        match self.db.take_board_result(&key) {
            Ok(Some(value)) => {
                if let Some(error) = value["error"].as_str() {
                    let form = self.board_forms.form.as_mut().unwrap();
                    form.error = Some(error.into());
                    form.pending = None;
                    cx.notify();
                } else {
                    let archived = self
                        .board_forms
                        .form
                        .as_ref()
                        .is_some_and(|form| form.action == BoardAction::ArchiveBoard);
                    let moved = self
                        .board_forms
                        .form
                        .as_ref()
                        .is_some_and(|form| form.action == BoardAction::MoveTask);
                    self.close_board_form(window, cx);
                    self.reload();
                    if archived {
                        self.switch(Screen::Inbox, cx);
                    }
                    if moved && self.screen == Screen::Task {
                        self.options.board = self
                            .detail
                            .as_ref()
                            .map_or(self.options.board, |detail| detail.task.project_id);
                    }
                    cx.notify();
                }
                false
            }
            Ok(None) => true,
            Err(error) => {
                self.error = Some(error.to_string());
                cx.notify();
                false
            }
        }
    }
    pub(super) fn board_form_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(form) = &self.board_forms.form else {
            return false;
        };
        let key = &event.keystroke;
        if key.key == "escape" {
            self.close_board_form(window, cx);
        } else if form.pending.is_none() {
            match key.key.as_str() {
                "enter" => self.submit_board_form(window, cx),
                "down" => {
                    let count = self.board_form_choices(cx).len();
                    let form = self.board_forms.form.as_mut().unwrap();
                    form.choice = (form.choice + 1).min(count.saturating_sub(1));
                }
                "up" => {
                    let form = self.board_forms.form.as_mut().unwrap();
                    form.choice = form.choice.saturating_sub(1);
                }
                _ => return true,
            }
        } else {
            cx.stop_propagation();
            return true;
        }
        cx.stop_propagation();
        cx.notify();
        true
    }
    pub(super) fn board_form_overlay(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(form) = &self.board_forms.form else {
            return div().into_any_element();
        };
        let t = self.colors;
        let title = match form.action {
            BoardAction::ArchiveBoard => "Archive board?",
            BoardAction::AddColumn => "Add Column",
            BoardAction::RenameColumn => "Rename board column",
            BoardAction::RenameBoard => "Change board name",
            BoardAction::DeleteColumn => "Delete board column?",
            BoardAction::MoveTask if form.destination.is_none() => "Select Board",
            BoardAction::MoveTask => "Select Column",
        };
        let mut content = div().flex().flex_col().gap(px(12.)).child(title);
        if !matches!(
            form.action,
            BoardAction::DeleteColumn | BoardAction::ArchiveBoard
        ) {
            content = content.child(self.board_forms.input.clone());
        }
        if form.action == BoardAction::MoveTask {
            content = content.children(self.board_form_choices(cx).into_iter().enumerate().map(
                |(i, (_, name))| {
                    div()
                        .id(("board-form-choice", i))
                        .p(px(8.))
                        .cursor_pointer()
                        .child(name)
                        .when(i == form.choice, |d| d.bg(color(t.hover)))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            if let Some(form) = &mut this.board_forms.form {
                                form.choice = i;
                            }
                            this.submit_board_form(window, cx);
                        }))
                        .layout_key(&self.layout, format!("board.form.choice.{i}"), None)
                },
            ));
        } else {
            if form.action == BoardAction::DeleteColumn {
                content = content.child(
                    self.columns
                        .iter()
                        .find(|c| c.id == form.section)
                        .map_or(String::new(), |c| {
                            format!("Delete board column “{}”?", c.title)
                        }),
                );
            }
            content = content.child(
                div()
                    .id("board-form-submit")
                    .cursor_pointer()
                    .child(if form.action == BoardAction::DeleteColumn {
                        "Delete board column"
                    } else if form.action == BoardAction::ArchiveBoard {
                        "Archive board"
                    } else {
                        "Save"
                    })
                    .on_click(cx.listener(|this, _, window, cx| this.submit_board_form(window, cx)))
                    .layout_key(&self.layout, "board.form.submit", None),
            );
        }
        content = content.when_some(form.error.clone(), |d, error| {
            d.child(div().text_color(color(0xcf222e)).child(error))
        });
        div()
            .id("board-form-backdrop")
            .absolute()
            .size_full()
            .top_0()
            .left_0()
            .bg(rgba(0x00000080))
            .flex()
            .items_center()
            .justify_center()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|_, _, _, cx| cx.stop_propagation()),
            )
            .on_click(cx.listener(|this, _, window, cx| this.close_board_form(window, cx)))
            .child(
                div()
                    .id("board-form-dialog")
                    .w(px(560.))
                    .max_h(px(600.))
                    .overflow_y_scroll()
                    .p(px(20.))
                    .bg(color(t.comment))
                    .rounded(px(5.))
                    .shadow(shadow())
                    .on_click(cx.listener(|_, _, _, cx| cx.stop_propagation()))
                    .child(content)
                    .layout_key(&self.layout, "board.form", None),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::shell::tests::{click, draw};
    use crate::ui::task_fields::tests::desktop;
    use gpui::{VisualTestContext, WindowHandle};
    use ht_core::{
        Transport,
        http::{Response, TransportError},
    };
    use serde_json::{Value, json};
    struct Server(Value);
    impl Transport for Server {
        fn get(&mut self, path: &str) -> Result<Response, TransportError> {
            assert_eq!(path, "/user/context");
            Ok(Response {
                status: 200,
                body: json!({"success":true,"user":{"id":42}}),
            })
        }
        fn post(
            &mut self,
            path: &str,
            body: &Value,
            key: &str,
        ) -> Result<Response, TransportError> {
            self.mutate("POST", path, body, key)
        }
        fn mutate(
            &mut self,
            _: &str,
            _: &str,
            _: &Value,
            key: &str,
        ) -> Result<Response, TransportError> {
            assert!(!key.is_empty());
            Ok(Response {
                status: 200,
                body: self.0.clone(),
            })
        }
    }
    fn setup(cx: &mut gpui::TestAppContext) -> (WindowHandle<Desktop>, VisualTestContext) {
        cx.update(crate::input::init);
        let view = cx.open_window(gpui::size(px(1440.), px(900.)), |window, cx| {
            let mut v = desktop(window, cx);
            v.screen = Screen::Board;
            v.reload();
            v
        });
        let mut cx = VisualTestContext::from_window(view.into(), cx);
        draw(view, &mut cx);
        (view, cx)
    }
    fn palette(cx: &mut VisualTestContext, name: &str) {
        cx.simulate_keystrokes("ctrl-k");
        cx.simulate_input(name);
        cx.run_until_parked();
        cx.simulate_keystrokes("enter");
    }
    #[gpui::test]
    fn add_column_palette_enter_is_single_flight_and_reveals_real_acknowledged_column(
        cx: &mut gpui::TestAppContext,
    ) {
        let (view, mut cx) = setup(cx);
        palette(&mut cx, "Add board column");
        cx.simulate_input("Review");
        cx.simulate_keystrokes("enter enter");
        view.update(&mut cx,|this,window,cx|{
            assert_eq!(this.db.mutations().unwrap().len(),1);assert_eq!(this.columns.len(),3);
            let mut server=Server(json!({"success":true,"section":{"id":12,"projectId":15,"section_title":"Review","visibility":true,"deleted":false,"ranking":"c"},"project_view":{"id":"pv","projectId":15,"user_project_views":[{}]}}));
            assert!(matches!(this.db.drain_once(&mut server,chrono::Utc::now().timestamp()+1).unwrap(),ht_core::Drain::Applied(_)));
            assert!(!this.poll_view(window,cx));assert_eq!(this.columns[2].title,"Review");
        }).unwrap();
    }
    #[gpui::test]
    fn rename_and_delete_commands_open_the_web_column_manager_without_writing(
        cx: &mut gpui::TestAppContext,
    ) {
        let (view, mut cx) = setup(cx);
        for name in ["Rename board column", "Delete board column"] {
            palette(&mut cx, name);
            crate::ui::shell::tests::measured(view, &mut cx, "board.columns.dialog");
            view.read_with(&cx, |this, _| assert!(this.db.mutations().unwrap().is_empty())).unwrap();
            cx.simulate_keystrokes("escape");
        }
    }
    #[gpui::test]
    fn move_board_shortcut_uses_search_then_destination_column_and_never_guesses(
        cx: &mut gpui::TestAppContext,
    ) {
        let (view, mut cx) = setup(cx);
        view.update(&mut cx,|this,_,_|{this.db.apply_response(&json!({"success":true,"projects":[{"id":16,"title":"Second"}],"sections":[{"id":20,"projectId":16,"section_title":"Target","visibility":true,"deleted":false,"ranking":"a"}]})).unwrap();}).unwrap();
        cx.simulate_keystrokes("shift-m");
        cx.simulate_input("Second");
        cx.run_until_parked();
        cx.simulate_keystrokes("enter");
        click(view, &mut cx, "board.form.choice.0");
        view.read_with(&mut cx, |this, _| {
            assert_eq!(
                this.db.mutations().unwrap()[0].payload_json["change"]["MoveTask"],
                json!({"task":1,"project":16,"section":20})
            );
            assert_eq!(this.db.task(1).unwrap().unwrap().project_id, 15);
        })
        .unwrap();
    }
    #[gpui::test]
    fn inline_create_tag_queues_name_then_only_assigns_acknowledged_server_label(
        cx: &mut gpui::TestAppContext,
    ) {
        let (view, mut cx) = setup(cx);
        view.update(&mut cx, |this, _, cx| this.open_task(1, cx))
            .unwrap();
        click(view, &mut cx, "task.meta.8");
        cx.simulate_input("Fresh");
        cx.run_until_parked();
        cx.simulate_keystrokes("enter");
        view.update(&mut cx, |this, _, _| {
            assert_eq!(
                this.db.mutations().unwrap()[0].payload_json["change"]["CreateTaskLabel"]["name"],
                "Fresh"
            );
            assert_eq!(this.db.task(1).unwrap().unwrap().label_count, 0);
            this.db
                .drain_once(
                    &mut Server(json!({"success":true,"label":{"id":"fresh-id","name":"Fresh"}})),
                    chrono::Utc::now().timestamp() + 1,
                )
                .unwrap();
            assert_eq!(
                this.db.mutations().unwrap()[0].payload_json,
                json!({"task_id":1,"labels":["fresh-id"]})
            );
        })
        .unwrap();
    }
}

#[cfg(test)]
mod archive_tests {
    use super::*;
    use crate::ui::task_fields::tests::desktop;
    use serde_json::json;
    #[gpui::test]
    fn archive_board_palette_is_owner_only_cancel_is_no_write_and_confirm_is_queued(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(crate::input::init);
        let (view, cx) = cx.add_window_view(|window, cx| {
            let mut v = desktop(window, cx);
            v.screen = Screen::Board;
            v.reload();
            v
        });
        view.update_in(cx, |this, window, cx| {
            this.open_board_form(BoardAction::ArchiveBoard, window, cx);
            assert!(this.board_forms.form.is_none());
            assert!(this.error.as_ref().unwrap().contains("Only board owner"));
            this.db
                .apply_response(
                    &json!({"success":true,"projects":[{"id":15,"title":"Board","ownerId":42}]}),
                )
                .unwrap();
        });
        cx.simulate_keystrokes("ctrl-k");
        cx.simulate_input("Archive board");
        cx.run_until_parked();
        cx.simulate_keystrokes("enter escape");
        view.read_with(cx, |this, _| {
            assert!(this.db.mutations().unwrap().is_empty());
            assert!(this.board_forms.form.is_none());
        });
        cx.simulate_keystrokes("ctrl-k");
        cx.simulate_input("Archive board");
        cx.run_until_parked();
        cx.simulate_keystrokes("enter enter");
        view.read_with(cx, |this, _| {
            assert_eq!(
                this.db.mutations().unwrap()[0].payload_json["change"]["ArchiveBoard"]["project"],
                15
            )
        });
    }
}
