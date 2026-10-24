use ht_core::{Db,Drain,Transport,Write,ai_models,http::{Response,TransportError}};
use serde_json::{Value,json};
struct Server{deny:Option<(&'static str,u16)>,writes:u16,calls:Vec<String>}
impl Transport for Server{
    fn get(&mut self,path:&str)->Result<Response,TransportError>{
        self.calls.push(path.into());
        let body=if path=="/user/context"{json!({"success":true,"user":{"id":42}})}else if path.starts_with("/api/teams/getTeam?"){json!({"id":"team","subscriptionPlan":[]})}else if path.starts_with("/api/teams/aiFeatureModels?"){json!({"aiChat":{"model":null,"effectiveModel":"gpt-6-luna"}})}else if path.starts_with("/api/teams/aiProviderSettings?"){json!({"providers":[{"key":"openai","enabled":true},{"key":"anthropic","enabled":false},{"key":"google","enabled":true}]})}else if path.starts_with("/api/teams/byokKeys?"){json!({"gdprSafeMode":true,"keys":[{"provider":"custom","enabled":true,"hasSecret":true,"baseUrl":"https://example.invalid","modelId":"custom-model","gdprCompliant":false}]})}else if path=="/api/users/preferences"{json!({"success":true,"settings":{"playGifs":false,"aiModelPreferences":{"aiChat":"gemini-3.1-flash-lite","teams":{"other":{"aiChat":"gpt-6.1-sol"},"team":{"aiChat":"gpt-5.6-luna-high"}}}}})}else{panic!("unexpected {path}")};
        Ok(if let Some((prefix,status))=self.deny.filter(|(prefix,_)|path.starts_with(prefix)){let _=prefix;Response{status,body:json!({"error":"Forbidden"})}}else{Response{status:200,body}})
    }
    fn post(&mut self,path:&str,body:&Value,_:&str)->Result<Response,TransportError>{assert_eq!(path,"/api/users/preferences");assert_eq!(body,&json!({"aiModelPreferences":{"aiChat":"gemini-3.5-flash-lite"},"aiModelPreferencesTeamId":"team"}));Ok(Response{status:self.writes,body:if self.writes==200{json!({"success":true,"settings":{"playGifs":false,"aiModelPreferences":{"teams":{"other":{"aiChat":"gpt-6.1-sol"},"team":{"aiChat":"gemini-3.5-flash-lite"}}}}})}else{json!({"error":"Unauthorized"})}})}
}
#[test]
fn configured_picker_reads_real_provider_custom_preference_and_plan_data(){
    let mut server=Server{deny:None,writes:200,calls:vec![]};let config=ai_models::load(&mut server,"team",None).unwrap();
    assert_eq!(config.user,42);assert_eq!(config.selected,"gpt-6-luna-high");assert_eq!(config.plan.as_deref(),Some("Free"));
    assert!(config.options.iter().any(|m|m["id"]=="gpt-6.1-sol-high"));assert!(!config.options.iter().any(|m|m["source"]=="claude" || m["source"]=="custom"));
    assert!(config.locked(ai_models::option("gpt-6.1-sol").unwrap()));assert!(!config.locked(ai_models::option("gpt-6-luna-high").unwrap()));
    let mut body=json!({});config.apply(&mut body,"gemini-3.5-flash-lite").unwrap();assert_eq!(body["model"],"google/gemini-3.5-flash-lite");assert_eq!(body["provider"],"gateway");assert_eq!(body["teamId"],"team");assert!(config.apply(&mut body,"claude-opus-5-5-thinking").is_err());
}
#[test]
fn each_model_read_refusal_is_an_error_not_a_configured_picker(){
    for path in ["/user/context","/api/teams/getTeam","/api/teams/aiFeatureModels","/api/teams/aiProviderSettings","/api/teams/byokKeys","/api/users/preferences"]{for status in [401,403]{let mut server=Server{deny:Some((path,status)),writes:200,calls:vec![]};assert!(ai_models::load(&mut server,"team",None).is_err());}}
}
#[test]
fn model_preference_is_queued_team_scoped_and_rolls_back_refusals_after_restart(){
    for status in [200,401,403]{
        let file=tempfile::NamedTempFile::new().unwrap();let mut db=Db::open(file.path()).unwrap();db.kv_set("current-user-id","42").unwrap();
        let before=json!({"playGifs":false,"aiModelPreferences":{"teams":{"other":{"aiChat":"gpt-6.1-sol"},"team":{"aiChat":"gpt-6-luna"}}}});db.cache_web_preferences(42,&before).unwrap();
        db.enqueue(Write::Preferences{updates:json!({"aiModelPreferences":{"aiChat":"gemini-3.5-flash-lite"},"aiModelPreferencesTeamId":"team"})},1).unwrap();
        assert_eq!(db.web_preferences().unwrap()["aiModelPreferences"]["teams"]["team"]["aiChat"],"gemini-3.5-flash-lite");assert_eq!(db.web_preferences().unwrap()["aiModelPreferences"]["teams"]["other"]["aiChat"],"gpt-6.1-sol");assert_eq!(db.web_preferences().unwrap()["playGifs"],false);
        drop(db);let mut db=Db::open(file.path()).unwrap();let mut server=Server{deny:None,writes:status,calls:vec![]};let result=db.drain_once(&mut server,2).unwrap();assert_eq!(matches!(result,Drain::Applied(_)),status==200);
        if status!=200{assert_eq!(db.web_preferences().unwrap(),before);}else{assert_eq!(db.web_preferences().unwrap()["aiModelPreferences"]["teams"]["team"]["aiChat"],"gemini-3.5-flash-lite");}
    }
}
