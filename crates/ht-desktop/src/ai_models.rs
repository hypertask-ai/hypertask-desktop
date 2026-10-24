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
    requested:Option<(Option<String>,Option<String>,Option<String>)>,
    pub(super) submenu:Option<bool>,
}
impl Models {
    pub(super) fn new(token:Option<String>)->Self{Self{token,team:None,pending:None,configuration:None,selected:None,previous:None,write:None,failed:false,error:None,pending_team:None,requested:None,submenu:None}}
}
#[derive(Clone)]
pub enum PreferenceRequest { Cache{user:i64,settings:Value}, Save{user:i64,settings:Value,updates:Value} }
impl gpui::EventEmitter<PreferenceRequest> for AiSidebar{}
impl AiSidebar {
    pub(super) fn retry_models(&mut self,cx:&mut Context<Self>){self.models.failed=false;self.load_models(cx);}
    pub(super) fn load_models(&mut self,cx:&mut Context<Self>){
        let team=self.scope.as_ref().and_then(|s|s.team_id.clone());
        let last=self.scope.as_ref().and_then(|s|s.last_team.clone());
        let board_default=self.scope.as_ref().and_then(|s|s.model_default.clone());
        let requested=(team.clone(),last.clone(),board_default.clone());
        if self.models.requested.as_ref()==Some(&requested) && (self.models.pending.is_some() || self.models.configuration.is_some() || self.models.failed){return;}
        self.models.requested=Some(requested);self.models.team=team.clone();self.models.configuration=None;self.models.selected=None;self.models.pending=None;self.models.failed=false;self.models.error=None;
        if let Some(token)=self.models.token.clone(){
            let(send,receive)=mpsc::channel();self.models.pending=Some(receive);
            thread::spawn(move||{let result=Http::new(token).and_then(|mut http|ht_core::ai_models::load_for_scope(&mut http,team.as_deref(),last.as_deref(),board_default.as_deref())).map_err(|e|e.to_string());let _=send.send(result);});
        }
        cx.notify();
    }
    pub(super) fn poll_models(&mut self,cx:&mut Context<Self>){
        if let Some(result)=self.models.pending.as_ref().and_then(|rx|rx.try_recv().ok()){
            self.models.pending=None;
            match result{
                Ok(config)=>{
                    self.models.team=Some(config.team.clone());
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
        if config.disabled(model){return;}
        if config.locked(model){cx.open_url("https://app.hypertask.ai/settings/plans");self.model_menu=false;cx.notify();return;}
        self.models.pending_team=self.models.team.clone();
        self.models.previous=self.models.selected.clone();self.models.selected=Some(id.into());self.models.submenu=None;
        cx.emit(PreferenceRequest::Save{user:config.user,settings:config.settings.clone(),updates:json!({"aiModelPreferences":{"aiChat":id},"aiModelPreferencesTeamId":config.team})});cx.notify();
    }
    pub(super) fn model_label(&self)->String {
        if let (Some(config),Some(model))=(&self.models.configuration,self.models.selected.as_deref().and_then(ht_core::ai_models::option)) {
            let effort=if self.mode()==DisplayMode::Fullscreen{ht_core::ai_models::effort_label(model)}else{""};
            return format!("{}{}{}",config.label(model),if effort.is_empty(){""}else{" · "},effort);
        }
        MODELS[self.model].label.into()
    }
    fn choose_family(&mut self,family:&str,cx:&mut Context<Self>){
        let effort=self.models.selected.as_deref().and_then(ht_core::ai_models::option).and_then(|model|model["effort"].as_str());
        let id=self.models.configuration.as_ref().and_then(|config|config.nearest(family,effort)).and_then(|model|model["id"].as_str()).map(str::to_owned);
        if let Some(id)=id{self.choose_model(&id,cx);}
    }
    pub(super) fn model_options(&self,cx:&mut Context<Self>)->gpui::AnyElement {
        let mut menu=div().id("ai-model-options").w(px(250.)).p(px(6.));
        if let Some(config)=&self.models.configuration {
            let selected=self.models.selected.as_deref().and_then(ht_core::ai_models::option);
            let family=selected.and_then(|model|model["family"].as_str()).unwrap_or("");
            let no_keys=config.plan.as_deref()==Some("BYOK") && !config.options.iter().any(|model|!config.locked(model));
            if no_keys{
                menu=menu.child(div().p(px(8.)).text_size(px(11.)).child("No API keys are enabled for this team.").child(div().id("ai-model-api-keys").cursor_pointer().child("Add API keys →").on_click(cx.listener(|this,_,_,cx|{this.model_menu=false;cx.open_url("https://app.hypertask.ai/settings/apiKeys");cx.notify();}))));
            }
            menu=menu.child(div().id("ai-model-submenu").p(px(8.)).cursor_pointer().child(format!("Model    {}  ›",selected.map(|model|config.label(model)).unwrap_or("Select model"))).on_click(cx.listener(|this,_,_,cx|{this.models.submenu=if this.models.submenu==Some(false){None}else{Some(false)};cx.notify();})));
            if self.models.submenu==Some(false){
                let mut models=div().id("ai-model-families").max_h(px(360.)).overflow_y_scroll();
                let mut provider=String::new();
                for(index,model)in config.families().into_iter().enumerate(){
                    let group=model["definition"]["provider"].as_str().unwrap_or("");
                    if group!=provider {provider=group.into();models=models.child(div().text_size(px(11.)).p(px(6.)).child(match group{"openai"=>"OpenAI","anthropic"=>"Anthropic","google"=>"Google","deepseek"=>"DeepSeek","moonshot"=>"Moonshot (Kimi)","alibaba"=>"Alibaba (Qwen)","zhipu"=>"Z.ai (GLM)","custom"=>"Custom",_=>group}.to_owned()));}
                    let key=model["family"].as_str().unwrap_or("").to_owned();let locked=config.locked(model);let disabled=config.disabled(model);
                    models=models.child(div().id(("ai-model",index)).p(px(8.)).text_size(px(12.)).when(!disabled,|d|d.cursor_pointer().on_click(cx.listener(move|this,_,_,cx|this.choose_family(&key,cx)))).when(model["family"]==family,|d|d.bg(color(self.colors.active))).child(config.label(model).to_owned()).when(locked,|d|d.child(" · Paid plan or BYOK")).child("$".repeat(model["definition"]["priceTier"].as_u64().unwrap_or(0) as usize)));
                }
                menu=menu.child(models);
            }
            let efforts=config.efforts(family);
            if !efforts.is_empty(){
                menu=menu.child(div().id("ai-effort-submenu").p(px(8.)).cursor_pointer().child(format!("Effort    {}  ›",selected.map(ht_core::ai_models::effort_label).unwrap_or("Select effort"))).on_click(cx.listener(|this,_,_,cx|{this.models.submenu=if this.models.submenu==Some(true){None}else{Some(true)};cx.notify();})));
                if self.models.submenu==Some(true){for(index,model)in efforts.into_iter().enumerate(){let id=model["id"].as_str().unwrap().to_owned();menu=menu.child(div().id(("ai-effort",index)).p(px(8.)).cursor_pointer().when(self.models.selected.as_deref()==Some(id.as_str()),|d|d.bg(color(self.colors.active))).child(format!("{} · {}",ht_core::ai_models::effort_label(model),match model["effort"].as_str(){Some("light")=>"fastest",Some("high")=>"deep reasoning",_=>"balanced"})).on_click(cx.listener(move|this,_,_,cx|this.choose_model(&id,cx))));}}
            }
            let default=config.default.clone();let disabled=ht_core::ai_models::option(&default).is_none_or(|model|config.plan.as_deref()==Some("BYOK") && config.locked(model));
            menu=menu.child(div().id("ai-model-reset").p(px(8.)).child("Reset to default").when(!disabled,|d|d.cursor_pointer().on_click(cx.listener(move|this,_,_,cx|this.choose_model(&default,cx)))));
        }
        menu.into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[gpui::test]
    fn family_effort_and_default_choices_preserve_web_menu_and_refusal_restores_previous(cx:&mut gpui::TestAppContext){
        let view=cx.add_window(|window,cx|{let focus=cx.focus_handle();AiSidebar::new(crate::theme::THEMES[1],None,focus,window,cx)});
        view.update(cx,|sidebar,_,cx|{
            sidebar.models.team=Some("team".into());
            sidebar.models.configuration=Some(Configuration{user:42,team:"team".into(),options:ht_core::ai_models::catalog().to_vec(),selected:"gpt-6-luna-high".into(),settings:json!({}),byok:json!([]),plan:Some("Pro".into()),default:"gpt-6-luna".into(),custom_label:Some("Endpoint".into())});
            for status in [401,403,200]{
                sidebar.models.selected=Some("gpt-6-luna-high".into());sidebar.model_menu=true;sidebar.models.submenu=Some(false);
                sidebar.choose_family("claude-opus-5-5",cx);assert_eq!(sidebar.models.selected.as_deref(),Some("claude-opus-5-5-thinking"));assert!(sidebar.model_menu);assert!(sidebar.models.submenu.is_none());assert_eq!(sidebar.model_label(),"Opus 5.5");sidebar.set_mode(DisplayMode::Fullscreen,cx);assert!(sidebar.model_label().ends_with("Thinking"));sidebar.set_mode(DisplayMode::Sidebar,cx);
                sidebar.preference_finished(if status==200{json!({"success":true})}else{json!({"error":format!("HTTP {status}")})},cx);
                assert_eq!(sidebar.models.selected.as_deref(),Some(if status==200{"claude-opus-5-5-thinking"}else{"gpt-6-luna-high"}));
            }
            sidebar.choose_model("gpt-6-luna",cx);assert_eq!(sidebar.model_label(),"6 Luna");sidebar.preference_finished(json!({"success":true}),cx);
            sidebar.choose_family("custom",cx);assert_eq!(sidebar.model_label(),"Endpoint");
        }).unwrap();
    }
    #[gpui::test]
    fn model_choice_queues_web_preference_and_both_refusals_restore_pick(cx:&mut gpui::TestAppContext){
        let view=cx.add_window(|window,cx|{let focus=cx.focus_handle();AiSidebar::new(crate::theme::THEMES[1],None,focus,window,cx)});
        view.update(cx,|sidebar,_,cx|{
            sidebar.models.team=Some("team".into());sidebar.models.selected=Some("gpt-6-luna".into());
            sidebar.models.configuration=Some(Configuration{user:42,team:"team".into(),options:ht_core::ai_models::catalog().to_vec(),selected:"gpt-6-luna".into(),settings:json!({}),byok:json!([]),plan:Some("Pro".into()),default:"gpt-6-luna".into(),custom_label:None});
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
