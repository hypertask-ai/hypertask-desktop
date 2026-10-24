use crate::{Result,Transport};
use anyhow::{Context,ensure};
use serde_json::{Value,json};
use std::sync::OnceLock;

pub fn catalog()->&'static [Value] {static MODELS:OnceLock<Vec<Value>>=OnceLock::new();MODELS.get_or_init(||serde_json::from_str(include_str!("ai_models.json")).unwrap())}
pub fn option(id:&str)->Option<&'static Value> {
    let id=id.trim();
    let aliases:Value=serde_json::from_str(include_str!("ai_model_aliases.json")).unwrap();
    let aliased=aliases[id].as_str().unwrap_or(id);
    let upgraded=if aliased.starts_with("claude-") && !aliased.contains("5-5"){
        if aliased.starts_with("claude-haiku-"){Some("claude-haiku-5-5".to_owned())}else if aliased.starts_with("claude-opus-") || aliased.starts_with("claude-sonnet-"){Some(format!("claude-{}-5-5-{}",if aliased.starts_with("claude-opus-"){ "opus" }else{"sonnet"},if aliased.contains("thinking"){"thinking"}else{"instant"}))}else{None}
    }else{None};
    let id=upgraded.as_deref().unwrap_or(aliased);catalog().iter().find(|m|m["id"]==id)
}
#[derive(Clone,Debug)]
pub struct Configuration {
    pub user:i64,
    pub team:String,
    pub options:Vec<Value>,
    pub selected:String,
    pub settings:Value,
    pub byok:Value,
    pub plan:Option<String>,
    pub default:String,
    pub custom_label:Option<String>,
}
fn read(transport:&mut impl Transport,path:&str)->Result<Value>{let response=transport.get(path)?;ensure!(response.status==200 && response.body["success"]!=false,"{}",response.body["message"].as_str().or(response.body["error"].as_str()).unwrap_or("Could not load models"));Ok(response.body)}
pub fn load(transport:&mut impl Transport,team:&str,board_default:Option<&str>)->Result<Configuration> {
    load_for_scope(transport,Some(team),None,board_default)
}
pub fn load_for_scope(transport:&mut impl Transport,team:Option<&str>,last_team:Option<&str>,board_default:Option<&str>)->Result<Configuration> {
    let identity=read(transport,"/user/context")?;let user=identity["user"]["id"].as_i64().filter(|id|*id>0).context("Unauthorized")?;
    let resolved;
    let team=if let Some(team)=team {team}else{
        let response=transport.post("/api/teams/getAllSidebar",&json!({"userId":user}),"ai-model-team-read")?;
        ensure!(response.status==200,"Could not load teams");
        let teams=response.body.as_array().context("Could not load teams")?;
        resolved=teams.iter().find(|team|team["id"].as_str()==last_team).or_else(||teams.first()).and_then(|team|team["id"].as_str()).context("No teams available")?.to_owned();
        &resolved
    };
    let params=url::form_urlencoded::Serializer::new(String::new()).append_pair("teamId",team).finish();
    let team_data=read(transport,&format!("/api/teams/getTeam?{params}"))?;ensure!(team_data["id"]==team,"Wrong model team");
    let features=read(transport,&format!("/api/teams/aiFeatureModels?{params}"))?;
    let providers=read(transport,&format!("/api/teams/aiProviderSettings?{params}"))?;
    let keys=read(transport,&format!("/api/teams/byokKeys?{params}"))?;
    let preferences=read(transport,"/api/users/preferences")?;ensure!(preferences["settings"].is_object(),"Missing preferences");
    let settings=preferences["settings"].clone();
    let enabled=providers["providers"].as_array().context("Missing model providers")?;
    let key_rows=keys["keys"].as_array().context("Missing configured keys")?;
    let custom_row=key_rows.iter().find(|row|row["provider"]=="custom");
    let custom=custom_row.is_some_and(|row|row["provider"]=="custom" && row["enabled"]==true && row["hasSecret"]==true && row["baseUrl"].as_str().is_some_and(|v|!v.trim().is_empty()) && row["modelId"].as_str().is_some_and(|v|!v.trim().is_empty()) && (keys["gdprSafeMode"]!=true || row["gdprCompliant"]==true));
    let options:Vec<Value>=catalog().iter().filter(|option|if option["source"]=="custom"{custom}else{enabled.iter().any(|provider|provider["key"]==option["definition"]["provider"] && provider["enabled"]==true)}).cloned().collect();
    ensure!(!options.is_empty(),"No models available");
    let byok=team_data["byokApiKeys"].as_array().map(|rows|Value::Array(rows.iter().map(|row|json!({"enabled":row["enabled"],"provider":row["provider"]})).collect())).unwrap_or_else(||json!([]));
    let plan=plan(&team_data);
    let flags=transport.get("/api/flags").ok().filter(|response|response.status==200).map(|response|response.body["flags"].clone()).unwrap_or(Value::Null);
    let context=&providers["defaultModelContext"];
    let openai_only=flags["htpr-7634-openai-only-default"]==true && matches!(plan.as_deref(),Some("Pro"|"AI")) && context["hasClaudeRoute"]==false && context["hasDirectOpenAiKey"]==true;
    let haiku= !openai_only && (flags["htpr-7038-haiku-default"]==true || flags["htpr-7075-background-claude"]==true || matches!(plan.as_deref(),Some("Pro"|"AI"|"BYOK")));
    let default_id=if haiku{"claude-haiku-5-5"}else{"gpt-6-luna"};
    let default=options.iter().find(|model|model["id"]==default_id).or_else(||options.iter().find(|model|model["definition"]["premium"]!=true && model["definition"]["priceTier"].as_i64().unwrap_or(1)<=1)).unwrap_or(&options[0])["id"].as_str().unwrap().to_owned();
    let selected=[settings["aiModelPreferences"]["teams"][team]["aiChat"].as_str(),settings["aiModelPreferences"]["aiChat"].as_str(),features["aiChat"]["model"].as_str(),board_default,Some(default.as_str())].into_iter().flatten().filter_map(option).find(|model|options.iter().any(|o|o["id"]==model["id"])).or_else(||options.iter().find(|m|m["id"]==features["aiChat"]["effectiveModel"])).unwrap_or(&options[0])["id"].as_str().unwrap().to_owned();
    Ok(Configuration{user,team:team.into(),options,selected,settings,byok,plan,default,custom_label:custom_row.and_then(|row|row["modelId"].as_str()).map(|id|id.trim().to_owned())})
}
fn plan(team:&Value)->Option<String>{
    if team["id"]=="08f79efd-770e-4e32-9728-65e6d29ec893"{return Some("Pro".into());}
    let rows=team["subscriptionPlan"].as_array().cloned().unwrap_or_default();
    let entitles=|row:&&Value|row["subscriptionStatus"].as_str().is_some_and(|s|matches!(s.to_ascii_lowercase().as_str(),"paid"|"active"|"trialing"|"past_due"));
    let row=rows.iter().filter(entitles).find(|row|row["subscriptionId"]==team["activeSubscriptionPlanId"]).or_else(||rows.iter().find(entitles));
    let paid=match row.and_then(|row|row["priceId"].as_str()){
        None=>Some("Free"),
        Some("price_1QjJeDIhmcH60VcciHzZ3mTJ"|"price_1TMlqDIhmcH60VccRZGW1xK3"|"price_1QjJf8IhmcH60VccNUrF7YJf"|"price_1QCKkpIhmcH60Vcc2RqVACTc")=>Some("Pro"),
        Some(id)=>Some(legacy_plan(id,&[
            option_env!("NEXT_PUBLIC_STRIPE_MONTHLY_PRICE_ID_01").filter(|id|!id.is_empty()).or(Some("price_1OSg0zIhmcH60VccTpAFDv32")),option_env!("NEXT_PUBLIC_STRIPE_MONTHLY_PRICE_ID_02").filter(|id|!id.is_empty()).or(Some("price_1QjJeDIhmcH60VcciHzZ3mTJ")),option_env!("NEXT_PUBLIC_STRIPE_MONTHLY_PRICE_ID_BYOK").filter(|id|!id.is_empty()).or(Some("price_1TMlqCIhmcH60Vccgq4fpSWu")),option_env!("NEXT_PUBLIC_STRIPE_MONTHLY_PRICE_ID_PRO").filter(|id|!id.is_empty()).or(Some("price_1QjJeDIhmcH60VcciHzZ3mTJ")),
        ],&[
            option_env!("NEXT_PUBLIC_STRIPE_YEARLY_PRICE_ID_01").filter(|id|!id.is_empty()).or(Some("price_1OSg35IhmcH60VccJP5LQo03")),option_env!("NEXT_PUBLIC_STRIPE_YEARLY_PRICE_ID_02").filter(|id|!id.is_empty()).or(Some("price_1QjJf8IhmcH60VccNUrF7YJf")),option_env!("NEXT_PUBLIC_STRIPE_YEARLY_PRICE_ID_BYOK").filter(|id|!id.is_empty()).or(Some("price_1TMlqCIhmcH60VccI6VjW2FM")),option_env!("NEXT_PUBLIC_STRIPE_YEARLY_PRICE_ID_PRO").filter(|id|!id.is_empty()).or(Some("price_1QjJf8IhmcH60VccNUrF7YJf")),
        ])),
    };
    let comped=team["compedUntil"].as_str().and_then(|s|chrono::DateTime::parse_from_rfc3339(s).ok()).is_some_and(|until|until.timestamp()>chrono::Utc::now().timestamp());
    if comped && team["compedPlan"]!="BYOK"{return Some("Pro".into());}
    if comped && paid==Some("Free"){return Some("BYOK".into());}
    paid.map(str::to_owned)
}
fn legacy_plan<'a>(id:&str,monthly:&[Option<&'a str>;4],yearly:&[Option<&'a str>;4])->&'static str {
    match monthly.iter().position(|price|*price==Some(id)).or_else(||yearly.iter().position(|price|*price==Some(id))) {
        Some(0|1)=>"AI",Some(2)=>"BYOK",Some(3)=>"Pro",_=>"Free",
    }
}
pub fn effort_label(model:&Value)->&str {
    match (model["family"].as_str(),model["effort"].as_str()) {
        (Some(family),Some("light")) if family.starts_with("claude-")=>"Instant",
        (Some(family),Some("high")) if family.starts_with("claude-")=>"Thinking",
        (_,Some("light"))=>"Light",(_,Some("standard"))=>"Standard",(_,Some("high"))=>"High",_=>"",
    }
}
impl Configuration {
    pub fn families(&self)->Vec<&Value> {
        let mut seen=std::collections::HashSet::new();
        self.options.iter().filter(|model|seen.insert(model["family"].as_str().unwrap_or(""))).collect()
    }
    pub fn efforts(&self,family:&str)->Vec<&Value> {
        ["light","standard","high"].into_iter().filter_map(|effort|self.options.iter().find(|model|model["family"]==family && model["effort"]==effort)).collect()
    }
    pub fn nearest(&self,family:&str,effort:Option<&str>)->Option<&Value> {
        if let Some(model)=self.options.iter().find(|model|model["family"]==family && model["effort"].is_null()){return Some(model);}
        let rank=|effort:&str|["light","standard","high"].iter().position(|e|*e==effort).unwrap_or(1);
        self.efforts(family).into_iter().min_by_key(|model|rank(model["effort"].as_str().unwrap()).abs_diff(rank(effort.unwrap_or("standard"))))
    }
    pub fn label<'a>(&'a self,model:&'a Value)->&'a str {
        if model["source"]=="custom" {self.custom_label.as_deref().unwrap_or("Custom endpoint")}else{model["definition"]["label"].as_str().unwrap_or("")}
    }
    pub fn customer_key(&self,source:&Value)->bool {
        self.byok.as_array().into_iter().flatten().any(|key|key["enabled"]==true && (key["provider"]=="gateway" || (*source!="gateway" && key["provider"]==*source)))
    }
    pub fn disabled(&self,model:&Value)->bool {self.plan.as_deref()==Some("BYOK") && model["source"]=="custom" && !self.customer_key(&model["source"])}

    pub fn locked(&self,model:&Value)->bool{
        let premium=model["definition"]["premium"]==true || model["definition"]["priceTier"].as_i64().unwrap_or(1)>1;
        let premium=premium && !(self.plan.as_deref()==Some("Free") && model["family"]=="gpt-6-luna");
        let customer_key=self.customer_key(&model["source"]);
        premium && (self.plan.as_deref()==Some("Free") || (self.plan.as_deref()==Some("BYOK") && !customer_key))
    }
    pub fn apply(&self,body:&mut Value,id:&str)->Result<()> {
        let model=self.options.iter().find(|m|m["id"]==id).context("Model is unavailable")?;
        ensure!(!self.locked(model) && !self.disabled(model),"Paid plan or BYOK");
        body["modelOptionId"]=model["id"].clone();body["model"]=model["model"].clone();body["provider"]=model["source"].clone();body["teamId"]=json!(self.team);body["byokProviderFlags"]=self.byok.clone();Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stripe_mapping_matches_web_legacy_slots_unknown_status_and_comp_precedence(){
        let month=[Some("m1"),Some("m2"),Some("mb"),Some("mp")];let year=[Some("y1"),Some("y2"),Some("yb"),Some("yp")];
        for(id,expected)in [("m1","AI"),("m2","AI"),("y1","AI"),("y2","AI"),("mb","BYOK"),("yb","BYOK"),("mp","Pro"),("yp","Pro"),("unknown","Free")]{assert_eq!(legacy_plan(id,&month,&year),expected);}
        for(id,expected)in [("price_1OSg0zIhmcH60VccTpAFDv32","AI"),("price_1OSg35IhmcH60VccJP5LQo03","AI"),("price_1TMlqCIhmcH60Vccgq4fpSWu","BYOK"),("price_1TMlqCIhmcH60VccI6VjW2FM","BYOK"),("price_1QjJeDIhmcH60VcciHzZ3mTJ","Pro"),("unknown","Free")]{let team=json!({"subscriptionPlan":[{"subscriptionStatus":"active","priceId":id}]});assert_eq!(plan(&team).as_deref(),Some(expected));}
        for status in ["paid","ACTIVE","trialing","past_due","canceled","unpaid","incomplete"]{
            let team=json!({"subscriptionPlan":[{"subscriptionId":"active","subscriptionStatus":status,"priceId":"price_1TMlqDIhmcH60VccRZGW1xK3"}],"activeSubscriptionPlanId":"active"});
            assert_eq!(plan(&team).as_deref(),Some(if matches!(status,"canceled"|"unpaid"|"incomplete"){"Free"}else{"Pro"}));
        }
        let mut team=json!({"compedUntil":"2999-01-01T00:00:00Z","compedPlan":"BYOK","subscriptionPlan":[]});assert_eq!(plan(&team).as_deref(),Some("BYOK"));
        team["subscriptionPlan"]=json!([{"subscriptionStatus":"active","priceId":"price_1TMlqDIhmcH60VccRZGW1xK3"}]);assert_eq!(plan(&team).as_deref(),Some("Pro"));
        team["subscriptionPlan"]=json!([{"subscriptionStatus":"active","priceId":"unknown"}]);assert_eq!(plan(&team).as_deref(),Some("BYOK"));
        team["compedUntil"]=json!("2000-01-01T00:00:00Z");assert_eq!(plan(&team).as_deref(),Some("Free"));
    }
}
