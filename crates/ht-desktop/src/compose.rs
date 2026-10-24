use super::*;
use gpui::MouseButton;
use serde_json::Value;

pub(super) struct Compose {
    open: bool,
    writing: bool,
    input: Entity<TextInput>,
    board_query: Entity<TextInput>,
    board_focus: FocusHandle,
    picker: bool,
    choice: usize,
    project: i64,
    pending: Option<String>,
    error: Option<String>,
}
impl Compose {
    pub fn new(cx: &mut Context<Desktop>) -> Self {
        let input = cx.new(|cx| TextInput::new(String::new(), "Describe the task", color(0x8e9093), cx).multiline());
        let board_query = cx.new(|cx| TextInput::new(String::new(), "Search boards", color(0x8e9093), cx));
        cx.observe(&input, |_, _, cx|cx.notify()).detach();
        cx.observe(&board_query, |this, _, cx| {this.compose.choice=0;cx.notify();}).detach();
        Self {open:false,writing:true,input,board_query,board_focus:cx.focus_handle(),picker:false,choice:0,project:0,pending:None,error:None}
    }
}
impl Desktop {
    pub(super) fn compose_open(&self) -> bool { self.compose.open }
    pub(super) fn open_compose(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.compose.open = true;
        self.compose.writing = true;
        self.compose.picker = false;
        self.interaction.modal = None;
        if self.compose.project <= 0 {
            self.compose.project = if self.screen != Screen::Inbox { self.options.board }
                else { self.db.kv_get("compose-last-board").ok().flatten().and_then(|s|s.parse().ok()).unwrap_or(self.options.board) };
        }
        window.focus(&self.compose.input.focus_handle(cx), cx);
        cx.notify();
    }
    fn close_compose(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.compose.pending.is_some() { return; }
        self.compose.open = false;
        self.compose.picker = false;
        self.interaction.modal = None;
        window.focus(&self.focus, cx);
        cx.notify();
    }
    fn compose_tab(&mut self, writing: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.compose.pending.is_some() { return; }
        self.compose.writing = writing;
        self.compose.picker = false;
        if writing {
            self.interaction.modal = None;
            window.focus(&self.compose.input.focus_handle(cx), cx);
        } else { self.show_modal(interactions::Modal::Commands, window, cx); }
        cx.notify();
    }
    fn compose_boards(&self, cx: &App) -> Vec<Value> {
        let query = self.compose.board_query.read(cx).content.to_lowercase();
        self.db.projects().unwrap_or_default().into_iter().filter(|p|p["title"].as_str().unwrap_or("").to_lowercase().contains(&query)).collect()
    }
    fn compose_pick_board(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(id) = self.compose_boards(cx).get(index).and_then(|p|p["id"].as_i64()) {
            self.compose.project = id;
            self.compose.picker = false;
            window.focus(&self.compose.input.focus_handle(cx), cx);
            cx.notify();
        }
    }
    fn compose_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.compose.pending.is_some() { return; }
        self.compose.picker = true;
        self.compose.choice = 0;
        self.compose.board_query.update(cx,|input,cx|input.set_content(String::new(),cx));
        window.focus(&self.compose.board_query.focus_handle(cx),cx);
        cx.notify();
    }
    pub(super) fn compose_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let key=&event.keystroke;
        let ctrl=key.modifiers.control || key.modifiers.platform;
        if ctrl && !key.modifiers.shift && !key.modifiers.alt && key.key=="j" {
            self.open_compose(window,cx);cx.stop_propagation();return true;
        }
        if !self.compose.open {return false;}
        if key.key=="escape" {
            if self.compose.picker {self.compose.picker=false;window.focus(&self.compose.board_focus,cx);cx.notify();}
            else {self.close_compose(window,cx);}
        } else if ctrl && key.key=="k" && !key.modifiers.shift && !key.modifiers.alt {
            if self.compose.pending.is_none() { self.compose.open=false;self.show_modal(interactions::Modal::Commands,window,cx); }
        } else if self.compose.pending.is_some() {
            // A queued send belongs to this captured note, not any later editor contents.
        } else if self.compose.picker {
            match key.key.as_str() {
                "down" => self.compose.choice=(self.compose.choice+1).min(self.compose_boards(cx).len().saturating_sub(1)),
                "up" => self.compose.choice=self.compose.choice.saturating_sub(1),
                "enter" => self.compose_pick_board(self.compose.choice,window,cx),
                _ => return false,
            }
            cx.notify();
        } else if key.key=="tab" && !ctrl && !key.modifiers.alt {
            if !self.compose.writing {self.compose_tab(true,window,cx);}
            else if self.compose.board_focus.is_focused(window) && key.modifiers.shift {window.focus(&self.compose.input.focus_handle(cx),cx);}
            else if self.compose.input.focus_handle(cx).is_focused(window) && !key.modifiers.shift {window.focus(&self.compose.board_focus,cx);}
            else {self.compose_tab(false,window,cx);}
        } else if self.compose.writing && self.compose.board_focus.is_focused(window) && matches!(key.key.as_str(),"enter"|"space") && !ctrl && !key.modifiers.alt {
            self.compose_picker(window,cx);
        } else if self.compose.writing && key.key=="enter" && !key.modifiers.shift && !ctrl && !key.modifiers.alt {
            self.send_compose(window,cx);
        } else {return false;}
        cx.stop_propagation();true
    }
    fn send_compose(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.compose.pending.is_some() || self.compose.input.read(cx).content.trim().is_empty() {return;}
        let text=self.compose.input.read(cx).content.to_string();
        match self.db.enqueue(Write::ComposeTask {project_id:self.compose.project,text,time_zone:iana_time_zone::get_timezone().unwrap_or_else(|_|"UTC".into())},chrono::Utc::now().timestamp()) {
            Ok(id)=> {
                self.compose.pending=self.db.mutations().ok().and_then(|rows|rows.into_iter().find(|r|r.id==id)).map(|r|r.client_id);
                self.compose.error=None;
                if let Some(worker)=&self.worker {let _=worker.handle().wake();}
                if !self.options.offline {
                    cx.spawn_in(window,async move|view,cx| {
                        loop {
                            cx.background_executor().timer(Duration::from_millis(100)).await;
                            let pending=view.update_in(cx,|this,window,cx|{this.poll_compose(window,cx);this.compose.pending.is_some()});
                            if !matches!(pending, Ok(true)) {break;}
                        }
                    }).detach();
                }
            }
            Err(error)=>self.compose.error=Some(error.to_string()),
        }
        cx.notify();
    }
    pub(super) fn poll_compose(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(key)=self.compose.pending.clone() else{return;};
        let receipt=format!("compose-result:{key}");
        let Some(value)=self.db.kv_get(&receipt).ok().flatten().and_then(|s|serde_json::from_str::<Value>(&s).ok()) else{return;};
        self.compose.pending=None;
        if let Some(error)=value["error"].as_str() {self.compose.error=Some(format!("Couldn’t create the task ({error}). Your note is still here."));}
        else if let Some(id)=value["tasks"][0]["id"].as_i64() {
            let _=self.db.kv_set("compose-last-board",&self.compose.project.to_string());
            self.compose.input.update(cx,|input,cx|input.reset(window,cx));
            self.compose.open=false;
            self.interaction.modal=None;
            self.open_task(id,cx);
            window.focus(&self.focus,cx);
            if value["writerFailed"]==true {self.error=Some("The task writer was unavailable; your original note was saved as the title and description.".into());}
        }
        let _=self.db.kv_set(&receipt,"null");
        cx.notify();
    }
    pub(super) fn compose_tabs(&self, cx: &mut Context<Self>) -> AnyElement {
        div().flex().gap(px(20.)).px(px(16.)).py(px(10.)).children([(false,"Search"),(true,"New Task")].into_iter().map(|(writing,label)|
            div().id(if writing {"compose-new-tab"}else{"compose-search-tab"}).cursor_pointer().text_color(color(if self.compose.writing==writing {self.colors.text}else{self.colors.muted})).child(label)
                .on_click(cx.listener(move|this,_,window,cx|{cx.stop_propagation();this.compose_tab(writing,window,cx);})).layout_key(&self.layout,if writing {"compose.tab.new"}else{"compose.tab.search"},None)
        )).into_any_element()
    }
    pub(super) fn compose_overlay(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        if !self.compose.open || !self.compose.writing {return div().into_any_element();}
        let t=self.colors;
        let title=ht_core::ai_chat::project_metadata(&self.db,self.compose.project).0;
        let content=if self.compose.pending.is_some() {div().p(px(24.)).child("Writing your ticket…").into_any_element()}
        else if self.compose.picker {
            div().p(px(12.)).child(self.compose.board_query.clone()).child(div().id("compose-board-results").max_h(px(300.)).overflow_y_scroll().children(self.compose_boards(cx).into_iter().enumerate().map(|(i,project)|
                div().id(("compose-board",i)).p(px(10.)).cursor_pointer().when(i==self.compose.choice,|d|d.bg(color(t.hover))).child(project["title"].as_str().unwrap_or("Board").to_owned()).on_click(cx.listener(move|this,_,window,cx|{cx.stop_propagation();this.compose_pick_board(i,window,cx);})).layout_key(&self.layout,format!("compose.board.{i}"),None)
            ))).into_any_element()
        } else {
            div().p(px(12.)).flex().flex_col().gap(px(12.)).child(div().min_h(px(80.)).child(self.compose.input.clone()).layout_key(&self.layout,"compose.prompt",None)).child(
                div().flex().items_center().gap(px(12.)).child(div().id("compose-board-chip").track_focus(&self.compose.board_focus).cursor_pointer().px(px(8.)).py(px(4.)).bg(color(t.hover)).rounded(px(4.)).child(title).on_click(cx.listener(|this,_,window,cx|{cx.stop_propagation();this.compose_picker(window,cx);})).layout_key(&self.layout,"compose.board-chip",None))
                .child(div().flex_1()).child(div().id("compose-send").cursor_pointer().child(icon("send",t.accent,18.)).on_click(cx.listener(|this,_,window,cx|{cx.stop_propagation();this.send_compose(window,cx);})).layout_key(&self.layout,"compose.send",None))
            ).child(div().text_size(px(11.)).text_color(color(t.muted)).child("Enter to send · Shift+Enter for newline · Tab to board"))
            .when(self.compose.error.is_some(),|d|d.child(self.compose.error.clone().unwrap())).into_any_element()
        };
        div().id("compose-backdrop").absolute().size_full().top_0().left_0().bg(rgba(0x00000080)).on_mouse_down(MouseButton::Left,cx.listener(|_,_,_,cx|cx.stop_propagation())).on_click(cx.listener(|this,_,window,cx|this.close_compose(window,cx)))
            .child(div().id("compose-dialog").absolute().top(px(f32::from(window.viewport_size().height)*0.24)).left(px((f32::from(window.viewport_size().width)-560.)/2.)).w(px(560.)).rounded(px(5.)).bg(color(t.comment)).shadow(shadow()).flex().flex_col().on_click(cx.listener(|_,_,_,cx|cx.stop_propagation())).child(self.compose_tabs(cx)).child(content)).into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn desktop(window:&mut Window,cx:&mut Context<Desktop>)->Desktop {
        let mut db=Db::memory().unwrap();db.kv_set("current-user-id","985").unwrap();
        db.apply_response(&json!({"success":true,"projects":[{"id":7283,"title":"QA","status":"Normal"},{"id":7284,"title":"Other","status":"Normal"}]})).unwrap();
        let options=Options::parse(["--offline","--screen","board","--board","7283"].map(String::from)).unwrap();
        let view=Desktop::new(db,options,window,cx);window.focus(&view.focus,cx);view
    }
    #[gpui::test]
    fn ctrlj_tabs_prompt_board_chip_picker_and_send_newline_are_real_keys(cx:&mut gpui::TestAppContext) {
        cx.update(crate::input::init);let(view,cx)=cx.add_window_view(desktop);
        cx.simulate_keystrokes("ctrl-j");cx.simulate_input("é🙂 note");cx.simulate_keystrokes("shift-enter");
        view.read_with(cx,|this,cx|{assert!(this.compose.open);assert!(this.compose.writing);assert_eq!(this.compose.input.read(cx).content.as_ref(),"é🙂 note\n");assert!(this.db.mutations().unwrap().is_empty());});
        cx.simulate_keystrokes("tab");view.update_in(cx,|this,window,cx|assert!(this.compose.board_focus.is_focused(window)));
        cx.simulate_keystrokes("shift-tab");view.update_in(cx,|this,window,cx|assert!(this.compose.input.focus_handle(cx).is_focused(window)));
        cx.simulate_keystrokes("tab space");view.read_with(cx,|this,_|assert!(this.compose.picker));
        cx.simulate_input("Other");cx.simulate_keystrokes("enter");view.read_with(cx,|this,_|assert_eq!(this.compose.project,7284));
        cx.simulate_keystrokes("tab tab");view.read_with(cx,|this,_|{assert!(!this.compose.writing);assert_eq!(this.interaction.modal,Some(interactions::Modal::Commands));});
        cx.simulate_keystrokes("shift-tab enter enter");
        view.read_with(cx,|this,cx|{assert!(this.compose.pending.is_some());let rows=this.db.mutations().unwrap();assert_eq!(rows.len(),1);assert_eq!(rows[0].kind,"compose_task");assert_eq!(rows[0].payload_json["project"]["id"],7284);assert_eq!(this.compose.input.read(cx).content.as_ref(),"é🙂 note\n");});
    }
    #[gpui::test]
    fn compose_error_keeps_note_and_success_navigates_only_after_server_receipt(cx:&mut gpui::TestAppContext) {
        cx.update(crate::input::init);let(view,cx)=cx.add_window_view(desktop);
        cx.simulate_keystrokes("ctrl-j");cx.simulate_input("Keep me");cx.simulate_keystrokes("enter");
        view.update_in(cx,|this,window,cx|{
            let key=this.compose.pending.clone().unwrap();this.db.kv_set(&format!("compose-result:{key}"),"{\"error\":\"HTTP 403\"}").unwrap();this.poll_compose(window,cx);
            assert!(this.compose.error.is_some());assert_eq!(this.compose.input.read(cx).content.as_ref(),"Keep me");assert!(this.compose.open);
            this.db.apply_task(&json!({"id":9,"projectId":7283,"title":"Created","status":"Normal"})).unwrap();
            this.compose.pending=Some("receipt".into());this.db.kv_set("compose-result:receipt",&json!({"success":true,"tasks":[{"id":9}]}).to_string()).unwrap();this.poll_compose(window,cx);
            assert!(!this.compose.open);assert_eq!(this.screen,Screen::Task);assert_eq!(this.options.task,9);assert!(this.compose.input.read(cx).content.is_empty());
        });
    }
}
