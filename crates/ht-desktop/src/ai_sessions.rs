use super::*;
use ht_core::AiSessionWrite;
use serde_json::{Value, json};

#[derive(Clone)]
pub struct SessionRequest {
    pub change: AiSessionWrite,
    pub sessions: Vec<Value>,
    pub continuation: Value,
}
impl gpui::EventEmitter<SessionRequest> for AiSidebar {}
pub(super) struct SessionPending {
    key: Option<String>,
    change: AiSessionWrite,
    chats: Vec<Chat>,
    history: Vec<Arc<Session>>,
    selected: usize,
    draft: String,
    quotes: Vec<(String,String)>,
    branch: Option<String>,
    send: Option<String>,
    context: DefaultContext,
}
impl AiSidebar {
    pub fn session_snapshots(&self)->Vec<Value> {
        let mut rows:Vec<Value>=self.history.iter().map(|s|serde_json::to_value(s.as_ref()).unwrap()).collect();
        for chat in self.chats.iter().filter(|chat|chat.persisted) {
            let session=json!({"id":chat.id,"title":chat.title,"messages":chat.messages.iter().map(|m|&m.message).collect::<Vec<_>>()});
            if let Some(row)=rows.iter_mut().find(|r|r["id"]==chat.id){*row=session;}else{rows.push(session);}
        }
        rows
    }
    pub fn restore_sessions(&mut self,db:&Db,cx:&mut Context<Self>) {
        if let Ok(rows)=db.ai_sessions(){
            self.history=rows.into_iter().filter_map(|row|serde_json::from_value(row).ok()).map(Arc::new).collect();
        }
        cx.notify();
    }
    pub(super) fn request_session(&mut self,change:AiSessionWrite,branch:Option<String>,send:Option<String>,cx:&mut Context<Self>) {
        if self.session_pending.is_some() || self.active.is_some(){return;}
        if self.client.is_none(){return;}
        if change.request().is_err(){return;}
        let sessions=self.session_snapshots();
        self.session_pending=Some(SessionPending{key:None,change:change.clone(),chats:self.chats.clone(),history:self.history.clone(),selected:self.selected,draft:self.input.read(cx).content.to_string(),quotes:self.draft_quotes.clone(),branch,send,context:self.request_context()});
        self.chat_pending=None;self.history_pending=None;
        match &change {
            AiSessionWrite::Create{..}=>{if !self.chats[self.selected].persisted && self.chats[self.selected].messages.is_empty(){self.chats[self.selected]=Chat::new();}else{self.chats.push(Chat::new());self.selected=self.chats.len()-1;}}
            AiSessionWrite::Rename{id,title}=>{
                for chat in &mut self.chats {if chat.id==*id {chat.title=title.trim().into();}}
                for session in &mut self.history {if session.id==*id {Arc::make_mut(session).title=Some(title.trim().into());}}
            }
            AiSessionWrite::Delete{id}=>{self.chats.retain(|c|c.id!=*id);self.history.retain(|s|s.id!=*id);self.selected=self.selected.min(self.chats.len().saturating_sub(1));}
            AiSessionWrite::DeleteAll=>{self.chats.clear();self.history.clear();self.selected=0;}
        }
        if self.chats.is_empty(){self.chats.push(Chat::new());}
        self.more_menu=false;self.history_menu=false;self.rename_open=false;
        let continuation=self.session_continuation();
        cx.emit(SessionRequest{change,sessions,continuation});cx.notify();
    }
    fn session_continuation(&self)->Value{
        let Some(p)=&self.session_pending else{return Value::Null;};
        json!({"chats":p.chats.iter().map(|c|json!({"id":c.id,"title":c.title,"messages":c.messages.iter().map(|m|&m.message).collect::<Vec<_>>(),"persisted":c.persisted})).collect::<Vec<_>>(),"history":p.history.iter().map(|s|s.as_ref()).collect::<Vec<_>>(),"selected":p.selected,"draft":p.draft,"quotes":p.quotes,"branch":p.branch,"send":p.send,"context":p.context})
    }
    pub fn restore_session_work(&mut self,db:&Db,cx:&mut Context<Self>){
        let Some(user)=db.current_user_id() else{return;};
        let Some(work)=db.kv_get(&format!("ai-session-pending:{user}")).ok().flatten().and_then(|s|serde_json::from_str::<Value>(&s).ok()).filter(Value::is_object) else{return;};
        let Some(key)=work["key"].as_str() else{return;};
        let Ok(change)=serde_json::from_value::<AiSessionWrite>(work["change"].clone()) else{return;};
        let saved=&work["continuation"];
        let chats:Vec<Chat>=saved["chats"].as_array().into_iter().flatten().filter_map(|v|serde_json::from_value::<Session>(v.clone()).ok().map(|session|{let mut chat=self.chat_from_session(&session);chat.persisted=v["persisted"]==true;chat})).collect();
        if chats.is_empty(){return;}
        let selected=(saved["selected"].as_u64().unwrap_or(0) as usize).min(chats.len()-1);
        self.chats=chats.clone();self.selected=selected;
        self.input.update(cx,|input,cx|input.set_content(saved["draft"].as_str().unwrap_or("").into(),cx));
        self.session_pending=Some(SessionPending{key:Some(key.into()),change,chats,history:saved["history"].as_array().into_iter().flatten().filter_map(|v|serde_json::from_value::<Session>(v.clone()).ok()).map(Arc::new).collect(),selected,draft:saved["draft"].as_str().unwrap_or("").into(),quotes:serde_json::from_value(saved["quotes"].clone()).unwrap_or_default(),branch:saved["branch"].as_str().map(str::to_owned),send:saved["send"].as_str().map(str::to_owned),context:serde_json::from_value(saved["context"].clone()).unwrap_or_default()});
        cx.notify();
    }
    pub fn session_queued(&mut self,key:String,cx:&mut Context<Self>){if let Some(pending)=&mut self.session_pending{pending.key=Some(key);}cx.notify();}
    pub fn session_key(&self)->Option<&str>{self.session_pending.as_ref()?.key.as_deref()}
    pub fn session_finished(&mut self,result:Value,window:&mut Window,cx:&mut Context<Self>) {
        let Some(pending)=self.session_pending.take() else{return;};
        let current_draft=self.input.read(cx).content.to_string();let current_quotes=self.draft_quotes.clone();let unchanged=current_draft==pending.draft;
        if !result["error"].is_null() {
            self.chats=pending.chats;self.history=pending.history;self.selected=pending.selected;
            if unchanged{self.draft_quotes=pending.quotes;self.input.update(cx,|input,cx|input.set_content(pending.draft,cx));}
            // Web history hooks roll back/refetch without an error toast on rename/delete failure.
            eprintln!("AI session mutation refused: {}",result["error"]);
        }else{
            match pending.change {
                AiSessionWrite::Create{..}=>{
                    if let Ok(session)=serde_json::from_value::<Session>(result["session"].clone()) {
                        if pending.context!=self.request_context(){
                            self.history.insert(0,Arc::new(session));self.chats=pending.chats;self.selected=pending.selected;cx.notify();return;
                        }
                        let chat=self.chat_from_session(&session);
                        if self.chats[self.selected].persisted{self.chats.push(chat);self.selected=self.chats.len()-1;}else{self.chats[self.selected]=chat;}
                        self.history.insert(0,Arc::new(session));
                        if unchanged{self.draft_quotes.clear();self.input.update(cx,|input,cx|input.reset(window,cx));}
                        if let Some(quote)=pending.branch{self.insert_comment_quote(quote,window,cx);}
                        if let Some(text)=pending.send {
                            if unchanged{self.input.update(cx,|input,cx|input.set_content(pending.draft,cx));}
                            self.draft_quotes=pending.quotes;
                            self.send(text,window,cx);
                            if !unchanged{self.input.update(cx,|input,cx|input.set_content(current_draft,cx));self.draft_quotes=current_quotes;}
                        }
                    }
                }
                AiSessionWrite::Delete{id}=>{self.chats.retain(|c|c.id!=id);self.history.retain(|s|s.id!=id);if self.chats.is_empty(){self.chats.push(Chat::new());}self.selected=self.selected.min(self.chats.len()-1);self.load_history(false,cx);},
                AiSessionWrite::DeleteAll=>{self.chats=vec![Chat::new()];self.chats[0].status="All chat sessions deleted successfully".into();self.selected=0;self.history.clear();self.load_history(false,cx);},
                AiSessionWrite::Rename{id,..}=>{if let Some(title)=result["session"]["title"].as_str(){for chat in &mut self.chats{if chat.id==id{chat.title=title.into();}}for row in &mut self.history{if row.id==id{Arc::make_mut(row).title=Some(title.into());}}}},
            }
        }
        cx.notify();
    }
    fn chat_from_session(&self,session:&Session)->Chat {
        Chat{id:session.id.clone(),title:session.title.clone().unwrap_or_else(||"AI Chat".into()),messages:session.messages.iter().map(|message|ChatMessage{message:message.clone(),rich:if message.role=="human"{RichText::default()}else{markdown(&message.content,self.colors)}}).collect(),status:String::new(),error:None,persisted:true}
    }
    pub(super) fn create_scope(&self)->AiSessionWrite {
        AiSessionWrite::Create{task:self.scope.as_ref().and_then(|s|s.context.task_id),project:self.scope.as_ref().and_then(|s|s.context.project_id)}
    }
    pub fn branch_comment(&mut self,quote:String,window:&mut Window,cx:&mut Context<Self>){
        self.set_open(true,cx);
        self.request_session(self.create_scope(),Some(quote),None,cx);
        window.focus(&self.input.focus_handle(cx),cx);
    }
    pub fn delete_all_sessions(&mut self,cx:&mut Context<Self>){self.request_session(AiSessionWrite::DeleteAll,None,None,cx);}
    pub(super) fn rename_session(&mut self,cx:&mut Context<Self>){
        let chat=&self.chats[self.selected];
        if !chat.persisted{return;}
        let title=self.rename_input.read(cx).content.trim().to_owned();
        if title.is_empty(){return;}
        self.request_session(AiSessionWrite::Rename{id:chat.id.clone(),title},None,None,cx);
    }
    pub(super) fn session_controls(&self,cx:&mut Context<Self>)->gpui::AnyElement {
        if !self.rename_open{return div().into_any_element();}
        div().id("ai-rename-dialog").p(px(16.)).bg(color(self.colors.card)).child("Rename chat").child(self.rename_input.clone())
            .child(div().id("ai-rename-save").p(px(6.)).cursor_pointer().child("Save").on_click(cx.listener(|this,_,_,cx|this.rename_session(cx))))
            .child(div().id("ai-rename-cancel").p(px(6.)).cursor_pointer().child("Cancel").on_click(cx.listener(|this,_,_,cx|{this.rename_open=false;cx.notify();}))).into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[gpui::test]
    fn create_and_branch_wait_for_real_ack_and_denial_restores_composer(cx:&mut gpui::TestAppContext){
        let view=cx.add_window(|window,cx|{let focus=cx.focus_handle();AiSidebar::new(crate::theme::THEMES[1],Some("mock-token".into()),focus,window,cx)});
        view.update(cx,|sidebar,window,cx|{
            sidebar.input.update(cx,|input,cx|input.set_content("Keep draft".into(),cx));
            let original=sidebar.chats[0].id.clone();
            sidebar.branch_comment("<blockquote>Branch quote</blockquote>".into(),window,cx);
            assert!(sidebar.session_pending.is_some());assert!(!sidebar.chats[sidebar.selected].persisted);
            assert_eq!(sidebar.input.read(cx).content.as_ref(),"Keep draft");
            sidebar.session_finished(json!({"error":"HTTP 403"}),window,cx);
            assert_eq!(sidebar.chats[sidebar.selected].id,original);assert_eq!(sidebar.input.read(cx).content.as_ref(),"Keep draft");assert!(sidebar.draft_quotes.is_empty());
            sidebar.branch_comment("<blockquote>Branch quote</blockquote>".into(),window,cx);
            sidebar.session_finished(json!({"success":true,"session":{"id":"server-chat","title":"Branch","messages":[]}}),window,cx);
            assert_eq!(sidebar.chats[sidebar.selected].id,"server-chat");assert!(sidebar.chats[sidebar.selected].persisted);assert_eq!(sidebar.input.read(cx).content.as_ref(),"Branch quote");assert!(sidebar.prompt_body("Branch quote").contains("<blockquote>Branch quote</blockquote>"));
        }).unwrap();
    }
    #[gpui::test]
    fn rename_delete_and_delete_all_optimism_roll_back_without_success_on_refusal(cx:&mut gpui::TestAppContext){
        let view=cx.add_window(|window,cx|{let focus=cx.focus_handle();AiSidebar::new(crate::theme::THEMES[1],Some("mock-token".into()),focus,window,cx)});
        view.update(cx,|sidebar,window,cx|{
            sidebar.chats[0].id="saved".into();sidebar.chats[0].persisted=true;sidebar.chats[0].title="Original".into();
            for status in [401,403,200]{
                sidebar.rename_input.update(cx,|input,cx|input.set_content("Renamed".into(),cx));sidebar.rename_session(cx);assert_eq!(sidebar.chats[0].title,"Renamed");
                sidebar.session_finished(if status==200{json!({"success":true,"session":{"id":"saved","title":"Renamed"}})}else{json!({"error":format!("HTTP {status}")})},window,cx);
                assert_eq!(sidebar.chats[0].title,if status==200{"Renamed"}else{"Original"});
            }
            for change in [AiSessionWrite::Delete{id:"saved".into()},AiSessionWrite::DeleteAll]{
                for status in [401,403]{
                    sidebar.request_session(change.clone(),None,None,cx);assert!(!sidebar.chats.iter().any(|c|c.id=="saved"));
                    sidebar.session_finished(json!({"error":format!("HTTP {status}")}),window,cx);assert!(sidebar.chats.iter().any(|c|c.id=="saved"));
                }
            }
        }).unwrap();
    }
}

#[cfg(test)]
mod recovery_tests {
    use super::*;
    #[gpui::test]
    fn restarted_branch_preserves_draft_on_refusal_and_inserts_quote_only_on_acceptance(cx:&mut gpui::TestAppContext){
        let view=cx.add_window(|window,cx|{let focus=cx.focus_handle();AiSidebar::new(crate::theme::THEMES[1],Some("mock-token".into()),focus,window,cx)});
        view.update(cx,|sidebar,window,cx|{
            for status in [401,403,200]{
                let mut db=Db::memory().unwrap();db.kv_set("current-user-id","42").unwrap();
                sidebar.set_token(Some("mock-token".into()),cx);sidebar.input.update(cx,|input,cx|input.set_content("Original draft".into(),cx));
                sidebar.branch_comment("<blockquote>Branch quote</blockquote>".into(),window,cx);
                let change=sidebar.session_pending.as_ref().unwrap().change.clone();let key=db.enqueue_ai_session(change,&[],&sidebar.session_continuation(),1).unwrap();
                sidebar.set_token(Some("mock-token".into()),cx);sidebar.restore_sessions(&db,cx);sidebar.restore_session_work(&db,cx);
                assert_eq!(sidebar.session_key(),Some(key.as_str()));assert_eq!(sidebar.input.read(cx).content.as_ref(),"Original draft");
                sidebar.session_finished(if status==200{json!({"success":true,"session":{"id":"durable-chat","title":"Branch","messages":[]}})}else{json!({"error":format!("HTTP {status}")})},window,cx);
                assert_eq!(sidebar.input.read(cx).content.as_ref(),if status==200{"Branch quote"}else{"Original draft"});assert_eq!(sidebar.chats[sidebar.selected].persisted,status==200);
            }
        }).unwrap();
    }
}

#[cfg(test)]
mod intent_tests {
    use super::*;
    #[gpui::test]
    fn refused_session_write_preserves_newer_typing_and_late_branch_does_not_hijack_navigation(cx:&mut gpui::TestAppContext){
        let view=cx.add_window(|window,cx|{let focus=cx.focus_handle();AiSidebar::new(crate::theme::THEMES[1],Some("mock-token".into()),focus,window,cx)});
        view.update(cx,|sidebar,window,cx|{
            sidebar.input.update(cx,|input,cx|input.set_content("Captured".into(),cx));sidebar.branch_comment("<blockquote>Old quote</blockquote>".into(),window,cx);sidebar.input.update(cx,|input,cx|input.set_content("Newer edits".into(),cx));sidebar.session_finished(json!({"error":"HTTP 403"}),window,cx);assert_eq!(sidebar.input.read(cx).content.as_ref(),"Newer edits");
            let db=Db::memory().unwrap();sidebar.scope=Some(Scope::load(&db,Screen::Board,15,None));let original=sidebar.chats[sidebar.selected].id.clone();sidebar.branch_comment("<blockquote>Old board quote</blockquote>".into(),window,cx);
            sidebar.scope=Some(Scope::load(&db,Screen::Board,16,None));sidebar.session_finished(json!({"success":true,"session":{"id":"old-board-chat","title":"Old board","messages":[]}}),window,cx);
            assert_eq!(sidebar.chats[sidebar.selected].id,original);assert_eq!(sidebar.input.read(cx).content.as_ref(),"Newer edits");assert!(sidebar.draft_quotes.is_empty());assert!(sidebar.history.iter().any(|s|s.id=="old-board-chat"));
        }).unwrap();
    }
}
