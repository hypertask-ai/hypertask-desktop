use super::*;
use ht_core::TimeWrite;

#[derive(Default)]
pub(super) struct TaskTime {
    loaded: Option<(i64,i64)>,
    generation:u64,
    loading:bool,
    ticking:bool,
    pending:Option<(String,i64)>,
}
impl Desktop {
    pub(super) fn load_task_time(&mut self,cx:&mut Context<Self>) {
        if self.options.offline || self.screen!=Screen::Task {return;}
        let Some(task)=self.selected_task() else{return;};let Some(user)=self.db.current_user_id() else{return;};
        if self.task_time.loaded==Some((user,task)) || self.task_time.pending.is_some(){return;}
        let Some(token)=self.cli_token() else{return;};
        self.task_time.loaded=Some((user,task));self.task_time.generation+=1;self.task_time.loading=true;
        let generation=self.task_time.generation;let expected=token.clone();let (send,receive)=std::sync::mpsc::channel();
        std::thread::spawn(move||{let result=ht_core::Http::new(token).and_then(|mut http|{
            let identity=ht_core::Transport::get(&mut http,"/user/context")?;
            anyhow::ensure!(identity.status==200 && identity.body["user"]["id"]==user,"Unauthorized");
            ht_core::task_time::summary(&mut http,task)
        }).map_err(|error|error.to_string());let _=send.send(result);});
        cx.spawn(async move|view,cx|{loop{cx.background_executor().timer(Duration::from_millis(50)).await;match receive.try_recv(){
            Ok(result)=>{let _=view.update(cx,|this,cx|{
                if this.task_time.generation!=generation{return;}
                this.task_time.loading=false;
                if this.db.current_user_id()!=Some(user) || this.cli_token().as_deref()!=Some(expected.as_str()){return;}
                match result.and_then(|summary|this.db.cache_task_time(user,task,summary,chrono::Utc::now().timestamp()).map_err(|error|error.to_string())) {Ok(())=>(),Err(error)=>this.error=Some(error)}
                cx.notify();
            });break;},Err(std::sync::mpsc::TryRecvError::Empty)=>(),_=>break,
        }}}).detach();
    }
    pub(super) fn tick_task_time(&mut self,cx:&mut Context<Self>) {
        if self.task_time.ticking || self.screen!=Screen::Task || self.selected_task().and_then(|task|self.db.task_time(task).ok().flatten()).is_none_or(|summary|summary["activeRunningEntryCount"].as_u64().unwrap_or(0)==0){return;}
        self.task_time.ticking=true;
        cx.spawn(async move|view,cx|{let mut ticks=0;loop{cx.background_executor().timer(Duration::from_secs(1)).await;ticks+=1;
            if !view.update(cx,|this,cx|{
                let active=this.screen==Screen::Task && this.selected_task().and_then(|task|this.db.task_time(task).ok().flatten()).is_some_and(|summary|summary["activeRunningEntryCount"].as_u64().unwrap_or(0)>0);
                if !active {this.task_time.ticking=false;return false;}
                if ticks%30==0 && !this.task_time.loading {this.task_time.loaded=None;this.load_task_time(cx);}
                cx.notify();true
            }).unwrap_or(false){break;}
        }}).detach();
    }
    pub(super) fn toggle_task_timer(&mut self,window:&mut Window,cx:&mut Context<Self>) {
        if self.screen!=Screen::Task || self.task_time.pending.is_some(){return;}
        let Some(task)=self.selected_task() else{return;};
        let Some(summary)=self.db.task_time(task).ok().flatten() else{self.load_task_time(cx);return;};
        if !summary["runningEntry"].is_null(){self.queue_time(TimeWrite::Stop{task},window,cx);}
        else if summary["enabled"]==true{self.queue_time(TimeWrite::Start{task},window,cx);}
    }
    pub(super) fn toggle_board_time(&mut self,window:&mut Window,cx:&mut Context<Self>) {
        if self.screen==Screen::Inbox || self.task_time.pending.is_some(){return;}
        if let Ok(project)=self.db.board_project(self.options.board){self.queue_time(TimeWrite::Board{project:self.options.board,enabled:project["timeTrackingEnabled"]!=true},window,cx);}
    }
    fn queue_time(&mut self,change:TimeWrite,window:&mut Window,cx:&mut Context<Self>) {
        match self.db.enqueue(Write::Time{change},chrono::Utc::now().timestamp()) {
            Ok(id)=>{
                let row=self.db.mutations().unwrap().into_iter().find(|row|row.id==id).unwrap();
                self.task_time.pending=Some((row.client_id,self.db.current_user_id().unwrap_or(0)));self.error=None;self.reload();self.request_sync();
                if !self.options.offline{cx.spawn_in(window,async move|view,cx|{loop{cx.background_executor().timer(Duration::from_millis(100)).await;if !view.update_in(cx,|this,_,cx|this.poll_time(cx)).unwrap_or(false){break;}}}).detach();}
            }
            Err(error)=>self.error=Some(error.to_string()),
        }
        cx.notify();
    }
    pub(super) fn poll_time(&mut self,cx:&mut Context<Self>)->bool {
        let Some((key,user))=self.task_time.pending.clone() else{return false;};
        let Some(result)=self.db.kv_get(&format!("time-result:{key}")).ok().flatten().and_then(|value|serde_json::from_str::<Value>(&value).ok()) else{return true;};
        self.task_time.pending=None;
        if self.db.current_user_id()==Some(user){
            self.error=result["error"].as_str().map(str::to_owned);self.reload();self.task_time.loaded=None;self.load_task_time(cx);
        }
        cx.notify();false
    }
    pub(super) fn add_time_commands(&self,rows:&mut Vec<crate::command_center::Command>) {
        use crate::command_center::{Command,CommandAction,ShellCommand};
        let project=self.db.board_project(self.options.board).unwrap_or(Value::Null);
        let mut commands=Vec::new();
        if matches!(self.screen,Screen::Task|Screen::Board) && project.is_object(){commands.push(Command{key:"toggleBoardTimeTracking".into(),name:if project["timeTrackingEnabled"]==true{"Turn off time tracking for this board"}else{"Turn on time tracking for this board"}.into(),group:"Time",keys:String::new(),keywords:"time tracking enable disable turn on off board timer show",action:CommandAction::Shell(ShellCommand::BoardTime)});}
        if self.screen==Screen::Task && project["timeTrackingEnabled"]==true {commands.push(Command{key:"toggleTimeTracking".into(),name:"Start or stop timer".into(),group:"Time",keys:"W".into(),keywords:"start stop toggle time tracking timer work log",action:CommandAction::Shell(ShellCommand::TaskTimer)});}
        let index=rows.first().map_or(0,|first|rows.iter().position(|row|row.group!=first.group).unwrap_or(rows.len()));
        rows.splice(index..index,commands);
    }
    pub(super) fn task_time_row(&self,cx:&mut Context<Self>)->AnyElement {
        let Some(summary)=self.selected_task().and_then(|task|self.db.task_time(task).ok().flatten()) else{return div().into_any_element();};
        let running=&summary["runningEntry"];let enabled=summary["enabled"]==true;
        if !enabled && running.is_null() && summary["taskTotalSeconds"].as_i64().unwrap_or(0)==0{return div().into_any_element();}
        let now=chrono::Utc::now().timestamp();
        let total=summary["taskTotalSeconds"].as_i64().unwrap_or(0)+summary["activeRunningEntryCount"].as_i64().unwrap_or(0)*(now-summary["calculatedAt"].as_i64().unwrap_or(now)).max(0);
        div().flex().items_center().gap(px(20.)).mt(px(24.))
            .child(div().w(px(260./3.)).flex_shrink_0().child("Time"))
            .child(div().flex().gap(px(8.))
                .when(!running.is_null(),|d|d.child(div().text_color(color(0x52a869)).child(ht_core::task_time::format_elapsed(ht_core::task_time::elapsed(running,now)))))
                .when(summary["otherEntriesSeconds"].as_i64().unwrap_or(0)>0,|d|d.child(format!("{}{}",ht_core::task_time::format_elapsed(total),if running.is_null(){""}else{" total"})))
                .when(enabled || !running.is_null(),|d|d.child(div().id("task-timer-toggle").cursor_pointer().when(self.task_time.pending.is_some(),|d|d.opacity(0.5)).child(if running.is_null(){"Start"}else{"Stop"}).on_click(cx.listener(|this,_,window,cx|this.toggle_task_timer(window,cx))).layout_key(&self.layout,"task.time.toggle",None))))
            .layout_key(&self.layout,"task.time",None).into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::{task_fields::tests::desktop,shell::tests::{click,draw,measured}};
    struct Server(u16,bool,bool);
    impl ht_core::Transport for Server {
        fn get(&mut self,path:&str)->Result<ht_core::http::Response,ht_core::http::TransportError>{assert_eq!(path,"/user/context");Ok(ht_core::http::Response{status:200,body:serde_json::json!({"success":true,"user":{"id":42}})})}
        fn post(&mut self,path:&str,body:&Value,key:&str)->Result<ht_core::http::Response,ht_core::http::TransportError>{assert!(!key.is_empty());assert_eq!(path,if self.2{"/api/projects/time-tracking"}else if self.1{"/api/time/stop"}else{"/api/time/start"});if !self.2{assert_eq!(*body,serde_json::json!({"taskId":1}));}
            let mut entry=serde_json::json!({"id":5,"taskId":1,"userId":42,"startedAt":"2026-10-01T09:00:00Z","endedAt":null,"pausedAt":null});if self.1{entry["endedAt"]=serde_json::json!("2026-10-01T09:01:00Z");}
            Ok(ht_core::http::Response{status:self.0,body:if self.0!=200{serde_json::json!({"error":"Time tracking is disabled for this board"})}else if self.2{serde_json::json!({"success":true,"enabled":false,"timeTrackingEnabled":false,"showTimeTotals":false})}else{serde_json::json!({"success":true,"entry":entry})}})
        }
    }
    fn seed(this:&mut Desktop,running:bool) {
        this.db.apply_response(&serde_json::json!({"success":true,"projects":[{"id":15,"title":"Board","timeTrackingEnabled":true,"showTimeTotals":true}]})).unwrap();
        this.db.cache_task_time(42,1,serde_json::json!({"enabled":true,"runningEntry":if running{serde_json::json!({"id":5,"taskId":1,"userId":42,"startedAt":"2026-10-01T09:00:00Z","endedAt":null,"pausedAt":null})}else{Value::Null},"activeRunningEntryCount":if running{1}else{0},"taskTotalSeconds":120,"myTotalSeconds":60,"otherEntriesSeconds":60}),chrono::Utc::now().timestamp()).unwrap();
    }
    #[gpui::test]
    fn task_timer_w_and_inline_control_queue_once_and_restore_refused_running_state(cx:&mut gpui::TestAppContext) {
        cx.update(crate::input::init);let view=cx.open_window(gpui::size(px(1440.),px(900.)),|window,cx|desktop(window,cx));let mut cx=gpui::VisualTestContext::from_window(view.into(),cx);
        for running in [false,true] {for status in [401,403,200] {
            view.update(&mut cx,|this,_,_|seed(this,running)).unwrap();draw(view,&mut cx);measured(view,&mut cx,"task.time.toggle");
            if running{click(view,&mut cx,"task.time.toggle");}else if status==200{cx.simulate_keystrokes("ctrl-k");cx.simulate_input("Start or stop timer");cx.simulate_keystrokes("enter");}else{cx.simulate_keystrokes("w w");}
            view.update(&mut cx,|this,_,cx|{assert_eq!(this.db.mutations().unwrap().iter().filter(|row|row.status!="failed").count(),1);assert_eq!(this.db.task_time(1).unwrap().unwrap()["runningEntry"].is_object(),!running);this.db.drain_once(&mut Server(status,running,false),chrono::Utc::now().timestamp()+1).unwrap();assert!(!this.poll_time(cx));assert_eq!(this.db.task_time(1).unwrap().unwrap()["runningEntry"].is_object(),if status==200{!running}else{running});if status!=200{assert_eq!(this.error.as_deref(),Some("Time tracking is disabled for this board"));}}).unwrap();
        }}
    }
    #[gpui::test]
    fn timer_palette_and_board_tracking_are_context_gated_and_restore_both_project_settings(cx:&mut gpui::TestAppContext) {
        cx.update(crate::input::init);let view=cx.open_window(gpui::size(px(1440.),px(900.)),|window,cx|desktop(window,cx));let mut cx=gpui::VisualTestContext::from_window(view.into(),cx);
        for status in [401,403,200] {
            view.update(&mut cx,|this,_,_|seed(this,false)).unwrap();draw(view,&mut cx);cx.simulate_keystrokes("ctrl-k");cx.simulate_input("Turn off time tracking for this board");cx.simulate_keystrokes("enter");
            view.update(&mut cx,|this,_,cx|{assert_eq!(this.db.board_project(15).unwrap()["timeTrackingEnabled"],false);this.db.drain_once(&mut Server(status,false,true),chrono::Utc::now().timestamp()+1).unwrap();this.poll_time(cx);let project=this.db.board_project(15).unwrap();assert_eq!(project["timeTrackingEnabled"],status!=200);assert_eq!(project["showTimeTotals"],status!=200);if status==200{assert!(!this.command_rows().iter().any(|row|row.key=="toggleTimeTracking"));}}).unwrap();
        }
        view.update(&mut cx,|this,_,_|{this.screen=Screen::Inbox;assert!(!this.command_rows().iter().any(|row|row.key=="toggleTimeTracking"||row.key=="toggleBoardTimeTracking"));}).unwrap();
    }
}
