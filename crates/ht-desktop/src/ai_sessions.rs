use super::*;
use ht_core::AiSessionWrite;
use serde_json::{Value, json};

#[derive(Clone)]
pub struct SessionRequest {
    pub change: AiSessionWrite,
    pub sessions: Vec<Value>,
}
impl gpui::EventEmitter<SessionRequest> for AiSidebar {}
struct SessionPending {
    key: Option<String>,
    change: AiSessionWrite,
    chats: Vec<Chat>,
    history: Vec<Arc<Session>>,
    selected: usize,
    draft: String,
    quotes: Vec<(String,String)>,
    branch: Option<String>,
    send: Option<String>,
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
    fn request_session(&mut self,change:AiSessionWrite,branch:Option<String>,send:Option<String>,cx:&mut Context<Self>) {
        if self.session_pending.is_some() || self.active.is_some(){return;}
        if self.client.is_none(){return;}
        if change.request().is_err(){return;}
        let sessions=self.session_snapshots();
        self.session_pending=Some(SessionPending{key:None,change:change.clone(),chats:self.chats.clone(),history:self.history.clone(),selected:self.selected,draft:self.input.read(cx).content.to_string(),quotes:self.draft_quotes.clone(),branch,send});
        self.chat_pending=None;self.history_pending=None;
        match &change {
            AiSessionWrite::Create{..}=>{self.chats.push(Chat::new());self.selected=self.chats.len()-1;}
            AiSessionWrite::Rename{id,title}=>{
                for chat in &mut self.chats {if chat.id==*id {chat.title=title.trim().into();}}
                for session in &mut self.history {if session.id==*id {Arc::make_mut(session).title=Some(title.trim().into());}}
            }
            AiSessionWrite::Delete{id}=>{self.chats.retain(|c|c.id!=*id);self.history.retain(|s|s.id!=*id);self.selected=self.selected.min(self.chats.len().saturating_sub(1));}
            AiSessionWrite::DeleteAll=>{self.chats.clear();self.history.clear();self.selected=0;}
        }
        if self.chats.is_empty(){self.chats.push(Chat::new());}
        self.more_menu=false;self.history_menu=false;self.rename_open=false;
        cx.emit(SessionRequest{change,sessions});cx.notify();
    }
    pub fn session_queued(&mut self,key:String,cx:&mut Context<Self>){if let Some(pending)=&mut self.session_pending{pending.key=Some(key);}cx.notify();}
    pub fn session_key(&self)->Option<&str>{self.session_pending.as_ref()?.key.as_deref()}
    pub fn session_finished(&mut self,result:Value,window:&mut Window,cx:&mut Context<Self>) {
        let Some(pending)=self.session_pending.take() else{return;};
        if !result["error"].is_null() {
            self.chats=pending.chats;self.history=pending.history;self.selected=pending.selected;
            self.draft_quotes=pending.quotes;
            self.input.update(cx,|input,cx|input.set_content(pending.draft,cx));
            // Web history hooks roll back/refetch without an error toast on rename/delete failure.
            eprintln!("AI session mutation refused: {}",result["error"]);
        }else{
            match pending.change {
                AiSessionWrite::Create{..}=>{
                    if let Ok(session)=serde_json::from_value::<Session>(result["session"].clone()) {
                        self.chats[self.selected]=self.chat_from_session(&session);
                        self.history.insert(0,Arc::new(session));
                        self.draft_quotes.clear();self.input.update(cx,|input,cx|input.reset(window,cx));
                        if let Some(quote)=pending.branch{self.insert_comment_quote(quote,window,cx);}
                        if let Some(text)=pending.send {
                            self.draft_quotes=pending.quotes;
                            self.send(text,window,cx);
                        }
                    }
                }
                AiSessionWrite::Delete{..}|AiSessionWrite::DeleteAll=>self.load_history(false,cx),
                _=>(),
            }
        }
        cx.notify();
    }
    fn chat_from_session(&self,session:&Session)->Chat {
        Chat{id:session.id.clone(),title:session.title.clone().unwrap_or_else(||"AI Chat".into()),messages:session.messages.iter().map(|message|ChatMessage{message:message.clone(),rich:if message.role=="human"{RichText::default()}else{markdown(&message.content,self.colors)}}).collect(),status:String::new(),error:None,persisted:true}
    }
    fn create_scope(&self)->AiSessionWrite {
        AiSessionWrite::Create{task:self.scope.as_ref().and_then(|s|s.context.task_id),project:self.scope.as_ref().and_then(|s|s.context.project_id)}
    }
    pub fn branch_comment(&mut self,quote:String,window:&mut Window,cx:&mut Context<Self>){
        self.set_open(true,cx);
        self.request_session(self.create_scope(),Some(quote),None,cx);
        window.focus(&self.input.focus_handle(cx),cx);
    }
    pub fn delete_all_sessions(&mut self,cx:&mut Context<Self>){self.request_session(AiSessionWrite::DeleteAll,None,None,cx);}
    fn rename_session(&mut self,cx:&mut Context<Self>){
        let chat=&self.chats[self.selected];
        if !chat.persisted{return;}
        let title=self.rename_input.read(cx).content.trim().to_owned();
        if title.is_empty(){return;}
        self.request_session(AiSessionWrite::Rename{id:chat.id.clone(),title},None,None,cx);
    }
    fn session_controls(&self,cx:&mut Context<Self>)->gpui::AnyElement {
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
