use super::*;
use ht_core::{Http,ai_models::Configuration};
use serde_json::{Value,json};

pub(super) struct Models {
    token:Option<String>,
    team:Option<String>,
    pending:Option<Receiver<Result<Configuration,String>>>,
    pub(super) configuration:Option<Configuration>,
    pub(super) selected:Option<String>,
    previous:Option<String>,
    pub(super) write:Option<String>,
    failed:bool,
    pub(super) error:Option<String>,
    pending_team:Option<String>,
}
impl Models {
    pub(super) fn new(token:Option<String>)->Self{Self{token,team:None,pending:None,configuration:None,selected:None,previous:None,write:None,failed:false,error:None,pending_team:None}}
}
#[derive(Clone)]
pub enum PreferenceRequest { Cache{user:i64,settings:Value}, Save{user:i64,settings:Value,updates:Value} }
impl gpui::EventEmitter<PreferenceRequest> for AiSidebar{}
impl AiSidebar {
    pub(super) fn retry_models(&mut self,cx:&mut Context<Self>){self.models.failed=false;self.load_models(cx);}
    pub(super) fn load_models(&mut self,cx:&mut Context<Self>){
        let team=self.scope.as_ref().and_then(|s|s.team_id.clone());
        if team==self.models.team && (self.models.pending.is_some() || self.models.configuration.is_some() || self.models.failed){return;}
        self.models.team=team.clone();self.models.configuration=None;self.models.selected=None;self.models.pending=None;self.models.failed=false;self.models.error=None;
        if let(Some(team),Some(token))=(team,self.models.token.clone()){
            let board_default=self.scope.as_ref().and_then(|s|s.model_default.clone());
            let(send,receive)=mpsc::channel();self.models.pending=Some(receive);
            thread::spawn(move||{let result=Http::new(token).and_then(|mut http|ht_core::ai_models::load(&mut http,&team,board_default.as_deref())).map_err(|e|e.to_string());let _=send.send(result);});
        }
        cx.notify();
    }
    pub(super) fn poll_models(&mut self,cx:&mut Context<Self>){
        if let Some(result)=self.models.pending.as_ref().and_then(|rx|rx.try_recv().ok()){
            self.models.pending=None;
            match result{
                Ok(config)=>{
                    self.models.selected=Some(config.selected.clone());
                    cx.emit(PreferenceRequest::Cache{user:config.user,settings:config.settings.clone()});
                    self.models.configuration=Some(config);
                }
                Err(error)=>{self.models.failed=true;self.models.error=Some(error.clone());eprintln!("AI model settings unavailable: {error}");}
            }
            cx.notify();
        }
    }
    pub fn preference_key(&self)->Option<&str>{self.models.write.as_deref()}
    pub fn preference_queued(&mut self,key:String,cx:&mut Context<Self>){self.models.write=Some(key);cx.notify();}
    pub fn preference_finished(&mut self,result:Value,cx:&mut Context<Self>){
        self.models.write=None;
        if self.models.pending_team.take()!=self.models.team {self.models.previous=None;cx.notify();return;}
        if !result["error"].is_null(){self.models.selected=self.models.previous.take();eprintln!("AI model preference refused: {}",result["error"]);}
        else{self.models.previous=None;if let Some(config)=&mut self.models.configuration{if result["settings"].is_object(){config.settings=result["settings"].clone();}}}
        cx.notify();
    }
    pub(super) fn choose_model(&mut self,id:&str,cx:&mut Context<Self>){
        if self.models.write.is_some() || self.models.previous.is_some(){return;}
        let Some(config)=&self.models.configuration else{return;};
        let Some(model)=config.options.iter().find(|model|model["id"]==id) else{return;};
        if config.locked(model){cx.open_url("https://app.hypertask.ai/settings/plans");self.model_menu=false;cx.notify();return;}
        self.models.pending_team=self.models.team.clone();
        self.models.previous=self.models.selected.clone();self.models.selected=Some(id.into());self.model_menu=false;
        cx.emit(PreferenceRequest::Save{user:config.user,settings:config.settings.clone(),updates:json!({"aiModelPreferences":{"aiChat":id},"aiModelPreferencesTeamId":config.team})});cx.notify();
    }
    pub(super) fn model_label(&self)->String {
        self.models.selected.as_deref().and_then(ht_core::ai_models::option).and_then(|m|m["title"].as_str()).unwrap_or(MODELS[self.model].label).into()
    }
    pub(super) fn model_options(&self,cx:&mut Context<Self>)->gpui::AnyElement {
        let mut menu=div().id("ai-model-options").max_h(px(320.)).overflow_y_scroll();
        if let Some(config)=&self.models.configuration {
            let mut groups:Vec<String>=vec![];
            for model in &config.options {
                let group=model["definition"]["provider"].as_str().unwrap_or("").to_owned();
                if !groups.contains(&group){groups.push(group);}
            }
            for group in &groups{
                menu=menu.child(div().text_size(px(11.)).p(px(6.)).child(match group.as_str(){"openai"=>"OpenAI","anthropic"=>"Anthropic","google"=>"Google","deepseek"=>"DeepSeek","moonshot"=>"Moonshot (Kimi)","alibaba"=>"Alibaba (Qwen)","zhipu"=>"Z.ai (GLM)","custom"=>"Custom endpoint",_=>group.as_str()}.to_owned()));
                for(index,model)in config.options.iter().enumerate().filter(|(_,m)|m["definition"]["provider"]==*group){
                    let id=model["id"].as_str().unwrap().to_owned();let locked=config.locked(model);
                    menu=menu.child(div().id(("ai-model",index)).p(px(6.)).text_size(px(12.)).cursor_pointer().when(self.models.selected.as_deref()==Some(id.as_str()),|d|d.bg(color(self.colors.active))).child(model["title"].as_str().unwrap_or("").to_owned()).when(locked,|d|d.child(" · Paid plan or BYOK")).on_click(cx.listener(move|this,_,_,cx|this.choose_model(&id,cx))));
                }
            }
        }
        menu.into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[gpui::test]
    fn model_choice_queues_web_preference_and_both_refusals_restore_pick(cx:&mut gpui::TestAppContext){
        let view=cx.add_window(|window,cx|{let focus=cx.focus_handle();AiSidebar::new(crate::theme::THEMES[1],None,focus,window,cx)});
        view.update(cx,|sidebar,_,cx|{
            sidebar.models.team=Some("team".into());sidebar.models.selected=Some("gpt-6-luna".into());
            sidebar.models.configuration=Some(Configuration{user:42,team:"team".into(),options:ht_core::ai_models::catalog().to_vec(),selected:"gpt-6-luna".into(),settings:json!({}),byok:json!([]),plan:Some("Pro".into())});
            for status in [401,403,200]{
                sidebar.choose_model("claude-opus-5-5-thinking",cx);assert_eq!(sidebar.models.selected.as_deref(),Some("claude-opus-5-5-thinking"));sidebar.preference_queued("key".into(),cx);assert_eq!(sidebar.preference_key(),Some("key"));
                sidebar.preference_finished(if status==200{json!({"success":true,"settings":{"aiModelPreferences":{"teams":{"team":{"aiChat":"claude-opus-5-5-thinking"}}}}})}else{json!({"error":format!("HTTP {status}")})},cx);
                assert_eq!(sidebar.models.selected.as_deref(),Some(if status==200{"claude-opus-5-5-thinking"}else{"gpt-6-luna"}));assert!(sidebar.preference_key().is_none());assert!(sidebar.chats[0].messages.is_empty());
            }
        }).unwrap();
    }
}

#[cfg(test)]
mod refusal_tests {
    use super::*;
    #[gpui::test]
    fn denied_model_configuration_never_sends_a_guessed_fallback_or_clears_the_note(cx:&mut gpui::TestAppContext){
        let view=cx.add_window(|window,cx|{let focus=cx.focus_handle();AiSidebar::new(crate::theme::THEMES[1],Some("mock-token".into()),focus,window,cx)});
        view.update(cx,|sidebar,window,cx|{
            let mut db=Db::memory().unwrap();db.apply_response(&json!({"success":true,"projects":[{"id":15,"title":"Board","teamId":"real-team-id"}]})).unwrap();sidebar.scope=Some(Scope::load(&db,Screen::Board,15,None));
            sidebar.models.error=Some("Forbidden".into());sidebar.input.update(cx,|input,cx|input.set_content("Keep this note".into(),cx));sidebar.send("Keep this note".into(),window,cx);
            assert!(sidebar.active.is_none());assert!(sidebar.session_pending.is_none());assert_eq!(sidebar.input.read(cx).content.as_ref(),"Keep this note");assert_eq!(sidebar.chats[0].error.as_deref(),Some("Forbidden"));
        }).unwrap();
    }
}
