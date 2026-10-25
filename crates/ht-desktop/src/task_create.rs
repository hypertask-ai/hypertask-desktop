use super::*;
use serde_json::{Value, json};
use chrono::{Datelike, NaiveDate, TimeZone};
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
    custom_date: bool,
    month: NaiveDate,
    selected_date: NaiveDate,
    source_loading: bool,
    edited: std::collections::BTreeSet<&'static str>,
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
            custom_date: false,
            month: chrono::Local::now().date_naive().with_day(1).unwrap(),
            selected_date: chrono::Local::now().date_naive(),
            source_loading: false,
            edited: Default::default(),
        }
    }
}
impl Desktop {
    pub(super) fn open_comment_task(&mut self, comment: &ht_core::CommentView, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if self.create_task.open || self.create_task.pending.is_some() || self.create_task.source_loading { return; }
        let Some(task) = self.detail.as_ref().map(|detail| detail.task.clone()) else { return; };
        if comment.task_id != task.id { return; }
        let Some(unique) = task.extra["uniqueIndex"].as_i64().filter(|index| *index > 0) else { return; };
        let Some(creator) = comment.extra.get("creator").filter(|creator| creator["id"].as_i64().is_some()) else { return; };
        let escape = ht_core::desktop_cli::escape;
        let name = escape(creator["displayName"].as_str().unwrap_or(""));
        let description = format!("<p><span data-type=\"mention\" class=\"mention\" data-id=\"{name}\" data-label=\"name-{}\" uniqueindex=\"\" projectid=\"\">{name}</span> said in <a target=\"_blank\" rel=\"noopener noreferrer nofollow\" href=\"/detail/project-{}/{unique}#comment-{index}\">{}</a><blockquote>{}</blockquote></p>", creator["id"], task.project_id, escape(task.ticket_number.as_deref().unwrap_or("")), comment.text);
        let files: Vec<_> = comment.extra.get("attachments").and_then(Value::as_array).into_iter().flatten()
            .filter(|file| file["fileSource"].as_str().is_some_and(|source| !source.is_empty()))
            .map(|file| {
                let mut value = json!({"urlString":file["fileSource"],"Attachment":true,"title":file["fileName"],"attachmentType":file["fileType"]});
                let size = file["fileSize"].as_u64().or_else(|| file["fileSize"].as_str().filter(|size| !size.is_empty() && size.bytes().all(|byte| byte.is_ascii_digit())).and_then(|size| size.parse::<u64>().ok()));
                if let Some(size) = size.filter(|size| *size <= 9_007_199_254_740_991) { value["fileSize"] = json!(size); }
                value
            }).collect();
        self.invalidate_task_create(cx);
        self.create_task.project = task.project_id;
        self.create_task.section = task.section_id.unwrap_or(0);
        self.create_task.fields = json!({"createTaskFromComment":{"task":task,"commentIndex":index},"urlsToAdd":files});
        self.reset_create_destination(cx);
        self.create_task.section = task.section_id.unwrap_or(0);
        self.create_task.initial = self.create_task.fields.clone();
        self.create_task.description.update(cx, |input, cx| input.set_html(&description, cx));
        self.open_create_task(window, cx);
    }
    pub(super) fn open_template_task(&mut self,id:i64,window:&mut Window,cx:&mut Context<Self>) {
        if self.create_task.open || self.create_task.pending.is_some(){return;}
        let project=self.task_actions.templates_project;
        let source=match ht_core::task_create::template_source(&self.task_actions.templates,project,id){Ok(source)=>source,Err(error)=>{self.error=Some(error.to_string());cx.notify();return;}};
        self.invalidate_task_create(cx);
        self.create_task.project=project;self.create_task.section=source["sectionId"].as_i64().unwrap_or(0);
        self.create_task.fields=json!({"tags":source["tags"]});
        self.create_task.edited.insert("tags");
        for key in ["priority","estimate"]{if source[key].is_object(){self.create_task.fields[key]=source[key].clone();self.create_task.edited.insert(key);}}
        self.create_task.initial=self.create_task.fields.clone();
        self.create_task.title.update(cx,|input,cx|input.set_content(source["title"].as_str().unwrap_or("").into(),cx));
        self.create_task.description.update(cx,|input,cx|input.set_html(source["description"].as_str().unwrap_or("<p></p>"),cx));
        self.open_create_task(window,cx);
    }
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
    pub(super) fn open_duplicate_task(&mut self, pick_board: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.create_task.open || self.create_task.pending.is_some() || self.create_task.source_loading { return; }
        let Some(task) = self.selected_task().filter(|id| *id > 0) else { return; };
        self.create_task.generation += 1;
        let generation = self.create_task.generation;
        self.create_task.source_loading = true;
        if self.options.offline {
            let result = self.db.task(task).map_err(|error|error.to_string()).and_then(|task| task.ok_or_else(||"Task is unavailable".into())).and_then(|task|serde_json::to_value(task).map_err(|error|error.to_string()));
            self.apply_duplicate_source(result, pick_board, window, cx);
            return;
        }
        let Some(token) = self.cli_token() else { self.create_task.source_loading=false; return; };
        let expected=token.clone();
        let (send,receive)=std::sync::mpsc::channel();
        std::thread::spawn(move||{let result=ht_core::Http::new(token).and_then(|mut http|ht_core::task_create::duplicate_source(&mut http,task)).map_err(|error|error.to_string());let _=send.send(result);});
        cx.spawn_in(window,async move|view,cx|{
            loop{
                cx.background_executor().timer(Duration::from_millis(50)).await;
                match receive.try_recv(){
                    Ok(result)=>{let _=view.update_in(cx,|this,window,cx|{
                        if this.create_task.generation!=generation{return;}
                        if this.cli_token().as_deref()!=Some(expected.as_str()){this.create_task.source_loading=false;return;}
                        this.apply_duplicate_source(result,pick_board,window,cx);
                    });break;}
                    Err(std::sync::mpsc::TryRecvError::Empty)=>(),_=>break,
                }
            }
        }).detach();
    }
    fn apply_duplicate_source(&mut self,result:Result<Value,String>,pick_board:bool,window:&mut Window,cx:&mut Context<Self>){
        self.create_task.source_loading=false;
        let task=match result{Ok(task)=>task,Err(error)=>{eprintln!("Error fetching task: {error}");cx.notify();return;}};
        let Some(project)=task["projectId"].as_i64() else{return;};
        let Some(section)=task["sectionId"].as_i64() else{return;};
        self.invalidate_task_create(cx);
        self.create_task.project=project;self.create_task.section=section;
        self.create_task.fields=json!({"assignees":[],"priority":task["priority"],"estimate":task["estimate"],"tags":task["taskLabels"].as_array().into_iter().flatten().map(|row|row["label"].clone()).collect::<Vec<_>>()});
        self.create_task.edited.extend(["assignees","priority","estimate","tags"]);
        self.create_task.initial=self.create_task.fields.clone();
        self.create_task.title.update(cx,|input,cx|input.set_content(task["title"].as_str().unwrap_or("").into(),cx));
        self.create_task.description.update(cx,|input,cx|input.set_html(task["description_"]["content"].as_str().filter(|description|!description.is_empty()).or(task["description"].as_str()).unwrap_or("<p></p>"),cx));
        self.create_task.open=true;
        if !self.options.offline{self.load_create_board(cx);}
        if pick_board{self.create_picker(Field::Board,window,cx);}else{window.focus(&self.create_task.title.focus_handle(cx),cx);}
        cx.notify();
    }
    fn reset_create_destination(&mut self, cx: &mut Context<Self>) {
        let project = self.create_task.project;
        let columns = self.db.board(project).unwrap_or_default();
        self.create_task.section = columns.first().map_or(0, |column| column.id);
        let defaults = self.db.board_project(project).map(|project| ht_core::task_create::view_defaults(&project)).unwrap_or(json!({"assignees":[]}));
        if !self.create_task.initial.is_object() { self.create_task.initial = json!({}); }
        for key in ["assignees", "priority", "estimate", "tags"] {
            if !self.create_task.edited.contains(key) {
                if let Some(value) = defaults.get(key) { self.create_task.fields[key] = value.clone(); self.create_task.initial[key] = value.clone(); }
                else { self.create_task.fields.as_object_mut().unwrap().remove(key); self.create_task.initial.as_object_mut().unwrap().remove(key); }
            }
        }
        if project == self.options.board {
            if let Some(column) = columns.get(self.column) {
                self.create_task.section = column.id;
            }
        }
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
                            this.apply_create_board(result, cx);
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
    fn apply_create_board(&mut self, result: Result<(i64, Value), String>, cx: &mut Context<Self>) {
        let result = result.and_then(|(user, data)| {
            if data["project"]["id"] != self.create_task.project {
                return Err("Wrong board response".into());
            }
            self.db
                .apply_web_board(user, &data)
                .map_err(|error| error.to_string())?;
            Ok(data["project"].clone())
        });
        match result {
            Ok(project) => {
                let defaults = ht_core::task_create::view_defaults(&project);
                for key in ["assignees", "priority", "estimate", "tags"] {
                    if !self.create_task.edited.contains(key) {
                        if let Some(value) = defaults.get(key) {
                            self.create_task.fields[key] = value.clone();
                            self.create_task.initial[key] = value.clone();
                        } else {
                            self.create_task.fields.as_object_mut().unwrap().remove(key);
                            self.create_task.initial.as_object_mut().unwrap().remove(key);
                        }
                    }
                }
                self.create_task.loaded = Some(self.create_task.project);
                self.create_task.error = None;
                if self.create_task.section == 0 {
                    self.create_task.section = project["section"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .find(|column| column["deleted"] != true && column["visibility"] == true)
                        .and_then(|column| column["id"].as_i64())
                        .unwrap_or(0);
                }
            }
            Err(error) => {
                self.create_task.loaded = None;
                self.create_task.error = Some(error);
            }
        }
        cx.notify();
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
                    .kv_get(&format!(
                        "create-people:{}:{}",
                        self.db.current_user_id().unwrap_or(0),
                        self.create_task.project
                    ))
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
            Some(field @ (Field::DueDate | Field::StartDate)) => {
                let query = self.create_task.date.read(cx).content.trim();
                let mut choices = if query.is_empty() {
                    let key = if field == Field::DueDate { "dueDate" } else { "startDate" };
                    let mut choices = vec![];
                    if !self.create_task.fields[key].is_null() { choices.push(("Reset".into(), Value::Null)); }
                    if field == Field::DueDate {
                        if let Some(last) = self.db.kv_get(&format!("due-last:{}", self.db.current_user_id().unwrap_or(0))).ok().flatten().filter(|date| chrono::DateTime::parse_from_rfc3339(date).is_ok()) { choices.push(("last used".into(), json!(last))); }
                    }
                    choices.extend(super::task_fields::due_presets(chrono::Local::now()).into_iter().map(|(name, value)| (name, json!(value))));
                    choices
                } else {
                    ht_core::date_options::relative_options(query, chrono::Local::now(), field == Field::StartDate)
                        .map(|rows| rows.into_iter().map(|row| (row.display, json!(row.date))).collect())
                        .unwrap_or_else(|| super::task_fields::parse_due(query, chrono::Local::now().date_naive()).map(|value| vec![(query.into(), json!(value))]).unwrap_or_default())
                };
                if field == Field::DueDate { choices.push(("Pick a custom date".into(), json!({"custom":true}))); }
                choices
            }
            _ => vec![],
        }
    }
    fn choose_create_field(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some((_, value)) = self.create_choices(cx).get(index).cloned() else {
            return;
        };
        if value["custom"] == true {
            self.create_task.custom_date = true;
            self.create_task.selected_date = self.create_task.fields["dueDate"].as_str().and_then(|date| chrono::DateTime::parse_from_rfc3339(date).ok()).map(|date| date.with_timezone(&chrono::Local).date_naive()).unwrap_or_else(|| chrono::Local::now().date_naive());
            self.create_task.month = self.create_task.selected_date.with_day(1).unwrap();
            window.focus(&self.focus, cx);
            cx.notify();
            return;
        }
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
            Some(Field::DueDate) => {
                if let Some(date) = value.as_str() { let _ = self.db.kv_set(&format!("due-last:{}", self.db.current_user_id().unwrap_or(0)), date); }
                self.create_task.fields["dueDate"] = value;
            }
            Some(Field::StartDate) => self.create_task.fields["startDate"] = value,
            Some(Field::Column) => self.create_task.section = value.as_i64().unwrap(),
            Some(Field::Priority) => {
                self.create_task.edited.insert("priority");
                self.create_task.fields["priority"] = if value["priority_index"] == 0 {
                    Value::Null
                } else {
                    value
                }
            }
            Some(Field::Size) => {
                self.create_task.edited.insert("estimate");
                self.create_task.fields["estimate"] = if value["estimate_index"] == 0 {
                    Value::Null
                } else {
                    value
                }
            }
            Some(field @ (Field::Tags | Field::Assignees)) => {
                let key = if field == Field::Tags {
                    "tags"
                } else {
                    "assignees"
                };
                self.create_task.edited.insert(key);
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
        self.create_task.custom_date = false;
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
            let _ = self.db.kv_set(
                &format!(
                    "create-people:{}:{}",
                    self.db.current_user_id().unwrap_or(0),
                    self.create_task.project
                ),
                "null",
            );
            if let Some(token) = self.cli_token() {
                let project = self.create_task.project;
                let expected = token.clone();
                let (send, receive) = std::sync::mpsc::channel();
                std::thread::spawn(move || {
                    let result = ht_core::Http::new(token)
                        .and_then(|mut http| {
                            let mut data = ht_core::board_views::filter_people(&mut http, project)?;
                            data["boardAgents"] =
                                ht_core::board_views::filter_agents(&mut http, project)
                                    .unwrap_or_else(|error| {
                                        eprintln!("Failed to load board agents: {error}");
                                        json!([])
                                    });
                            Ok(data)
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
                                                &format!(
                                                    "create-people:{}:{project}",
                                                    this.db.current_user_id().unwrap_or(0)
                                                ),
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
    fn confirm_create_date(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(date) = self.create_task.selected_date.and_hms_opt(9, 0, 0).and_then(|date| chrono::Local.from_local_datetime(&date).earliest()) else { return; };
        let date = date.to_rfc3339();
        let _ = self.db.kv_set(&format!("due-last:{}", self.db.current_user_id().unwrap_or(0)), &date);
        self.create_task.fields["dueDate"] = json!(date);
        self.create_task.custom_date = false;
        self.create_task.picker = None;
        window.focus(&self.create_task.title.focus_handle(cx), cx);
        cx.notify();
    }
    fn create_date_calendar(&self, cx: &mut Context<Self>) -> AnyElement {
        let sunday = self.db.kv_get("calendarSettingsAtom").ok().flatten().and_then(|value| serde_json::from_str::<Value>(&value).ok()).is_none_or(|settings| settings["weekStartsOn"] != "monday");
        let mut months = div().flex().gap(px(16.));
        for index in 0..2 {
            let month = self.create_task.month.checked_add_months(chrono::Months::new(index)).unwrap_or(self.create_task.month);
            let first = month - chrono::Duration::days(if sunday { month.weekday().num_days_from_sunday() } else { month.weekday().num_days_from_monday() } as i64);
            let mut calendar = div().flex().flex_col()
                .child(div().flex().justify_between()
                    .child(div().id(("create-date-prev", index as usize)).cursor_pointer().child("‹").on_click(cx.listener(|this, _, _, cx| { this.create_task.month = this.create_task.month.checked_sub_months(chrono::Months::new(1)).unwrap_or(this.create_task.month); cx.notify(); })))
                    .child(month.format("%B %Y").to_string())
                    .child(div().id(("create-date-next", index as usize)).cursor_pointer().child("›").on_click(cx.listener(|this, _, _, cx| { this.create_task.month = this.create_task.month.checked_add_months(chrono::Months::new(1)).unwrap_or(this.create_task.month); cx.notify(); }))));
            calendar = calendar.child(div().flex().children((0..7).map(|day| div().w(px(28.)).child(if sunday { ["Su","Mo","Tu","We","Th","Fr","Sa"][day] } else { ["Mo","Tu","We","Th","Fr","Sa","Su"][day] }))));
            for week in 0..6 {
                let mut row = div().flex();
                for day in 0..7 {
                    let date = first + chrono::Duration::days(week * 7 + day);
                    row = row.child(div().id(("create-date-day", index as usize * 42 + (week * 7 + day) as usize)).w(px(28.)).h(px(28.)).cursor_pointer()
                        .when(date == self.create_task.selected_date, |d| d.bg(color(self.colors.active)))
                        .when(date.month() != month.month(), |d| d.text_color(color(self.colors.muted)))
                        .child(date.day().to_string()).on_click(cx.listener(move |this, _, _, cx| { this.create_task.selected_date = date; cx.notify(); })));
                }
                calendar = calendar.child(row);
            }
            months = months.child(calendar);
        }
        div().flex().flex_col().gap(px(8.)).child("SHIFT+ESC to go back").child(self.create_task.selected_date.format("%b %d, %Y").to_string()).child(months)
            .child(div().flex().justify_end().gap(px(12.))
                .child(div().id("create-date-back").cursor_pointer().child("Back").on_click(cx.listener(|this, _, window, cx| { this.create_task.custom_date = false; window.focus(&this.create_task.date.focus_handle(cx), cx); cx.notify(); })))
                .child(div().id("create-date-confirm").cursor_pointer().child("Confirm").on_click(cx.listener(|this, _, window, cx| this.confirm_create_date(window, cx)))))
            .into_any_element()
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
            self.create_task.edited.clear();
            self.create_task.fields = json!({});
            self.create_task.initial = Value::Null;
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
        if self.create_task.custom_date {
            if key.key == "escape" {
                self.create_task.custom_date = false;
                if !key.modifiers.shift { self.create_task.picker = None; }
                window.focus(&self.create_task.date.focus_handle(cx), cx);
            } else if key.key == "enter" && (key.modifiers.control || key.modifiers.platform) {
                self.confirm_create_date(window, cx);
            } else {
                let delta = match key.key.as_str() { "left" => -1, "right" => 1, "up" => -7, "down" => 7, _ => 0 };
                self.create_task.selected_date += chrono::Duration::days(delta);
                if self.create_task.selected_date < self.create_task.month || self.create_task.selected_date >= self.create_task.month.checked_add_months(chrono::Months::new(2)).unwrap_or(self.create_task.month) { self.create_task.month = self.create_task.selected_date.with_day(1).unwrap(); }
            }
            cx.stop_propagation();
            cx.notify();
            return true;
        }
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
        content = content.child(div().flex().flex_wrap().gap(px(8.)).children(
            self.create_task.fields["urlsToAdd"].as_array().into_iter().flatten().enumerate().map(|(index,file)| {
                let title = file["title"].as_str().unwrap_or("").to_owned();
                let url = file["urlString"].as_str().unwrap_or("").to_owned();
                div().id(("create-task-attachment",index)).p(px(8.)).bg(color(self.colors.container)).rounded(px(4.)).flex().gap(px(8.))
                    .child(div().cursor_pointer().child(title).on_click(move |_,_,cx| { if url.starts_with("https://") || url.starts_with("http://") { cx.open_url(&url); } })))
                    .child(div().id(("create-task-attachment-remove",index)).cursor_pointer().child("×").layout_key(&self.layout, &format!("create.attachment.remove.{index}"), None).on_click(cx.listener(move |this,_,_,cx| {
                        if this.create_task.pending.is_none() {
                            if let Some(files)=this.create_task.fields["urlsToAdd"].as_array_mut() { if index < files.len() { files.remove(index); } }
                            cx.notify();
                        }
                    })))
            })
        ));
        if matches!(
            self.create_task.picker,
            Some(Field::DueDate | Field::StartDate)
        ) {
            content = if self.create_task.custom_date { content.child(self.create_date_calendar(cx)) } else { content.child(self.create_task.date.clone()) };
        }
        if self.create_task.picker.is_some() && !self.create_task.custom_date {
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
                                this.create_task.edited.clear();
                                this.create_task.fields = json!({});
                                this.create_task.initial = Value::Null;
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
    fn comment_palette_create_quotes_the_source_keeps_files_and_preserves_refused_draft(cx: &mut gpui::TestAppContext) {
        use crate::ui::shell::tests::{draw, click};
        struct CommentCreate(u16);
        impl ht_core::Transport for CommentCreate {
            fn get(&mut self, path:&str)->Result<ht_core::http::Response,ht_core::http::TransportError> {
                assert_eq!(path,"/user/context");
                Ok(ht_core::http::Response {status:200,body:json!({"success":true,"user":{"id":42}})})
            }
            fn post(&mut self,path:&str,body:&Value,_:&str)->Result<ht_core::http::Response,ht_core::http::TransportError> {
                assert_eq!(path,"/api/tasks/createGlobally");
                assert_eq!(body["title"],"Follow up");
                assert_eq!(body["createTaskFromComment"]["task"]["id"],1);
                assert_eq!(body["createTaskFromComment"]["task"]["uniqueIndex"],7);
                assert_eq!(body["createTaskFromComment"]["commentIndex"],0);
                assert_eq!(body["urlsToAdd"],json!([{"urlString":"https://files.example/source.pdf","Attachment":true,"fileSize":1024,"title":"source.pdf","attachmentType":"application/pdf"}]));
                assert!(body["description"].as_str().unwrap().contains("<blockquote><p>Keep <strong>this</strong></p></blockquote>"));
                assert!(body["description"].as_str().unwrap().contains("/detail/project-15/7#comment-0"));
                assert!(body.get("parentTask").is_none());
                let mut task=body.clone();task["id"]=json!(500);task["status"]=json!("Normal");
                Ok(ht_core::http::Response {status:self.0,body:if self.0==200 {json!({"newTask":task})} else {json!({"message":"Forbidden"})}})
            }
        }
        cx.update(crate::input::init);
        let view=cx.open_window(gpui::size(px(1440.),px(900.)),desktop);
        let mut cx=gpui::VisualTestContext::from_window(view.into(),cx);
        for status in [401,403,200] {
            view.update(&mut cx,|this,window,cx| {
                this.invalidate_task_create(cx);
                this.db.apply_task(&json!({"id":1,"projectId":15,"sectionId":10,"section":"Todo","title":"Source","uniqueIndex":7,"ticketNumber":"HT-7","status":"Normal"})).unwrap();
                this.db.apply_comments(1,&json!({"success":true,"comments":[{"id":71,"taskId":1,"text":"<p>Keep <strong>this</strong></p>","createdAt":"2026-10-01T00:00:00Z","creatorId":7,"creator":{"id":7,"displayName":"Alex & Bea"},"attachments":[{"id":"file-1","fileName":"source.pdf","fileSize":"1024","fileSource":"https://files.example/source.pdf","fileType":"application/pdf"},{"id":"file-2","fileName":"remove.txt","fileSize":null,"fileSource":"https://files.example/remove.txt","fileType":"text/plain"}]}]})).unwrap();
                this.reload();this.interaction.task_part=1;window.focus(&this.focus,cx);
            }).unwrap();
            draw(view,&mut cx);
            cx.simulate_keystrokes("ctrl-k");cx.simulate_input("Create task from comment");cx.simulate_keystrokes("enter");
            draw(view,&mut cx);
            view.read_with(&cx,|this,cx| {
                assert!(this.create_task.open);
                assert_eq!(this.create_task.section,10);
                assert!(this.create_task.title.read(cx).content.is_empty());
                assert!(this.create_task.description.read(cx).html().contains("Alex &amp; Bea"));
                assert_eq!(this.create_task.fields["urlsToAdd"].as_array().unwrap().len(),2);
                assert!(this.create_task.fields["urlsToAdd"][1].get("fileSize").is_none());
                assert!(this.db.mutations().unwrap().iter().all(|row|row.status=="failed"));
            }).unwrap();
            click(view,&mut cx,"create.attachment.remove.1");
            cx.simulate_input("Follow up");cx.simulate_keystrokes("ctrl-enter");
            view.update(&mut cx,|this,window,cx| {
                let id=this.db.mutations().unwrap().into_iter().find(|row|row.status!="failed").unwrap().id;
                assert!(this.db.task(-id).unwrap().is_some());
                this.interaction.task_part=0;
                this.db.drain_once(&mut CommentCreate(status),chrono::Utc::now().timestamp()+1).unwrap();
                assert!(!this.poll_create_task(window,cx));
                assert!(this.db.task(-id).unwrap().is_none());
                if status!=200 {
                    assert!(this.create_task.open);
                    assert_eq!(this.create_task.title.read(cx).content.as_ref(),"Follow up");
                    assert_eq!(this.create_task.fields["createTaskFromComment"]["commentIndex"],0);
                    assert_eq!(this.create_task.fields["urlsToAdd"][0]["title"],"source.pdf");
                    assert_eq!(this.create_task.error.as_deref(),Some("Task could not be created"));
                } else {assert!(!this.create_task.open);assert!(this.db.task(500).unwrap().is_some());}
            }).unwrap();
        }
    }

    #[gpui::test]
    fn delayed_board_defaults_merge_only_untouched_fields_and_refusal_keeps_the_form(
        cx: &mut gpui::TestAppContext,
    ) {
        struct Metadata(u16);
        impl ht_core::Transport for Metadata {
            fn get(
                &mut self,
                _: &str,
            ) -> Result<ht_core::http::Response, ht_core::http::TransportError> {
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
                assert_eq!(path, "/api/projects/boardTasks");
                assert_eq!(body, &json!({"projectId":15,"userId":42}));
                Ok(ht_core::http::Response {
                    status: self.0,
                    body: if self.0 == 200 {
                        json!({"project":{"id":15,"title":"Board","section":[{"id":10,"projectId":15,"section_title":"Todo","ranking":"a","visibility":true,"deleted":false}],"project_view":{"user_project_views":[{"unsavedView":{"board_filters":{"matchFilters":"ALL","addedFilters":[{"type":"Priority","searchPayload":[{"priority_index":4}]},{"type":"Size","searchPayload":[{"estimate_index":5}]},{"type":"Assignees","searchPayload":[{"id":42,"uid":"owner"}]}]}}}]}},"allViews":[],"tasks":[]})
                    } else {
                        json!({"message":"Board access denied"})
                    },
                })
            }
        }
        cx.update(crate::input::init);
        let (view, cx) = cx.add_window_view(|window, cx| desktop(window, cx));
        for status in [401, 403, 200] {
            view.update_in(cx, |this, window, cx| {
                this.invalidate_task_create(cx);
                this.open_create_task(window, cx);
                this.create_task
                    .title
                    .update(cx, |input, cx| input.set_content("Keep note".into(), cx));
                this.create_picker(Field::Priority, window, cx);
                this.choose_create_field(2, window, cx);
                let result = ht_core::board_views::load(&mut Metadata(status), 15)
                    .map_err(|error| error.to_string());
                this.apply_create_board(result, cx);
                assert_eq!(
                    this.create_task.title.read(cx).content.as_ref(),
                    "Keep note"
                );
                assert_eq!(this.create_task.fields["priority"]["priority_index"], 2);
                if status == 200 {
                    assert_eq!(this.create_task.fields["estimate"]["estimate_index"], 5);
                    assert_eq!(this.create_task.fields["assignees"][0]["id"], 42);
                    assert_eq!(this.create_task.loaded, Some(15));
                } else {
                    assert!(this.create_task.loaded.is_none());
                    assert_eq!(
                        this.create_task.error.as_deref(),
                        Some("Board access denied")
                    );
                }
                assert!(this.db.mutations().unwrap().is_empty());
            });
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
    #[gpui::test]
    fn create_custom_due_calendar_confirms_nine_am_and_mocked_refusals_preserve_date_and_draft(cx: &mut gpui::TestAppContext) {
        struct CalendarServer { status: u16, date: Value }
        impl ht_core::Transport for CalendarServer {
            fn get(&mut self, path: &str) -> Result<ht_core::http::Response, ht_core::http::TransportError> { ht_core::Transport::get(&mut Server(self.status), path) }
            fn post(&mut self, path: &str, body: &Value, key: &str) -> Result<ht_core::http::Response, ht_core::http::TransportError> {
                assert_eq!(body["dueDate"], self.date);
                ht_core::Transport::post(&mut Server(self.status), path, body, key)
            }
        }
        cx.update(crate::input::init);
        let (view, cx) = cx.add_window_view(|window, cx| desktop(window, cx));
        for status in [401, 403, 200] {
            view.update_in(cx, |this, window, cx| {
                this.invalidate_task_create(cx);
                this.open_create_task(window, cx);
                this.create_task.title.update(cx, |input, cx| input.set_content("New form task".into(), cx));
                this.create_task.description.update(cx, |input, cx| input.set_html("<p>Draft description</p>", cx));
                this.create_picker(Field::Priority, window, cx); this.choose_create_field(2, window, cx);
                this.create_picker(Field::DueDate, window, cx);
                assert!(!this.create_choices(cx).iter().any(|(name,_)|name=="Reset"));
                let custom = this.create_choices(cx).iter().position(|(name,_)|name=="Pick a custom date").unwrap();
                this.choose_create_field(custom, window, cx);
                assert!(this.create_task.custom_date);
                this.create_task.selected_date = NaiveDate::from_ymd_opt(2026, 12, 15).unwrap();
                this.confirm_create_date(window, cx);
                let date = this.create_task.fields["dueDate"].clone();
                let local = chrono::DateTime::parse_from_rfc3339(date.as_str().unwrap()).unwrap().with_timezone(&chrono::Local);
                assert_eq!(local.date_naive(), NaiveDate::from_ymd_opt(2026,12,15).unwrap());
                assert_eq!(local.time(), chrono::NaiveTime::from_hms_opt(9,0,0).unwrap());
                assert!(!this.create_task.custom_date); assert!(this.create_task.picker.is_none());
                this.create_picker(Field::DueDate, window, cx);
                assert!(this.create_choices(cx).iter().any(|(name,_)|name=="Reset"));
                assert!(this.create_choices(cx).iter().any(|(name,value)|name=="last used" && *value==date));
                this.create_task.picker = None;
                this.submit_create_task(window, cx);
                let id = this.db.mutations().unwrap().into_iter().find(|row|row.status!="failed").unwrap().id;
                assert_eq!(this.db.task(-id).unwrap().unwrap().due_date.as_deref(), date.as_str());
                this.db.drain_once(&mut CalendarServer { status, date:date.clone() }, chrono::Utc::now().timestamp()+1).unwrap();
                assert!(!this.poll_create_task(window, cx));
                assert!(this.db.task(-id).unwrap().is_none());
                if status != 200 {
                    assert!(this.create_task.open);
                    assert_eq!(this.create_task.fields["dueDate"], date);
                    assert_eq!(this.create_task.title.read(cx).content.as_ref(), "New form task");
                    assert_eq!(this.create_task.error.as_deref(), Some("Task could not be created"));
                }
            });
        }
    }

    #[gpui::test]
    fn duplicate_commands_read_real_source_copy_web_fields_and_queue_global_create_with_refusal_rollback(cx:&mut gpui::TestAppContext){
        struct Duplicate {read:u16,write:u16}
        impl ht_core::Transport for Duplicate{
            fn get(&mut self,path:&str)->Result<ht_core::http::Response,ht_core::http::TransportError>{
                let body=if path=="/user/context"{return Ok(ht_core::http::Response{status:200,body:json!({"success":true,"user":{"id":42}})});}else{assert_eq!(path,"/api/tasks/single?id=1");if self.read==200{json!({"id":1,"projectId":15,"sectionId":10,"section":"Todo","title":"Original","status":"Normal","description":"<p>Old</p>","description_":{"content":"<p>Rich <strong>source</strong></p>"},"priority":ht_core::task_create::priority(4).unwrap(),"estimate":ht_core::task_create::estimate(3).unwrap(),"dueDate":"2026-12-01T09:00:00Z","startDate":"2026-11-01T09:00:00Z","parentTask":{"id":7},"assignees":[{"id":42,"uid":"owner"}],"taskLabels":[{"label":{"id":"11111111-1111-1111-1111-111111111111","value":"Copied"}}]})}else{json!({"message":"Task access denied"})}};
                Ok(ht_core::http::Response{status:self.read,body})
            }
            fn post(&mut self,path:&str,body:&Value,_:&str)->Result<ht_core::http::Response,ht_core::http::TransportError>{
                assert_eq!(path,"/api/tasks/createGlobally");assert_eq!(body["title"],"Original");assert_eq!(body["description"],"<p>Rich <strong>source</strong></p>");assert_eq!(body["assignees"],json!([]));assert_eq!(body["priority"]["priority_index"],4);assert_eq!(body["estimate"]["estimate_index"],3);assert_eq!(body["tags"][0]["value"],"Copied");assert!(body.get("parentTask").is_none());assert!(body.get("dueDate").is_none());assert!(body.get("startDate").is_none());
                let mut task=body.clone();task["id"]=json!(501);task["status"]=json!("Normal");Ok(ht_core::http::Response{status:self.write,body:if self.write==200{json!({"newTask":task})}else{json!({"message":"Create access denied"})}})
            }
        }
        cx.update(crate::input::init);let(view,cx)=cx.add_window_view(|window,cx|desktop(window,cx));
        for pick_board in [false,true]{for read in [401,403,200]{for write in [401,403,200]{
            view.update_in(cx,|this,window,cx|{
                this.invalidate_task_create(cx);let mut server=Duplicate{read,write};let result=ht_core::task_create::duplicate_source(&mut server,1).map_err(|error|error.to_string());this.apply_duplicate_source(result,pick_board,window,cx);
                if read!=200{assert!(!this.create_task.open);assert!(!this.db.mutations().unwrap().iter().any(|row|row.status!="failed"));return;}
                assert_eq!(this.create_task.picker,if pick_board{Some(Field::Board)}else{None});
                let before=this.create_task.fields.clone();
                this.apply_create_board(Ok((42,json!({"project":{"id":15,"title":"Board","section":[{"id":10,"projectId":15,"section_title":"Todo","ranking":"a","visibility":true,"deleted":false}],"project_view":{"user_project_views":[{"unsavedView":{"board_filters":{"addedFilters":[{"type":"Priority","searchPayload":[{"priority_index":1}]},{"type":"Assignees","searchPayload":[{"id":42,"uid":"owner"}]}]}}}]}},"allViews":[],"tasks":[]}))),cx);
                assert_eq!(this.create_task.fields,before);
                this.create_task.picker=None;this.submit_create_task(window,cx);let id=this.db.mutations().unwrap().into_iter().find(|row|row.status!="failed").unwrap().id;assert!(this.db.task(-id).unwrap().is_some());this.db.drain_once(&mut server,chrono::Utc::now().timestamp()+1).unwrap();assert!(!this.poll_create_task(window,cx));assert!(this.db.task(-id).unwrap().is_none());
                if write!=200{assert!(this.create_task.open);assert_eq!(this.create_task.fields,before);assert_eq!(this.create_task.title.read(cx).content.as_ref(),"Original");}
            });
        }}}
    }
    #[gpui::test]
    fn explicit_create_fields_and_dates_survive_board_switch_and_late_defaults_even_when_equal_to_opening_defaults(cx:&mut gpui::TestAppContext){
        cx.update(crate::input::init);let(view,cx)=cx.add_window_view(|window,cx|desktop(window,cx));
        view.update_in(cx,|this,window,cx|{
            this.db.apply_response(&json!({"success":true,"projects":[{"id":16,"title":"Other","project_view":{"user_project_views":[{"unsavedView":{"board_filters":{"addedFilters":[{"type":"Priority","searchPayload":[{"priority_index":1}]},{"type":"Size","searchPayload":[{"estimate_index":6}]}]}}}]}}],"sections":[{"id":20,"projectId":16,"section_title":"Todo","ranking":"a","visibility":true,"deleted":false}]})).unwrap();
            this.open_create_task(window,cx);this.create_picker(Field::Priority,window,cx);this.choose_create_field(0,window,cx);this.create_task.fields["dueDate"]=json!("2026-12-01T09:00:00Z");
            this.create_picker(Field::Board,window,cx);let index=this.create_choices(cx).iter().position(|(_,id)|*id==16).unwrap();this.choose_create_field(index,window,cx);
            assert!(this.create_task.fields["priority"].is_null());assert_eq!(this.create_task.fields["estimate"]["estimate_index"],6);assert_eq!(this.create_task.fields["dueDate"],"2026-12-01T09:00:00Z");
            assert_eq!(this.create_task.section,20);assert!(this.db.mutations().unwrap().is_empty());
        });
    }

    #[gpui::test]
    fn template_palette_prefills_live_properties_and_preserves_refused_create_form(cx:&mut gpui::TestAppContext) {
        struct Templates(u16);
        impl ht_core::Transport for Templates {
            fn get(&mut self,path:&str)->Result<ht_core::http::Response,ht_core::http::TransportError>{
                let body=if path=="/user/context"{json!({"success":true,"user":{"id":42}})}else{assert_eq!(path,"/api/task-templates?projectId=15");json!({"success":true,"templates":[{"id":81,"name":"Daily template","title":"Check everything","descriptionHtml":"<p>Checklist</p>","priorityIndex":2,"estimateIndex":4,"labelIds":["live","gone"]}],"labels":[{"id":"live","value":"Ready","projectId":15}],"targetSection":{"id":10,"section_title":"Todo"}})};
                Ok(ht_core::http::Response{status:if path=="/user/context"{200}else{self.0},body})
            }
            fn post(&mut self,path:&str,body:&Value,_:&str)->Result<ht_core::http::Response,ht_core::http::TransportError>{assert_eq!(path,"/api/tasks/createGlobally");assert_eq!(body["title"],"Check everything");assert_eq!(body["description"],"<p>Checklist</p>");assert_eq!(body["priority"]["priority_index"],2);assert_eq!(body["estimate"]["estimate_index"],4);assert_eq!(body["tags"][0]["id"],"live");assert_eq!(body["tags"].as_array().unwrap().len(),1);let mut task=body.clone();task["id"]=json!(500);task["status"]=json!("Normal");Ok(ht_core::http::Response{status:self.0,body:if self.0==200{json!({"newTask":task})}else{json!({"error":"Unauthorized"})}})}
        }
        cx.update(crate::input::init);let view=cx.open_window(gpui::size(px(1440.),px(900.)),desktop);let mut cx=gpui::VisualTestContext::from_window(view.into(),cx);
        for read in [401,403,200]{for write in [401,403,200]{
            view.update(&mut cx,|this,window,cx|{this.invalidate_task_create(cx);window.focus(&this.focus,cx);}).unwrap();
            crate::ui::shell::tests::draw(view,&mut cx);
            cx.simulate_keystrokes("ctrl-k");cx.simulate_input("New task from template");cx.simulate_keystrokes("enter");
            view.update(&mut cx,|this,_,cx|{let result=ht_core::task_create::templates(&mut Templates(read),15).map_err(|e|e.to_string());this.apply_task_templates(result,cx);}).unwrap();
            if read!=200{view.update(&mut cx,|this,_,cx|{assert!(this.choices(cx).is_empty());assert!(this.db.mutations().unwrap().iter().all(|row|row.status=="failed"));}).unwrap();cx.simulate_keystrokes("escape");continue;}
            crate::ui::shell::tests::draw(view,&mut cx);
            view.update(&mut cx,|this,_,cx|{assert_eq!(this.interaction.modal,Some(interactions::Modal::Templates));assert_eq!(this.choices(cx).len(),1);}).unwrap();
            cx.simulate_input("Daily");cx.simulate_keystrokes("enter ctrl-enter");
            view.update(&mut cx,|this,window,cx|{
                assert!(this.create_task.open);assert!(this.create_task.pending.is_some());assert_eq!(this.create_task.title.read(cx).content,"Check everything");
                this.db.drain_once(&mut Templates(write),chrono::Utc::now().timestamp()+1).unwrap();this.poll_create_task(window,cx);
                if write!=200{assert!(this.create_task.open);assert!(this.create_task.error.is_some());assert_eq!(this.create_task.description.read(cx).html(),"<p>Checklist</p>");assert_eq!(this.create_task.fields["tags"][0]["id"],"live");}else{assert!(!this.create_task.open);assert!(this.db.task(500).unwrap().is_some());}
                this.invalidate_task_create(cx);
            }).unwrap();
        }}
    }

}
