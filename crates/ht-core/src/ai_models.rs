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
}
fn read(transport:&mut impl Transport,path:&str)->Result<Value>{let response=transport.get(path)?;ensure!(response.status==200 && response.body["success"]!=false,"{}",response.body["message"].as_str().or(response.body["error"].as_str()).unwrap_or("Could not load models"));Ok(response.body)}
pub fn load(transport:&mut impl Transport,team:&str,board_default:Option<&str>)->Result<Configuration> {
    let identity=read(transport,"/user/context")?;let user=identity["user"]["id"].as_i64().filter(|id|*id>0).context("Unauthorized")?;
    let params=url::form_urlencoded::Serializer::new(String::new()).append_pair("teamId",team).finish();
    let team_data=read(transport,&format!("/api/teams/getTeam?{params}"))?;ensure!(team_data["id"]==team,"Wrong model team");
    let features=read(transport,&format!("/api/teams/aiFeatureModels?{params}"))?;
    let providers=read(transport,&format!("/api/teams/aiProviderSettings?{params}"))?;
    let keys=read(transport,&format!("/api/teams/byokKeys?{params}"))?;
    let preferences=read(transport,"/api/users/preferences")?;ensure!(preferences["settings"].is_object(),"Missing preferences");
    let settings=preferences["settings"].clone();
    let enabled=providers["providers"].as_array().context("Missing model providers")?;
    let key_rows=keys["keys"].as_array().context("Missing configured keys")?;
    let custom=key_rows.iter().any(|row|row["provider"]=="custom" && row["enabled"]==true && row["hasSecret"]==true && row["baseUrl"].as_str().is_some_and(|v|!v.trim().is_empty()) && row["modelId"].as_str().is_some_and(|v|!v.trim().is_empty()) && (keys["gdprSafeMode"]!=true || row["gdprCompliant"]==true));
    let options:Vec<Value>=catalog().iter().filter(|option|if option["source"]=="custom"{custom}else{enabled.iter().any(|provider|provider["key"]==option["definition"]["provider"] && provider["enabled"]==true)}).cloned().collect();
    ensure!(!options.is_empty(),"No models available");
    // The effective default is evaluated server-side using the same flags, plan and provider context as the web.
    let selected=[settings["aiModelPreferences"]["teams"][team]["aiChat"].as_str(),settings["aiModelPreferences"]["aiChat"].as_str(),features["aiChat"]["model"].as_str(),board_default,features["aiChat"]["effectiveModel"].as_str()].into_iter().flatten().filter_map(option).find(|model|options.iter().any(|o|o["id"]==model["id"])).or_else(||options.iter().find(|m|m["id"]==features["aiChat"]["effectiveModel"])).unwrap_or(&options[0])["id"].as_str().unwrap().to_owned();
    let byok=Value::Array(key_rows.iter().map(|row|json!({"enabled":row["enabled"],"provider":row["provider"]})).collect());
    Ok(Configuration{user,team:team.into(),options,selected,settings,byok,plan:plan(&team_data)})
}
fn plan(team:&Value)->Option<String>{
    if team["id"]=="08f79efd-770e-4e32-9728-65e6d29ec893"{return Some("Pro".into());}
    let rows=team["subscriptionPlan"].as_array()?;
    let entitles=|row:&&Value|row["subscriptionStatus"].as_str().is_some_and(|s|matches!(s.to_ascii_lowercase().as_str(),"paid"|"active"|"trialing"|"past_due"));
    let row=rows.iter().filter(entitles).find(|row|row["subscriptionId"]==team["activeSubscriptionPlanId"]).or_else(||rows.iter().find(entitles));
    let paid=match row.and_then(|row|row["priceId"].as_str()){
        None=>Some("Free"),
        Some("price_1QjJeDIhmcH60VcciHzZ3mTJ"|"price_1TMlqDIhmcH60VccRZGW1xK3"|"price_1QjJf8IhmcH60VccNUrF7YJf"|"price_1QCKkpIhmcH60Vcc2RqVACTc")=>Some("Pro"),
        // Legacy price IDs are deployment configuration, not a plan discriminator to guess from a subscription.
        Some(_)=>None,
    };
    let comped=team["compedUntil"].as_str().and_then(|s|chrono::DateTime::parse_from_rfc3339(s).ok()).is_some_and(|until|until.timestamp()>chrono::Utc::now().timestamp());
    if comped && team["compedPlan"]!="BYOK"{return Some("Pro".into());}
    if comped && paid==Some("Free"){return Some("BYOK".into());}
    paid.map(str::to_owned)
}
impl Configuration {
    pub fn locked(&self,model:&Value)->bool{
        let premium=model["definition"]["premium"]==true || model["definition"]["priceTier"].as_i64().unwrap_or(1)>1;
        let premium=premium && !(self.plan.as_deref()==Some("Free") && model["family"]=="gpt-6-luna");
        let customer_key=self.byok.as_array().into_iter().flatten().any(|key|key["enabled"]==true && (key["provider"]=="gateway" || (model["source"]!="gateway" && key["provider"]==model["source"])));
        premium && (self.plan.as_deref()==Some("Free") || (self.plan.as_deref()==Some("BYOK") && !customer_key))
    }
    pub fn apply(&self,body:&mut Value,id:&str)->Result<()> {
        let model=self.options.iter().find(|m|m["id"]==id).context("Model is unavailable")?;
        ensure!(!self.locked(model),"Paid plan or BYOK");
        body["modelOptionId"]=model["id"].clone();body["model"]=model["model"].clone();body["provider"]=model["source"].clone();body["teamId"]=json!(self.team);body["byokProviderFlags"]=self.byok.clone();Ok(())
    }
}
