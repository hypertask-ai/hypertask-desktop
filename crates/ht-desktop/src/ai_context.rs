use super::*;
use ht_core::{Http,ai_context};
use serde_json::{Value,json};
use std::{ops::Range,time::Instant};

pub(super) struct ContextPicker {
    token: Option<String>,
    pub(super) scope: Value,
    fullscreen_scope:Value,
    teams: Vec<Value>,
    recent: Vec<i64>,
    ticket_enabled: bool,
    scope_open: bool,
    scope_pending: Option<Receiver<Result<(i64,Vec<Value>,bool),String>>>,
    scope_error: bool,
    user: Option<i64>,
    mention: Option<(Option<i64>,Range<usize>,String)>,
    pending: Option<Receiver<Result<Vec<Value>,String>>>,
    changed: Instant,
    rows: Vec<Value>,
    pub(super) selected: Vec<Value>,
    choice: usize,
    dismissed: bool,
    fetched: bool,
}
impl ContextPicker {
    pub(super) fn new(token:Option<String>)->Self {
        Self{token,scope:Value::Null,fullscreen_scope:Value::Null,teams:vec![],recent:vec![],ticket_enabled:false,scope_open:false,scope_pending:None,scope_error:false,user:None,mention:None,pending:None,changed:Instant::now(),rows:vec![],selected:vec![],choice:0,dismissed:false,fetched:false}
    }
}
impl AiSidebar {
    pub fn restore_context(&mut self,db:&Db){
        self.context_picker.user=db.current_user_id();
        if let Some(user)=db.current_user_id(){
            if let Ok(Some(value))=db.kv_get(&format!("ai-scope:{user}")){if let Ok(value)=serde_json::from_str::<Value>(&value){self.context_picker.scope=value["scope"].clone();self.context_picker.fullscreen_scope=value["fullscreen"].clone();self.context_picker.recent=serde_json::from_value(value["recent"].clone()).unwrap_or_default();}}
        }
    }
    pub fn persist_context(&self,db:&Db)->ht_core::Result<()> {
        if let Some(user)=db.current_user_id().filter(|user|Some(*user)==self.context_picker.user){db.kv_set(&format!("ai-scope:{user}"),&json!({"scope":self.context_picker.scope,"fullscreen":self.context_picker.fullscreen_scope,"recent":self.context_picker.recent}).to_string())?;}
        Ok(())
    }
    pub(super) fn context_project(&self)->Option<i64> {
        if self.mode()==DisplayMode::Fullscreen{return self.context_picker.fullscreen_scope.as_i64();}
        if self.context_picker.scope=="all" || !self.include_context{return None;}
        self.context_picker.scope.as_i64().or_else(||self.scope.as_ref().and_then(|s|s.context.project_id))
    }
    pub(super) fn request_context(&self)->DefaultContext {
        let mut context=self.scope.as_ref().map(|s|s.context.clone()).unwrap_or_default();
        context.project_id=self.context_project();
        if self.mode()==DisplayMode::Fullscreen{context.surface="chat".into();context.surface_path="/chat".into();context.task_id=None;context.view_name=None;return context;}
        if self.context_picker.scope=="all" || !self.include_context {context.task_id=None;context.view_name=None;}
        if let Some(project)=self.context_picker.scope.as_i64(){
            context.task_id=None;
            context.view_name=self.context_picker.teams.iter().flat_map(|team|team["projects"].as_array().into_iter().flatten()).find(|p|p["id"]==project).and_then(|p|p["title"].as_str().or(p["name"].as_str())).map(str::to_owned);
        }
        context
    }
    pub(super) fn context_label(&self)->String {
        if self.mode()==DisplayMode::Fullscreen{
            return self.context_picker.fullscreen_scope.as_i64().and_then(|id|self.context_picker.teams.iter().flat_map(|team|team["projects"].as_array().into_iter().flatten()).find(|p|p["id"]==id)).and_then(|p|p["title"].as_str().or(p["name"].as_str())).unwrap_or("All boards").into();
        }
        if self.context_picker.scope=="all" || !self.include_context{return "All boards".into();}
        if let Some(project)=self.context_picker.scope.as_i64(){
            return self.context_picker.teams.iter().flat_map(|team|team["projects"].as_array().into_iter().flatten()).find(|p|p["id"]==project).and_then(|p|p["title"].as_str().or(p["name"].as_str())).unwrap_or("Board").into();
        }
        self.scope.as_ref().map(|scope|scope.label.clone()).unwrap_or_else(||"Current board".into())
    }
    pub(super) fn open_scopes(&mut self,cx:&mut Context<Self>){
        self.context_picker.scope_open=!self.context_picker.scope_open;
        self.model_menu=false;
        if self.context_picker.scope_open && self.context_picker.scope_pending.is_none(){
            if let Some(token)=self.context_picker.token.clone(){
                let(send,receive)=mpsc::channel();self.context_picker.scope_pending=Some(receive);self.context_picker.scope_error=false;
                thread::spawn(move||{let result=Http::new(token).and_then(|mut http|ai_context::scopes(&mut http)).map_err(|e|e.to_string());let _=send.send(result);});
            }else{self.context_picker.scope_error=true;}
        }
        cx.notify();
    }
    pub(super) fn select_scope(&mut self,value:Value,cx:&mut Context<Self>){
        if value=="ticket" && (self.mode()==DisplayMode::Fullscreen || !self.context_picker.ticket_enabled || self.scope.as_ref().and_then(|s|s.context.task_id).is_none()){return;}
        if let Some(id)=value.as_i64(){
            if !self.context_picker.teams.iter().any(|team|team["projects"].as_array().into_iter().flatten().any(|p|p["id"]==id)){return;}
            self.context_picker.recent.retain(|old|*old!=id);self.context_picker.recent.insert(0,id);self.context_picker.recent.truncate(12);
        }
        if self.mode()==DisplayMode::Fullscreen{self.context_picker.fullscreen_scope=if value.is_number(){value}else{Value::Null};}else{self.context_picker.scope=value;}self.include_context=true;self.context_picker.scope_open=false;self.context_picker.mention=None;self.context_picker.pending=None;cx.notify();
    }
    pub(super) fn poll_context(&mut self,cx:&mut Context<Self>){
        if let Some(result)=self.context_picker.scope_pending.as_ref().and_then(|rx|rx.try_recv().ok()){
            self.context_picker.scope_pending=None;
            match result {Ok((user,teams,ticket)) if self.context_picker.user.is_none_or(|expected|expected==user)=>{self.context_picker.user=Some(user);self.context_picker.teams=teams;self.context_picker.ticket_enabled=ticket;self.context_picker.scope_error=false;},_=>{self.context_picker.teams.clear();self.context_picker.scope_error=true;}}
            cx.notify();
        }
        if self.context_picker.scope=="ticket" && (!self.context_picker.ticket_enabled || self.scope.as_ref().and_then(|s|s.context.task_id).is_none()){self.context_picker.scope=Value::Null;cx.notify();}
        let next=self.input.read(cx).mention_token().map(|(range,query)|(self.context_project(),range,query));
        if self.context_picker.mention!=next {
            self.context_picker.mention=next;self.context_picker.pending=None;self.context_picker.rows.clear();self.context_picker.choice=0;self.context_picker.dismissed=false;self.context_picker.fetched=false;self.context_picker.changed=Instant::now();cx.notify();
        }
        if self.context_picker.mention.is_none() || self.context_picker.dismissed{return;}
        if let Some(result)=self.context_picker.pending.as_ref().and_then(|rx|rx.try_recv().ok()){
            self.context_picker.pending=None;self.context_picker.fetched=true;
            self.context_picker.rows=result.unwrap_or_else(|_|vec![json!({"type":"error","name":"Error loading results"})]);
            self.context_picker.choice=self.context_picker.rows.iter().position(ai_context::selectable).unwrap_or(0);cx.notify();
        }
        if !self.context_picker.fetched && self.context_picker.pending.is_none() && self.context_picker.changed.elapsed()>=Duration::from_millis(180){
            if let Some(token)=self.context_picker.token.clone(){
                let(project,_,query)=self.context_picker.mention.clone().unwrap();let(send,receive)=mpsc::channel();self.context_picker.pending=Some(receive);
                thread::spawn(move||{let result=Http::new(token).and_then(|mut http|ai_context::mentions(&mut http,&query,project)).map_err(|e|e.to_string());let _=send.send(result);});
            }else{self.context_picker.fetched=true;self.context_picker.rows=vec![json!({"type":"error","name":"Error loading results"})];cx.notify();}
        }
    }
    pub(super) fn choose_context(&mut self,index:usize,window:&mut Window,cx:&mut Context<Self>){
        let Some(row)=self.context_picker.rows.get(index).filter(|row|ai_context::selectable(row)).cloned() else{return;};
        let Some((_,range,_))=self.context_picker.mention.clone() else{return;};
        let kind=row["type"].as_str().unwrap();let name=row["name"].as_str().unwrap_or("");
        let text=match kind {"task"=>format!("{} {name}",row["ticketNumber"].as_str().unwrap_or("")),"project"=>format!("{} {name}",row["identifier"].as_str().unwrap_or("")),_=>name.to_owned()};
        let escape=|text:&str|text.replace('&',"&amp;").replace('<',"&lt;").replace('>',"&gt;").replace('"',"&quot;");
        let attr=|v:&Value|escape(&v.as_str().map(str::to_owned).unwrap_or_else(||if v.is_null(){String::new()}else{v.to_string()}));
        let project=if kind=="project"{&row["id"]}else{&row["project_id"]};
        let html=format!("<span data-type=\"mention\" class=\"mention\" data-id=\"{}\" data-label=\"{kind}\" projectid=\"{}\" uniqueindex=\"{}\" text=\"{}\">{}</span>",attr(&row["id"]),attr(project),attr(&row["index"]),escape(&text),escape(&text));
        self.input.update(cx,|input,cx|input.replace_token(range,&format!("{text} "),window,cx));
        self.draft_quotes.push((text,html));
        if kind!="name" && !self.context_picker.selected.iter().any(|old|old["type"]==row["type"] && old["id"]==row["id"] && (kind!="task" || old["project_id"]==row["project_id"])) {self.context_picker.selected.push(row);}
        self.context_picker.mention=None;self.context_picker.pending=None;self.context_picker.rows.clear();cx.notify();
    }
    pub(super) fn context_key(&mut self,event:&KeyDownEvent,window:&mut Window,cx:&mut Context<Self>){
        if !self.input.focus_handle(cx).is_focused(window){return;}
        self.poll_context(cx);
        if self.context_picker.mention.is_none() || self.context_picker.dismissed{return;}
        match event.keystroke.key.as_str(){
            "escape"=>self.context_picker.dismissed=true,
            "up"|"down"=>{let rows=&self.context_picker.rows;let count=rows.len();if count>0{let direction=if event.keystroke.key=="down"{1}else{count-1};for step in 1..=count{let index=(self.context_picker.choice+step*direction)%count;if ai_context::selectable(&rows[index]){self.context_picker.choice=index;break;}}}},
            "enter" if !event.keystroke.modifiers.shift=>self.choose_context(self.context_picker.choice,window,cx),
            _=>return,
        }
        cx.stop_propagation();cx.notify();
    }
    pub(super) fn context_menu(&self,cx:&mut Context<Self>)->gpui::AnyElement {
        let mut menu=div().id("ai-context-menu").max_h(px(240.)).overflow_y_scroll().bg(color(self.colors.card));
        if self.context_picker.scope_open {
            let mut rows=vec![("All boards".to_owned(),json!("all"))];
            if self.mode()!=DisplayMode::Fullscreen{rows.insert(0,("Current board".into(),Value::Null));}
            if self.mode()!=DisplayMode::Fullscreen && self.context_picker.ticket_enabled && self.scope.as_ref().and_then(|s|s.context.task_id).is_some(){rows.insert(0,("Current ticket".into(),json!("ticket")));}
            for team in ai_context::sort_scopes(&self.context_picker.teams,&self.context_picker.recent){
                rows.push((team["title"].as_str().unwrap_or("").into(),json!({"heading":true})));
                for project in team["projects"].as_array().into_iter().flatten(){rows.push((project["title"].as_str().or(project["name"].as_str()).unwrap_or("").into(),project["id"].clone()));}
            }
            for(i,(name,value))in rows.into_iter().enumerate(){let heading=value["heading"]==true;menu=menu.child(div().id(("ai-scope-choice",i)).p(px(6.)).child(name).when(!heading,|d|d.cursor_pointer().on_click(cx.listener(move|this,_,_,cx|this.select_scope(value.clone(),cx)))));}
            if self.context_picker.scope_pending.is_some(){menu=menu.child("Loading…");}
            if self.context_picker.scope_error{menu=menu.child("Could not load boards");}
        }else if self.context_picker.mention.is_some() && !self.context_picker.dismissed {
            if !self.context_picker.fetched {menu=menu.child("Loading…");}
            for(i,row)in self.context_picker.rows.iter().enumerate(){menu=menu.child(div().id(("ai-context-choice",i)).p(px(6.)).when(i==self.context_picker.choice,|d|d.bg(color(self.colors.active))).child(row["name"].as_str().unwrap_or("").to_owned()).when(ai_context::selectable(row),|d|d.cursor_pointer().on_click(cx.listener(move|this,_,window,cx|this.choose_context(i,window,cx)))));}
        }
        menu.into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[gpui::test]
    fn context_picker_uses_server_groups_recency_flag_and_round_trips_scope(cx:&mut gpui::TestAppContext){
        cx.update(crate::input::init);
        let view=cx.add_window(|window,cx|{let focus=cx.focus_handle();AiSidebar::new(crate::theme::THEMES[1],None,focus,window,cx)});
        view.update(cx,|sidebar,_,cx|{
            let db=Db::memory().unwrap();db.kv_set("current-user-id","42").unwrap();sidebar.restore_context(&db);
            let(send,receive)=mpsc::channel();sidebar.context_picker.scope_pending=Some(receive);
            send.send(Ok((42,vec![json!({"id":"team","title":"Team","projects":[{"id":15,"title":"Board"},{"id":16,"title":"Other"}]})],false))).unwrap();sidebar.poll_context(cx);
            sidebar.select_scope(json!("ticket"),cx);assert!(sidebar.context_picker.scope.is_null());
            sidebar.select_scope(json!(16),cx);assert_eq!(sidebar.context_project(),Some(16));assert_eq!(sidebar.context_label(),"Other");assert_eq!(sidebar.context_picker.recent,vec![16]);sidebar.persist_context(&db).unwrap();
            sidebar.context_picker=ContextPicker::new(None);sidebar.restore_context(&db);assert_eq!(sidebar.context_picker.scope,16);assert_eq!(sidebar.context_picker.recent,vec![16]);
            sidebar.select_scope(json!("all"),cx);assert!(sidebar.request_context().project_id.is_none());assert!(sidebar.request_context().task_id.is_none());
            sidebar.context_picker.teams=vec![json!({"id":"team","projects":[{"id":16,"title":"Other"}]})];
            sidebar.set_mode(DisplayMode::Fullscreen,cx);assert!(sidebar.request_context().project_id.is_none());assert_eq!(sidebar.request_context().surface,"chat");sidebar.select_scope(json!(16),cx);assert_eq!(sidebar.request_context().project_id,Some(16));assert!(sidebar.request_context().task_id.is_none());
            sidebar.set_mode(DisplayMode::Sidebar,cx);assert!(sidebar.request_context().project_id.is_none());
        }).unwrap();
    }
    #[gpui::test]
    fn mentions_keep_server_ids_context_dedup_people_and_web_error_without_sending(cx:&mut gpui::TestAppContext){
        cx.update(crate::input::init);
        let view=cx.add_window(|window,cx|{let focus=cx.focus_handle();AiSidebar::new(crate::theme::THEMES[1],None,focus,window,cx)});
        view.update(cx,|sidebar,window,cx|{
            for row in [json!({"type":"task","id":7,"project_id":15,"index":2,"ticketNumber":"QA-2","name":"Café🙂"}),json!({"type":"project","id":15,"identifier":"QA","name":"Board"}),json!({"type":"agent","id":"agent-uuid","name":"Helper"}),json!({"type":"name","id":6,"name":"Human"})]{
                sidebar.input.update(cx,|input,cx|{input.set_content("@".into(),cx);use gpui::EntityInputHandler;input.set_selected_text_range(1..1,window,cx);});
                sidebar.poll_context(cx);let(send,receive)=mpsc::channel();sidebar.context_picker.pending=Some(receive);send.send(Ok(vec![json!({"type":"taskHeading","name":"Tasks"}),row.clone()])).unwrap();sidebar.poll_context(cx);assert_eq!(sidebar.context_picker.choice,1);
                sidebar.choose_context(1,window,cx);assert!(!sidebar.input.read(cx).content.contains('@'));assert!(sidebar.chats[0].messages.is_empty());assert!(sidebar.active.is_none());
                if row["type"]!="name"{assert!(sidebar.context_picker.selected.iter().any(|selected|selected==&row));}
            }
            assert_eq!(sidebar.context_picker.selected.len(),3);assert_eq!(sidebar.context_picker.selected[0]["project_id"],15);assert_eq!(sidebar.context_picker.selected[2]["id"],"agent-uuid");
            sidebar.input.update(cx,|input,cx|{input.set_content("@denied".into(),cx);use gpui::EntityInputHandler;input.set_selected_text_range(7..7,window,cx);});sidebar.poll_context(cx);
            let(send,receive)=mpsc::channel();sidebar.context_picker.pending=Some(receive);send.send(Err("HTTP 403".into())).unwrap();sidebar.poll_context(cx);assert_eq!(sidebar.context_picker.rows[0]["name"],"Error loading results");assert!(!ai_context::selectable(&sidebar.context_picker.rows[0]));assert_eq!(sidebar.input.read(cx).content.as_ref(),"@denied");
        }).unwrap();
    }
}
