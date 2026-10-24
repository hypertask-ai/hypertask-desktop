use ht_core::{Db,Drain,TimeWrite,Transport,Write,task_time,http::{Response,TransportError}};
use serde_json::{Value,json};
fn entry()->Value{json!({"id":5,"taskId":1,"userId":42,"startedAt":"2026-10-01T09:00:00Z","endedAt":null,"pausedAt":null})}
fn seed(db:&mut Db,running:bool) {
    db.kv_set("current-user-id","42").unwrap();db.apply_response(&json!({"success":true,"projects":[{"id":15,"title":"Board","timeTrackingEnabled":true,"showTimeTotals":true}],"tasks":[{"id":1,"projectId":15,"title":"Task","status":"Normal"}]})).unwrap();
    db.cache_task_time(42,1,json!({"enabled":true,"runningEntry":if running{entry()}else{Value::Null},"activeRunningEntryCount":if running{1}else{0},"myTotalSeconds":60,"taskTotalSeconds":120,"otherEntriesSeconds":60}),100).unwrap();
}
struct Server{status:u16,path:&'static str,body:Value,reply:Value,user:i64,calls:usize}
impl Transport for Server {
    fn get(&mut self,path:&str)->Result<Response,TransportError>{assert_eq!(path,"/user/context");Ok(Response{status:200,body:json!({"success":true,"user":{"id":self.user}})})}
    fn post(&mut self,path:&str,body:&Value,key:&str)->Result<Response,TransportError>{assert_eq!(path,self.path);assert_eq!(*body,self.body);assert!(!key.is_empty());self.calls+=1;Ok(Response{status:self.status,body:if self.status<300{self.reply.clone()}else{json!({"success":false,"error":"Time tracking is disabled for this board"})}})}
}
#[test]
fn timer_start_stop_and_board_setting_are_queued_optimistically_with_restart_refusal_rollback() {
    for (change,path,body) in [(TimeWrite::Start{task:1},"/api/time/start",json!({"taskId":1})),(TimeWrite::Stop{task:1},"/api/time/stop",json!({"taskId":1})),(TimeWrite::Board{project:15,enabled:false},"/api/projects/time-tracking",json!({"projectId":15,"enabled":false}))] {
        for status in [200,401,403] {
            let file=tempfile::NamedTempFile::new().unwrap();let mut db=Db::open(file.path()).unwrap();seed(&mut db,matches!(change,TimeWrite::Stop{..}));let before=db.task_time(1).unwrap().unwrap();
            let id=db.enqueue(Write::Time{change:change.clone()},100).unwrap();let key=db.mutations().unwrap()[0].client_id.clone();
            match change{TimeWrite::Entry{..}=>unreachable!(),TimeWrite::Start{..}=>assert_eq!(db.task_time(1).unwrap().unwrap()["runningEntry"]["id"],-id),TimeWrite::Stop{..}=>assert!(db.task_time(1).unwrap().unwrap()["runningEntry"].is_null()),TimeWrite::Board{..}=>{let project=db.board_project(15).unwrap();assert_eq!(project["timeTrackingEnabled"],false);assert_eq!(project["showTimeTotals"],false);}}
            drop(db);let mut db=Db::open(file.path()).unwrap();
            let mut stopped=entry();stopped["endedAt"]=json!("2026-10-01T09:01:00Z");
            let reply=match change{TimeWrite::Entry{..}=>unreachable!(),TimeWrite::Start{..}=>json!({"success":true,"entry":entry()}),TimeWrite::Stop{..}=>json!({"success":true,"entry":stopped}),TimeWrite::Board{..}=>json!({"success":true,"enabled":false,"timeTrackingEnabled":false,"showTimeTotals":false})};
            assert_eq!(db.drain_once(&mut Server{status,path,body:body.clone(),reply,user:42,calls:0},100).unwrap(),if status==200{Drain::Applied(id)}else{Drain::Failed(id)});
            let result:Value=serde_json::from_str(&db.kv_get(&format!("time-result:{key}")).unwrap().unwrap()).unwrap();
            if status!=200 {assert_eq!(db.task_time(1).unwrap().unwrap(),before);assert_eq!(db.board_project(15).unwrap()["timeTrackingEnabled"],true);assert_eq!(db.board_project(15).unwrap()["showTimeTotals"],true);assert_eq!(result["error"],"Time tracking is disabled for this board");}
            else{assert!(result["error"].is_null());if matches!(change,TimeWrite::Start{..}){assert_eq!(db.task_time(1).unwrap().unwrap()["runningEntry"]["id"],5);}}
        }
    }
}
#[test]
fn uncertain_timer_write_is_not_replayed_and_account_switch_never_sends() {
    for user in [42,99] {
        let mut db=Db::memory().unwrap();seed(&mut db,false);let before=db.task_time(1).unwrap().unwrap();let id=db.enqueue(Write::Time{change:TimeWrite::Start{task:1}},100).unwrap();
        let mut server=Server{status:500,path:"/api/time/start",body:json!({"taskId":1}),reply:Value::Null,user,calls:0};
        assert_eq!(db.drain_once(&mut server,100).unwrap(),Drain::Failed(id));assert_eq!(server.calls,if user==42{1}else{0});assert_eq!(db.task_time(1).unwrap().unwrap(),before);
        assert!(matches!(db.drain_once(&mut server,1000).unwrap(),Drain::Empty));
    }
}
#[test]
fn timer_summary_read_and_elapsed_paused_boundaries_follow_web() {
    struct Read(u16);
    impl Transport for Read {
        fn get(&mut self,path:&str)->Result<Response,TransportError>{assert_eq!(path,"/api/time/task?taskId=1");Ok(Response{status:self.0,body:if self.0==200{json!({"success":true,"enabled":true,"runningEntry":entry(),"taskTotalSeconds":120,"activeRunningEntryCount":1})}else{json!({"error":"Unauthorized"})}})}
        fn post(&mut self,_:&str,_:&Value,_:&str)->Result<Response,TransportError>{panic!("read only")}
    }
    assert!(task_time::summary(&mut Read(200),1).is_ok());for status in [401,403]{assert_eq!(task_time::summary(&mut Read(status),1).unwrap_err().to_string(),"Unauthorized");}
    let mut paused=entry();paused["pausedAt"]=json!("2026-10-01T09:01:02Z");assert_eq!(task_time::elapsed(&paused,i64::MAX),62);assert_eq!(task_time::format_elapsed(3662),"1:01:02");assert_eq!(task_time::format_elapsed(-1),"0:00:00");
}
