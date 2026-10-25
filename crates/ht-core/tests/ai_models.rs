use ht_core::{Db,Drain,Transport,Write,ai_models,http::{Response,TransportError}};
use serde_json::{Value,json};
struct Server{deny:Option<(&'static str,u16)>,writes:u16,calls:Vec<String>}
impl Transport for Server{
    fn get(&mut self,path:&str)->Result<Response,TransportError>{
        self.calls.push(path.into());
        let body=if path=="/api/flags"{json!({"flags":{}})}else if path=="/user/context"{json!({"success":true,"user":{"id":42}})}else if path.starts_with("/api/teams/getTeam?"){json!({"id":path.split("teamId=").nth(1).unwrap(),"subscriptionPlan":[]})}else if path.starts_with("/api/teams/aiFeatureModels?"){json!({"aiChat":{"model":null,"effectiveModel":"gpt-6-luna"}})}else if path.starts_with("/api/teams/aiProviderSettings?"){json!({"providers":[{"key":"openai","enabled":true},{"key":"anthropic","enabled":false},{"key":"google","enabled":true}]})}else if path.starts_with("/api/teams/byokKeys?"){json!({"gdprSafeMode":true,"keys":[{"provider":"custom","enabled":true,"hasSecret":true,"baseUrl":"https://example.invalid","modelId":"custom-model","gdprCompliant":false}]})}else if path=="/api/users/preferences"{json!({"success":true,"settings":{"playGifs":false,"aiModelPreferences":{"aiChat":"gemini-3.1-flash-lite","teams":{"other":{"aiChat":"gpt-6.1-sol"},"team":{"aiChat":"gpt-5.6-luna-high"}}}}})}else{panic!("unexpected {path}")};
        Ok(if let Some((prefix,status))=self.deny.filter(|(prefix,_)|path.starts_with(prefix)){let _=prefix;Response{status,body:json!({"error":"Forbidden"})}}else{Response{status:200,body}})
    }
    fn post(&mut self,path:&str,body:&Value,_:&str)->Result<Response,TransportError>{
        if path=="/api/teams/getAllSidebar"{assert_eq!(body,&json!({"userId":42}));self.calls.push(path.into());return Ok(if let Some((_,status))=self.deny.filter(|(prefix,_)|path.starts_with(prefix)){Response{status,body:json!({"error":"Forbidden"})}}else{Response{status:200,body:json!([{"id":"other"},{"id":"team"}])}});}
        assert_eq!(path,"/api/users/preferences");assert_eq!(body,&json!({"aiModelPreferences":{"aiChat":"gemini-3.5-flash-lite"},"aiModelPreferencesTeamId":"team"}));Ok(Response{status:self.writes,body:if self.writes==200{json!({"success":true,"settings":{"playGifs":false,"aiModelPreferences":{"teams":{"other":{"aiChat":"gpt-6.1-sol"},"team":{"aiChat":"gemini-3.5-flash-lite"}}}}})}else{json!({"error":"Unauthorized"})}})}
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

#[test]
fn duplicate_or_wrong_account_preference_request_cannot_replace_an_optimistic_pick(){
    let mut db=Db::memory().unwrap();db.kv_set("current-user-id","42").unwrap();let before=json!({"playGifs":false,"aiModelPreferences":{"teams":{"team":{"aiChat":"gpt-6-luna"}}}});
    let update=json!({"aiModelPreferences":{"aiChat":"gemini-3.5-flash-lite"},"aiModelPreferencesTeamId":"team"});
    db.enqueue_web_preferences(42,&before,update.clone(),1).unwrap();let optimistic=db.web_preferences().unwrap();
    assert!(db.enqueue_web_preferences(42,&before,update.clone(),2).is_err());assert_eq!(db.web_preferences().unwrap(),optimistic);
    assert!(db.enqueue_web_preferences(99,&before,update,2).is_err());assert_eq!(db.web_preferences().unwrap(),optimistic);
}

#[test]
fn no_board_model_team_uses_verified_last_membership_or_first_and_never_ignores_refusal(){
    for last in [Some("team"),Some("revoked"),None]{
        let mut server=Server{deny:None,writes:200,calls:vec![]};
        let result=ai_models::load_for_scope(&mut server,None,last,None);
        assert_eq!(result.unwrap().team,if last==Some("team"){"team"}else{"other"});
        assert_eq!(&server.calls[..2],&["/user/context","/api/teams/getAllSidebar"]);
    }
    for status in [401,403]{let mut server=Server{deny:Some(("/api/teams/getAllSidebar",status)),writes:200,calls:vec![]};assert!(ai_models::load_for_scope(&mut server,None,Some("team"),None).is_err());assert_eq!(server.calls.len(),2);}
}

#[test]
fn family_effort_nearest_reset_and_custom_labels_use_the_web_catalog(){
    let mut server=Server{deny:None,writes:200,calls:vec![]};let mut config=ai_models::load(&mut server,"team",None).unwrap();
    config.options=ai_models::catalog().to_vec();config.custom_label=Some("My endpoint".into());
    assert_eq!(config.default,"gpt-6-luna");assert_eq!(config.nearest("gpt-6.1-sol",Some("high")).unwrap()["id"],"gpt-6.1-sol-high");
    assert_eq!(config.nearest("claude-opus-5-5",Some("standard")).unwrap()["id"],"claude-opus-5-5-instant");
    assert_eq!(config.nearest("gemini-3.8-flash",Some("high")).unwrap()["id"],"gemini-3.8-flash");
    assert_eq!(config.efforts("gpt-6-luna").iter().map(|model|ai_models::effort_label(model)).collect::<Vec<_>>(),vec!["Light","Standard","High"]);
    assert_eq!(config.efforts("claude-opus-5-5").iter().map(|model|ai_models::effort_label(model)).collect::<Vec<_>>(),vec!["Instant","Thinking"]);
    assert_eq!(config.families().iter().filter(|model|model["family"]=="gpt-6-luna").count(),1);
    assert_eq!(config.label(ai_models::option("custom").unwrap()),"My endpoint");
}

#[test]
fn every_catalog_effort_and_reset_id_uses_the_team_queue_and_restores_both_refusals(){
    struct Picker{status:u16,id:String}
    impl Transport for Picker{
        fn get(&mut self,path:&str)->Result<Response,TransportError>{assert_eq!(path,"/user/context");Ok(Response{status:200,body:json!({"success":true,"user":{"id":42}})})}
        fn post(&mut self,path:&str,body:&Value,_:&str)->Result<Response,TransportError>{assert_eq!(path,"/api/users/preferences");assert_eq!(body,&json!({"aiModelPreferences":{"aiChat":self.id},"aiModelPreferencesTeamId":"team"}));Ok(Response{status:self.status,body:if self.status==200{json!({"settings":{"aiModelPreferences":{"teams":{"team":{"aiChat":self.id}}}}})}else{json!({"error":"Unauthorized"})}})}
    }
    for model in ai_models::catalog(){for status in [200,401,403]{
        let mut db=Db::memory().unwrap();db.kv_set("current-user-id","42").unwrap();let before=json!({"aiModelPreferences":{"teams":{"team":{"aiChat":"gpt-6-luna"}}}});db.cache_web_preferences(42,&before).unwrap();let id=model["id"].as_str().unwrap();db.enqueue_web_preferences(42,&before,json!({"aiModelPreferences":{"aiChat":id},"aiModelPreferencesTeamId":"team"}),100).unwrap();assert_eq!(db.web_preferences().unwrap()["aiModelPreferences"]["teams"]["team"]["aiChat"],id);
        db.drain_once(&mut Picker{status,id:id.into()},101).unwrap();assert_eq!(db.web_preferences().unwrap()["aiModelPreferences"]["teams"]["team"]["aiChat"],if status==200{id}else{"gpt-6-luna"});
    }}
}

#[test]
fn model_defaults_provider_order_custom_gdpr_and_real_billing_are_server_derived(){
    struct Models{status:u16,team:Value,keys:Value,flags:Value,context:Value,preferences:Value,feature:Value}
    impl Transport for Models{
        fn get(&mut self,path:&str)->Result<Response,TransportError>{
            let body=if self.status!=200{json!({"error":"Unauthorized"})}else if path=="/user/context"{json!({"success":true,"user":{"id":42}})}else if path.starts_with("/api/teams/getTeam?"){self.team.clone()}else if path.starts_with("/api/teams/aiFeatureModels?"){json!({"aiChat":{"model":self.feature,"effectiveModel":"claude-haiku-5-5"}})}else if path.starts_with("/api/teams/aiProviderSettings?"){json!({"providers":[{"key":"openai","enabled":true},{"key":"anthropic","enabled":true},{"key":"google","enabled":true},{"key":"deepseek","enabled":true}],"defaultModelContext":self.context})}else if path.starts_with("/api/teams/byokKeys?"){self.keys.clone()}else if path=="/api/users/preferences"{json!({"settings":self.preferences})}else{assert_eq!(path,"/api/flags");json!({"flags":self.flags})};Ok(Response{status:self.status,body})
        }
        fn post(&mut self,_:&str,_:&Value,_:&str)->Result<Response,TransportError>{panic!("model configuration is read only")}
    }
    for price in ["price_1OSg0zIhmcH60VccTpAFDv32","price_1TMlqCIhmcH60Vccgq4fpSWu","price_1QjJeDIhmcH60VcciHzZ3mTJ","unknown"]{for status in [200,401,403]{
        let mut server=Models{status,team:json!({"id":"team","subscriptionPlan":[{"subscriptionStatus":"active","priceId":price}],"byokApiKeys":[{"provider":"openai","enabled":true}]}),keys:json!({"gdprSafeMode":true,"keys":[{"provider":"custom","enabled":true,"hasSecret":true,"baseUrl":" https://example.test ","modelId":" My model ","gdprCompliant":false}]}),flags:json!({}),context:json!({}),preferences:json!({}),feature:Value::Null};
        let result=ai_models::load(&mut server,"team",None);if status!=200{assert!(result.is_err());continue;}let config=result.unwrap();assert_eq!(config.plan.as_deref(),Some(match price{"price_1OSg0zIhmcH60VccTpAFDv32"=>"AI","price_1TMlqCIhmcH60Vccgq4fpSWu"=>"BYOK","unknown"=>"Free",_=>"Pro"}));assert_eq!(config.default,if price=="unknown"{"gpt-6-luna"}else{"claude-haiku-5-5"});assert!(!config.options.iter().any(|model|model["source"]=="custom"));
        assert_eq!(config.families().iter().take(7).map(|model|model["definition"]["provider"].as_str().unwrap()).collect::<Vec<_>>(),vec!["openai","openai","openai","anthropic","anthropic","anthropic","google"]);
        server.keys["keys"][0]["gdprCompliant"]=json!(true);let config=ai_models::load(&mut server,"team",None).unwrap();assert!(config.options.iter().any(|model|model["source"]=="custom"));assert_eq!(config.label(ai_models::option("custom").unwrap()),"My model");
        if price=="price_1TMlqCIhmcH60Vccgq4fpSWu"{assert!(!config.locked(ai_models::option("gpt-6.1-sol-high").unwrap()));assert!(config.locked(ai_models::option("claude-opus-5-5-thinking").unwrap()));assert!(config.disabled(ai_models::option("custom").unwrap()));}
        if price=="price_1QjJeDIhmcH60VcciHzZ3mTJ"{server.flags=json!({"htpr-7634-openai-only-default":true});server.context=json!({"hasClaudeRoute":false,"hasDirectOpenAiKey":true});assert_eq!(ai_models::load(&mut server,"team",None).unwrap().default,"gpt-6-luna");}
        server.feature=json!("gpt-6-luna-high");assert_eq!(ai_models::load(&mut server,"team",Some("gpt-6.1-sol-light")).unwrap().selected,"gpt-6-luna-high");
        server.preferences=json!({"aiModelPreferences":{"aiChat":"gpt-6.1-sol-light","teams":{"team":{"aiChat":"gpt-6-luna-light"}}}});assert_eq!(ai_models::load(&mut server,"team",None).unwrap().selected,"gpt-6-luna-light");
    }}
}
