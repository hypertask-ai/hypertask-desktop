use crate::{Db,Result,Transport,http::{Response,TransportError}};
use anyhow::{Context,ensure};
use serde_json::{Value,json};

fn key(user:i64)->String{format!("web-preferences:{user}")}
pub fn validate(updates:&Value)->Result<()> {
    let fields=updates.as_object().filter(|o|!o.is_empty()).context("Invalid preferences")?;
    for(name,value)in fields {
        let valid=match name.as_str(){
            "commentsStacked"|"shareReadReceipts"|"notification"|"muteAnnouncements"|"playGifs"|"autoDescriptionSuggestions"|"inboxAdvanceOnSend"=>value.is_boolean(),
            "displayAvatar"=>matches!(value.as_str(),Some("Hidden"|"Show")),
            "scrollSetting"=>matches!(value.as_str(),Some("Top"|"Bottom")),
            "notificationPreference"=>matches!(value.as_str(),Some("direct"|"all")),
            "aiModelPreferencesTeamId"=>value.as_str().is_some_and(|v|!v.trim().is_empty()) && updates["aiModelPreferences"].is_object(),
            "aiModelPreferences"=>value.as_object().is_some_and(|o|!o.is_empty() && o.iter().all(|(surface,id)|surface=="aiChat" && id.as_str().is_some_and(|id|crate::ai_models::catalog().iter().any(|model|model["id"]==id)))),
            _=>false,
        };
        ensure!(valid,"Invalid preference {name}");
    }
    Ok(())
}
pub fn merge(before:&Value,updates:&Value)->Value {
    let mut settings=if before.is_object(){before.clone()}else{json!({})};
    for(name,value)in updates.as_object().into_iter().flatten(){
        if name=="aiModelPreferencesTeamId"{continue;}
        if name=="aiModelPreferences" {
            if !settings[name].is_object(){settings[name]=json!({});}
            let target=if let Some(team)=updates["aiModelPreferencesTeamId"].as_str(){
                if !settings[name]["teams"].is_object(){settings[name]["teams"]=json!({});}
                if !settings[name]["teams"][team].is_object(){settings[name]["teams"][team]=json!({});}
                &mut settings[name]["teams"][team]
            }else{&mut settings[name]};
            for(k,v)in value.as_object().into_iter().flatten(){target[k]=v.clone();}
        }else{settings[name]=value.clone();}
    }
    settings
}
impl Db {
    pub fn web_preferences(&self)->Result<Value>{let user=self.current_user_id().context("Unauthorized")?;self.kv_get(&key(user))?.map(|s|serde_json::from_str(&s).map_err(Into::into)).transpose()?.context("Load preferences before changing them")}
    pub fn cache_web_preferences(&self,user:i64,settings:&Value)->Result<()> {ensure!(self.current_user_id()==Some(user) && settings.is_object(),"Wrong preference account");self.kv_set(&key(user),&settings.to_string())}
    pub(crate) fn optimistic_preferences(&self,payload:&Value)->Result<()> {if self.current_user_id()==payload["user_id"].as_i64(){let settings=merge(&self.web_preferences()?,&payload["updates"]);self.cache_web_preferences(payload["user_id"].as_i64().unwrap(),&settings)?;}Ok(())}
    pub(crate) fn rollback_preferences(&self,payload:&Value)->Result<()> {self.kv_set(&key(payload["user_id"].as_i64().context("Missing preference account")?),&payload["before"].to_string())}
    pub(crate) fn send_preferences(&self,transport:&mut impl Transport,payload:&Value,client:&str)->std::result::Result<Response,TransportError>{
        let identity=transport.get("/user/context")?;
        if !(200..300).contains(&identity.status){return Ok(identity);}
        if identity.body["success"]!=true || identity.body["user"]["id"]!=payload["user_id"] || self.current_user_id()!=payload["user_id"].as_i64(){return Ok(Response{status:403,body:json!({"error":"Forbidden"})});}
        transport.post("/api/users/preferences",&payload["updates"],client)
    }
}
