use super::*;
use serde_json::{Value, json};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Panel {
    Sort,
    Filters,
    Priority,
    Size,
    Labels,
    Assignees,
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
}
impl Views {
    pub fn new(cx: &mut Context<Desktop>) -> Self {
        let input = cx.new(|cx| TextInput::new(String::new(), "View name", color(0x8e9093), cx));
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
        }
    }
}
impl Desktop {
    pub(super) fn invalidate_board_views(&mut self, cx: &mut Context<Self>) {
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
            let result = ht_core::Http::new(token)
                .and_then(|mut http| ht_core::board_views::load(&mut http, board));
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
    fn view_settings(&self) -> Value {
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
        self.views.choice = 0;
        if matches!(panel,Panel::SavePrivate|Panel::SaveTeam|Panel::RenameView) {
            self.views
                .input
                .update(cx, |input, cx| input.set_content(String::new(), cx));
            window.focus(&self.views.input.focus_handle(cx), cx);
        } else {
            window.focus(&self.focus, cx);
        }
        cx.notify();
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
                self.views.pending_route=route.into();
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
            self.views.panel = None;
            window.focus(&self.focus, cx);
        }else{
            self.error=Some(match self.views.pending_route.as_str(){
                "rename-view"=>"Error renaming view".into(),"delete-view"=>"Error deleting view".into(),"update-order"=>"Unable to save view order".into(),"reset-order"=>"Unable to reset view order".into(),"set-default-order"=>"Unable to set the default view order".into(),_=>result["error"].as_str().unwrap_or("Could not save the view").into()
            });
        }
        cx.notify();
        false
    }
    fn view_choices(&self) -> Vec<(String, Value)> {
        match self.views.panel {
            Some(Panel::Save)=>["Save as current view","Set as default board for everyone","Save as additional view for team","Save as view for me only","Reset View"].into_iter().enumerate().map(|(i,name)|(name.into(),json!(i))).collect(),
            Some(Panel::SavePrivate|Panel::SaveTeam)=>vec![("Save view".into(),Value::Null)],
            Some(Panel::RenameView)=>vec![("Save".into(),json!("rename")),("DELETE VIEW".into(),json!("delete"))],
            Some(Panel::DeleteView)=>vec![("Delete view".into(),json!("confirm")),("Cancel".into(),json!("cancel"))],
            Some(Panel::Manage)=>{
                let project=self.db.board_project(self.options.board).unwrap_or(Value::Null);
                let mut choices=vec![("Sort alphabetically".into(),json!({"action":"alphabetical"})),("Reset to default order".into(),json!({"action":"reset"}))];
                if self.can_set_view_default(&project){choices.push(("Set as default order for everyone".into(),json!({"action":"default"})));}
                choices.extend(ht_core::board_views::ordered_views(&project).into_iter().map(|view|(view["title"].as_str().unwrap_or("Untitled view").into(),view)));
                choices
            }
            Some(Panel::Sort) => [
                ("Manual", "Manual"),
                ("Priority", "Priority"),
                ("Due Date", "DueDate"),
                ("Size", "Size"),
                ("Last Updated", "UpdatedAt"),
                ("Creation Date", "CreatedAt"),
                ("Last comment", "LastCommentAt"),
                ("Column changed", "SectionChangedAt"),
                ("Title", "Title"),
                ("Ticket number", "TicketNumber"),
            ]
            .into_iter()
            .map(|(name, value)| (name.into(), json!(value)))
            .chain([
                ("Ascending".into(), json!({"order":"Ascending"})),
                ("Descending".into(), json!({"order":"Descending"})),
            ])
            .collect(),
            Some(Panel::Filters) => ["Priority", "Size", "Tags", "Assignees", "Clear filters"]
                .into_iter()
                .map(|name| (name.into(), json!(name)))
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
            Some(Panel::Assignees) => self
                .db
                .project_members(self.options.board)
                .unwrap_or_default()
                .into_iter()
                .map(|v| (v["displayName"].as_str().unwrap_or("").into(), v))
                .collect(),
            _ => vec![],
        }
    }
    fn choose_view_option(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if self.views.pending.is_some() {
            return;
        }
        match self.views.panel {
            Some(Panel::SavePrivate|Panel::SaveTeam)=>{
                let title=self.views.input.read(cx).content.trim().to_owned();if title.is_empty(){self.error=Some("Give your view a name first".into());cx.notify();return;}
                let public=self.views.panel==Some(Panel::SaveTeam);
                self.queue_view("create-view",json!({"userId":self.db.current_user_id(),"viewTitle":title,"visibility":if public{"Public"}else{"Private"},"setAsDefault":false,"view_settings":self.view_settings()}),window,cx);return;
            }
            Some(Panel::Save)=>{
                match index {
                    2=>self.open_view_panel(Panel::SaveTeam,window,cx),3=>self.open_view_panel(Panel::SavePrivate,window,cx),
                    4=>self.queue_view("reset-to-default",json!({"mode":"ResetCurrent"}),window,cx),
                    0|1=>{
                        let project=self.db.board_project(self.options.board).unwrap_or(Value::Null);
                        let applied=project["project_view"]["user_project_views"][0]["appliedView"]["id"].clone();
                        if index==0 && applied.is_string(){self.queue_view("update-view",json!({"viewId":applied,"view_settings":self.view_settings()}),window,cx);}
                        else{self.queue_view("create-view",json!({"userId":self.db.current_user_id(),"viewTitle":format!("default-view-id-{}",self.options.board),"visibility":"Public","setAsDefault":true,"view_settings":self.view_settings()}),window,cx);}
                    }
                    _=>(),
                }
                return;
            }
            _=>(),
        }
        let Some((_, value)) = self.view_choices().get(index).cloned() else {
            return;
        };
        let mut settings = self.view_settings();
        match self.views.panel {
            Some(Panel::Manage)=>{
                if let Some(action)=value["action"].as_str(){
                    if action=="reset"{self.queue_view("reset-order",json!({}),window,cx);return;}
                    let project=self.db.board_project(self.options.board).unwrap_or(Value::Null);
                    let mut views=ht_core::board_views::ordered_views(&project);
                    if action=="alphabetical"{views.sort_by_key(|v|(v["id"]!=project["project_view"]["default_view_id"],v["title"].as_str().unwrap_or("").to_lowercase()));}
                    self.queue_view(if action=="default"{"set-default-order"}else{"update-order"},json!({"viewOrder":views.iter().map(|v|v["id"].clone()).collect::<Vec<_>>()}),window,cx);return;
                }
                self.views.editing=value["id"].as_str().map(str::to_owned);
                self.open_view_panel(Panel::RenameView,window,cx);
                self.views.input.update(cx,|input,cx|input.set_content(value["title"].as_str().unwrap_or("").into(),cx));return;
            }
            Some(Panel::RenameView)=>{
                if value=="delete"{self.views.panel=Some(Panel::DeleteView);self.views.choice=0;cx.notify();return;}
                let title=self.views.input.read(cx).content.to_string();if title.chars().count()<2{self.error=Some("View name must be at least 2 characters".into());cx.notify();return;}
                self.queue_view("rename-view",json!({"viewId":self.views.editing,"title":title}),window,cx);return;
            }
            Some(Panel::DeleteView)=>{
                if value=="cancel"{self.views.panel=Some(Panel::Manage);cx.notify();return;}
                let project=self.db.board_project(self.options.board).unwrap_or(Value::Null);
                if project["project_view"]["default_view_id"]==json!(self.views.editing){return;}
                self.queue_view("delete-view",json!({"viewId":self.views.editing}),window,cx);return;
            }
            Some(Panel::Sort) => {
                if value.is_string() {
                    settings["board_sorting_mode"] = value;
                    settings["board_sorting_stack"] = json!([]);
                } else {
                    settings["board_sorting_order"] = value["order"].clone();
                }
            }
            Some(Panel::Filters) => {
                match value.as_str().unwrap_or("") {
                    "Priority" => self.views.panel = Some(Panel::Priority),
                    "Size" => self.views.panel = Some(Panel::Size),
                    "Tags" => self.views.panel = Some(Panel::Labels),
                    "Assignees" => self.views.panel = Some(Panel::Assignees),
                    _ => {
                        settings["board_filters"] = json!({"addedFilters":[],"matchFilters":"ANY"});
                        self.queue_view("unsaved-view", settings, window, cx);
                        return;
                    }
                }
                self.views.choice = 0;
                cx.notify();
                return;
            }
            Some(panel @ (Panel::Priority | Panel::Size | Panel::Labels | Panel::Assignees)) => {
                let kind = match panel {
                    Panel::Priority => "Priority",
                    Panel::Size => "Size",
                    Panel::Labels => "Labels",
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
                filters.retain(|f| f["type"] != kind);
                if !selected.is_empty() {
                    let mut filter = json!({"type":kind,"searchPayload":selected});
                    if matches!(panel, Panel::Labels | Panel::Assignees) {
                        filter["match"] = json!("ANY");
                    }
                    filters.push(filter);
                }
                settings["board_filters"]["addedFilters"] = json!(filters);
            }
            _ => return,
        }
        self.queue_view("unsaved-view", settings, window, cx);
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
            "escape" => {
                self.views.panel = None;
                window.focus(&self.focus, cx);
            }
            "enter" => self.choose_view_option(self.views.choice, window, cx),
            "down" => {
                self.views.choice =
                    (self.views.choice + 1).min(self.view_choices().len().saturating_sub(1))
            }
            "up" => self.views.choice = self.views.choice.saturating_sub(1),
            _ => return !matches!(self.views.panel,Some(Panel::SavePrivate|Panel::SaveTeam|Panel::RenameView)),
        }
        cx.stop_propagation();
        cx.notify();
        true
    }
    pub(super) fn board_view_tabs(&self, cx: &mut Context<Self>) -> AnyElement {
        let project = self
            .db
            .board_project(self.options.board)
            .unwrap_or(Value::Null);
        if !project["project_view"].is_object() {
            return div().into_any_element();
        }
        let views = ht_core::board_views::ordered_views(&project);
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
                        this.queue_view("switch-view", json!({"newViewId":id}), window, cx)
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
            .child(div().id("manage-board-views").cursor_pointer().child("Manage views").on_click(cx.listener(|this,_,window,cx|this.open_view_panel(Panel::Manage,window,cx))))
            .into_any_element()
    }
    fn can_set_view_default(&self,project:&Value)->bool{
        self.db.current_user_id().is_some_and(|user|project["ownerId"]==user || project["members"].as_array().into_iter().flatten().any(|m|m["userId"]==user && m["status"]=="Accepted" && m["role"]=="Admin"))
    }
    pub(super) fn view_overlay(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(panel) = self.views.panel else {
            return div().into_any_element();
        };
        let t = self.colors;
        let title = match panel {
            Panel::Sort => "Sort",
            Panel::Save => "Save this view",
            Panel::SaveTeam=>"Save as new view for team",Panel::SavePrivate=>"Save as view for me only",
            Panel::Manage=>"Manage views",Panel::RenameView=>"View settings",Panel::DeleteView=>"Delete this view? That cannot be undone.",
            _ => "Filter",
        };
        div()
            .absolute()
            .inset_0()
            .bg(rgba(0x00000040))
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .id("board-view-dialog")
                    .w(px(480.))
                    .max_h(px(600.))
                    .overflow_y_scroll()
                    .p(px(16.))
                    .bg(color(t.comment))
                    .rounded(px(6.))
                    .child(title)
                    .when(matches!(panel,Panel::SavePrivate|Panel::SaveTeam|Panel::RenameView),|d|d.child(self.views.input.clone()))
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
                assert_eq!(this.views.panel.is_none(), status == 200);
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
    struct Web{status:u16,expected:ht_core::ViewWrite,reply:Value}
    impl ht_core::Transport for Web {
        fn get(&mut self,_:&str)->Result<ht_core::http::Response,ht_core::http::TransportError>{Ok(ht_core::http::Response{status:200,body:json!({"success":true,"user":{"id":42}})})}
        fn post(&mut self,path:&str,body:&Value,key:&str)->Result<ht_core::http::Response,ht_core::http::TransportError>{self.mutate("POST",path,body,key)}
        fn mutate(&mut self,method:&str,path:&str,body:&Value,_:&str)->Result<ht_core::http::Response,ht_core::http::TransportError>{let(m,p,b)=self.expected.request().unwrap();assert_eq!(method,m);assert_eq!(path,p);assert_eq!(body,&b);Ok(ht_core::http::Response{status:self.status,body:if self.status==200{self.reply.clone()}else{json!({"message":"Board access denied"})}})}
    }
    #[gpui::test]
    fn save_view_menu_uses_all_five_web_flows_and_keeps_name_on_both_refusals(cx:&mut gpui::TestAppContext){
        cx.update(crate::input::init);
        let(view,cx)=cx.add_window_view(|window,cx|{let mut this=desktop(window,cx);this.screen=Screen::Board;this});
        for option in 0..5{for status in [401,403,200]{view.update_in(cx,|this,window,cx|{
            this.db.apply_response(&json!({"success":true,"projects":[{"id":15,"title":"Board","project_view":{"id":"pv","projectId":15,"user_project_views":[{"appliedView":{"id":"v1","title":"Daily"}}],"allViews":[{"id":"v1","title":"Daily"}]}}]})).unwrap();this.reload();
            this.open_view_panel(Panel::Save,window,cx);assert_eq!(this.view_choices().len(),5);this.choose_view_option(option,window,cx);
            if matches!(option,2|3){this.views.input.update(cx,|input,cx|input.set_content("Named view".into(),cx));this.choose_view_option(0,window,cx);}
            let row=this.db.mutations().unwrap().into_iter().find(|r|r.status!="failed").unwrap();let change:ht_core::ViewWrite=serde_json::from_value(row.payload_json["change"].clone()).unwrap();
            if option==1||option==2||option==3{assert_eq!(change.route,"create-view");assert_eq!(change.body["userId"],42);assert_eq!(change.body["visibility"],if option==3{"Private"}else{"Public"});assert_eq!(change.body["setAsDefault"],option==1);}
            if option==0{assert_eq!(change.route,"update-view");assert_eq!(change.body["viewId"],"v1");}if option==4{assert_eq!(change.body["mode"],"ResetCurrent");}
            let pv=this.db.board_project(15).unwrap()["project_view"].clone();let reply=if change.route=="create-view"{json!({"project_view_updated":pv})}else{pv};
            this.db.drain_once(&mut Web{status,expected:change,reply},chrono::Utc::now().timestamp()+1).unwrap();assert!(!this.poll_view(window,cx));assert_eq!(this.views.panel.is_none(),status==200);
            if status!=200{assert_eq!(this.error.as_deref(),Some("Board access denied"));if option==2||option==3{assert_eq!(this.views.input.read(cx).content.as_ref(),"Named view");}}
        });}}
    }
}
