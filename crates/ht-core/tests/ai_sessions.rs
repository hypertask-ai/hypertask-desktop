use ht_core::{AiSessionWrite,Db,Drain,Transport,Write,http::{Response,TransportError}};
use serde_json::{Value,json};
struct Server {status:u16,user:i64,calls:usize,change:AiSessionWrite}
impl Transport for Server {
    fn get(&mut self,path:&str)->Result<Response,TransportError>{assert_eq!(path,"/user/context");Ok(Response{status:200,body:json!({"success":true,"user":{"id":self.user}})})}
    fn post(&mut self,path:&str,body:&Value,key:&str)->Result<Response,TransportError>{self.mutate("POST",path,body,key)}
    fn mutate(&mut self,method:&str,path:&str,body:&Value,key:&str)->Result<Response,TransportError>{
        let(expected_method,expected_path,expected_body)=self.change.request().unwrap();
        assert_eq!(method,expected_method);assert_eq!(path,expected_path);assert_eq!(body,&expected_body);assert!(!key.is_empty());self.calls+=1;
        Ok(Response{status:self.status,body:if self.status!=200{json!({"error":"Unauthorized"})}else{match &self.change{
            AiSessionWrite::Create{..}=>json!({"success":true,"session":{"id":"created","userId":42,"title":"New chat","messages":[]}}),
            AiSessionWrite::Rename{id,title}=>json!({"success":true,"session":{"id":id,"userId":42,"title":title.trim()}}),
            _=>json!({"message":"Session deleted successfully"}),
        }}})
    }
}
fn seed(db:&Db){db.kv_set("current-user-id","42").unwrap();db.cache_ai_sessions(&[json!({"id":"saved","title":"Original","messages":[{"role":"human","content":"Keep transcript"}]}),json!({"id":"other","title":"Other","messages":[]})]).unwrap();}
#[test]
fn lifecycle_routes_are_durable_optimistic_and_rollback_both_refusals(){
    for change in [AiSessionWrite::Create{task:Some(7),project:Some(15)},AiSessionWrite::Rename{id:"saved".into(),title:"  Renamed  ".into()},AiSessionWrite::Delete{id:"saved".into()},AiSessionWrite::DeleteAll] {
        for status in [200,401,403] {
            let file=tempfile::NamedTempFile::new().unwrap();let mut db=Db::open(file.path()).unwrap();seed(&db);let before=db.ai_sessions().unwrap();
            db.enqueue(Write::AiSession{change:change.clone()},1).unwrap();let row=db.mutations().unwrap().pop().unwrap();
            match &change {AiSessionWrite::Rename{..}=>assert_eq!(db.ai_sessions().unwrap()[0]["title"],"Renamed"),AiSessionWrite::Delete{..}=>assert_eq!(db.ai_sessions().unwrap().len(),1),AiSessionWrite::DeleteAll=>assert!(db.ai_sessions().unwrap().is_empty()),_=>assert_eq!(db.ai_sessions().unwrap(),before)}
            drop(db);let mut db=Db::open(file.path()).unwrap();let mut server=Server{status,user:42,calls:0,change:change.clone()};
            assert_eq!(db.drain_once(&mut server,2).unwrap(),if status==200{Drain::Applied(row.id)}else{Drain::Failed(row.id)});assert_eq!(server.calls,1);
            let receipt:Value=serde_json::from_str(&db.kv_get(&format!("ai-session-result:{}",row.client_id)).unwrap().unwrap()).unwrap();
            assert_eq!(receipt["error"].is_null(),status==200);
            if status!=200{assert_eq!(db.ai_sessions().unwrap(),before);}else{match &change{
                AiSessionWrite::Create{..}=>assert!(db.ai_sessions().unwrap().iter().any(|s|s["id"]=="created")),
                AiSessionWrite::Rename{..}=>{assert_eq!(db.ai_sessions().unwrap()[0]["title"],"Renamed");assert_eq!(db.ai_sessions().unwrap()[0]["messages"],before[0]["messages"]);},
                AiSessionWrite::Delete{..}=>assert_eq!(db.ai_sessions().unwrap().len(),1),AiSessionWrite::DeleteAll=>assert!(db.ai_sessions().unwrap().is_empty()),
            }}
        }
    }
}
#[test]
fn identity_outages_do_not_mark_create_attempted_and_other_accounts_cannot_write(){
    let mut db=Db::memory().unwrap();seed(&db);let change=AiSessionWrite::Create{task:None,project:None};db.enqueue(Write::AiSession{change:change.clone()},1).unwrap();
    let mut server=Server{status:200,user:99,calls:0,change};assert!(matches!(db.drain_once(&mut server,2).unwrap(),Drain::Failed(_)));assert_eq!(server.calls,0);
}
#[test]
fn uncertain_create_is_never_replayed_after_restart(){
    struct Lost;
    impl Transport for Lost{
        fn get(&mut self,_:&str)->Result<Response,TransportError>{Ok(Response{status:200,body:json!({"success":true,"user":{"id":42}})})}
        fn post(&mut self,_:&str,_:&Value,_:&str)->Result<Response,TransportError>{panic!("uses mutate")}
        fn mutate(&mut self,_:&str,_:&str,_:&Value,_:&str)->Result<Response,TransportError>{Err(TransportError::Timeout)}
    }
    let file=tempfile::NamedTempFile::new().unwrap();let mut db=Db::open(file.path()).unwrap();seed(&db);let change=AiSessionWrite::Create{task:None,project:None};db.enqueue(Write::AiSession{change:change.clone()},1).unwrap();assert!(matches!(db.drain_once(&mut Lost,2).unwrap(),Drain::Retrying(_)));
    drop(db);let mut db=Db::open(file.path()).unwrap();let mut server=Server{status:200,user:42,calls:0,change};assert!(matches!(db.drain_once(&mut server,4).unwrap(),Drain::Failed(_)));assert_eq!(server.calls,0);
}
