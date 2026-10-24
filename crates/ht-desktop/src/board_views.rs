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
    AddColumn,
    EditColumn,
    DeleteColumn,
    ColumnAssignees,
    Subtasks,
    Save,
    SavePrivate,
    SaveTeam,
    Manage,
    RenameView,
    SmartSplit,
    DeleteSmartSplit,
    DeleteView,
}
#[derive(Clone)]
struct ViewDrag { id: String, title: String }
impl Render for ViewDrag {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().p(px(8.)).bg(color(0x30343b)).text_color(color(0xffffff)).child(self.title.clone())
    }
}
pub(super) struct Views {
    pub loaded: i64,
    loading: bool,
    ready: bool,
    generation: u64,
    pub panel: Option<Panel>,
    input: Entity<TextInput>,
    prompt: Entity<TextInput>,
    column_edit: Option<i64>,
    pub(super) column_auto_select: bool,
    column_deferred: std::collections::VecDeque<(String, Value)>,
    column_reorder: bool,
    column_error: String,
    column_title_blur: Option<gpui::Subscription>,
    pending: Option<String>,
    choice: usize,
    editing: Option<String>,
    pending_route: String,
    people: Option<Value>,
    pub(super) builtin: std::collections::BTreeMap<i64, String>,
    decisions: bool,
    previous_builtin: Option<(i64, String)>,
    self_filter: bool,
    support_pending: bool,
    archived_board: Option<(i64, i64)>,
    archived_loading: bool,
    archived_generation: u64,
    query: Entity<TextInput>,
    query_text: String,
    month: NaiveDate,
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
}
impl Views {
    pub fn new(cx: &mut Context<Desktop>) -> Self {
        let input = cx.new(|cx| TextInput::new(String::new(), "View name", color(0x8e9093), cx));
        let prompt = cx.new(|cx| TextInput::new(String::new(), "Describe which tasks belong in this split", color(0x8e9093), cx).multiline());
        cx.observe(&prompt, |_, _, cx| cx.notify()).detach();
        cx.observe(&input, |_, _, cx| cx.notify()).detach();
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
            prompt,
            column_edit: None,
            column_auto_select: false,
            column_deferred: Default::default(),
            column_reorder: false,
            column_error: String::new(),
            column_title_blur: None,
            pending: None,
            choice: 0,
            editing: None,
            pending_route: String::new(),
            people: None,
            builtin: Default::default(),
            decisions: false,
            previous_builtin: None,
            self_filter: false,
            support_pending: false,
            archived_board: None,
            archived_loading: false,
            archived_generation: 0,
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
        self.views.column_title_blur = None;
        self.views.column_deferred.clear();
        self.views.column_auto_select = false;
        self.views.column_edit = None;
        self.views.column_reorder = false;
        self.views.builtin.clear();
        self.views.decisions = false;
        self.views.previous_builtin = None;
        self.views.self_filter = false;
        self.views.archived_board = None;
        self.views.archived_loading = false;
        self.views.archived_generation += 1;
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
                match ht_core::smart_splits::labels(&mut http, board) {
                    Ok(labels) => { body["desktop_smart_labels"] = labels; body["desktop_smart_labels_error"] = Value::Null; }
                    Err(error) => { body["desktop_smart_labels"] = Value::Null; body["desktop_smart_labels_error"] = json!(error.to_string()); }
                }
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
    pub(super) fn show_archived_on_board(&self) -> bool {
        self.view_settings()["board_show_archived"].as_bool().unwrap_or_else(|| self.db.kv_get("showArchivedOnBoard").ok().flatten().as_deref() == Some("true"))
    }
    pub(super) fn toggle_archived_on_board(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.views.pending.is_some() || !self.board_metadata_ready(cx) { return; }
        let mut settings = self.view_settings();
        let next = !self.show_archived_on_board();
        if settings["board_show_archived"].is_null() {
            if let Err(error) = self.db.kv_set("showArchivedOnBoard", if next { "true" } else { "false" }) { self.error = Some(error.to_string()); return; }
        }
        settings["board_show_archived"] = json!(next);
        self.queue_view("unsaved-view", settings, window, cx);
        if next { self.views.archived_board = None; }
        self.load_archived_on_board(cx);
    }
    pub(super) fn load_archived_on_board(&mut self, cx: &mut Context<Self>) {
        let board = self.options.board;
        let Some(user) = self.db.current_user_id() else { return; };
        if self.options.offline || !self.show_archived_on_board() || self.views.archived_loading || self.views.archived_board == Some((user, board)) { return; }
        let Some(token) = self.cli_token() else { return; };
        self.views.archived_loading = true;
        let expected = token.clone();
        self.views.archived_generation += 1;
        let generation = self.views.archived_generation;
        let (send, receive) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let result = ht_core::Http::new(token).and_then(|mut http| ht_core::board_views::archived_tasks(&mut http, board)).map_err(|error| error.to_string());
            let _ = send.send(result);
        });
        cx.spawn(async move |view, cx| {
            loop {
                cx.background_executor().timer(Duration::from_millis(50)).await;
                match receive.try_recv() {
                    Ok(result) => {
                        let _ = view.update(cx, |this, cx| {
                            if this.views.archived_generation != generation { return; }
                            this.views.archived_loading = false;
                            if this.options.board != board || this.db.current_user_id() != Some(user) || this.cli_token().as_deref() != Some(expected.as_str()) { return; }
                            this.views.archived_board = Some((user, board));
                            match result.and_then(|tasks| this.db.apply_archived_board_tasks(user, board, &tasks).map_err(|error| error.to_string())) {
                                Ok(()) => this.reload(),
                                Err(error) => this.error = Some(error),
                            }
                            cx.notify();
                        });
                        break;
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => (),
                    _ => break,
                }
            }
        }).detach();
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
            Panel::CreatedBy | Panel::UpdatedBy | Panel::BlockedByPerson | Panel::Assignees | Panel::ColumnAssignees
        ) && !self.options.offline
        {
            self.load_view_support(false, window, cx);
        }
        if panel == Panel::Columns { self.error = None; }
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
            Panel::SavePrivate | Panel::SaveTeam | Panel::RenameView | Panel::AddColumn | Panel::EditColumn
        ) {
            self.views
                .input
                .update(cx, |input, cx| { input.configure("View name", false, cx); input.set_content(String::new(), cx); });
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
                if route.starts_with("smart-split-") {
                    self.views.input.update(cx, |input, cx| input.set_read_only(true, cx));
                    self.views.prompt.update(cx, |input, cx| input.set_read_only(true, cx));
                }
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
        self.views.input.update(cx, |input, cx| input.set_read_only(false, cx));
        self.views.prompt.update(cx, |input, cx| input.set_read_only(false, cx));
        self.reload();
        if self.views.pending_route.starts_with("column-") {
            return self.poll_column_result(&result,window,cx);
        }
        if result["error"].is_null() {
            self.views.previous_builtin = None;
            if self.views.pending_route.starts_with("smart-split-") {
                self.views.loaded = 0;
                self.load_board_views(cx);
            }
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
                        | Panel::Manage
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
        if self.views.pending.is_none() {
            if let Some((route,body))=self.views.column_deferred.pop_front() {self.queue_column(&route,body,window,cx);}
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
                    self.ordered_board_views(&project)
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
                    let mut views = self.ordered_board_views(&project);
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
                if value["id"].as_str().is_some_and(|id| id.starts_with("builtin:")) { return; }
                let project = self.db.board_project(self.options.board).unwrap_or(Value::Null);
                let cached_labels = self.db.project_labels(self.options.board).unwrap_or_default();
                let Some(labels) = project["desktop_smart_labels"].as_array().or_else(|| self.options.offline.then_some(&cached_labels)) else {
                    self.error = Some(project["desktop_smart_labels_error"].as_str().unwrap_or("Smart split details are still loading").into());
                    cx.notify(); return;
                };
                let views = project["project_view"]["allViews"].as_array().cloned().unwrap_or_default();
                if ht_core::smart_splits::label(&value, labels, &views).is_some() {
                    self.open_smart_split(Some(value), window, cx); return;
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
                    if kind == "AssignedToMe" {
                        self.toggle_self_filter(window, cx);
                        return;
                    }
                    let mut filters = settings["board_filters"]["addedFilters"]
                        .as_array()
                        .cloned()
                        .unwrap_or_default();
                    if filters.iter().any(|filter| filter["type"] == kind) {
                        filters.retain(|filter| filter["type"] != kind);
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
    fn toggle_self_filter(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(user) = self.db.current_user_id() else {
            return;
        };
        let project = self
            .db
            .board_project(self.options.board)
            .unwrap_or(Value::Null);
        let people = self.views.people.as_ref().unwrap_or(&project);
        let person = std::iter::once(&people["owner"])
            .chain(
                people["members"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|member| &member["user"]),
            )
            .find(|person| person["id"] == user && person["uid"].is_string())
            .cloned();
        let Some(person) = person else {
            if self.views.people.is_none() && !self.options.offline {
                self.views.self_filter = true;
                self.load_view_support(false, window, cx);
            } else {
                self.error = Some("Could not load your board profile".into());
                cx.notify();
            }
            return;
        };
        let mut settings = self.view_settings();
        let mut filters = settings["board_filters"]["addedFilters"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        if let Some(filter) = filters
            .iter_mut()
            .find(|filter| filter["type"] == "Assignees")
        {
            let mut values = filter["searchPayload"]
                .as_array()
                .cloned()
                .unwrap_or_default();
            if values
                .iter()
                .any(|person| person["id"] == user && person.get("uid").is_some())
            {
                values.retain(|person| !(person["id"] == user && person.get("uid").is_some()));
            } else {
                values.push(person);
            }
            filter["searchPayload"] = json!(values);
            filters.retain(|filter| {
                filter["type"] != "Assignees"
                    || filter["searchPayload"]
                        .as_array()
                        .is_some_and(|values| !values.is_empty())
            });
        } else {
            filters.push(json!({"type":"Assignees","searchPayload":[person],"match":"ANY"}));
        }
        settings["board_filters"]["addedFilters"] = json!(filters);
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
        let agents = matches!(self.views.panel, Some(Panel::Assignees | Panel::UpdatedBy | Panel::ColumnAssignees));
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
                                        if std::mem::take(&mut this.views.self_filter) {
                                            this.toggle_self_filter(window, cx);
                                        }
                                    }
                                }
                                Err(error) => {
                                    this.error = Some(error);
                                    this.views.people = None;
                                    this.views.self_filter = false;
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
        if matches!(self.views.panel, Some(Panel::Columns | Panel::AddColumn | Panel::EditColumn | Panel::DeleteColumn | Panel::ColumnAssignees)) {
            return self.column_panel_key(event, window, cx);
        }
        if matches!(self.views.panel, Some(Panel::SmartSplit | Panel::DeleteSmartSplit)) {
            return self.smart_split_key(event, window, cx);
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
    fn ordered_board_views(&self, project: &Value) -> Vec<Value> {
        let mut views = ht_core::board_views::ordered_views(project);
        views.extend(ht_core::board_views::builtin_views(project, self.views.decisions));
        let order = project["project_view"]["user_project_views"][0]["view_order"].as_array().or_else(|| project["project_view"]["default_view_order"].as_array());
        views.sort_by_key(|view| if view["id"] == project["project_view"]["default_view_id"] { 0 } else { 1 + order.and_then(|order| order.iter().position(|id| *id == view["id"])).unwrap_or(usize::MAX - 1) });
        views
    }
    fn hidden_view_tabs(&self) -> Value {
        self.db.kv_get("hiddenViewTabIds").ok().flatten().and_then(|value| serde_json::from_str(&value).ok()).filter(Value::is_object).unwrap_or(json!({}))
    }
    fn show_empty_view_tabs(&self) -> bool { self.db.kv_get("showEmptyViewTabs").ok().flatten().as_deref() == Some("true") }
    fn view_tab_key(&self, id: &str) -> String { if id.starts_with("builtin:") { format!("{}:{id}", self.options.board) } else { id.into() } }
    fn toggle_view_tab(&mut self, id: &str, cx: &mut Context<Self>) {
        let mut hidden = self.hidden_view_tabs();
        let key = self.view_tab_key(id);
        hidden[&key] = json!(hidden[&key] != true);
        if let Err(error) = self.db.kv_set("hiddenViewTabIds", &hidden.to_string()) { self.error = Some(error.to_string()); }
        cx.notify();
    }
    fn rendered_board_views(&self, project: &Value) -> Vec<Value> {
        let hidden = self.hidden_view_tabs();
        let active = self.views.builtin.get(&self.options.board).map(|id| json!(id)).unwrap_or_else(|| project["project_view"]["user_project_views"][0]["appliedView"]["id"].as_str().map(|id| json!(id)).unwrap_or_else(|| project["project_view"]["default_view_id"].clone()));
        self.ordered_board_views(project).into_iter().filter(|view| {
            view["id"] == active || view["id"] == project["project_view"]["default_view_id"] ||
            (hidden[self.view_tab_key(view["id"].as_str().unwrap_or(""))] != true && (self.show_empty_view_tabs() || self.db.view_task_count(self.options.board, view).unwrap_or(0) > 0))
        }).collect()
    }
    fn reorder_view_tab(&mut self, moved: &str, before: &str, window: &mut Window, cx: &mut Context<Self>) {
        let project = self.db.board_project(self.options.board).unwrap_or(Value::Null);
        if project["project_view"]["default_view_id"] == moved || project["project_view"]["default_view_id"] == before || moved == before { return; }
        let mut views = self.ordered_board_views(&project);
        let Some(source) = views.iter().position(|view| view["id"] == moved) else { return; };
        let Some(destination) = views.iter().position(|view| view["id"] == before) else { return; };
        let view = views.remove(source);
        views.insert(destination, view);
        self.queue_view("update-order", json!({"viewOrder":views.iter().map(|view| &view["id"]).collect::<Vec<_>>()}), window, cx);
    }
    pub(super) fn board_view_tabs(&self, cx: &mut Context<Self>) -> AnyElement {
        let project = self.db.board_project(self.options.board).unwrap_or(Value::Null);
        if !project["project_view"].is_object() { return div().into_any_element(); }
        div().px(px(24.)).flex().gap(px(12.))
            .children(self.rendered_board_views(&project).into_iter().enumerate().map(|(i, view)| {
                let id = view["id"].clone();
                let title = if id == project["project_view"]["default_view_id"] { project["title"].as_str().unwrap_or("") } else { view["title"].as_str().unwrap_or("") }.to_owned();
                div().id(("saved-view", i)).cursor_pointer().child(title.clone())
                    .on_click(cx.listener(move |this, _, window, cx| this.select_board_view(id.clone(), window, cx)))
                    .layout_key(&self.layout, format!("board.view.tab.{i}"), Some(title))
            }))
            .child(div().id("save-current-view").cursor_pointer().child("Save view").on_click(cx.listener(|this, _, window, cx| this.open_view_panel(Panel::Save, window, cx))))
            .child(div().id("manage-board-views").cursor_pointer().child("Manage views").on_click(cx.listener(|this, _, window, cx| this.open_view_panel(Panel::Manage, window, cx))))
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
        if matches!(panel, Panel::Columns | Panel::AddColumn | Panel::EditColumn | Panel::DeleteColumn | Panel::ColumnAssignees) { return self.column_overlay(cx); }
        if matches!(panel, Panel::SmartSplit | Panel::DeleteSmartSplit) { return self.smart_split_overlay(cx); }
        let t = self.colors;
        let title = match panel {
            Panel::Sort => "Sort",
            Panel::Columns | Panel::AddColumn | Panel::EditColumn | Panel::DeleteColumn | Panel::ColumnAssignees => unreachable!(),
            Panel::Subtasks => "Sub-task Settings",
            Panel::Date("DueDate") | Panel::DateRange("DueDate") => "Filter by due date",
            Panel::Date("CreatedAt") | Panel::DateRange("CreatedAt") => "Filter by created date",
            Panel::Date(_) | Panel::DateRange(_) => "Filter by last updated date",
            Panel::Save => "Save this view",
            Panel::SaveTeam => "Save as new view for team",
            Panel::SavePrivate => "Save as view for me only",
            Panel::Manage => "Manage views",
            Panel::RenameView => "View settings",
            Panel::SmartSplit | Panel::DeleteSmartSplit => unreachable!(),
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
                    .when(panel == Panel::Manage, |d| d.child(div().id("show-empty-view-tabs").p(px(8.)).cursor_pointer().child(format!("Show views with 0 tasks    {}", if self.show_empty_view_tabs() { "✓" } else { "" })).on_click(cx.listener(|this, _, _, cx| {
                        if let Err(error) = this.db.kv_set("showEmptyViewTabs", if this.show_empty_view_tabs() { "false" } else { "true" }) { this.error = Some(error.to_string()); }
                        cx.notify();
                    }))))
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
                            .map(|(i, (name, value))| {
                                let view_id = value["id"].as_str().unwrap_or("").to_owned();
                                let is_view = panel == Panel::Manage && !view_id.is_empty();
                                let is_default = is_view && self.db.board_project(self.options.board).is_ok_and(|project| project["project_view"]["default_view_id"] == view_id);
                                let hidden = self.hidden_view_tabs()[self.view_tab_key(&view_id)] == true;
                                let builtin = view_id.starts_with("builtin:");
                                div()
                                    .id(("view-option", i))
                                    .p(px(8.))
                                    .cursor_pointer()
                                    .when(i == self.views.choice, |d| d.bg(color(t.active)))
                                    .flex().gap(px(8.))
                                    .when(is_view && !is_default, |d| d.on_drag(ViewDrag { id: view_id.clone(), title: name.clone() }, |drag, _, _, cx| cx.new(|_| drag.clone())).on_drop(cx.listener({ let id = view_id.clone(); move |this, drag: &ViewDrag, window, cx| this.reorder_view_tab(&drag.id, &id, window, cx) })))
                                    .child(name)
                                    .when(!builtin, |d| d.on_click(cx.listener(move |this, _, window, cx| this.choose_view_option(i, window, cx))))
                                    .when(is_view, |d| d.child(div().text_color(color(t.muted)).child(self.db.view_task_count(self.options.board, &value).unwrap_or(0).to_string())))
                                    .when(is_view && !is_default, |d| d.child(div().id(("view-tab-visibility", i)).cursor_pointer().child(if hidden { "Show" } else { "Hide" }).on_click(cx.listener(move |this, _, _, cx| { cx.stop_propagation(); this.toggle_view_tab(&view_id, cx); }))))
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
    #[gpui::test]
    fn view_visibility_and_zero_task_tabs_are_local_persistent_and_keep_default_and_active(cx: &mut gpui::TestAppContext) {
        cx.update(crate::input::init);
        let (view, cx) = cx.add_window_view(|window, cx| desktop(window, cx));
        view.update_in(cx, |this, window, cx| {
            this.db.apply_response(&json!({"success":true,"projects":[{"id":15,"title":"Board","project_view":{"id":"pv","projectId":15,"default_view_id":"default","allViews":[{"id":"default","title":"Default"},{"id":"nonempty","title":"All"},{"id":"empty","title":"Nobody","board_filters":{"addedFilters":[{"type":"Priority","searchPayload":[{"priority_index":1}]}]}}],"user_project_views":[{}]}}]})).unwrap();
            this.switch(Screen::Board, cx);
            let project = this.db.board_project(15).unwrap();
            let contains = |views: Vec<Value>, id: &str| views.iter().any(|view| view["id"] == id);
            assert!(!contains(this.rendered_board_views(&project), "empty"));
            assert!(contains(this.rendered_board_views(&project), "default"));
            this.db.kv_set("showEmptyViewTabs", "true").unwrap();
            assert!(contains(this.rendered_board_views(&project), "empty"));
            this.toggle_view_tab("empty", cx);
            assert_eq!(this.hidden_view_tabs()["empty"], true);
            assert!(!contains(this.rendered_board_views(&project), "empty"));
            this.toggle_view_tab("default", cx);
            assert!(contains(this.rendered_board_views(&project), "default"));
            this.toggle_view_tab("builtin:my-tasks", cx);
            assert_eq!(this.hidden_view_tabs()["15:builtin:my-tasks"], true);
            this.select_board_view(json!("builtin:my-tasks"), window, cx);
            assert!(contains(this.rendered_board_views(&project), "builtin:my-tasks"));
            this.options.board = 16;
            assert_ne!(this.hidden_view_tabs()[this.view_tab_key("builtin:my-tasks")], true);
            assert!(this.db.mutations().unwrap().is_empty());
        });
    }
    #[gpui::test]
    fn dragging_view_order_keeps_default_fixed_includes_builtins_and_rolls_back_both_refusals(cx: &mut gpui::TestAppContext) {
        cx.update(crate::input::init);
        let (view, cx) = cx.add_window_view(|window, cx| desktop(window, cx));
        for status in [401, 403, 200] {
            view.update_in(cx, |this, window, cx| {
                this.db.apply_response(&json!({"success":true,"projects":[{"id":15,"title":"Board","project_view":{"id":"pv","projectId":15,"default_view_id":"default","allViews":[{"id":"default","title":"Default"},{"id":"first","title":"First"},{"id":"last","title":"Last"}],"user_project_views":[{}]}}]})).unwrap();
                this.open_view_panel(Panel::Manage, window, cx);
                let before = this.db.board_project(15).unwrap();
                this.reorder_view_tab("default", "last", window, cx);
                assert!(this.views.pending.is_none());
                this.reorder_view_tab("last", "first", window, cx);
                let row = this.db.mutations().unwrap().into_iter().find(|row| row.status != "failed").unwrap();
                let change: ht_core::ViewWrite = serde_json::from_value(row.payload_json["change"].clone()).unwrap();
                let order = change.body["viewOrder"].as_array().unwrap();
                assert_eq!(&order[..3], &[json!("default"), json!("last"), json!("first")]);
                assert!(order.contains(&json!("builtin:my-tasks")));
                assert_eq!(this.db.board_project(15).unwrap()["project_view"]["user_project_views"][0]["view_order"], json!(order));
                this.db.drain_once(&mut Web { status, reply:json!({"viewOrder":order}), expected:change.clone() }, chrono::Utc::now().timestamp()+1).unwrap();
                assert!(!this.poll_view(window, cx));
                assert_eq!(this.views.panel, Some(Panel::Manage));
                if status != 200 {
                    assert_eq!(this.db.board_project(15).unwrap()["project_view"], before["project_view"]);
                    assert_eq!(this.error.as_deref(), Some("Unable to save view order"));
                }
            });
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
    fn archived_command_queues_full_unsaved_view_and_refusal_restores_the_previous_override(cx: &mut gpui::TestAppContext) {
        cx.update(crate::input::init);
        let (view, cx) = cx.add_window_view(|window, cx| desktop(window, cx));
        for status in [401, 403, 200] {
            view.update_in(cx, |this, window, cx| {
                this.db.apply_response(&json!({"success":true,"projects":[{"id":15,"title":"Board","project_view":{"id":"pv","projectId":15,"default_view":{"id":"default","board_show_archived":false},"user_project_views":[{}]}}]})).unwrap();
                this.db.apply_archived_board_tasks(42, 15, &[json!({"id":4,"projectId":15,"sectionId":10,"title":"Archived","status":"Archive","archivedAt":"2026-01-01T00:00:00Z"})]).unwrap();
                this.switch(Screen::Board, cx);
                let before = this.view_settings();
                this.run_shell_command(crate::command_center::ShellCommand::ArchivedOnBoard, window, cx);
                assert!(this.show_archived_on_board());
                assert!(this.columns.iter().flat_map(|column| &column.cards).any(|task| task.id == 4));
                let row = this.db.mutations().unwrap().into_iter().find(|row| row.status != "failed").unwrap();
                assert_eq!(row.payload_json["change"]["body"]["board_show_archived"], true);
                settle(this, status, window, cx);
                assert_eq!(this.show_archived_on_board(), status == 200);
                if status != 200 { assert_eq!(this.view_settings(), before); }
            });
        }
    }
    #[gpui::test]
    fn assigned_to_me_uses_real_profile_and_toggles_only_self_preserving_other_values_and_match(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(crate::input::init);
        let (view, cx) = cx.add_window_view(|window, cx| desktop(window, cx));
        for status in [401, 403, 200] {
            view.update_in(cx,|this,window,cx|{
            this.db.apply_response(&json!({"success":true,"projects":[{"id":15,"title":"Board","project_view":{"id":"pv","projectId":15,"user_project_views":[{"unsavedView":{"board_filters":{"addedFilters":[{"type":"Assignees","searchPayload":[{"id":7,"uid":"other-real-uid"}],"match":"ALL"}],"matchFilters":"ANY"}}}]}}]})).unwrap();this.views.people=Some(json!({"owner":{"id":42,"uid":"actual-owner-uid","displayName":"Owner"},"members":[]}));this.open_view_panel(Panel::Filters,window,cx);let before=this.view_settings();let index=this.view_choices().iter().position(|(_,value)|*value=="AssignedToMe").unwrap();this.choose_view_option(index,window,cx);
            let filter=this.view_settings()["board_filters"]["addedFilters"][0].clone();assert_eq!(filter["match"],"ALL");assert_eq!(filter["searchPayload"][0]["id"],7);assert_eq!(filter["searchPayload"][1]["uid"],"actual-owner-uid");settle(this,status,window,cx);if status!=200{assert_eq!(this.view_settings(),before);}else{this.toggle_self_filter(window,cx);settle(this,200,window,cx);assert_eq!(this.view_settings(),before);}
        });
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
            for mode in ["Priority","Title","DueDate"]{let index=this.view_choices().iter().position(|(_,value)|*value==mode).unwrap();this.choose_view_option(index,window,cx);settle(this,200,window,cx);assert_eq!(this.views.panel,Some(Panel::Sort));}
            assert_eq!(this.sort_levels().len(),3);assert_eq!(this.sort_levels()[1]["mode"],"Title");assert_eq!(this.sort_levels()[1]["order"],"Ascending");
            assert_eq!(this.view_choices(),vec![("Manual".into(),json!("Manual"))]);assert!(this.views.pending.is_none());assert_eq!(this.sort_levels().len(),3);
            let mut levels=this.sort_levels();levels.remove(0);this.save_sort_levels(levels,window,cx);settle(this,200,window,cx);assert_eq!(this.view_settings()["board_sorting_mode"],"Title");
        });
    }
}

impl Desktop {
    pub(super) fn open_smart_split(&mut self, view: Option<Value>, window: &mut Window, cx: &mut Context<Self>) {
        if self.views.pending.is_some() || !self.board_metadata_ready(cx) { return; }
        let project = self.db.board_project(self.options.board).unwrap_or(Value::Null);
        let prompt = if let Some(view) = &view {
            let labels = project["desktop_smart_labels"].as_array().cloned().unwrap_or_default();
            let views = project["project_view"]["allViews"].as_array().cloned().unwrap_or_default();
            let Some(label) = ht_core::smart_splits::label(view, &labels, &views) else { return; };
            label["ai_prompt"].as_str().unwrap_or("").to_owned()
        } else { String::new() };
        self.error = None;
        self.views.editing = view.as_ref().and_then(|view| view["id"].as_str().map(str::to_owned));
        self.views.panel = Some(Panel::SmartSplit);
        self.views.input.update(cx, |input, cx| { input.configure("Name this split", false, cx); input.set_read_only(false, cx); input.set_content(view.as_ref().and_then(|view| view["title"].as_str()).unwrap_or("").into(), cx); });
        self.views.prompt.update(cx, |input, cx| { input.set_read_only(false, cx); input.set_content(prompt, cx); });
        window.focus(&self.views.input.focus_handle(cx), cx);
        cx.notify();
    }
    fn save_smart_split(&mut self, delete: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.views.pending.is_some() { return; }
        let mut body = json!({"projectId":self.options.board});
        if let Some(id) = &self.views.editing { body["viewId"] = json!(id); }
        let route = if delete { "smart-split-delete" } else {
            body["name"] = json!(self.views.input.read(cx).content.trim());
            let prompt = self.views.prompt.read(cx).content.to_string();
            if prompt.encode_utf16().count() > 1000 { self.error = Some("Keep the prompt to 1,000 characters or fewer".into()); cx.notify(); return; }
            body["prompt"] = json!(prompt.trim());
            if self.views.editing.is_some() { "smart-split-edit" } else { "smart-split-create" }
        };
        self.error = None;
        if delete { self.views.panel = Some(Panel::SmartSplit); }
        self.queue_view(route, body, window, cx);
    }
    fn close_smart_split(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.views.pending.is_some() { return; }
        if self.views.panel == Some(Panel::DeleteSmartSplit) { self.views.panel = Some(Panel::SmartSplit); }
        else if self.views.editing.is_some() { self.open_view_panel(Panel::Manage, window, cx); }
        else { self.views.panel = None; window.focus(&self.focus, cx); }
        cx.notify();
    }
    fn smart_split_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.views.pending.is_some() { cx.stop_propagation(); return true; }
        match event.keystroke.key.as_str() {
            "escape" => self.close_smart_split(window, cx),
            "enter" if self.views.panel == Some(Panel::DeleteSmartSplit) => self.save_smart_split(true, window, cx),
            "enter" if event.keystroke.modifiers.control || event.keystroke.modifiers.platform || self.views.input.focus_handle(cx).is_focused(window) => self.save_smart_split(false, window, cx),
            "tab" => { if self.views.input.focus_handle(cx).is_focused(window) { window.focus(&self.views.prompt.focus_handle(cx), cx); } else { window.focus(&self.views.input.focus_handle(cx), cx); } },
            _ => return true,
        }
        cx.stop_propagation(); true
    }
    fn smart_split_overlay(&self, cx: &mut Context<Self>) -> AnyElement {
        let editing = self.views.editing.is_some();
        let pending = self.views.pending.is_some();
        let count = self.views.prompt.read(cx).content.encode_utf16().count();
        let can_save = self.views.panel == Some(Panel::SmartSplit) && !pending && !self.views.input.read(cx).content.trim().is_empty() && !self.views.prompt.read(cx).content.trim().is_empty() && count <= 1000;
        let mut content = div().flex().flex_col().gap(px(12.))
            .child(div().flex().justify_between().child(if editing { "Edit smart split" } else { "Add smart split" }).child(div().id("smart-split-close").cursor_pointer().child(if editing { "←" } else { "×" }).on_click(cx.listener(|this, _, window, cx| this.close_smart_split(window, cx))).layout_key(&self.layout,"board.smart-split.close",None)))
            .child("Name").child(self.views.input.clone())
            .child("Matching prompt").child(div().min_h(px(96.)).child(self.views.prompt.clone()))
            .child(div().flex().justify_between().text_color(color(self.colors.muted)).child("AI keeps the split updated as tasks change.").child(format!("{count}/1000")))
            .child(div().flex().justify_between()
                .when(editing, |d| d.child(div().id("smart-split-delete").cursor_pointer().child("Delete smart split").on_click(cx.listener(|this, _, _, cx| { if this.views.pending.is_none() { this.views.panel = Some(Panel::DeleteSmartSplit); cx.notify(); } })).layout_key(&self.layout,"board.smart-split.delete",None)))
                .child(div().id("smart-split-save").cursor_pointer().when(!can_save, |d| d.opacity(0.5)).child(if pending { "Saving…" } else if editing { "Save   CTRL+ENTER" } else { "Create   CTRL+ENTER" }).on_click(cx.listener(move |this, _, window, cx| { if can_save { this.save_smart_split(false, window, cx); } })).layout_key(&self.layout,"board.smart-split.save",None)))
            .when_some(self.error.clone(), |d, error| d.child(div().text_color(color(0xcf222e)).child(error)));
        if self.views.panel == Some(Panel::DeleteSmartSplit) {
            content = content.child(div().p(px(12.)).flex().flex_col().gap(px(8.))
                .child(format!("Delete {}? Its hidden smart tag is also removed.",self.views.input.read(cx).content))
                .child(div().id("smart-split-confirm-delete").cursor_pointer().child("Delete smart split").on_click(cx.listener(|this, _, window, cx| this.save_smart_split(true, window, cx))).layout_key(&self.layout,"board.smart-split.confirm-delete",None))
                .child(div().id("smart-split-cancel-delete").cursor_pointer().child("Cancel").on_click(cx.listener(|this, _, window, cx| this.close_smart_split(window, cx))).layout_key(&self.layout,"board.smart-split.cancel-delete",None)));
        }
        div().absolute().inset_0().bg(rgba(0x00000040)).flex().items_center().justify_center()
            .on_mouse_down(gpui::MouseButton::Left,cx.listener(|this, _, window, cx| this.close_smart_split(window,cx)))
            .child(div().id("smart-split-dialog").w(px(560.)).p(px(24.)).bg(color(self.colors.comment)).rounded(px(6.))
                .on_mouse_down(gpui::MouseButton::Left,|_,_,cx|cx.stop_propagation()).child(content).layout_key(&self.layout,"board.smart-split.dialog",None)).into_any_element()
    }
}

#[cfg(test)]
mod smart_split_tests {
    use super::*;
    use crate::ui::{task_fields::tests::desktop, shell::tests::{click, draw, measured}};
    struct Server(u16, &'static str, Value);
    impl ht_core::Transport for Server {
        fn get(&mut self,path:&str)->Result<ht_core::http::Response,ht_core::http::TransportError> { assert_eq!(path,"/user/context"); Ok(ht_core::http::Response {status:200,body:json!({"success":true,"user":{"id":42}})}) }
        fn post(&mut self,path:&str,body:&Value,key:&str)->Result<ht_core::http::Response,ht_core::http::TransportError> { self.mutate("POST",path,body,key) }
        fn mutate(&mut self,method:&str,path:&str,body:&Value,key:&str)->Result<ht_core::http::Response,ht_core::http::TransportError> {
            assert_eq!(method,self.1); assert_eq!(path,"/api/projects/views/smart-split"); assert_eq!(body["projectId"],15); assert!(!key.is_empty());
            Ok(ht_core::http::Response {status:self.0,body:if self.0<300 {self.2.clone()} else {json!({"message":"Sign in to manage smart splits"})}})
        }
    }
    fn seed(this:&mut Desktop) {
        this.db.apply_response(&json!({"success":true,"projects":[{"id":15,"title":"Board","desktop_smart_labels":[{"id":"smart","projectId":15,"value":"Bugs","ai_prompt":"Broken tasks"}],"project_view":{"id":"pv","projectId":15,"default_view":{"id":"default"},"default_view_id":"default","allViews":[{"id":"smart","title":"Bugs","board_filters":{"addedFilters":[{"type":"Labels","searchPayload":[{"id":"smart","value":"Bugs"}]}]}}],"user_project_views":[{}]}}]})).unwrap(); this.screen=Screen::Board; this.reload();
    }
    #[gpui::test]
    fn smart_split_palette_form_uses_real_name_prompt_focus_and_preserves_refused_edits(cx:&mut gpui::TestAppContext) {
        cx.update(crate::input::init);
        let view=cx.open_window(gpui::size(px(1440.),px(900.)),|window,cx|{let mut this=desktop(window,cx);seed(&mut this);this});
        let mut cx=gpui::VisualTestContext::from_window(view.into(),cx); draw(view,&mut cx);
        for status in [401,403,201] {
            cx.simulate_keystrokes("ctrl-k"); cx.simulate_input("Add smart split"); cx.simulate_keystrokes("enter"); draw(view,&mut cx); measured(view,&mut cx,"board.smart-split.dialog");
            cx.simulate_input("Urgent bugs"); cx.simulate_keystrokes("tab"); cx.simulate_input("Broken urgent tasks"); cx.simulate_keystrokes("ctrl-enter ctrl-enter escape");
            view.update(&mut cx,|this,window,cx|{
                let row=this.db.mutations().unwrap().into_iter().find(|row|row.status!="failed").unwrap();
                assert_eq!(row.payload_json["change"]["body"],json!({"projectId":15,"name":"Urgent bugs","prompt":"Broken urgent tasks"}));
                assert_eq!(this.views.panel,Some(Panel::SmartSplit)); assert!(this.views.pending.is_some());
                let reply=json!({"view":{"id":"new","title":"Urgent bugs","slug":"urgent-bugs","project_view_id":"pv"},"label":{"id":"new","projectId":15,"value":"Urgent bugs","ai_prompt":"Broken urgent tasks"}});
                this.db.drain_once(&mut Server(status,"POST",reply),chrono::Utc::now().timestamp()+1).unwrap(); assert!(!this.poll_view(window,cx));
                if status>=300 { assert_eq!(this.views.panel,Some(Panel::SmartSplit)); assert_eq!(this.views.input.read(cx).content.as_ref(),"Urgent bugs"); assert_eq!(this.views.prompt.read(cx).content.as_ref(),"Broken urgent tasks"); assert_eq!(this.error.as_deref(),Some("Sign in to manage smart splits")); }
                else { assert!(this.views.panel.is_none()); assert_eq!(this.db.board_project(15).unwrap()["project_view"]["allViews"][1]["id"],"new"); }
            }).unwrap();
            if status>=300 { cx.simulate_keystrokes("escape"); }
        }
    }
    #[gpui::test]
    fn smart_split_manage_edit_and_confirmed_delete_keep_paired_tag_and_rollback_refusals(cx:&mut gpui::TestAppContext) {
        cx.update(crate::input::init);
        let view=cx.open_window(gpui::size(px(1440.),px(900.)),|window,cx|{let mut this=desktop(window,cx);seed(&mut this);this});
        let mut cx=gpui::VisualTestContext::from_window(view.into(),cx); draw(view,&mut cx);
        for delete in [false,true] { for status in [401,403,200] {
            view.update(&mut cx,|this,window,cx|{seed(this);this.open_view_panel(Panel::Manage,window,cx);let index=this.view_choices().iter().position(|(_,view)|view["id"]=="smart").unwrap();this.choose_view_option(index,window,cx);assert_eq!(this.views.panel,Some(Panel::SmartSplit));assert_eq!(this.views.prompt.read(cx).content.as_ref(),"Broken tasks");}).unwrap(); draw(view,&mut cx);
            if delete {
                click(view,&mut cx,"board.smart-split.delete"); click(view,&mut cx,"board.smart-split.cancel-delete");
                view.read_with(&cx,|this,_|assert!(!this.db.mutations().unwrap().iter().any(|row|row.status!="failed"))).unwrap();
                click(view,&mut cx,"board.smart-split.delete"); click(view,&mut cx,"board.smart-split.confirm-delete");
            } else { cx.simulate_keystrokes("ctrl-a"); cx.simulate_input("Urgent bugs"); click(view,&mut cx,"board.smart-split.save"); }
            view.update(&mut cx,|this,window,cx|{
                let row=this.db.mutations().unwrap().into_iter().find(|row|row.status!="failed").unwrap(); assert_eq!(row.payload_json["change"]["body"]["viewId"],"smart");
                this.db.drain_once(&mut Server(status,if delete {"DELETE"} else {"PATCH"},json!({"success":true,"slug":"urgent-bugs"})),chrono::Utc::now().timestamp()+1).unwrap();this.poll_view(window,cx);
                let project=this.db.board_project(15).unwrap();
                if status>=300 {assert_eq!(project["project_view"]["allViews"][0]["title"],"Bugs");assert_eq!(project["desktop_smart_labels"][0]["ai_prompt"],"Broken tasks");assert_eq!(this.views.panel,Some(Panel::SmartSplit));}
                else if delete {assert_eq!(project["project_view"]["allViews"],json!([]));assert_eq!(project["desktop_smart_labels"],json!([]));}
                else {assert_eq!(project["project_view"]["allViews"][0]["title"],"Urgent bugs");assert_eq!(project["desktop_smart_labels"][0]["value"],"Urgent bugs");}
                this.views.panel=None;
            }).unwrap();
        }}
    }
}

#[derive(Clone)]
struct ColumnDrag { id: i64, title: String }
impl Render for ColumnDrag {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement { div().p(px(8.)).bg(color(0x30343b)).text_color(color(0xffffff)).child(self.title.clone()) }
}
impl Desktop {
    fn managed_columns(&self) -> Vec<Value> {
        let project=self.db.board_project(self.options.board).unwrap_or(Value::Null);
        let settings=ht_core::board_views::settings(&project);
        project["section"].as_array().into_iter().flatten().map(|section| {
            let mut section=section.clone();
            section["visibility"]=json!(settings["board_columns_view"].as_array().into_iter().flatten().any(|column|column.get("id").or_else(||column.get("sectionId"))==section.get("id") && column["visibility"]==true)); section
        }).collect()
    }
    fn edit_column(&mut self, id:i64, window:&mut Window, cx:&mut Context<Self>) {
        if id<=0 {return;}
        let Some(section)=self.managed_columns().into_iter().find(|section|section["id"]==id) else{return;};
        self.views.column_title_blur=None;
        self.views.column_edit=Some(id);
        if self.views.column_auto_select {
            self.views.column_auto_select=false;
            self.open_view_panel(Panel::ColumnAssignees,window,cx);
            window.focus(&self.views.query.focus_handle(cx),cx);
        } else {
            self.open_view_panel(Panel::EditColumn,window,cx);
            self.views.input.update(cx,|input,cx|{input.configure("Enter column name",false,cx);input.set_content(section["section_title"].as_str().unwrap_or("").into(),cx);});
            let handle=self.views.input.focus_handle(cx);
            self.views.column_title_blur=Some(cx.on_blur(&handle,window,|this,window,cx| {
                if this.views.panel==Some(Panel::EditColumn) {this.save_column_title(window,cx);}
            }));
        }
        cx.notify();
    }
    fn queue_column(&mut self, route:&str, mut body:Value, window:&mut Window, cx:&mut Context<Self>) {
        if self.views.panel==Some(Panel::EditColumn) && !(route=="column-update" && body["newSection"]["section_title"].is_string()) {
            self.save_column_title(window,cx);
        }
        body["projectId"]=json!(self.options.board);
        if route=="column-update" {body["userId"]=json!(self.db.current_user_id());}
        if self.views.pending.is_some() {self.views.column_deferred.push_back((route.into(),body));cx.notify();return;}
        self.error=None;
        self.views.column_error=match route {
            "column-create"=>"Error creating column",
            "column-visibility"=>if body["visible"]==true {"Column could not be shown in all views"} else {"Column could not be hidden in all views"},
            "column-autoassign"=>"Unable to update column auto-assign",
            _ if body["newSection"]["deleted"]==true=>"Column could not be deleted",
            _ if body["newSection"]["ranking"].is_string()=>"Column order could not be saved",
            _=>"",
        }.into();
        self.queue_view(route,body,window,cx);
    }
    fn save_column_title(&mut self, window:&mut Window, cx:&mut Context<Self>) {
        let Some(id)=self.views.column_edit else{return;};
        let title=self.views.input.read(cx).content.trim().to_owned();
        if title.is_empty() || self.managed_columns().iter().any(|section|section["id"]==id && section["section_title"]==title) {return;}
        self.queue_column("column-update",json!({"sectionId":id,"newSection":{"section_title":title}}),window,cx);
    }
    fn create_column_form(&mut self, window:&mut Window, cx:&mut Context<Self>) {
        if self.views.pending.is_some() {return;}
        let title=self.views.input.read(cx).content.trim().to_owned();
        if !title.is_empty() {self.queue_column("column-create",json!({"title":title}),window,cx);}
    }
    fn reorder_column(&mut self, source:i64, destination:i64, window:&mut Window, cx:&mut Context<Self>) {
        if source==destination || self.views.pending.is_some() {return;}
        let mut columns=self.managed_columns();
        let Some(from)=columns.iter().position(|column|column["id"]==source) else{return;};
        let Some(to)=columns.iter().position(|column|column["id"]==destination) else{return;};
        columns.remove(from);
        let previous=to.checked_sub(1).and_then(|i|columns.get(i)).and_then(|column|column["ranking"].as_str());
        let next=columns.get(to).and_then(|column|column["ranking"].as_str());
        match ht_core::task_rank(previous,next) {
            Ok(rank)=>{self.views.column_reorder=true;self.queue_column("column-update",json!({"sectionId":source,"newSection":{"ranking":rank}}),window,cx);}
            Err(_)=>{self.error=Some("Column order could not be saved".into());cx.notify();}
        }
    }
    fn poll_column_result(&mut self, result:&Value, window:&mut Window, cx:&mut Context<Self>) -> bool {
        let reorder=self.views.column_reorder; self.views.column_reorder=false;
        if let Some(error)=result["error"].as_str() {
            self.error=Some(if error.contains("last column") || self.views.column_error.is_empty() {error.into()} else {self.views.column_error.clone()});
        } else {
            if self.views.pending_route=="column-create" {self.views.panel=None;window.focus(&self.focus,cx);}
            if self.views.panel==Some(Panel::DeleteColumn) {self.open_view_panel(Panel::Columns,window,cx);}
            if self.views.panel==Some(Panel::ColumnAssignees) {if let Some(id)=self.views.column_edit {self.edit_column(id,window,cx);}}
            if reorder {
                let project=self.db.board_project(self.options.board).unwrap_or(Value::Null);
                self.queue_view("unsaved-view",ht_core::board_columns::reordered_settings(&project),window,cx);
            }
        }
        if self.views.pending.is_none() {
            if let Some((route,body))=self.views.column_deferred.pop_front() { self.queue_column(&route,body,window,cx); }
        }
        cx.notify();false
    }
    fn column_people(&self) -> Vec<Value> {
        let project=self.db.board_project(self.options.board).unwrap_or(Value::Null);
        let data=self.views.people.as_ref().unwrap_or(&project);
        let mut rows=vec![json!({"id":0,"displayName":"Nobody"})];
        rows.extend(std::iter::once(&data["owner"]).chain(data["members"].as_array().into_iter().flatten().map(|member|&member["user"])).filter(|person|person["id"].as_i64().is_some_and(|id|id>0)).cloned());
        rows.extend(data["boardAgents"].as_array().into_iter().flatten().filter(|agent|agent["revokedAt"].is_null()).cloned());
        let query=self.views.query_text.to_lowercase();
        rows.into_iter().filter(|person|person["displayName"].as_str().unwrap_or("").to_lowercase().contains(&query)).collect()
    }
    fn choose_column_person(&mut self,index:usize,window:&mut Window,cx:&mut Context<Self>) {
        let Some(section)=self.views.column_edit else{return;};
        let Some(person)=self.column_people().get(index).cloned() else{return;};
        let user=person["id"].as_i64().filter(|id|*id>0);
        let agent=person["id"].as_str();
        self.queue_column("column-autoassign",json!({"sectionId":section,"autoAssignUserId":user,"autoAssignAgentId":agent}),window,cx);
    }
    fn column_back(&mut self,window:&mut Window,cx:&mut Context<Self>) {
        match self.views.panel {
            Some(Panel::Columns | Panel::AddColumn)=>{self.views.panel=None;self.views.column_auto_select=false;window.focus(&self.focus,cx);}
            Some(Panel::EditColumn)=>{self.save_column_title(window,cx);self.open_view_panel(Panel::Columns,window,cx);}
            _=>{if let Some(id)=self.views.column_edit {self.edit_column(id,window,cx);}},
        }
        cx.notify();
    }
    fn column_panel_key(&mut self,event:&KeyDownEvent,window:&mut Window,cx:&mut Context<Self>)->bool {
        match event.keystroke.key.as_str() {
            "escape"=>self.column_back(window,cx),
            "enter"=>match self.views.panel {
                Some(Panel::AddColumn)=>self.create_column_form(window,cx),
                Some(Panel::EditColumn)=>{self.save_column_title(window,cx);self.open_view_panel(Panel::Columns,window,cx);},
                Some(Panel::DeleteColumn)=>{if let Some(id)=self.views.column_edit {self.queue_column("column-update",json!({"sectionId":id,"newSection":{"deleted":true}}),window,cx);}},
                Some(Panel::ColumnAssignees)=>self.choose_column_person(self.views.choice,window,cx),
                _=>{let columns:Vec<_>=self.managed_columns().into_iter().filter(|section|!self.views.column_auto_select || section["visibility"]==true).collect();if let Some(id)=columns.get(self.views.choice).and_then(|section|section["id"].as_i64()){self.edit_column(id,window,cx);}},
            },
            "up"=>self.views.choice=self.views.choice.saturating_sub(1),
            "down"=>{let count=if self.views.panel==Some(Panel::ColumnAssignees){self.column_people().len()}else{self.managed_columns().len()};self.views.choice=(self.views.choice+1).min(count.saturating_sub(1));},
            _=>return true,
        }
        cx.stop_propagation();cx.notify();true
    }
    fn column_overlay(&self,cx:&mut Context<Self>)->AnyElement {
        let panel=self.views.panel.unwrap();
        let mut content=div().flex().flex_col().gap(px(12.)).child(div().flex().justify_between()
            .child(match panel {Panel::Columns if self.views.column_auto_select=>"Auto-assign: choose a column",Panel::Columns=>"Manage board columns",Panel::AddColumn=>"Add Column",Panel::ColumnAssignees=>"Auto-assign",_=>"Edit board column"})
            .child(div().id("columns-back").cursor_pointer().child(if matches!(panel,Panel::Columns|Panel::AddColumn){"×"}else{"←"}).on_click(cx.listener(|this,_,window,cx|this.column_back(window,cx))).layout_key(&self.layout,"board.columns.back",None)));
        if panel==Panel::Columns && self.views.column_auto_select {
            for (index,section) in self.managed_columns().into_iter().enumerate().filter(|(_,section)|section["visibility"]==true) {
                let id=section["id"].as_i64().unwrap_or(0);
                content=content.child(div().id(("autoassign-column-choice",index)).p(px(8.)).cursor_pointer()
                    .child(section["section_title"].as_str().unwrap_or("").to_owned())
                    .on_click(cx.listener(move|this,_,window,cx|this.edit_column(id,window,cx)))
                    .layout_key(&self.layout,format!("board.columns.edit.{id}"),None));
            }
        } else if panel==Panel::Columns {
            content=content.child(div().flex().justify_between()
                .child(div().id("columns-empty").cursor_pointer().child(format!("Show empty board columns   {}",if self.view_settings()["board_empty_sections"]=="Hidden"{"□"}else{"✓"})).on_click(cx.listener(|this,_,window,cx|this.toggle_empty_columns(window,cx))).layout_key(&self.layout,"board.columns.empty",None))
                .child(div().id("columns-add").cursor_pointer().child("+").on_click(cx.listener(|this,_,window,cx|this.open_view_panel(Panel::AddColumn,window,cx))).layout_key(&self.layout,"board.columns.add",None)));
            for (index,section) in self.managed_columns().into_iter().enumerate() {
                let id=section["id"].as_i64().unwrap_or(0);
                let name=section["section_title"].as_str().unwrap_or("").to_owned();
                let count=self.db.column_task_count(self.options.board,id).unwrap_or(0);
                content=content.child(div().id(("managed-column",index)).flex().items_center().justify_between().p(px(8.)).when(index==self.views.choice,|d|d.bg(color(self.colors.active)))
                    .on_drag(ColumnDrag{id,title:name.clone()},|drag,_,_,cx|cx.new(|_|drag.clone()))
                    .on_drop(cx.listener(move|this,drag:&ColumnDrag,window,cx|this.reorder_column(drag.id,id,window,cx)))
                    .child(format!("{name}   {count}"))
                    .child(div().flex().gap(px(16.)).child("☰")
                        .child(div().id(("column-settings",index)).cursor_pointer().child("⚙").on_click(cx.listener(move|this,_,window,cx|this.edit_column(id,window,cx))).layout_key(&self.layout,format!("board.columns.edit.{id}"),None))
                        .child(div().id(("column-visible",index)).cursor_pointer().child(if section["visibility"]==true{"☑"}else{"□"}).on_click(cx.listener(move|this,_,window,cx|this.choose_view_option(index,window,cx))).layout_key(&self.layout,format!("board.columns.visible.{id}"),None)))
                    .layout_key(&self.layout,format!("board.columns.row.{id}"),None));
            }
        } else if panel==Panel::AddColumn {content=content.child(self.views.input.clone());}
        else if panel==Panel::ColumnAssignees {
            content=content.child(self.views.query.clone());
            for (index,person) in self.column_people().into_iter().enumerate() {
                content=content.child(div().id(("column-person",index)).p(px(8.)).cursor_pointer().child(person["displayName"].as_str().unwrap_or("").to_owned()).on_click(cx.listener(move|this,_,window,cx|this.choose_column_person(index,window,cx))).layout_key(&self.layout,format!("board.columns.person.{index}"),None));
            }
        } else if let Some(section)=self.managed_columns().into_iter().find(|section|section["id"]==json!(self.views.column_edit)) {
            let id=section["id"].as_i64().unwrap();
            let name=section["section_title"].as_str().unwrap_or("").trim().to_lowercase();
            let done=section["isDone"].as_bool().unwrap_or(["done","complete","completed","shipped","closed","finished","live","released"].contains(&name.as_str()));
            let project=self.db.board_project(self.options.board).unwrap_or(Value::Null);
            let (visible,total)=ht_core::board_columns::visible_counts(&project,id);
            let description=if total==0{"No views yet".into()}else if total==1{if visible==1{"Visible in the only view".into()}else{"Hidden in the only view".into()}}else if visible==total{format!("Visible in all {total} views")}else if visible==0{format!("Hidden in all {total} views")}else{format!("Visible in {visible} of {total} views")};
            let assigned=self.column_people().into_iter().find(|person|person["id"]==section["autoAssignUserId"] || person["id"]==section["autoAssignAgentId"]).and_then(|person|person["displayName"].as_str().map(str::to_owned)).unwrap_or_else(||if section["autoAssignAgentId"].is_string(){"Agent".into()}else if section["autoAssignUserId"].is_number(){"Member".into()}else{"Nobody".into()});
            content=content.child("Name").child(self.views.input.clone())
                .child(div().id("column-is-done").cursor_pointer().child(format!("Tickets here are finished    {}",if done{"✓"}else{"□"})).child(div().text_color(color(self.colors.muted)).child("Ends the ticket's cycle. Reports, Inbox Zero and agents all treat it as done.")).on_click(cx.listener(move|this,_,window,cx|this.queue_column("column-update",json!({"sectionId":id,"newSection":{"isDone":!done}}),window,cx))).layout_key(&self.layout,"board.columns.done",None))
                .child("Views").child(description)
                .child(div().flex().gap(px(16.)).children([(true,"Show in all views"),(false,"Hide in all views")].into_iter().map(|(visible,name)|div().id(if visible{"column-show-all"}else{"column-hide-all"}).cursor_pointer().child(name).on_click(cx.listener(move|this,_,window,cx|this.queue_column("column-visibility",json!({"sectionId":id,"visible":visible}),window,cx))).layout_key(&self.layout,if visible{"board.columns.show-all"}else{"board.columns.hide-all"},None))))
                .child(div().id("column-autoassign").cursor_pointer().child(format!("Auto-assign    {assigned}")).on_click(cx.listener(|this,_,window,cx|{this.save_column_title(window,cx);this.open_view_panel(Panel::ColumnAssignees,window,cx);window.focus(&this.views.query.focus_handle(cx),cx);})).layout_key(&self.layout,"board.columns.autoassign",None))
                .child("Danger zone").child("Delete board column").child("Its tickets move to the first column.")
                .child(div().id("column-delete").cursor_pointer().child("Delete").on_click(cx.listener(|this,_,window,cx|{this.save_column_title(window,cx);this.views.panel=Some(Panel::DeleteColumn);cx.notify();})).layout_key(&self.layout,"board.columns.delete",None));
            if panel==Panel::DeleteColumn {content=content.child(format!("Delete board column “{}”?",section["section_title"].as_str().unwrap_or("")))
                .child(div().id("column-confirm-delete").cursor_pointer().child("Delete board column").on_click(cx.listener(move|this,_,window,cx|this.queue_column("column-update",json!({"sectionId":id,"newSection":{"deleted":true}}),window,cx))).layout_key(&self.layout,"board.columns.confirm-delete",None))
                .child(div().id("column-cancel-delete").cursor_pointer().child("Cancel").on_click(cx.listener(|this,_,_,cx|{this.views.panel=Some(Panel::EditColumn);cx.notify();})).layout_key(&self.layout,"board.columns.cancel-delete",None));}
        }
        content=content.when_some(self.error.clone(),|d,error|d.child(div().text_color(color(0xcf222e)).child(error)));
        div().absolute().inset_0().bg(rgba(0x00000040)).flex().items_center().justify_center().on_mouse_down(gpui::MouseButton::Left,cx.listener(|this,_,window,cx|this.column_back(window,cx)))
            .child(div().id("column-manager").w(px(560.)).max_h(px(600.)).overflow_y_scroll().p(px(16.)).bg(color(self.colors.comment)).rounded(px(6.)).on_mouse_down(gpui::MouseButton::Left,|_,_,cx|cx.stop_propagation()).child(content).layout_key(&self.layout,"board.columns.dialog",None)).into_any_element()
    }
}

#[cfg(test)]
mod column_manager_tests {
    use super::*;
    use crate::ui::{task_fields::tests::desktop,shell::tests::{click,draw,measured}};
    struct Server(u16,String,Value,Value);
    impl ht_core::Transport for Server {
        fn get(&mut self,path:&str)->Result<ht_core::http::Response,ht_core::http::TransportError>{assert_eq!(path,"/user/context");Ok(ht_core::http::Response{status:200,body:json!({"success":true,"user":{"id":42}})})}
        fn post(&mut self,path:&str,body:&Value,key:&str)->Result<ht_core::http::Response,ht_core::http::TransportError>{assert_eq!(path,self.1);assert_eq!(*body,self.2);assert!(!key.is_empty());Ok(ht_core::http::Response{status:self.0,body:if self.0<300 {self.3.clone()}else{json!({"message":"Forbidden"})}})}
    }
    fn seed(this:&mut Desktop) {
        let columns=json!([{"id":10,"projectId":15,"section_title":"Todo","ranking":"A0100","visibility":true,"deleted":false,"isDone":null,"autoAssignUserId":null,"autoAssignAgentId":null},{"id":11,"projectId":15,"section_title":"Done","ranking":"A0200","visibility":true,"deleted":false,"isDone":null,"autoAssignUserId":null,"autoAssignAgentId":null}]);
        this.db.apply_response(&json!({"success":true,"projects":[{"id":15,"title":"Board","owner":{"id":42,"uid":"u42","displayName":"Owner"},"members":[{"user":{"id":7,"uid":"u7","displayName":"Member"}}],"boardAgents":[{"id":"agent","displayName":"Agent"}],"project_view":{"id":"pv","projectId":15,"default_view_id":"default","default_view":{"id":"default","board_columns_view":columns},"allViews":[{"id":"default","board_columns_view":columns}],"user_project_views":[{}]}}],"sections":columns})).unwrap();this.screen=Screen::Board;this.views.panel=None;this.reload();
    }
    fn settle(this:&mut Desktop,status:u16,window:&mut Window,cx:&mut Context<Desktop>) {
        let row=this.db.mutations().unwrap().into_iter().find(|row|row.status!="failed").unwrap();let change:ht_core::ViewWrite=serde_json::from_value(row.payload_json["change"].clone()).unwrap();let (_,path,body)=change.request().unwrap();
        let project=this.db.board_project(15).unwrap();
        let reply=match change.route.as_str(){
            "column-update"=>if change.body["newSection"]["deleted"]==true {Value::Null}else{project["section"].as_array().unwrap().iter().find(|section|section["id"]==change.body["sectionId"]).unwrap().clone()},
            "column-visibility"=>json!({"success":true,"sectionId":change.body["sectionId"],"visible":change.body["visible"]}),
            "column-autoassign"=>json!({"success":true,"sectionId":change.body["sectionId"],"autoAssignUserId":change.body["autoAssignUserId"],"autoAssignAgentId":change.body["autoAssignAgentId"]}),
            "unsaved-view"=>{let mut pv=project["project_view"].clone();pv["user_project_views"][0]["unsavedView"]=change.body.clone();pv},
            _=>panic!("Unexpected route {}",change.route),
        };
        let status=if status==200 && reply.is_null(){204}else{status};
        this.db.drain_once(&mut Server(status,path,body,reply),chrono::Utc::now().timestamp()+1).unwrap();this.poll_view(window,cx);
    }
    #[gpui::test]
    fn column_create_reuses_web_form_and_retains_text_after_refused_optimistic_create(cx:&mut gpui::TestAppContext) {
        cx.update(crate::input::init);let (view,cx)=cx.add_window_view(|window,cx|{let mut this=desktop(window,cx);seed(&mut this);this});
        for status in [401,403,200] {view.update_in(cx,|this,window,cx|{
            seed(this);this.open_view_panel(Panel::AddColumn,window,cx);this.views.input.update(cx,|input,cx|input.set_content("Review".into(),cx));this.create_column_form(window,cx);
            let row=this.db.mutations().unwrap().into_iter().find(|row|row.status!="failed").unwrap();assert!(this.columns.iter().any(|column|column.id==-row.id));
            let reply=json!({"section":{"id":12,"projectId":15,"section_title":"Review","ranking":"A0300","visibility":true,"deleted":false},"project_view":{"id":"pv","projectId":15,"user_project_views":[{}]}});
            this.db.drain_once(&mut Server(status,"/api/section/create".into(),json!({"projectId":15,"title":"Review"}),reply),chrono::Utc::now().timestamp()+1).unwrap();this.poll_view(window,cx);
            if status>=300 {assert_eq!(this.views.panel,Some(Panel::AddColumn));assert_eq!(this.views.input.read(cx).content.as_ref(),"Review");assert!(!this.columns.iter().any(|column|column.id<0));assert_eq!(this.error.as_deref(),Some("Error creating column"));}
            else {assert!(this.views.panel.is_none());assert!(this.columns.iter().any(|column|column.id==12));}
        });}
    }
    #[gpui::test]
    fn column_manager_controls_queue_web_settings_and_restore_401_403(cx:&mut gpui::TestAppContext) {
        cx.update(crate::input::init);let view=cx.open_window(gpui::size(px(1440.),px(900.)),|window,cx|{let mut this=desktop(window,cx);seed(&mut this);this});let mut cx=gpui::VisualTestContext::from_window(view.into(),cx);draw(view,&mut cx);
        for control in ["board.columns.done","board.columns.show-all","board.columns.hide-all"] {for status in [401,403,200] {
            view.update(&mut cx,|this,window,cx|{seed(this);this.edit_column(10,window,cx);}).unwrap();draw(view,&mut cx);click(view,&mut cx,control);
            view.update(&mut cx,|this,window,cx|{
                let row=this.db.mutations().unwrap().into_iter().find(|row|row.status!="failed").unwrap();
                assert_eq!(row.payload_json["change"]["body"]["sectionId"],10);
                if control=="board.columns.done"{assert_eq!(this.db.board_project(15).unwrap()["section"][0]["isDone"],true);}
                settle(this,status,window,cx);
                if status>=300{assert!(this.error.is_some());assert!(this.db.board_project(15).unwrap()["section"][0]["isDone"].is_null());assert_eq!(ht_core::board_columns::visible_counts(&this.db.board_project(15).unwrap(),10),(1,1));}
            }).unwrap();
        }}
    }
    #[gpui::test]
    fn column_rename_blur_and_done_are_serialized_without_losing_the_second_intent(cx:&mut gpui::TestAppContext) {
        cx.update(crate::input::init);let view=cx.open_window(gpui::size(px(1440.),px(900.)),|window,cx|{let mut this=desktop(window,cx);seed(&mut this);this});let mut cx=gpui::VisualTestContext::from_window(view.into(),cx);draw(view,&mut cx);
        for status in [401,403,200] {
            view.update(&mut cx,|this,window,cx|{seed(this);this.edit_column(10,window,cx);}).unwrap();cx.simulate_keystrokes("ctrl-a");cx.simulate_input("Backlog");
            click(view,&mut cx,"board.columns.done");
            view.update(&mut cx,|this,window,cx|{assert_eq!(this.views.column_deferred.len(),1);settle(this,status,window,cx);assert_eq!(this.db.mutations().unwrap().iter().filter(|row|row.status!="failed").count(),1);settle(this,200,window,cx);let project=this.db.board_project(15).unwrap();assert_eq!(project["section"][0]["section_title"],if status==200{"Backlog"}else{"Todo"});assert_eq!(project["section"][0]["isDone"],true);}).unwrap();
        }
    }
    #[gpui::test]
    fn column_autoassign_member_agent_nobody_and_delete_confirmation_have_real_refusal_rollback(cx:&mut gpui::TestAppContext) {
        cx.update(crate::input::init);let view=cx.open_window(gpui::size(px(1440.),px(900.)),|window,cx|{let mut this=desktop(window,cx);seed(&mut this);this});let mut cx=gpui::VisualTestContext::from_window(view.into(),cx);draw(view,&mut cx);
        for person in [2,3,0] {for status in [401,403,200] {
            view.update(&mut cx,|this,window,cx|{seed(this);this.edit_column(10,window,cx);}).unwrap();draw(view,&mut cx);click(view,&mut cx,"board.columns.autoassign");click(view,&mut cx,&format!("board.columns.person.{person}"));
            view.update(&mut cx,|this,window,cx|{let project=this.db.board_project(15).unwrap();if person==2{assert_eq!(project["section"][0]["autoAssignUserId"],7);}if person==3{assert_eq!(project["section"][0]["autoAssignAgentId"],"agent");}settle(this,status,window,cx);let project=this.db.board_project(15).unwrap();if status>=300{assert_eq!(this.error.as_deref(),Some("Unable to update column auto-assign"));assert!(project["section"][0]["autoAssignUserId"].is_null());assert!(project["section"][0]["autoAssignAgentId"].is_null());}}).unwrap();
        }}
        for status in [401,403,200] {
            view.update(&mut cx,|this,window,cx|{seed(this);this.edit_column(10,window,cx);}).unwrap();draw(view,&mut cx);click(view,&mut cx,"board.columns.delete");click(view,&mut cx,"board.columns.cancel-delete");view.read_with(&cx,|this,_|assert!(!this.db.mutations().unwrap().iter().any(|row|row.status!="failed"))).unwrap();click(view,&mut cx,"board.columns.delete");click(view,&mut cx,"board.columns.confirm-delete");
            view.update(&mut cx,|this,window,cx|{assert!(!this.columns.iter().any(|column|column.id==10));settle(this,status,window,cx);assert_eq!(this.columns.iter().any(|column|column.id==10),status>=300);if status>=300{assert_eq!(this.error.as_deref(),Some("Column could not be deleted"));} }).unwrap();
        }
    }
    #[gpui::test]
    fn column_drag_order_persists_rank_then_current_view_and_rolls_back_each_refusal(cx:&mut gpui::TestAppContext) {
        cx.update(crate::input::init);let (view,cx)=cx.add_window_view(|window,cx|{let mut this=desktop(window,cx);seed(&mut this);this});
        for status in [401,403,200] {view.update_in(cx,|this,window,cx|{seed(this);this.open_view_panel(Panel::Columns,window,cx);this.reorder_column(10,11,window,cx);assert_eq!(this.columns[0].id,11);settle(this,status,window,cx);if status>=300{assert_eq!(this.columns[0].id,10);assert_eq!(this.error.as_deref(),Some("Column order could not be saved"));}else{let row=this.db.mutations().unwrap().into_iter().find(|row|row.status!="failed").unwrap();assert_eq!(row.payload_json["change"]["route"],"unsaved-view");assert_eq!(row.payload_json["change"]["body"]["board_columns_view"][0]["id"],11);settle(this,200,window,cx);assert_eq!(this.columns[0].id,11);}});}
    }
    #[gpui::test]
    fn autoassign_palette_command_is_board_only_and_opens_column_then_person_picker(cx:&mut gpui::TestAppContext) {
        assert!(!crate::command_center::commands(Screen::Task,true).iter().any(|row|row.key=="autoAssignColumn"));
        cx.update(crate::input::init);let view=cx.open_window(gpui::size(px(1440.),px(900.)),|window,cx|{let mut this=desktop(window,cx);seed(&mut this);this});let mut cx=gpui::VisualTestContext::from_window(view.into(),cx);draw(view,&mut cx);
        cx.simulate_keystrokes("ctrl-k");cx.simulate_input("Auto-assign for this column");cx.simulate_keystrokes("enter");draw(view,&mut cx);measured(view,&mut cx,"board.columns.dialog");click(view,&mut cx,"board.columns.edit.10");measured(view,&mut cx,"board.columns.person.0");view.read_with(&cx,|this,_|assert_eq!(this.views.panel,Some(Panel::ColumnAssignees))).unwrap();
    }
}
