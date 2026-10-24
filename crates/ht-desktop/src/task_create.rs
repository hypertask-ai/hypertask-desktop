use super::*;
use serde_json::{Value, json};
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Field {
    Board,
    Column,
    Priority,
    Size,
    Tags,
    Assignees,
    DueDate,
    StartDate,
}
pub(super) struct Create {
    open: bool,
    title: Entity<TextInput>,
    description: Entity<TextInput>,
    date: Entity<TextInput>,
    project: i64,
    section: i64,
    fields: Value,
    picker: Option<Field>,
    choice: usize,
    pending: Option<String>,
    error: Option<String>,
    discard: bool,
    loaded: Option<i64>,
    loading: bool,
    generation: u64,
    initial: Value,
}
impl Create {
    pub fn new(cx: &mut Context<Desktop>) -> Self {
        let title = cx.new(|cx| TextInput::new(String::new(), "Task title", color(0x8e9093), cx));
        let description = cx.new(|cx| {
            TextInput::new(String::new(), "Add Description", color(0x8e9093), cx).multiline_editor()
        });
        let date = cx.new(|cx| TextInput::new(String::new(), "Set a date…", color(0x8e9093), cx));
        for input in [&title, &description, &date] {
            cx.observe(input, |_, _, cx| cx.notify()).detach();
        }
        Self {
            open: false,
            title,
            description,
            date,
            project: 0,
            section: 0,
            fields: json!({}),
            picker: None,
            choice: 0,
            pending: None,
            error: None,
            discard: false,
            loaded: None,
            loading: false,
            generation: 0,
            initial: Value::Null,
        }
    }
}
impl Desktop {
    pub(super) fn invalidate_task_create(&mut self, cx: &mut Context<Self>) {
        let generation = self.create_task.generation + 1;
        self.create_task = Create::new(cx);
        self.create_task.generation = generation;
    }
    pub(super) fn open_create_task(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.create_task.pending.is_some() {
            return;
        }
        if self.create_task.project == 0 {
            self.create_task.project = self
                .detail
                .as_ref()
                .filter(|_| self.screen == Screen::Task)
                .map_or(self.options.board, |detail| detail.task.project_id);
            self.reset_create_destination(cx);
        }
        self.create_task.open = true;
        self.create_task.discard = false;
        if !self.options.offline {
            self.load_create_board(cx);
        }
        self.interaction.modal = None;
        window.focus(&self.create_task.title.focus_handle(cx), cx);
        cx.notify();
    }
    fn reset_create_destination(&mut self, cx: &mut Context<Self>) {
        let project = self.create_task.project;
        let columns = self.db.board(project).unwrap_or_default();
        self.create_task.section = columns.first().map_or(0, |column| column.id);
        self.create_task.fields = self
            .db
            .board_project(project)
            .map(|project| ht_core::task_create::view_defaults(&project))
            .unwrap_or(json!({"assignees":[]}));
        if project == self.options.board {
            if let Some(column) = columns.get(self.column) {
                self.create_task.section = column.id;
            }
        }
        self.create_task.initial = self.create_task.fields.clone();
        cx.notify();
    }
    fn load_create_board(&mut self, cx: &mut Context<Self>) {
        let project = self.create_task.project;
        if self.create_task.loaded == Some(project) || self.create_task.loading {
            return;
        }
        let Some(token) = self.cli_token() else {
            return;
        };
        let expected = token.clone();
        self.create_task.generation += 1;
        let generation = self.create_task.generation;
        self.create_task.loading = true;
        self.create_task.loaded = None;
        let (send, receive) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let result = ht_core::Http::new(token)
                .and_then(|mut http| ht_core::board_views::load(&mut http, project))
                .map_err(|error| error.to_string());
            let _ = send.send(result);
        });
        cx.spawn(async move |view, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(50))
                    .await;
                match receive.try_recv() {
                    Ok(result) => {
                        let _ = view.update(cx, |this, cx| {
                            if this.create_task.generation != generation {
                                return;
                            }
                            this.create_task.loading = false;
                            if this.create_task.project != project
                                || this.cli_token().as_deref() != Some(expected.as_str())
                            {
                                return;
                            }
                            match result {
                                Ok((user, data)) => match this.db.apply_web_board(user, &data) {
                                    Ok(()) => {
                                        this.create_task.loaded = Some(project);
                                        this.create_task.error = None;
                                        if this.create_task.section == 0 {
                                            this.create_task.section = this
                                                .db
                                                .board(project)
                                                .unwrap_or_default()
                                                .first()
                                                .map_or(0, |column| column.id);
                                        }
                                    }
                                    Err(error) => this.create_task.error = Some(error.to_string()),
                                },
                                Err(error) => this.create_task.error = Some(error),
                            }
                            cx.notify();
                        });
                        break;
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => (),
                    _ => break,
                }
            }
        })
        .detach();
    }
    fn create_choices(&self, cx: &App) -> Vec<(String, Value)> {
        match self.create_task.picker {
            Some(Field::Board) => self
                .db
                .projects()
                .unwrap_or_default()
                .into_iter()
                .map(|project| {
                    (
                        project["title"].as_str().unwrap_or("").into(),
                        project["id"].clone(),
                    )
                })
                .collect(),
            Some(Field::Column) => self
                .db
                .board_project(self.create_task.project)
                .ok()
                .into_iter()
                .flat_map(|project| project["section"].as_array().cloned().unwrap_or_default())
                .filter(|column| column["deleted"] != true && column["visibility"] == true)
                .map(|column| {
                    (
                        column["section_title"].as_str().unwrap_or("").into(),
                        column["id"].clone(),
                    )
                })
                .collect(),
            Some(Field::Priority) => (0..5)
                .filter_map(ht_core::task_create::priority)
                .map(|value| (value["Priority_Value"].as_str().unwrap().into(), value))
                .collect(),
            Some(Field::Size) => [0, 2, 3, 4, 5, 6]
                .into_iter()
                .filter_map(ht_core::task_create::estimate)
                .map(|value| (value["estimate_value"].as_str().unwrap().into(), value))
                .collect(),
            Some(Field::Tags) => self
                .db
                .project_labels(self.create_task.project)
                .unwrap_or_default()
                .into_iter()
                .map(|value| {
                    (
                        value["name"]
                            .as_str()
                            .or(value["value"].as_str())
                            .unwrap_or("")
                            .into(),
                        value,
                    )
                })
                .collect(),
            Some(Field::Assignees) => {
                let data = self
                    .db
                    .kv_get(&format!("create-people:{}", self.create_task.project))
                    .ok()
                    .flatten()
                    .and_then(|value| serde_json::from_str::<Value>(&value).ok())
                    .unwrap_or(Value::Null);
                let mut people = vec![];
                for person in std::iter::once(&data["owner"])
                    .chain(
                        data["members"]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .map(|member| &member["user"]),
                    )
                    .chain(data["boardAgents"].as_array().into_iter().flatten())
                {
                    if !person.is_null()
                        && !people.iter().any(|old: &Value| old["id"] == person["id"])
                    {
                        people.push(person.clone());
                    }
                }
                people
                    .into_iter()
                    .map(|value| (value["displayName"].as_str().unwrap_or("").into(), value))
                    .collect()
            }
            Some(Field::DueDate | Field::StartDate) => {
                let query = self.create_task.date.read(cx).content.trim();
                if query.is_empty() {
                    std::iter::once(("Reset".into(), Value::Null))
                        .chain(
                            super::task_fields::due_presets(chrono::Local::now())
                                .into_iter()
                                .map(|(name, value)| (name, json!(value))),
                        )
                        .collect()
                } else {
                    super::task_fields::parse_due(query, chrono::Local::now().date_naive())
                        .map(|value| vec![(query.into(), json!(value))])
                        .unwrap_or_default()
                }
            }
            _ => vec![],
        }
    }
    fn choose_create_field(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some((_, value)) = self.create_choices(cx).get(index).cloned() else {
            return;
        };
        match self.create_task.picker {
            Some(Field::Board) => {
                self.create_task.project = value.as_i64().unwrap();
                self.create_task.loading = false;
                self.create_task.loaded = None;
                self.reset_create_destination(cx);
                if !self.options.offline {
                    self.load_create_board(cx);
                }
            }
            Some(Field::DueDate) => self.create_task.fields["dueDate"] = value,
            Some(Field::StartDate) => self.create_task.fields["startDate"] = value,
            Some(Field::Column) => self.create_task.section = value.as_i64().unwrap(),
            Some(Field::Priority) => self.create_task.fields["priority"] = value,
            Some(Field::Size) => self.create_task.fields["estimate"] = value,
            Some(field @ (Field::Tags | Field::Assignees)) => {
                let key = if field == Field::Tags {
                    "tags"
                } else {
                    "assignees"
                };
                let mut values = self.create_task.fields[key]
                    .as_array()
                    .cloned()
                    .unwrap_or_default();
                if let Some(index) = values.iter().position(|old| old["id"] == value["id"]) {
                    values.remove(index);
                } else {
                    values.push(value);
                }
                self.create_task.fields[key] = json!(values);
                cx.notify();
                return;
            }
            _ => return,
        }
        self.create_task.picker = None;
        window.focus(&self.create_task.title.focus_handle(cx), cx);
        cx.notify();
    }
    fn create_picker(&mut self, field: Field, window: &mut Window, cx: &mut Context<Self>) {
        if self.create_task.pending.is_some() {
            return;
        }
        self.create_task.picker = Some(field);
        self.create_task.choice = 0;
        if matches!(field, Field::DueDate | Field::StartDate) {
            self.create_task
                .date
                .update(cx, |input, cx| input.set_content(String::new(), cx));
            window.focus(&self.create_task.date.focus_handle(cx), cx);
        } else {
            window.focus(&self.focus, cx);
        }
        if field == Field::Assignees && !self.options.offline {
            if let Some(token) = self.cli_token() {
                let project = self.create_task.project;
                let expected = token.clone();
                let (send, receive) = std::sync::mpsc::channel();
                std::thread::spawn(move || {
                    let result = ht_core::Http::new(token)
                        .and_then(|mut http| {
                            ht_core::board_views::filter_people(&mut http, project)
                        })
                        .map_err(|error| error.to_string());
                    let _ = send.send(result);
                });
                cx.spawn(async move |view, cx| {
                    loop {
                        cx.background_executor()
                            .timer(Duration::from_millis(50))
                            .await;
                        match receive.try_recv() {
                            Ok(result) => {
                                let _ = view.update(cx, |this, cx| {
                                    if this.create_task.project != project
                                        || this.cli_token().as_deref() != Some(expected.as_str())
                                    {
                                        return;
                                    }
                                    match result {
                                        Ok(data) => {
                                            let _ = this.db.kv_set(
                                                &format!("create-people:{project}"),
                                                &data.to_string(),
                                            );
                                        }
                                        Err(error) => this.create_task.error = Some(error),
                                    }
                                    cx.notify();
                                });
                                break;
                            }
                            Err(std::sync::mpsc::TryRecvError::Empty) => (),
                            _ => break,
                        }
                    }
                })
                .detach();
            }
        }
        cx.notify();
    }
    fn submit_create_task(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.create_task.pending.is_some()
            || self.create_task.loading
            || (!self.options.offline && self.create_task.loaded != Some(self.create_task.project))
        {
            return;
        }
        let mut fields = self.create_task.fields.clone();
        fields["description"] = json!(self.create_task.description.read(cx).html());
        let result = self.db.enqueue(
            Write::CreateTaskForm {
                project_id: self.create_task.project,
                section_id: self.create_task.section,
                title: self.create_task.title.read(cx).content.to_string(),
                fields,
            },
            chrono::Utc::now().timestamp(),
        );
        match result {
            Ok(id) => {
                self.create_task.pending = self
                    .db
                    .mutations()
                    .ok()
                    .and_then(|rows| rows.into_iter().find(|row| row.id == id))
                    .map(|row| row.client_id);
                self.create_task.error = None;
                self.reload();
                self.request_sync();
                if !self.options.offline {
                    cx.spawn_in(window, async move |view, cx| {
                        loop {
                            cx.background_executor()
                                .timer(Duration::from_millis(100))
                                .await;
                            if !view
                                .update_in(cx, |this, window, cx| this.poll_create_task(window, cx))
                                .unwrap_or(false)
                            {
                                break;
                            }
                        }
                    })
                    .detach();
                }
            }
            Err(error) => self.create_task.error = Some(error.to_string()),
        }
        cx.notify();
    }
    pub(super) fn poll_create_task(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(key) = self
            .create_task
            .pending
            .as_ref()
            .map(|key| format!("create-result:{key}"))
        else {
            return false;
        };
        let Some(result) = self
            .db
            .kv_get(&key)
            .ok()
            .flatten()
            .and_then(|value| serde_json::from_str::<Value>(&value).ok())
        else {
            return true;
        };
        self.create_task.pending = None;
        self.reload();
        if result["error"].is_null() {
            self.create_task.open = false;
            self.create_task.project = 0;
            for input in [
                &self.create_task.title,
                &self.create_task.description,
                &self.create_task.date,
            ] {
                input.update(cx, |input, cx| input.reset(window, cx));
            }
            window.focus(&self.focus, cx);
        } else {
            self.create_task.error = Some("Task could not be created".into());
        }
        cx.notify();
        false
    }
    pub(super) fn create_task_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.create_task.open {
            return false;
        }
        let key = &event.keystroke;
        match key.key.as_str() {
            "escape" if self.create_task.pending.is_none() => {
                if self.create_task.picker.take().is_none() {
                    if self.create_task.title.read(cx).is_empty()
                        && self.create_task.description.read(cx).is_empty()
                        && self.create_task.fields == self.create_task.initial
                    {
                        self.create_task.open = false;
                        window.focus(&self.focus, cx);
                    } else {
                        self.create_task.discard = true;
                    }
                }
            }
            "enter" if key.modifiers.control || key.modifiers.platform => {
                self.submit_create_task(window, cx)
            }
            "enter" if self.create_task.picker.is_some() => {
                self.choose_create_field(self.create_task.choice, window, cx)
            }
            "up" if self.create_task.picker.is_some() => {
                self.create_task.choice = self.create_task.choice.saturating_sub(1)
            }
            "down" if self.create_task.picker.is_some() => {
                self.create_task.choice = (self.create_task.choice + 1)
                    .min(self.create_choices(cx).len().saturating_sub(1))
            }
            _ => return true,
        }
        cx.stop_propagation();
        cx.notify();
        true
    }
    pub(super) fn create_task_overlay(&self, cx: &mut Context<Self>) -> AnyElement {
        if !self.create_task.open {
            return div().into_any_element();
        }
        let mut content = div()
            .id("create-task-form")
            .w(px(760.))
            .max_h(px(680.))
            .overflow_y_scroll()
            .p(px(24.))
            .bg(color(self.colors.card))
            .rounded(px(6.))
            .flex()
            .flex_col()
            .gap(px(12.))
            .child("Create task")
            .child(self.create_task.title.clone())
            .child(self.create_task.description.clone());
        content = content.child(
            div().flex().gap(px(12.)).children(
                [
                    (Field::Board, "Board"),
                    (Field::Column, "Status"),
                    (Field::Assignees, "Assignees"),
                    (Field::Priority, "Priority"),
                    (Field::Size, "Task size"),
                    (Field::Tags, "Tags"),
                    (Field::DueDate, "Due date"),
                    (Field::StartDate, "Start date"),
                ]
                .into_iter()
                .enumerate()
                .map(|(index, (field, label))| {
                    div()
                        .id(("create-task-field", index))
                        .cursor_pointer()
                        .child(format!(
                            "{label}: {}",
                            match field {
                                Field::Board => self
                                    .db
                                    .board_project(self.create_task.project)
                                    .ok()
                                    .and_then(|project| project["title"]
                                        .as_str()
                                        .map(str::to_owned))
                                    .unwrap_or_default(),
                                Field::Column => self
                                    .db
                                    .board(self.create_task.project)
                                    .unwrap_or_default()
                                    .into_iter()
                                    .find(|column| column.id == self.create_task.section)
                                    .map(|column| column.title)
                                    .unwrap_or_default(),
                                Field::Priority =>
                                    self.create_task.fields["priority"]["Priority_Value"]
                                        .as_str()
                                        .unwrap_or("No Priority")
                                        .into(),
                                Field::Size =>
                                    self.create_task.fields["estimate"]["estimate_value"]
                                        .as_str()
                                        .unwrap_or("No size")
                                        .into(),
                                Field::Tags | Field::Assignees =>
                                    self.create_task.fields[if field == Field::Tags {
                                        "tags"
                                    } else {
                                        "assignees"
                                    }]
                                    .as_array()
                                    .into_iter()
                                    .flatten()
                                    .filter_map(|value| value["displayName"]
                                        .as_str()
                                        .or(value["name"].as_str())
                                        .or(value["value"].as_str()))
                                    .collect::<Vec<_>>()
                                    .join(", "),
                                Field::DueDate | Field::StartDate => self.create_task.fields
                                    [if field == Field::DueDate {
                                        "dueDate"
                                    } else {
                                        "startDate"
                                    }]
                                .as_str()
                                .unwrap_or("")
                                .into(),
                            }
                        ))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.create_picker(field, window, cx)
                        }))
                }),
            ),
        );
        if matches!(
            self.create_task.picker,
            Some(Field::DueDate | Field::StartDate)
        ) {
            content = content.child(self.create_task.date.clone());
        }
        if self.create_task.picker.is_some() {
            content = content.children(self.create_choices(cx).into_iter().enumerate().map(
                |(index, (label, _))| {
                    div()
                        .id(("create-task-option", index))
                        .p(px(8.))
                        .cursor_pointer()
                        .when(index == self.create_task.choice, |d| {
                            d.bg(color(self.colors.active))
                        })
                        .child(label)
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.choose_create_field(index, window, cx)
                        }))
                },
            ));
        }
        content = content
            .when_some(self.create_task.error.clone(), |d, error| d.child(error))
            .child(
                div()
                    .id("create-task-save")
                    .p(px(8.))
                    .cursor_pointer()
                    .child("Create task")
                    .on_click(
                        cx.listener(|this, _, window, cx| this.submit_create_task(window, cx)),
                    ),
            );
        if self.create_task.discard {
            content = content.child(
                div()
                    .child("Discard unsaved changes?")
                    .child(
                        div()
                            .id("create-task-discard")
                            .cursor_pointer()
                            .child("Discard")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.create_task.open = false;
                                this.create_task.discard = false;
                                this.create_task.project = 0;
                                for input in [
                                    &this.create_task.title,
                                    &this.create_task.description,
                                    &this.create_task.date,
                                ] {
                                    input.update(cx, |input, cx| input.reset(window, cx));
                                }
                                window.focus(&this.focus, cx);
                                cx.notify();
                            })),
                    )
                    .child(
                        div()
                            .id("create-task-keep")
                            .cursor_pointer()
                            .child("Cancel")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.create_task.discard = false;
                                cx.notify();
                            })),
                    ),
            );
        }
        div()
            .absolute()
            .inset_0()
            .bg(rgba(0x00000040))
            .flex()
            .items_center()
            .justify_center()
            .child(content)
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::task_fields::tests::desktop;
    struct Server(u16);
    impl ht_core::Transport for Server {
        fn get(
            &mut self,
            path: &str,
        ) -> Result<ht_core::http::Response, ht_core::http::TransportError> {
            assert_eq!(path, "/user/context");
            Ok(ht_core::http::Response {
                status: 200,
                body: json!({"success":true,"user":{"id":42}}),
            })
        }
        fn post(
            &mut self,
            path: &str,
            body: &Value,
            _: &str,
        ) -> Result<ht_core::http::Response, ht_core::http::TransportError> {
            assert_eq!(path, "/api/tasks/createGlobally");
            assert_eq!(body["title"], "New form task");
            assert_eq!(body["description"], "<p>Draft description</p>");
            assert_eq!(body["priority"]["priority_index"], 2);
            assert_eq!(body["fullScreenTask"], true);
            Ok(ht_core::http::Response {
                status: self.0,
                body: if self.0 == 200 {
                    let mut task = body.clone();
                    task["id"] = json!(500);
                    task["status"] = json!("Normal");
                    json!({"newTask":task})
                } else {
                    json!({"error":"Unauthorized"})
                },
            })
        }
    }
    #[gpui::test]
    fn expanded_create_form_keeps_all_edits_and_rolls_back_optimistic_task_on_both_refusals(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(crate::input::init);
        let (view, cx) = cx.add_window_view(|window, cx| desktop(window, cx));
        for status in [401, 403, 200] {
            view.update_in(cx, |this, window, cx| {
                this.open_create_task(window, cx);
                this.create_task.title.update(cx, |input, cx| {
                    input.set_content("New form task".into(), cx)
                });
                this.create_task.description.update(cx, |input, cx| {
                    input.set_html("<p>Draft description</p>", cx)
                });
                this.create_picker(Field::Priority, window, cx);
                this.choose_create_field(2, window, cx);
                assert_eq!(this.create_task.fields["priority"]["priority_index"], 2);
                this.submit_create_task(window, cx);
                let rows = this.db.mutations().unwrap();
                let row = rows.iter().find(|row| row.status != "failed").unwrap();
                let id = row.id;
                assert!(this.db.task(-id).unwrap().is_some());
                this.submit_create_task(window, cx);
                assert_eq!(this.db.mutations().unwrap().len(), rows.len());
                this.db
                    .drain_once(&mut Server(status), chrono::Utc::now().timestamp() + 1)
                    .unwrap();
                assert!(!this.poll_create_task(window, cx));
                assert!(this.db.task(-id).unwrap().is_none());
                assert_eq!(this.create_task.open, status != 200);
                if status != 200 {
                    assert_eq!(
                        this.create_task.title.read(cx).content.as_ref(),
                        "New form task"
                    );
                    assert_eq!(
                        this.create_task.description.read(cx).html(),
                        "<p>Draft description</p>"
                    );
                    assert_eq!(
                        this.create_task.error.as_deref(),
                        Some("Task could not be created")
                    );
                } else {
                    assert!(this.db.task(500).unwrap().is_some());
                    assert!(this.create_task.title.read(cx).is_empty());
                }
            });
        }
    }
    #[gpui::test]
    fn create_destination_dates_multi_values_and_account_reset_do_not_mutate_existing_tasks(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(crate::input::init);
        let (view, cx) = cx.add_window_view(|window, cx| desktop(window, cx));
        view.update_in(cx, |this, window, cx| {
            this.open_create_task(window, cx);
            this.create_picker(Field::Column, window, cx);
            this.choose_create_field(1, window, cx);
            assert_eq!(this.create_task.section, this.columns[1].id);
            this.create_picker(Field::DueDate, window, cx);
            this.create_task.date.update(cx, |input, cx| {
                input.set_content("2026-12-15 13:45".into(), cx)
            });
            this.choose_create_field(0, window, cx);
            assert!(
                this.create_task.fields["dueDate"]
                    .as_str()
                    .unwrap()
                    .contains("13:45")
            );
            let before = this.db.task(1).unwrap().unwrap().title;
            this.invalidate_task_create(cx);
            assert!(!this.create_task.open);
            assert!(this.create_task.title.read(cx).is_empty());
            assert_eq!(this.db.task(1).unwrap().unwrap().title, before);
            assert!(this.db.mutations().unwrap().is_empty());
        });
    }
}
