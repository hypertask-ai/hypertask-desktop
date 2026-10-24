use super::*;
use chrono::{Datelike, NaiveDate, TimeZone};
use serde_json::{Value, json};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Panel {
    Sort,
    Filters,
    Priority,
    Size,
    Labels,
    Assignees,
    CreatedBy,
    UpdatedBy,
    BlockedByPerson,
    Date(&'static str),
    DateRange(&'static str),
    ValueMatch,
    Columns,
    Subtasks,
    Save,
    SavePrivate,
    SaveTeam,
    Manage,
    RenameView,
    DeleteView,
}
pub(super) struct Views {
    pub loaded: i64,
    loading: bool,
    ready: bool,
    generation: u64,
    pub panel: Option<Panel>,
    input: Entity<TextInput>,
    pending: Option<String>,
    choice: usize,
    editing: Option<String>,
    pending_route: String,
    people: Option<Value>,
    pub(super) builtin: std::collections::BTreeMap<i64, String>,
    decisions: bool,
    previous_builtin: Option<(i64, String)>,
    support_pending: bool,
    query: Entity<TextInput>,
    query_text: String,
    month: NaiveDate,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
}
impl Views {
    pub fn new(cx: &mut Context<Desktop>) -> Self {
        let input = cx.new(|cx| TextInput::new(String::new(), "View name", color(0x8e9093), cx));
        let query = cx.new(|cx| TextInput::new(String::new(), "Search…", color(0x8e9093), cx));
        cx.observe(&query, |this, query, cx| {
            this.views.query_text = query.read(cx).content.to_string();
            this.views.choice = 0;
            cx.notify();
        })
        .detach();
        Self {
            loaded: 0,
            loading: false,
            ready: false,
            generation: 0,
            panel: None,
            input,
            pending: None,
            choice: 0,
            editing: None,
            pending_route: String::new(),
            people: None,
            builtin: Default::default(),
            decisions: false,
            previous_builtin: None,
            support_pending: false,
            query,
            query_text: String::new(),
            month: chrono::Local::now().date_naive().with_day(1).unwrap(),
            from: None,
            to: None,
        }
    }
}
impl Desktop {
    pub(super) fn invalidate_board_views(&mut self, cx: &mut Context<Self>) {
        self.views.builtin.clear();
        self.views.decisions = false;
        self.views.previous_builtin = None;
        self.views.people = None;
        self.views.support_pending = false;
        self.views.loaded = 0;
        self.views.loading = false;
        self.views.ready = false;
        self.views.generation += 1;
        self.views.panel = None;
        self.views.pending = None;
        self.views
            .input
            .update(cx, |input, cx| input.set_content(String::new(), cx));
    }
    pub(super) fn load_board_views(&mut self, cx: &mut Context<Self>) {
        let board = self.options.board;
        if self.options.offline || self.views.loaded == board {
            return;
        }
        let Some(token) = self.cli_token() else {
            return;
        };
        self.views.loaded = board;
        self.views.loading = true;
        self.views.ready = false;
        self.views.generation += 1;
        let generation = self.views.generation;
        let expected_token = token.clone();
        let (send, receive) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let result = ht_core::Http::new(token).and_then(|mut http| {
                let (user, mut body) = ht_core::board_views::load(&mut http, board)?;
                body["desktop_decisions"] = json!(
                    ht_core::Transport::get(&mut http, "/api/flags")
                        .ok()
                        .is_some_and(|response| response.status == 200
                            && response.body["flags"]["htpr-7092-decisions"] == true)
                );
                Ok((user, body))
            });
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
                            if this.views.generation != generation {
                                return;
                            }
                            this.views.loading = false;
                            if this.options.board != board
                                || this.cli_token().as_deref() != Some(expected_token.as_str())
                            {
                                return;
                            }
                            if let Ok((user, body)) = result {
                                let _ = this.db.kv_set("current-user-id", &user.to_string());
                                if this.db.apply_web_board(user, &body).is_ok() {
                                    this.views.decisions = body["desktop_decisions"] == true;
                                    this.views.ready = true;
                                    this.reload();
                                }
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
    pub(super) fn board_metadata_ready(&mut self, cx: &mut Context<Self>) -> bool {
        if self.options.offline {
            return true;
        }
        if self.views.ready && self.views.loaded == self.options.board {
            return true;
        }
        if !self.views.loading {
            self.views.loaded = 0;
            self.load_board_views(cx);
        }
        false
    }
    pub(super) fn view_settings(&self) -> Value {
        self.db
            .board_project(self.options.board)
            .map(|p| ht_core::board_views::settings(&p))
            .unwrap_or(json!({}))
    }
    pub(super) fn open_view_panel(
        &mut self,
        panel: Panel,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.views.panel = Some(panel);
        if matches!(
            panel,
            Panel::CreatedBy | Panel::UpdatedBy | Panel::BlockedByPerson | Panel::Assignees
        ) && !self.options.offline
        {
            self.load_view_support(false, window, cx);
        }
        self.views.choice = 0;
        self.views
            .query
            .update(cx, |input, cx| input.set_content(String::new(), cx));
        if let Panel::DateRange(kind) = panel {
            let settings = self.view_settings();
            let payload = settings["board_filters"]["addedFilters"]
                .as_array()
                .into_iter()
                .flatten()
                .find(|filter| filter["type"] == kind)
                .map(|filter| filter["searchPayload"][0].clone())
                .unwrap_or(Value::Null);
            let parse = |value: &Value| {
                value
                    .as_str()
                    .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
                    .map(|date| date.with_timezone(&chrono::Local).date_naive())
            };
            let today = chrono::Local::now().date_naive();
            let mut from = today;
            let mut days = 0;
            while days < 3 {
                from -= chrono::Duration::days(1);
                if from.weekday().number_from_monday() <= 5 {
                    days += 1;
                }
            }
            self.views.from = Some(parse(&payload["fromDate"]).unwrap_or(from));
            self.views.to = Some(parse(&payload["toDate"]).unwrap_or(today));
            self.views.month = self.views.from.unwrap().with_day(1).unwrap();
        }
        if matches!(
            panel,
            Panel::SavePrivate | Panel::SaveTeam | Panel::RenameView
        ) {
            self.views
                .input
                .update(cx, |input, cx| input.set_content(String::new(), cx));
            window.focus(&self.views.input.focus_handle(cx), cx);
        } else if Self::view_searchable(panel) {
            window.focus(&self.views.query.focus_handle(cx), cx);
        } else {
            window.focus(&self.focus, cx);
        }
        cx.notify();
    }
    fn view_searchable(panel: Panel) -> bool {
        matches!(
            panel,
            Panel::Sort
                | Panel::Filters
                | Panel::Priority
                | Panel::Size
                | Panel::Labels
                | Panel::Assignees
                | Panel::CreatedBy
                | Panel::UpdatedBy
                | Panel::BlockedByPerson
                | Panel::Date(_)
        )
    }
    fn queue_view(
        &mut self,
        route: &str,
        mut body: Value,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.views.pending.is_some() || !self.board_metadata_ready(cx) {
            return;
        }
        body["projectId"] = json!(self.options.board);
        if route == "unsaved-view" {
            let project = self
                .db
                .board_project(self.options.board)
                .unwrap_or(Value::Null);
            body["baseViewId"] =
                project["project_view"]["user_project_views"][0]["appliedView"]["id"].clone();
        }
        match self.db.enqueue(
            Write::View {
                change: ht_core::ViewWrite {
                    project: self.options.board,
                    route: route.into(),
                    body,
                },
            },
            chrono::Utc::now().timestamp(),
        ) {
            Ok(id) => {
                self.views.pending_route = route.into();
                self.views.pending = self
                    .db
                    .mutations()
                    .ok()
                    .and_then(|rows| rows.into_iter().find(|r| r.id == id))
                    .map(|r| r.client_id);
                self.reload();
                self.request_sync();
                if !self.options.offline {
                    cx.spawn_in(window, async move |view, cx| {
                        loop {
                            cx.background_executor()
                                .timer(Duration::from_millis(100))
                                .await;
                            if !view
                                .update_in(cx, |this, window, cx| this.poll_view(window, cx))
                                .unwrap_or(false)
                            {
                                break;
                            }
                        }
                    })
                    .detach();
                }
            }
            Err(error) => self.error = Some(error.to_string()),
        }
        cx.notify();
    }
    pub(super) fn poll_view(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some(key) = self
            .views
            .pending
            .as_ref()
            .map(|key| format!("view-result:{key}"))
        else {
            return false;
        };
        let Some(result) = self
            .db
            .kv_get(&key)
            .ok()
            .flatten()
            .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        else {
            return true;
        };
        self.views.pending = None;
        self.reload();
        if result["error"].is_null() {
            self.views.previous_builtin = None;
            if !matches!(
                self.views.panel,
                Some(
                    Panel::Sort
                        | Panel::Priority
                        | Panel::Size
                        | Panel::Labels
                        | Panel::Assignees
                        | Panel::CreatedBy
                        | Panel::UpdatedBy
                        | Panel::BlockedByPerson
                        | Panel::ValueMatch
                        | Panel::Columns
                )
            ) {
                self.views.panel = None;
                window.focus(&self.focus, cx);
            }
        } else {
            if self.views.pending_route == "switch-view" {
                if let Some((board, previous)) = self.views.previous_builtin.take() {
                    self.views.builtin.entry(board).or_insert(previous);
                    self.reload();
                }
            }
            self.error = Some(match self.views.pending_route.as_str() {
                "rename-view" => "Error renaming view".into(),
                "delete-view" => "Error deleting view".into(),
                "update-order" => "Unable to save view order".into(),
                "reset-order" => "Unable to reset view order".into(),
                "set-default-order" => "Unable to set the default view order".into(),
                _ => result["error"]
                    .as_str()
                    .unwrap_or("Could not save the view")
                    .into(),
            });
        }
        cx.notify();
        false
    }
    fn view_choices(&self) -> Vec<(String, Value)> {
        let choices: Vec<(String, Value)> = match self.views.panel {
            Some(Panel::Save) => [
                "Save as current view",
                "Set as default board for everyone",
                "Save as additional view for team",
                "Save as view for me only",
                "Reset View",
            ]
            .into_iter()
            .enumerate()
            .map(|(i, name)| (name.into(), json!(i)))
            .collect(),
            Some(Panel::SavePrivate | Panel::SaveTeam) => vec![("Save view".into(), Value::Null)],
            Some(Panel::RenameView) => vec![
                ("Save".into(), json!("rename")),
                ("DELETE VIEW".into(), json!("delete")),
            ],
            Some(Panel::DeleteView) => vec![
                ("Delete view".into(), json!("confirm")),
                ("Cancel".into(), json!("cancel")),
            ],
            Some(Panel::Manage) => {
                let project = self
                    .db
                    .board_project(self.options.board)
                    .unwrap_or(Value::Null);
                let mut choices = vec![
                    (
                        "Sort alphabetically".into(),
                        json!({"action":"alphabetical"}),
                    ),
                    ("Reset to default order".into(), json!({"action":"reset"})),
                ];
                if self.can_set_view_default(&project) {
                    choices.push((
                        "Set as default order for everyone".into(),
                        json!({"action":"default"}),
                    ));
                }
                choices.extend(
                    ht_core::board_views::ordered_views(&project)
                        .into_iter()
                        .map(|view| {
                            (
                                view["title"].as_str().unwrap_or("Untitled view").into(),
                                view,
                            )
                        }),
                );
                choices
            }
            Some(Panel::Subtasks) => [
                ("Hide on board, show on task page", "None"),
                (
                    "Show as card property and flattened list combined",
                    "Flattened_Card",
                ),
                ("Show as flattened list", "Flattened"),
                ("Show only parents", "Parent"),
                ("Show as card property", "Card"),
            ]
            .into_iter()
            .map(|(name, value)| (name.into(), json!(value)))
            .collect(),
            Some(Panel::Columns) => self
                .db
                .board_project(self.options.board)
                .ok()
                .into_iter()
                .flat_map(|project| project["section"].as_array().cloned().unwrap_or_default())
                .filter(|section| section["deleted"] != true)
                .map(|section| {
                    (
                        section["section_title"].as_str().unwrap_or("").into(),
                        section,
                    )
                })
                .collect(),
            Some(Panel::Sort) => ht_core::board_views::SORT_MODES
                .iter()
                .filter(|(_, mode)| {
                    *mode == "Manual"
                        || (self.sort_levels().len() < 3
                            && !self
                                .sort_levels()
                                .iter()
                                .any(|level| level["mode"] == *mode))
                })
                .map(|(name, value)| ((*name).into(), json!(value)))
                .collect(),
            Some(Panel::Filters) => [
                ("Clear all filters", "Clear"),
                ("Toggle Match Criteria", "Match"),
                ("Updated by", "UpdatedBy"),
                ("Created by", "CreatedBy"),
                ("Has unread notifications", "Unread"),
                ("Tasks assigned to me", "AssignedToMe"),
                ("Due date", "DueDate"),
                ("Priority", "Priority"),
                ("Assignees", "Assignees"),
                ("Blocked by person", "BlockedByPerson"),
                ("Tags", "Labels"),
                ("Task Size", "Size"),
                ("Date updated at", "UpdatedRange"),
                ("Date created at", "CreatedAt"),
                ("Present in inbox", "Inbox"),
                ("Stale without comment", "NoRecentComment"),
                ("Stale in column", "StuckInColumn"),
                ("Has a running timer", "RunningTimer"),
                ("Stale on board", "StaleOnBoard"),
                ("Not stale", "NotStale"),
            ]
            .into_iter()
            .filter(|(_, kind)| {
                !matches!(*kind, "Clear" | "Match")
                    || self.view_settings()["board_filters"]["addedFilters"]
                        .as_array()
                        .is_some_and(|filters| !filters.is_empty())
            })
            .map(|(name, kind)| (name.into(), json!(kind)))
            .collect(),
            Some(Panel::ValueMatch) => [("Tags", "Labels"), ("Assignees", "Assignees")]
                .into_iter()
                .filter(|(_, kind)| {
                    self.view_settings()["board_filters"]["addedFilters"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .any(|filter| {
                            filter["type"] == *kind
                                && filter["searchPayload"]
                                    .as_array()
                                    .is_some_and(|values| !values.is_empty())
                        })
                })
                .map(|(name, kind)| (name.into(), json!(kind)))
                .collect(),
            Some(Panel::DateRange(_)) => vec![
                ("Back".into(), json!("BACK")),
                ("Confirm".into(), json!("CONFIRM")),
            ],
            Some(Panel::Date(kind)) => [
                ("Reset", "RESET"),
                ("Any", "ANY"),
                ("Today", "TODAY"),
                ("Yesterday", "YESTERDAY"),
                ("Last 7 days", "LAST_7_DAYS"),
                ("Last 30 days", "LAST_30_DAYS"),
                ("This week", "THIS_WEEK"),
                ("This month", "THIS_MONTH"),
                ("Pick a custom date range", "CUSTOM"),
            ]
            .into_iter()
            .filter(|(_, value)| {
                (*value != "ANY" || kind == "DueDate")
                    && (*value != "RESET"
                        || self.view_settings()["board_filters"]["addedFilters"]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .any(|filter| filter["type"] == kind))
            })
            .map(|(name, value)| (name.into(), json!(value)))
            .collect(),
            Some(Panel::Priority) => (0..5)
                .filter_map(ht_core::task_create::priority)
                .map(|v| (v["Priority_Value"].as_str().unwrap().into(), v))
                .collect(),
            Some(Panel::Size) => [0, 2, 3, 4, 5, 6]
                .into_iter()
                .filter_map(ht_core::task_create::estimate)
                .map(|v| (v["estimate_value"].as_str().unwrap().into(), v))
                .collect(),
            Some(Panel::Labels) => self
                .db
                .project_labels(self.options.board)
                .unwrap_or_default()
                .into_iter()
                .map(|v| {
                    (
                        v["name"]
                            .as_str()
                            .or(v["value"].as_str())
                            .unwrap_or("")
                            .into(),
                        v,
                    )
                })
                .collect(),
            Some(
                panel @ (Panel::Assignees
                | Panel::CreatedBy
                | Panel::UpdatedBy
                | Panel::BlockedByPerson),
            ) => {
                let mut people = vec![];
                if let Some(data) = &self.views.people {
                    for person in std::iter::once(&data["owner"]).chain(
                        data["members"]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .map(|member| &member["user"]),
                    ) {
                        if !person.is_null()
                            && !people.iter().any(|old: &Value| old["id"] == person["id"])
                        {
                            people.push(person.clone());
                        }
                    }
                    if matches!(panel, Panel::Assignees | Panel::UpdatedBy) {
                        people.extend(
                            data["boardAgents"]
                                .as_array()
                                .into_iter()
                                .flatten()
                                .cloned(),
                        );
                    }
                } else if self.options.offline {
                    people = self
                        .db
                        .project_members(self.options.board)
                        .unwrap_or_default();
                }
                if panel == Panel::BlockedByPerson {
                    people.insert(0, json!({"id":0,"displayName":"Anyone"}));
                }
                people
                    .into_iter()
                    .map(|person| (person["displayName"].as_str().unwrap_or("").into(), person))
                    .collect()
            }
            _ => vec![],
        };
        if self.views.panel.is_some_and(Self::view_searchable) && !self.views.query_text.is_empty()
        {
            let query = self.views.query_text.to_lowercase();
            let mut choices: Vec<_> = choices
                .into_iter()
                .filter(|(name, _)| name.to_lowercase().contains(&query))
                .collect();
            if matches!(self.views.panel, Some(Panel::Date(_))) {
                if let Some(date) = super::task_fields::parse_due(
                    &self.views.query_text,
                    chrono::Local::now().date_naive(),
                ) {
                    choices.push((self.views.query_text.clone(),json!({"selectedDate":date,"fromDate":date,"toDate":null,"condition":"AFTER","dynamicRange":null})));
                }
            }
            choices
        } else {
            choices
        }
    }
    fn choose_view_option(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if self.views.pending.is_some() {
            return;
        }
        match self.views.panel {
            Some(Panel::SavePrivate | Panel::SaveTeam) => {
                let title = self.views.input.read(cx).content.trim().to_owned();
                if title.is_empty() {
                    self.error = Some("Give your view a name first".into());
                    cx.notify();
                    return;
                }
                let public = self.views.panel == Some(Panel::SaveTeam);
                self.queue_view("create-view",json!({"userId":self.db.current_user_id(),"viewTitle":title,"visibility":if public{"Public"}else{"Private"},"setAsDefault":false,"view_settings":self.view_settings()}),window,cx);
                return;
            }
            Some(Panel::Save) => {
                match index {
                    2 => self.open_view_panel(Panel::SaveTeam, window, cx),
                    3 => self.open_view_panel(Panel::SavePrivate, window, cx),
                    4 => self.queue_view(
                        "reset-to-default",
                        json!({"mode":"ResetCurrent"}),
                        window,
                        cx,
                    ),
                    0 | 1 => {
                        let project = self
                            .db
                            .board_project(self.options.board)
                            .unwrap_or(Value::Null);
                        let applied =
                            project["project_view"]["user_project_views"][0]["appliedView"]["id"]
                                .clone();
                        if index == 0 && applied.is_string() {
                            self.queue_view(
                                "update-view",
                                json!({"viewId":applied,"view_settings":self.view_settings()}),
                                window,
                                cx,
                            );
                        } else {
                            self.queue_view("create-view",json!({"userId":self.db.current_user_id(),"viewTitle":format!("default-view-id-{}",self.options.board),"visibility":"Public","setAsDefault":true,"view_settings":self.view_settings()}),window,cx);
                        }
                    }
                    _ => (),
                }
                return;
            }
            _ => (),
        }
        let Some((_, value)) = self.view_choices().get(index).cloned() else {
            return;
        };
        let mut settings = self.view_settings();
        match self.views.panel {
            Some(Panel::Manage) => {
                if let Some(action) = value["action"].as_str() {
                    if action == "reset" {
                        self.queue_view("reset-order", json!({}), window, cx);
                        return;
                    }
                    let project = self
                        .db
                        .board_project(self.options.board)
                        .unwrap_or(Value::Null);
                    let mut views = ht_core::board_views::ordered_views(&project);
                    if action == "alphabetical" {
                        views.sort_by_key(|v| {
                            (
                                v["id"] != project["project_view"]["default_view_id"],
                                v["title"].as_str().unwrap_or("").to_lowercase(),
                            )
                        });
                    }
                    self.queue_view(if action=="default"{"set-default-order"}else{"update-order"},json!({"viewOrder":views.iter().map(|v|v["id"].clone()).collect::<Vec<_>>()}),window,cx);
                    return;
                }
                self.views.editing = value["id"].as_str().map(str::to_owned);
                self.open_view_panel(Panel::RenameView, window, cx);
                self.views.input.update(cx, |input, cx| {
                    input.set_content(value["title"].as_str().unwrap_or("").into(), cx)
                });
                return;
            }
            Some(Panel::RenameView) => {
                if value == "delete" {
                    self.views.panel = Some(Panel::DeleteView);
                    self.views.choice = 0;
                    cx.notify();
                    return;
                }
                let title = self.views.input.read(cx).content.to_string();
                if title.chars().count() < 2 {
                    self.error = Some("View name must be at least 2 characters".into());
                    cx.notify();
                    return;
                }
                self.queue_view(
                    "rename-view",
                    json!({"viewId":self.views.editing,"title":title}),
                    window,
                    cx,
                );
                return;
            }
            Some(Panel::DeleteView) => {
                if value == "cancel" {
                    self.views.panel = Some(Panel::Manage);
                    cx.notify();
                    return;
                }
                let project = self
                    .db
                    .board_project(self.options.board)
                    .unwrap_or(Value::Null);
                if project["project_view"]["default_view_id"] == json!(self.views.editing) {
                    return;
                }
                self.queue_view(
                    "delete-view",
                    json!({"viewId":self.views.editing}),
                    window,
                    cx,
                );
                return;
            }
            Some(Panel::Subtasks) => settings["board_subtask_setting"] = value,
            Some(Panel::Columns) => {
                let mut columns = settings["board_columns_view"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default();
                if let Some(column) = columns.iter_mut().find(|column| {
                    column.get("id").or_else(|| column.get("sectionId")) == value.get("id")
                }) {
                    column["visibility"] = json!(column["visibility"] != true);
                } else {
                    let mut column = value;
                    column["visibility"] = json!(true);
                    columns.push(column);
                }
                settings["board_columns_view"] = json!(columns);
            }
            Some(Panel::Sort) => {
                let mode = value.as_str().unwrap();
                let mut levels = self.sort_levels();
                if mode == "Manual" {
                    levels.clear();
                } else {
                    if levels.len() >= 3 || levels.iter().any(|level| level["mode"] == mode) {
                        return;
                    }
                    levels.push(json!({"mode":mode,"order":if matches!(mode,"DueDate"|"CreatedAt"|"Assignee"|"Title"|"TicketNumber"){"Ascending"}else{"Descending"}}));
                }
                self.views
                    .query
                    .update(cx, |input, cx| input.set_content(String::new(), cx));
                self.save_sort_levels(levels, window, cx);
                return;
            }
            Some(Panel::Filters) => {
                let kind = value.as_str().unwrap_or("");
                let panel = match kind {
                    "Priority" => Some(Panel::Priority),
                    "Size" => Some(Panel::Size),
                    "Labels" => Some(Panel::Labels),
                    "Assignees" => Some(Panel::Assignees),
                    "CreatedBy" => Some(Panel::CreatedBy),
                    "UpdatedBy" => Some(Panel::UpdatedBy),
                    "BlockedByPerson" => Some(Panel::BlockedByPerson),
                    "DueDate" => Some(Panel::Date("DueDate")),
                    "UpdatedRange" => Some(Panel::Date("UpdatedRange")),
                    "CreatedAt" => Some(Panel::Date("CreatedAt")),
                    _ => None,
                };
                if let Some(panel) = panel {
                    self.open_view_panel(panel, window, cx);
                    return;
                }
                if kind == "Clear" {
                    settings["board_filters"] = json!({"addedFilters":[],"matchFilters":"ANY"});
                } else if kind == "Match" {
                    settings["board_filters"]["matchFilters"] =
                        json!(if settings["board_filters"]["matchFilters"] == "ALL" {
                            "ANY"
                        } else {
                            "ALL"
                        });
                } else {
                    if kind == "RunningTimer"
                        && !self.options.offline
                        && self
                            .db
                            .board_project(self.options.board)
                            .is_ok_and(|project| project["timeTrackingEnabled"] == true)
                    {
                        self.load_view_support(true, window, cx);
                        return;
                    }
                    let mut filters = settings["board_filters"]["addedFilters"]
                        .as_array()
                        .cloned()
                        .unwrap_or_default();
                    if filters.iter().any(|filter| filter["type"] == kind) {
                        filters.retain(|filter| filter["type"] != kind);
                    } else if kind == "AssignedToMe" {
                        let Some(user) = self.db.current_user_id() else {
                            return;
                        };
                        filters.retain(|filter| filter["type"] != "Assignees");
                        filters.push(json!({"type":"Assignees","searchPayload":[{"id":user,"uid":"current-user"}],"match":"ANY"}));
                    } else {
                        filters.push(json!({"type":kind,"searchPayload":[]}));
                    }
                    settings["board_filters"]["addedFilters"] = json!(filters);
                }
            }
            Some(Panel::ValueMatch) => {
                for filter in settings["board_filters"]["addedFilters"]
                    .as_array_mut()
                    .into_iter()
                    .flatten()
                {
                    if filter["type"] == value {
                        filter["match"] = json!(if filter["match"] == "ALL" {
                            "ANY"
                        } else {
                            "ALL"
                        });
                    }
                }
            }
            Some(Panel::DateRange(kind)) => {
                if value == "BACK" {
                    self.open_view_panel(Panel::Date(kind), window, cx);
                    return;
                }
                let Some(from) = self.views.from else {
                    self.error = Some("Please select a date first!".into());
                    cx.notify();
                    return;
                };
                let iso = |date: NaiveDate| {
                    chrono::Local
                        .from_local_datetime(&date.and_hms_opt(0, 0, 0).unwrap())
                        .earliest()
                        .map(|date| date.to_rfc3339())
                };
                let mut filters = settings["board_filters"]["addedFilters"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default();
                filters.retain(|filter| filter["type"] != kind);
                filters.push(json!({"type":kind,"searchPayload":[{"fromDate":iso(from),"toDate":self.views.to.and_then(iso),"selectedDate":iso(from),"condition":null,"dynamicRange":null}]}));
                settings["board_filters"]["addedFilters"] = json!(filters);
            }
            Some(Panel::Date(kind)) => {
                if value == "CUSTOM" {
                    self.open_view_panel(Panel::DateRange(kind), window, cx);
                    return;
                }
                let mut filters = settings["board_filters"]["addedFilters"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default();
                filters.retain(|filter| filter["type"] != kind);
                if value != "RESET" {
                    let payload = if value.is_object() {
                        value
                    } else {
                        json!({"selectedDate":null,"fromDate":null,"toDate":null,"condition":if value=="ANY"{json!("ANY")}else{Value::Null},"dynamicRange":value})
                    };
                    filters.push(json!({"type":kind,"searchPayload":[payload]}));
                }
                settings["board_filters"]["addedFilters"] = json!(filters);
            }
            Some(
                panel @ (Panel::Priority
                | Panel::Size
                | Panel::Labels
                | Panel::Assignees
                | Panel::CreatedBy
                | Panel::UpdatedBy
                | Panel::BlockedByPerson),
            ) => {
                let kind = match panel {
                    Panel::Priority => "Priority",
                    Panel::Size => "Size",
                    Panel::Labels => "Labels",
                    Panel::CreatedBy => "CreatedBy",
                    Panel::UpdatedBy => "UpdatedBy",
                    Panel::BlockedByPerson => "BlockedByPerson",
                    _ => "Assignees",
                };
                let mut filters = settings["board_filters"]["addedFilters"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default();
                let mut selected = filters
                    .iter()
                    .find(|f| f["type"] == kind)
                    .and_then(|f| f["searchPayload"].as_array())
                    .cloned()
                    .unwrap_or_default();
                let id = |v: &Value| match panel {
                    Panel::Priority => v["priority_index"].clone(),
                    Panel::Size => v["estimate_index"].clone(),
                    _ => v["id"].clone(),
                };
                if let Some(i) = selected.iter().position(|v| id(v) == id(&value)) {
                    selected.remove(i);
                } else {
                    selected.push(value);
                }
                let previous_match = filters
                    .iter()
                    .find(|filter| filter["type"] == kind)
                    .and_then(|filter| filter["match"].as_str())
                    .unwrap_or("ANY")
                    .to_owned();
                filters.retain(|f| f["type"] != kind);
                if !selected.is_empty() {
                    let mut filter = json!({"type":kind,"searchPayload":selected});
                    if matches!(panel, Panel::Labels | Panel::Assignees) {
                        filter["match"] = json!(previous_match);
                    }
                    filters.push(filter);
                }
                settings["board_filters"]["addedFilters"] = json!(filters);
            }
            _ => return,
        }
        self.queue_view("unsaved-view", settings, window, cx);
    }
    pub(super) fn toggle_empty_columns(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let project = self
            .db
            .board_project(self.options.board)
            .unwrap_or(Value::Null);
        let setting = if self.view_settings()["board_empty_sections"] == "Hidden" {
            "Show"
        } else {
            "Hidden"
        };
        let target = project["project_view"]["user_project_views"][0]["appliedView"]["id"]
            .as_str()
            .or_else(|| project["project_view"]["default_view"]["id"].as_str());
        if let Some(id) = target {
            self.queue_view("update-view",json!({"viewId":id,"updateMode":"personal-empty-sections","view_settings":{"board_empty_sections":setting}}),window,cx);
        } else {
            let mut settings = self.view_settings();
            settings["board_empty_sections"] = json!(setting);
            self.queue_view("unsaved-view", settings, window, cx);
        }
    }
    pub(super) fn hide_current_column(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.columns.get(self.column).map(|column| column.id) else {
            return;
        };
        let mut settings = self.view_settings();
        for column in settings["board_columns_view"]
            .as_array_mut()
            .into_iter()
            .flatten()
        {
            if column.get("id").or_else(|| column.get("sectionId")) == Some(&json!(id)) {
                column["visibility"] = json!(false);
            }
        }
        self.queue_view("unsaved-view", settings, window, cx);
    }
    pub(super) fn command_sort(&mut self, mode: &str, window: &mut Window, cx: &mut Context<Self>) {
        let mut levels = vec![json!({"mode":mode,"order":"Descending"})];
        levels.extend(
            self.view_settings()["board_sorting_stack"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|level| level["mode"] != mode)
                .take(2)
                .cloned(),
        );
        self.save_sort_levels(levels, window, cx);
    }
    fn sort_levels(&self) -> Vec<Value> {
        let settings = self.view_settings();
        if settings["board_sorting_mode"] == "Manual" {
            return vec![];
        }
        let mut levels = vec![
            json!({"mode":settings["board_sorting_mode"],"order":settings["board_sorting_order"]}),
        ];
        levels.extend(
            settings["board_sorting_stack"]
                .as_array()
                .into_iter()
                .flatten()
                .cloned(),
        );
        levels
    }
    fn save_sort_levels(
        &mut self,
        levels: Vec<Value>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mut settings = self.view_settings();
        settings["board_sorting_mode"] = levels
            .first()
            .map(|level| level["mode"].clone())
            .unwrap_or(json!("Manual"));
        if let Some(level) = levels.first() {
            settings["board_sorting_order"] = level["order"].clone();
        }
        settings["board_sorting_stack"] = json!(levels.into_iter().skip(1).collect::<Vec<_>>());
        self.queue_view("unsaved-view", settings, window, cx);
    }
    fn toggle_value_match(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let kind = if self.views.panel == Some(Panel::Labels) {
            "Labels"
        } else {
            "Assignees"
        };
        let mut settings = self.view_settings();
        let Some(filter) = settings["board_filters"]["addedFilters"]
            .as_array_mut()
            .and_then(|filters| filters.iter_mut().find(|filter| filter["type"] == kind))
        else {
            return;
        };
        filter["match"] = json!(if filter["match"] == "ALL" {
            "ANY"
        } else {
            "ALL"
        });
        self.queue_view("unsaved-view", settings, window, cx);
    }
    fn load_view_support(&mut self, timers: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.views.support_pending {
            return;
        }
        let Some(token) = self.cli_token() else {
            return;
        };
        let board = self.options.board;
        let agents = matches!(self.views.panel, Some(Panel::Assignees | Panel::UpdatedBy));
        let generation = self.views.generation;
        self.views.support_pending = true;
        if !timers {
            self.views.people = None;
        }
        let (send, receive) = std::sync::mpsc::channel();
        let expected_token = token.clone();
        std::thread::spawn(move || {
            let result = ht_core::Http::new(token)
                .and_then(|mut http| {
                    if timers {
                        ht_core::board_views::running_timers(&mut http, board)
                    } else {
                        let mut data = ht_core::board_views::filter_people(&mut http, board)?;
                        if agents {
                            data["boardAgents"] =
                                ht_core::board_views::filter_agents(&mut http, board)
                                    .unwrap_or_else(|error| {
                                        eprintln!("Failed to load board agents: {error}");
                                        json!([])
                                    });
                        }
                        Ok(data)
                    }
                })
                .map_err(|error| error.to_string());
            let _ = send.send(result);
        });
        cx.spawn_in(window, async move |view, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(50))
                    .await;
                match receive.try_recv() {
                    Ok(result) => {
                        let _ = view.update_in(cx, |this, window, cx| {
                            if this.views.generation != generation {
                                return;
                            }
                            this.views.support_pending = false;
                            if this.options.board != board
                                || this.cli_token().as_deref() != Some(expected_token.as_str())
                            {
                                return;
                            }
                            match result {
                                Ok(data) => {
                                    if timers {
                                        let _ = this.db.kv_set(
                                            &format!(
                                                "web-running-timers:{}:{board}",
                                                this.db.current_user_id().unwrap_or(0)
                                            ),
                                            &data.to_string(),
                                        );
                                        let mut settings = this.view_settings();
                                        let mut filters = settings["board_filters"]["addedFilters"]
                                            .as_array()
                                            .cloned()
                                            .unwrap_or_default();
                                        if filters
                                            .iter()
                                            .any(|filter| filter["type"] == "RunningTimer")
                                        {
                                            filters
                                                .retain(|filter| filter["type"] != "RunningTimer");
                                        } else {
                                            filters.push(
                                                json!({"type":"RunningTimer","searchPayload":[]}),
                                            );
                                        }
                                        settings["board_filters"]["addedFilters"] = json!(filters);
                                        this.queue_view("unsaved-view", settings, window, cx);
                                    } else {
                                        this.views.people = Some(data);
                                    }
                                }
                                Err(error) => {
                                    this.error = Some(error);
                                    this.views.people = None;
                                }
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
    pub(super) fn view_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.views.panel.is_none() {
            return false;
        }
        match event.keystroke.key.as_str() {
            "escape" if event.keystroke.modifiers.shift => {
                let next = match self.views.panel {
                    Some(Panel::DateRange(kind)) => Panel::Date(kind),
                    _ => Panel::Filters,
                };
                self.open_view_panel(next, window, cx);
            }
            "escape" => {
                self.views.panel = None;
                window.focus(&self.focus, cx);
            }
            "left" | "right"
                if matches!(self.views.panel, Some(Panel::Labels | Panel::Assignees)) =>
            {
                self.toggle_value_match(window, cx)
            }
            "tab" | "left" | "right" if self.views.panel == Some(Panel::Sort) => {
                let mut levels = self.sort_levels();
                if let Some(level) = levels.last_mut() {
                    level["order"] = json!(if level["order"] == "Ascending" {
                        "Descending"
                    } else {
                        "Ascending"
                    });
                    self.save_sort_levels(levels, window, cx);
                }
            }
            "enter" => self.choose_view_option(self.views.choice, window, cx),
            "down" => {
                self.views.choice =
                    (self.views.choice + 1).min(self.view_choices().len().saturating_sub(1))
            }
            "up" => self.views.choice = self.views.choice.saturating_sub(1),
            _ => {
                return !self.views.panel.is_some_and(|panel| {
                    Self::view_searchable(panel)
                        || matches!(
                            panel,
                            Panel::SavePrivate | Panel::SaveTeam | Panel::RenameView
                        )
                });
            }
        }
        cx.stop_propagation();
        cx.notify();
        true
    }
    fn select_board_view(&mut self, id: Value, window: &mut Window, cx: &mut Context<Self>) {
        if id.as_str().is_some_and(|id| id.starts_with("builtin:")) {
            let project = self
                .db
                .board_project(self.options.board)
                .unwrap_or(Value::Null);
            if !ht_core::board_views::builtin_views(&project, self.views.decisions)
                .iter()
                .any(|view| view["id"] == id)
            {
                return;
            }
            self.views
                .builtin
                .insert(self.options.board, id.as_str().unwrap().into());
            self.reload();
            cx.notify();
            return;
        }
        if self.views.pending.is_some() {
            return;
        }
        self.views.previous_builtin = self
            .views
            .builtin
            .remove(&self.options.board)
            .map(|previous| (self.options.board, previous));
        self.queue_view("switch-view", json!({"newViewId":id}), window, cx);
        if self.views.pending.is_none() {
            if let Some((board, previous)) = self.views.previous_builtin.take() {
                self.views.builtin.entry(board).or_insert(previous);
            }
        }
    }
    pub(super) fn board_view_tabs(&self, cx: &mut Context<Self>) -> AnyElement {
        let project = self
            .db
            .board_project(self.options.board)
            .unwrap_or(Value::Null);
        if !project["project_view"].is_object() {
            return div().into_any_element();
        }
        let mut views = ht_core::board_views::ordered_views(&project);
        views.extend(
            ht_core::board_views::builtin_views(&project, self.views.decisions)
                .into_iter()
                .filter(|view| {
                    self.views
                        .builtin
                        .get(&self.options.board)
                        .is_some_and(|id| view["id"] == *id)
                        || self
                            .db
                            .board_in_view(self.options.board, view["id"].as_str())
                            .is_ok_and(|columns| {
                                columns.iter().any(|column| !column.cards.is_empty())
                            })
                }),
        );
        let order = project["project_view"]["user_project_views"][0]["view_order"]
            .as_array()
            .or_else(|| project["project_view"]["default_view_order"].as_array());
        views.sort_by_key(|view| {
            if view["id"] == project["project_view"]["default_view_id"] {
                0
            } else {
                1 + order
                    .and_then(|order| order.iter().position(|id| *id == view["id"]))
                    .unwrap_or(usize::MAX - 1)
            }
        });
        div()
            .px(px(24.))
            .flex()
            .gap(px(12.))
            .children(views.into_iter().enumerate().map(|(i, view)| {
                let id = view["id"].clone();
                let title = view["title"].as_str().unwrap_or("").to_owned();
                div()
                    .id(("saved-view", i))
                    .cursor_pointer()
                    .child(title)
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.select_board_view(id.clone(), window, cx);
                    }))
            }))
            .child(
                div()
                    .id("save-current-view")
                    .cursor_pointer()
                    .child("Save view")
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.open_view_panel(Panel::Save, window, cx)
                    })),
            )
            .child(
                div()
                    .id("manage-board-views")
                    .cursor_pointer()
                    .child("Manage views")
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.open_view_panel(Panel::Manage, window, cx)
                    })),
            )
            .into_any_element()
    }
    fn can_set_view_default(&self, project: &Value) -> bool {
        self.db.current_user_id().is_some_and(|user| {
            project["ownerId"] == user
                || project["members"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .any(|m| {
                        m["userId"] == user && m["status"] == "Accepted" && m["role"] == "Admin"
                    })
        })
    }
    fn view_date_calendar(&self, cx: &mut Context<Self>) -> AnyElement {
        let mut months = div().flex().gap(px(16.));
        for index in 0..2 {
            let month = self
                .views
                .month
                .checked_add_months(chrono::Months::new(index))
                .unwrap_or(self.views.month);
            let first =
                month - chrono::Duration::days(month.weekday().num_days_from_sunday() as i64);
            let mut calendar = div().flex().flex_col().child(
                div()
                    .flex()
                    .justify_between()
                    .child(
                        div()
                            .id(("filter-month-prev", index))
                            .cursor_pointer()
                            .child("‹")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.views.month = this
                                    .views
                                    .month
                                    .checked_sub_months(chrono::Months::new(1))
                                    .unwrap_or(this.views.month);
                                cx.notify();
                            })),
                    )
                    .child(month.format("%B %Y").to_string())
                    .child(
                        div()
                            .id(("filter-month-next", index))
                            .cursor_pointer()
                            .child("›")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.views.month = this
                                    .views
                                    .month
                                    .checked_add_months(chrono::Months::new(1))
                                    .unwrap_or(this.views.month);
                                cx.notify();
                            })),
                    ),
            );
            for week in 0..6 {
                let mut row = div().flex();
                for day in 0..7 {
                    let date = first + chrono::Duration::days(week * 7 + day);
                    let selected = self
                        .views
                        .from
                        .is_some_and(|from| date >= from && date <= self.views.to.unwrap_or(from));
                    row = row.child(
                        div()
                            .id((
                                "filter-range-day",
                                (index as usize) * 42 + (week * 7 + day) as usize,
                            ))
                            .w(px(28.))
                            .h(px(28.))
                            .cursor_pointer()
                            .when(selected, |d| d.bg(color(self.colors.active)))
                            .child(date.day().to_string())
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if let Some(from) =
                                    this.views.from.filter(|_| this.views.to.is_none())
                                {
                                    this.views.from = Some(from.min(date));
                                    this.views.to = Some(from.max(date));
                                } else {
                                    this.views.from = Some(date);
                                    this.views.to = None;
                                }
                                cx.notify();
                            })),
                    );
                }
                calendar = calendar.child(row);
            }
            months = months.child(calendar);
        }
        months.into_any_element()
    }
    pub(super) fn view_overlay(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(panel) = self.views.panel else {
            return div().into_any_element();
        };
        let t = self.colors;
        let title = match panel {
            Panel::Sort => "Sort",
            Panel::Columns => "Manage board columns",
            Panel::Subtasks => "Sub-task Settings",
            Panel::Date("DueDate") | Panel::DateRange("DueDate") => "Filter by due date",
            Panel::Date("CreatedAt") | Panel::DateRange("CreatedAt") => "Filter by created date",
            Panel::Date(_) | Panel::DateRange(_) => "Filter by last updated date",
            Panel::Save => "Save this view",
            Panel::SaveTeam => "Save as new view for team",
            Panel::SavePrivate => "Save as view for me only",
            Panel::Manage => "Manage views",
            Panel::RenameView => "View settings",
            Panel::DeleteView => "Delete this view? That cannot be undone.",
            _ => "Filter",
        };
        div()
            .absolute()
            .inset_0()
            .bg(rgba(0x00000040))
            .flex()
            .items_center()
            .justify_center()
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|this, _, window, cx| {
                    this.views.panel = None;
                    window.focus(&this.focus, cx);
                    cx.notify();
                }),
            )
            .child(
                div()
                    .id("board-view-dialog")
                    .on_mouse_down(
                        gpui::MouseButton::Left,
                        cx.listener(|_, _, _, cx| cx.stop_propagation()),
                    )
                    .w(px(480.))
                    .max_h(px(600.))
                    .overflow_y_scroll()
                    .p(px(16.))
                    .bg(color(t.comment))
                    .rounded(px(6.))
                    .child(title)
                    .when(Self::view_searchable(panel), |d| {
                        d.child(self.views.query.clone())
                    })
                    .when(matches!(panel, Panel::DateRange(_)), |d| {
                        d.child(self.view_date_calendar(cx))
                    })
                    .when(
                        matches!(
                            panel,
                            Panel::SavePrivate | Panel::SaveTeam | Panel::RenameView
                        ),
                        |d| d.child(self.views.input.clone()),
                    )
                    .when(panel == Panel::Sort, |d| {
                        d.children(
                            self.sort_levels()
                                .into_iter()
                                .enumerate()
                                .map(|(i, level)| {
                                    div()
                                        .flex()
                                        .child(format!(
                                            "{} · {}",
                                            level["mode"].as_str().unwrap_or(""),
                                            level["order"].as_str().unwrap_or("")
                                        ))
                                        .child(
                                            div()
                                                .id(("sort-level-order", i))
                                                .p(px(8.))
                                                .cursor_pointer()
                                                .child("↕")
                                                .on_click(cx.listener(
                                                    move |this, _, window, cx| {
                                                        let mut levels = this.sort_levels();
                                                        if let Some(level) = levels.get_mut(i) {
                                                            level["order"] =
                                                                json!(if level["order"]
                                                                    == "Ascending"
                                                                {
                                                                    "Descending"
                                                                } else {
                                                                    "Ascending"
                                                                });
                                                            this.save_sort_levels(
                                                                levels, window, cx,
                                                            );
                                                        }
                                                    },
                                                )),
                                        )
                                        .child(
                                            div()
                                                .id(("sort-level-remove", i))
                                                .p(px(8.))
                                                .cursor_pointer()
                                                .child("×")
                                                .on_click(cx.listener(
                                                    move |this, _, window, cx| {
                                                        let mut levels = this.sort_levels();
                                                        if i < levels.len() {
                                                            levels.remove(i);
                                                            this.save_sort_levels(
                                                                levels, window, cx,
                                                            );
                                                        }
                                                    },
                                                )),
                                        )
                                }),
                        )
                    })
                    .when(matches!(panel, Panel::Labels | Panel::Assignees), |d| {
                        d.child(
                            div()
                                .id("filter-value-match")
                                .p(px(8.))
                                .cursor_pointer()
                                .child("Match any / all")
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.toggle_value_match(window, cx)
                                })),
                        )
                    })
                    .children(
                        self.view_choices()
                            .into_iter()
                            .enumerate()
                            .map(|(i, (name, _))| {
                                div()
                                    .id(("view-option", i))
                                    .p(px(8.))
                                    .cursor_pointer()
                                    .when(i == self.views.choice, |d| d.bg(color(t.active)))
                                    .child(name)
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.choose_view_option(i, window, cx)
                                    }))
                                    .layout_key(
                                        &self.layout,
                                        format!("board.view.option.{i}"),
                                        None,
                                    )
                            }),
                    )
                    .layout_key(&self.layout, "board.view.dialog", None),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::task_fields::tests::desktop;
    struct Server(u16, Value);
    impl ht_core::Transport for Server {
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
            assert_eq!(path, "/api/projects/views/unsaved-view");
            assert_eq!(body["projectId"], 15);
            self.1["user_project_views"][0]["unsavedView"] = body.clone();
            Ok(ht_core::http::Response {
                status: self.0,
                body: self.1.clone(),
            })
        }
    }
    #[gpui::test]
    fn sort_picker_queues_full_settings_and_refusal_rolls_back_without_closing_picker(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(crate::input::init);
        let(view,cx)=cx.add_window_view(|window,cx|{let mut this=desktop(window,cx);this.screen=Screen::Board;
            this.db.apply_response(&json!({"success":true,"projects":[{"id":15,"title":"Board","project_view":{"id":"pv","projectId":15,"user_project_views":[{}]}}]})).unwrap();this.reload();this});
        for status in [401, 403, 200] {
            view.update_in(cx, |this, window, cx| {
                this.open_view_panel(Panel::Sort, window, cx);
                this.choose_view_option(1, window, cx);
                let rows = this.db.mutations().unwrap();
                let row = rows.iter().find(|r| r.status != "failed").unwrap();
                assert_eq!(row.kind, "web_view");
                assert_eq!(
                    row.payload_json["change"]["body"]["board_sorting_mode"],
                    "Priority"
                );
                assert!(
                    row.payload_json["change"]["body"]
                        .get("board_filters")
                        .is_some()
                );
                this.db
                    .drain_once(
                        &mut Server(
                            status,
                            json!({"id":"pv","projectId":15,"user_project_views":[{}]}),
                        ),
                        chrono::Utc::now().timestamp() + 1,
                    )
                    .unwrap();
                assert!(!this.poll_view(window, cx));
                assert_eq!(this.views.panel, Some(Panel::Sort));
                assert_eq!(
                    this.view_settings()["board_sorting_mode"],
                    if status == 200 { "Priority" } else { "Manual" }
                );
            });
        }
    }
}

#[cfg(test)]
mod management_tests {
    use super::*;
    use crate::ui::task_fields::tests::desktop;
    struct Web {
        status: u16,
        expected: ht_core::ViewWrite,
        reply: Value,
    }
    impl ht_core::Transport for Web {
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
            key: &str,
        ) -> Result<ht_core::http::Response, ht_core::http::TransportError> {
            self.mutate("POST", path, body, key)
        }
        fn mutate(
            &mut self,
            method: &str,
            path: &str,
            body: &Value,
            _: &str,
        ) -> Result<ht_core::http::Response, ht_core::http::TransportError> {
            let (m, p, b) = self.expected.request().unwrap();
            assert_eq!(method, m);
            assert_eq!(path, p);
            assert_eq!(body, &b);
            Ok(ht_core::http::Response {
                status: self.status,
                body: if self.status == 200 {
                    self.reply.clone()
                } else {
                    json!({"message":"Board access denied"})
                },
            })
        }
    }
    #[gpui::test]
    fn save_view_menu_uses_all_five_web_flows_and_keeps_name_on_both_refusals(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(crate::input::init);
        let (view, cx) = cx.add_window_view(|window, cx| {
            let mut this = desktop(window, cx);
            this.screen = Screen::Board;
            this
        });
        for option in 0..5 {
            for status in [401, 403, 200] {
                view.update_in(cx,|this,window,cx|{
            this.db.apply_response(&json!({"success":true,"projects":[{"id":15,"title":"Board","project_view":{"id":"pv","projectId":15,"user_project_views":[{"appliedView":{"id":"v1","title":"Daily"}}],"allViews":[{"id":"v1","title":"Daily"}]}}]})).unwrap();this.reload();
            this.open_view_panel(Panel::Save,window,cx);assert_eq!(this.view_choices().len(),5);this.choose_view_option(option,window,cx);
            if matches!(option,2|3){this.views.input.update(cx,|input,cx|input.set_content("Named view".into(),cx));this.choose_view_option(0,window,cx);}
            let row=this.db.mutations().unwrap().into_iter().find(|r|r.status!="failed").unwrap();let change:ht_core::ViewWrite=serde_json::from_value(row.payload_json["change"].clone()).unwrap();
            if option==1||option==2||option==3{assert_eq!(change.route,"create-view");assert_eq!(change.body["userId"],42);assert_eq!(change.body["visibility"],if option==3{"Private"}else{"Public"});assert_eq!(change.body["setAsDefault"],option==1);}
            if option==0{assert_eq!(change.route,"update-view");assert_eq!(change.body["viewId"],"v1");}if option==4{assert_eq!(change.body["mode"],"ResetCurrent");}
            let pv=this.db.board_project(15).unwrap()["project_view"].clone();let reply=if change.route=="create-view"{json!({"project_view_updated":pv})}else{pv};
            this.db.drain_once(&mut Web{status,expected:change,reply},chrono::Utc::now().timestamp()+1).unwrap();assert!(!this.poll_view(window,cx));assert_eq!(this.views.panel.is_none(),status==200);
            if status!=200{assert_eq!(this.error.as_deref(),Some("Board access denied"));if option==2||option==3{assert_eq!(this.views.input.read(cx).content.as_ref(),"Named view");}}
        });
            }
        }
    }
}

#[cfg(test)]
mod advanced_control_tests {
    use super::*;
    use crate::ui::task_fields::tests::desktop;
    struct Server {
        status: u16,
        body: Value,
        reply: Value,
    }
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
            assert_eq!(
                path,
                if self.body.get("newViewId").is_some() {
                    "/api/projects/views/switch-view"
                } else {
                    "/api/projects/views/unsaved-view"
                }
            );
            assert_eq!(body, &self.body);
            Ok(ht_core::http::Response {
                status: self.status,
                body: if self.status == 200 {
                    self.reply.clone()
                } else {
                    json!({"message":"Board access denied"})
                },
            })
        }
    }
    fn settle(this: &mut Desktop, status: u16, window: &mut Window, cx: &mut Context<Desktop>) {
        let row = this
            .db
            .mutations()
            .unwrap()
            .into_iter()
            .find(|row| row.status != "failed")
            .unwrap();
        let body = row.payload_json["change"]["body"].clone();
        let mut reply = this.db.board_project(15).unwrap()["project_view"].clone();
        reply["user_project_views"][0]["unsavedView"] = body.clone();
        this.db
            .drain_once(
                &mut Server {
                    status,
                    body,
                    reply,
                },
                chrono::Utc::now().timestamp() + 1,
            )
            .unwrap();
        assert!(!this.poll_view(window, cx));
        if status != 200 {
            assert_eq!(this.error.as_deref(), Some("Board access denied"));
        }
    }
    #[gpui::test]
    fn builtin_selection_is_session_local_and_saved_switch_refusal_preserves_board_and_newer_intent(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(crate::input::init);
        let (view, cx) = cx.add_window_view(|window, cx| desktop(window, cx));
        for newer in [false, true] {
            for status in [401, 403, 200] {
                view.update_in(cx,|this,window,cx|{
            this.db.apply_response(&json!({"success":true,"projects":[{"id":15,"title":"Board","project_view":{"id":"pv","projectId":15,"allViews":[{"id":"v1","title":"Daily"}],"user_project_views":[{}]}}]})).unwrap();this.select_board_view(json!("builtin:my-tasks"),window,cx);assert_eq!(this.views.builtin.get(&15).map(String::as_str),Some("builtin:my-tasks"));assert!(!this.db.mutations().unwrap().iter().any(|row|row.status!="failed"));
            this.select_board_view(json!("v1"),window,cx);assert!(!this.views.builtin.contains_key(&15));if newer{this.select_board_view(json!("builtin:agents"),window,cx);}
            settle(this,status,window,cx);assert_eq!(this.views.builtin.get(&15).map(String::as_str),if newer{Some("builtin:agents")}else if status!=200{Some("builtin:my-tasks")}else{None});this.views.builtin.clear();
        });
            }
        }
    }
    #[gpui::test]
    fn custom_calendar_range_and_typed_date_search_queue_exact_web_payload_and_restore_refusals(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(crate::input::init);
        let (view, cx) = cx.add_window_view(|window, cx| desktop(window, cx));
        for kind in ["DueDate", "CreatedAt", "UpdatedRange"] {
            for status in [401, 403, 200] {
                view.update_in(cx,|this,window,cx|{
            this.db.apply_response(&json!({"success":true,"projects":[{"id":15,"title":"Board","project_view":{"id":"pv","projectId":15,"user_project_views":[{}]}}]})).unwrap();this.open_view_panel(Panel::Date(kind),window,cx);
            let custom=this.view_choices().iter().position(|(_,value)|*value=="CUSTOM").unwrap();this.choose_view_option(custom,window,cx);assert_eq!(this.views.panel,Some(Panel::DateRange(kind)));
            this.views.from=Some(NaiveDate::from_ymd_opt(2026,10,1).unwrap());this.views.to=Some(NaiveDate::from_ymd_opt(2026,10,15).unwrap());let before=this.view_settings();this.choose_view_option(1,window,cx);
            let payload=this.view_settings()["board_filters"]["addedFilters"][0]["searchPayload"][0].clone();assert!(payload["fromDate"].as_str().unwrap().starts_with("2026-10-01T00:00:00"));assert!(payload["toDate"].as_str().unwrap().starts_with("2026-10-15T00:00:00"));assert!(payload["dynamicRange"].is_null());settle(this,status,window,cx);if status!=200{assert_eq!(this.view_settings(),before);assert_eq!(this.views.panel,Some(Panel::DateRange(kind)));}
        });
            }
        }
        view.update_in(cx, |this, window, cx| {
            this.open_view_panel(Panel::Date("DueDate"), window, cx);
            this.views.query_text = "2026-12-01".into();
            let choices = this.view_choices();
            assert_eq!(choices.len(), 1);
            assert_eq!(choices[0].1["condition"], "AFTER");
            this.choose_view_option(0, window, cx);
            settle(this, 200, window, cx);
            this.open_view_panel(Panel::Filters, window, cx);
            this.views.query_text = "stale".into();
            assert!(
                this.view_choices()
                    .iter()
                    .all(|(name, _)| name.to_lowercase().contains("stale"))
            );
        });
    }
    #[gpui::test]
    fn actor_date_match_and_sort_stack_controls_queue_web_settings_and_rollback_both_refusals(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(crate::input::init);
        let (view, cx) = cx.add_window_view(|window, cx| desktop(window, cx));
        for status in [401, 403, 200] {
            for panel in [
                Panel::CreatedBy,
                Panel::UpdatedBy,
                Panel::BlockedByPerson,
                Panel::Date("DueDate"),
                Panel::Date("CreatedAt"),
                Panel::Date("UpdatedRange"),
                Panel::ValueMatch,
                Panel::Sort,
                Panel::Columns,
                Panel::Subtasks,
            ] {
                view.update_in(cx,|this,window,cx|{
            this.db.apply_response(&json!({"success":true,"projects":[{"id":15,"title":"Board","project_view":{"id":"pv","projectId":15,"user_project_views":[{}]}}]})).unwrap();this.reload();
            this.views.people=Some(json!({"owner":{"id":42,"uid":"u42","displayName":"Owner"},"members":[],"boardAgents":[{"id":"agent","displayName":"Helper"}]}));
            this.open_view_panel(panel,window,cx);
            if panel==Panel::ValueMatch{this.views.panel=Some(Panel::Assignees);this.choose_view_option(0,window,cx);settle(this,200,window,cx);this.views.panel=Some(Panel::ValueMatch);}
            let before=this.view_settings();this.choose_view_option(if matches!(panel,Panel::Date(_)){2}else{0},window,cx);
            assert_ne!(this.view_settings(),before,"{panel:?}");settle(this,status,window,cx);
            if status!=200{assert_eq!(this.view_settings(),before);}
        });
            }
        }
        view.update_in(cx,|this,window,cx|{
            this.db.apply_response(&json!({"success":true,"projects":[{"id":15,"title":"Board","project_view":{"id":"pv","projectId":15,"user_project_views":[{}]}}]})).unwrap();this.open_view_panel(Panel::Sort,window,cx);
            for index in [1,5,2]{this.choose_view_option(index,window,cx);settle(this,200,window,cx);assert_eq!(this.views.panel,Some(Panel::Sort));}
            assert_eq!(this.sort_levels().len(),3);assert_eq!(this.sort_levels()[1]["mode"],"Title");assert_eq!(this.sort_levels()[1]["order"],"Ascending");
            this.choose_view_option(4,window,cx);assert!(this.views.pending.is_none());assert_eq!(this.sort_levels().len(),3);
            let mut levels=this.sort_levels();levels.remove(0);this.save_sort_levels(levels,window,cx);settle(this,200,window,cx);assert_eq!(this.view_settings()["board_sorting_mode"],"Title");
        });
    }
}
