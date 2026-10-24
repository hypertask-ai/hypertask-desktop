use ht_core::{Db, Drain, ShellWrite, Transport, Write, team_settings, http::{Response, TransportError}};
use serde_json::{Value, json};
struct Server { status: u16, deny: Option<String>, path: Option<String>, body: Value, reply: Value, calls: Vec<String> }
impl Transport for Server {
    fn get(&mut self, path: &str) -> Result<Response, TransportError> {
        self.calls.push(path.into());
        if self.deny.as_deref() == Some(path) { return Ok(Response { status:self.status, body:json!({"message":"Team access denied"}) }); }
        let body = match path {
            "/user/context" => json!({"success":true,"user":{"id":42,"accountId":"account"}}),
            "/api/teams/getTeam?teamId=team" => json!({"id":"team","title":"Team & me","googleAccountId":"account","stripe_customer_id":"cus_1","totalSeats":3,"subscriptionPlan":[{"subscriptionStatus":"active","priceId":"price_1QjJeDIhmcH60VcciHzZ3mTJ"}]}),
            "/api/members/getAllTeamMembers?teamId=team" => json!({"owner":{"id":42},"members":[]}),
            "/api/teams/byokKeys?teamId=team" => json!({"keys":[{"provider":"openai","enabled":true,"hasSecret":true,"maskedKey":"sk…1234"}],"gdprSafeMode":true}),
            "/api/teams/aiProviderSettings?teamId=team" => json!({"providers":[{"key":"openai","enabled":true}]}),
            "/api/flags" => json!({"flags":{"htpr-7551-start-plan-checkout":true}}),
            _ => panic!("Unexpected read {path}"),
        };
        Ok(Response { status:200, body })
    }
    fn post(&mut self, path: &str, body: &Value, _: &str) -> Result<Response, TransportError> {
        self.calls.push(path.into()); assert_eq!(Some(path),self.path.as_deref()); assert_eq!(body,&self.body);
        Ok(Response { status:self.status, body:if self.status==200 {self.reply.clone()} else {json!({"message":"Team access denied"})} })
    }
}
fn server() -> Server { Server { status:200, deny:None, path:None, body:Value::Null, reply:Value::Null, calls:vec![] } }
#[test]
fn team_targeted_model_handoff_reads_exact_web_settings_and_all_refusals_are_errors() {
    let mut web = server(); let snapshot = team_settings::load(&mut web,"team",true).unwrap();
    assert!(snapshot.owner()); assert!(snapshot.manage_keys()); assert_eq!(snapshot.plan(),"Pro"); assert_eq!(snapshot.billing_interval(),Some(false));
    assert_eq!(web.calls,vec!["/user/context","/api/teams/getTeam?teamId=team","/api/members/getAllTeamMembers?teamId=team","/api/teams/byokKeys?teamId=team","/api/teams/aiProviderSettings?teamId=team","/api/flags"]);
    for path in &web.calls[..5] { for status in [401,403] { let mut denied=server(); denied.status=status; denied.deny=Some(path.clone()); assert_eq!(team_settings::load(&mut denied,"team",true).unwrap_err().to_string(),"Team access denied"); } }
    let mut other=snapshot.clone();other.user=json!({"id":99,"accountId":"another"});assert!(!other.owner());assert!(!other.manage_keys());
}
#[test]
fn plans_use_verified_legacy_prices_exact_checkout_team_payload_and_owner_gating() {
    let mut snapshot=team_settings::load(&mut server(),"team",false).unwrap();
    for (plan,yearly,price) in [("BYOK",false,"price_1TMlqCIhmcH60Vccgq4fpSWu"),("BYOK",true,"price_1TMlqCIhmcH60VccI6VjW2FM"),("Pro",false,"price_1QjJeDIhmcH60VcciHzZ3mTJ"),("Pro",true,"price_1QjJf8IhmcH60VccNUrF7YJf")] {
        let body=snapshot.checkout(plan,yearly).unwrap(); assert_eq!(body["priceId"],price); assert_eq!(body["teamId"],"team"); assert_eq!(body["teamTitle"],"Team & me"); assert_eq!(body["quantity"],3); assert_eq!(body["googleAccountId"],"account"); assert_eq!(body["stripe_customer_id"],"cus_1"); assert_eq!(body["mode"],"Normal");
        let success=url::Url::parse(body["returnUrl"].as_str().unwrap()).unwrap();let params:std::collections::HashMap<_,_>=success.query_pairs().collect();assert_eq!(params["teamId"],"team");assert_eq!(params["teamTitle"],"Team & me");assert_eq!(params["success"],"1");assert!(body["cancelUrl"].as_str().unwrap().ends_with("preSelect=Upgrade&success=0"));
    }
    for (price,interval) in [("price_1QjJf8IhmcH60VccNUrF7YJf",Some(true)),("price_1QCKkpIhmcH60Vcc2RqVACTc",Some(false)),("unknown",None)] { snapshot.team["subscriptionPlan"][0]["priceId"]=json!(price);assert_eq!(snapshot.billing_interval(),interval); }
    snapshot.team["compedUntil"]=json!("2999-01-01T00:00:00Z");assert_eq!(snapshot.billing_interval(),None);
    snapshot.user["id"]=json!(99);assert!(snapshot.checkout("Pro",false).is_err());
}
#[test]
fn api_key_billing_and_test_writes_are_queued_exact_accept_or_refuse_and_scrub_rejected_secrets() {
    for (path,body,reply) in [
        ("/api/teams/byokKeys",json!({"teamId":"team","provider":"openai","apiKey":"fixture-secret"}),json!({"ok":true})),
        ("/api/teams/byokKeys",json!({"teamId":"team","provider":"claude","enabled":false}),json!({"ok":true})),
        ("/api/teams/byokKeys",json!({"teamId":"team","provider":"google","clearSecret":true}),json!({"ok":true})),
        ("/api/teams/byokKeys",json!({"teamId":"team","provider":"custom","baseUrl":"https://example.test/v1","modelId":"model","apiKey":"fixture-secret","gdprCompliant":true}),json!({"ok":true})),
        ("/api/settings/byok-test",json!({"teamId":"team","provider":"openai","apiKey":"fixture-secret"}),json!({"result":"works","status":200,"elapsedMs":123,"model":"model"})),
        ("/api/stripe/ensure-customer",json!({"teamId":"team"}),json!({"stripe_customer_id":"cus_1"})),
        ("/api/stripe/cancelSubscription",json!({"stripe_customer_id":"cus_1"}),json!({"message":"Success"})),
        ("/api/stripe/session/checkout",team_settings::load(&mut server(),"team",false).unwrap().checkout("Pro",true).unwrap(),json!({"url":"https://checkout.stripe.com/c/pay/session"})),
    ] { for status in [200,401,403] {
        let file=tempfile::NamedTempFile::new().unwrap();let mut db=Db::open(file.path()).unwrap();db.kv_set("current-user-id","42").unwrap();
        let id=db.enqueue(Write::Shell {request:ShellWrite::new(42,"POST",path,Some(body.clone())).unwrap()},100).unwrap();let key=db.mutations().unwrap()[0].client_id.clone();drop(db);let mut db=Db::open(file.path()).unwrap();
        let mut web=server();web.status=status;web.path=Some(path.into());web.body=body.clone();web.reply=reply.clone();
        assert_eq!(db.drain_once(&mut web,101).unwrap(),if status==200 {Drain::Applied(id)}else{Drain::Failed(id)});
        let result=db.take_shell_result(&key).unwrap().unwrap();
        if status==200 { for (key,value) in reply.as_object().unwrap() {assert_eq!(&result["value"][key],value);} assert!(db.mutations().unwrap().is_empty()); }
        else {assert_eq!(result["error"],"Team access denied");assert!(result["value"].is_null());assert!(db.mutations().unwrap()[0].payload_json["body"].get("apiKey").is_none());}
    }}
}
#[test]
fn checkout_cannot_replay_an_uncertain_creation_and_malformed_key_ack_never_succeeds() {
    let mut db=Db::memory().unwrap();db.kv_set("current-user-id","42").unwrap();
    let body=team_settings::load(&mut server(),"team",false).unwrap().checkout("Pro",false).unwrap();
    let id=db.enqueue(Write::Shell{request:ShellWrite::new(42,"POST","/api/stripe/session/checkout",Some(body.clone())).unwrap()},100).unwrap();let key=db.mutations().unwrap()[0].client_id.clone();
    let mut web=server();web.status=503;web.path=Some("/api/stripe/session/checkout".into());web.body=body;
    assert_eq!(db.drain_once(&mut web,100).unwrap(),Drain::Retrying(id));assert_eq!(db.drain_once(&mut web,101).unwrap(),Drain::Failed(id));assert_eq!(web.calls.iter().filter(|path|path.ends_with("checkout")).count(),1);assert!(db.take_shell_result(&key).unwrap().unwrap()["error"].as_str().unwrap().contains("uncertain"));
    let body=json!({"teamId":"team","provider":"openai","enabled":false});let id=db.enqueue(Write::Shell{request:ShellWrite::new(42,"POST","/api/teams/byokKeys",Some(body.clone())).unwrap()},102).unwrap();web.status=200;web.path=Some("/api/teams/byokKeys".into());web.body=body;web.reply=json!({"success":true});assert_eq!(db.drain_once(&mut web,102).unwrap(),Drain::Retrying(id));
}

#[test]
fn old_account_web_settings_queue_cannot_send_with_an_old_still_valid_bearer() {
    let mut db=Db::memory().unwrap();db.kv_set("current-user-id","42").unwrap();
    let body=json!({"teamId":"team","provider":"openai","enabled":false});
    let id=db.enqueue(Write::Shell{request:ShellWrite::new(42,"POST","/api/teams/byokKeys",Some(body.clone())).unwrap()},100).unwrap();
    db.kv_set("current-user-id","99").unwrap();
    let mut web=server();web.path=Some("/api/teams/byokKeys".into());web.body=body;web.reply=json!({"ok":true});
    assert_eq!(db.drain_once(&mut web,100).unwrap(),Drain::Failed(id));assert_eq!(web.calls,vec!["/user/context"]);
}
