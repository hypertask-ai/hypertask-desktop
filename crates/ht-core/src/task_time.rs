use crate::{Db, Result, Transport};
use anyhow::{Context, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value,json};

#[derive(Clone,Copy,Debug,PartialEq,Eq,Serialize,Deserialize)]
pub enum TimeWrite { Start { task:i64 }, Stop { task:i64 }, Board { project:i64, enabled:bool } }
impl TimeWrite {
    pub fn request(&self)->Result<(String,Value)> {
        Ok(match self {
            Self::Start{task}|Self::Stop{task}=>{ensure!(*task>0,"Task is unavailable");(format!("/api/time/{}",if matches!(self,Self::Start{..}){"start"}else{"stop"}),json!({"taskId":task}))},
            Self::Board{project,enabled}=>{ensure!(*project>0,"Board is unavailable");("/api/projects/time-tracking".into(),json!({"projectId":project,"enabled":enabled}))},
        })
    }
}
pub fn summary(transport:&mut impl Transport,task:i64)->Result<Value> {
    let response=transport.get(&format!("/api/time/task?taskId={task}"))?;
    ensure!(response.status==200 && response.body["success"]==true,"{}",response.body["error"].as_str().unwrap_or("Time tracking request failed"));
    let value=&response.body;
    ensure!(value["enabled"].is_boolean() && (value["runningEntry"].is_null() || value["runningEntry"].is_object()) && value["taskTotalSeconds"].as_i64().is_some_and(|n|n>=0) && value["activeRunningEntryCount"].as_u64().is_some(),"Invalid task time summary");
    if !value["runningEntry"].is_null() {ensure!(value["runningEntry"]["taskId"]==task && value["runningEntry"]["id"].as_i64().is_some_and(|id|id>0),"Wrong task timer");}
    Ok(response.body)
}
fn key(user:i64,task:i64)->String {format!("task-time:{user}:{task}")}
fn instant(value:&Value)->Option<i64>{chrono::DateTime::parse_from_rfc3339(value.as_str()?).ok().map(|date|date.timestamp())}
pub fn elapsed(entry:&Value,now:i64)->i64 {let end=instant(&entry["endedAt"]).or_else(||instant(&entry["pausedAt"])).unwrap_or(now);instant(&entry["startedAt"]).map(|start|(end-start).max(0)).unwrap_or(0)}
pub fn format_elapsed(seconds:i64)->String {let seconds=seconds.max(0);format!("{}:{:02}:{:02}",seconds/3600,(seconds%3600)/60,seconds%60)}
impl Db {
    pub fn task_time(&self,task:i64)->Result<Option<Value>> {let user=self.current_user_id().context("Unauthorized")?;self.kv_get(&key(user,task))?.map(|value|serde_json::from_str(&value).map_err(Into::into)).transpose()}
    pub fn cache_task_time(&self,user:i64,task:i64,mut summary:Value,now:i64)->Result<()> {
        ensure!(self.current_user_id()==Some(user),"Timer belongs to another account");
        if let Some(id)=summary["runningEntry"]["userId"].as_i64(){ensure!(id==user,"Timer belongs to another account");}
        summary["calculatedAt"]=json!(now);self.kv_set(&key(user,task),&summary.to_string())
    }
    pub(crate) fn prepare_time(&self,change:&TimeWrite)->Result<Value> {
        change.request()?;let user=self.current_user_id().context("Unauthorized")?;
        ensure!(!self.mutations()?.iter().any(|row|row.kind=="web_time" && row.status!="failed"),"Wait for the previous timer update");
        let before=match change {
            TimeWrite::Board{project,..}=>self.board_project(*project)?,
            TimeWrite::Start{task}|TimeWrite::Stop{task}=>{
                ensure!(self.task(*task)?.is_some(),"Task is unavailable");
                let summary=self.task_time(*task)?.context("Time tracking is still loading")?;
                if matches!(change,TimeWrite::Start{..}) {ensure!(summary["enabled"]==true && summary["runningEntry"].is_null(),"Time tracking is disabled for this board");}
                else{ensure!(summary["runningEntry"].is_object(),"No running timer");}
                summary
            }
        };
        Ok(json!({"user_id":user,"change":change,"before":before,"attempted":false}))
    }
    pub(crate) fn optimistic_time(&self,payload:&Value,mutation:i64,now:i64)->Result<()> {
        if self.current_user_id()!=payload["user_id"].as_i64(){return Ok(());}
        let change:TimeWrite=serde_json::from_value(payload["change"].clone())?;
        match change {
            TimeWrite::Board{project,enabled}=>{let mut row=self.board_project(project)?;row["timeTrackingEnabled"]=json!(enabled);if !enabled{row["showTimeTotals"]=json!(false);}self.upsert("Project",&row)?;}
            TimeWrite::Start{task}|TimeWrite::Stop{task}=>{
                let mut summary=payload["before"].clone();
                let delta=(now-summary["calculatedAt"].as_i64().unwrap_or(now)).max(0);
                summary["taskTotalSeconds"]=json!(summary["taskTotalSeconds"].as_i64().unwrap_or(0)+summary["activeRunningEntryCount"].as_i64().unwrap_or(0)*delta);
                if matches!(change,TimeWrite::Start{..}) {
                    summary["runningEntry"]=json!({"id":-mutation,"taskId":task,"userId":payload["user_id"],"startedAt":chrono::DateTime::from_timestamp(now,0).map(|date|date.to_rfc3339()),"endedAt":null,"pausedAt":null,"pending":true});
                    summary["activeRunningEntryCount"]=json!(summary["activeRunningEntryCount"].as_u64().unwrap_or(0)+1);
                }else{summary["runningEntry"]=Value::Null;summary["activeRunningEntryCount"]=json!(summary["activeRunningEntryCount"].as_u64().unwrap_or(0).saturating_sub(u64::from(payload["before"]["runningEntry"]["pausedAt"].is_null())));}
                self.cache_task_time(payload["user_id"].as_i64().unwrap(),task,summary,now)?;
            }
        }
        Ok(())
    }
    pub(crate) fn resolve_time(&self,payload:&Value,response:&Value,now:i64)->Result<()> {
        let user=payload["user_id"].as_i64().context("Unauthorized")?;ensure!(self.current_user_id()==Some(user),"Timer belongs to another account");
        let change:TimeWrite=serde_json::from_value(payload["change"].clone())?;
        match change {
            TimeWrite::Board{enabled,..}=>ensure!(response["enabled"]==enabled && response["timeTrackingEnabled"]==enabled && (enabled || response["showTimeTotals"]==false),"Time tracking was not acknowledged"),
            TimeWrite::Start{task}|TimeWrite::Stop{task}=>{
                let entry=&response["entry"];
                if !entry.is_null() {ensure!(entry["id"].as_i64().is_some_and(|id|id>0) && entry["taskId"]==task && entry["userId"]==user && instant(&entry["startedAt"]).is_some(),"Wrong timer acknowledgement");}
                let mut summary=self.task_time(task)?.context("Missing timer summary")?;
                if matches!(change,TimeWrite::Start{..}) {ensure!(entry.is_object() && entry["endedAt"].is_null(),"Timer did not start");summary["runningEntry"]=entry.clone();}
                else {ensure!(entry.is_null() || instant(&entry["endedAt"]).is_some(),"Timer did not stop");summary["runningEntry"]=Value::Null;}
                self.cache_task_time(user,task,summary,now)?;
            }
        }
        Ok(())
    }
    pub(crate) fn rollback_time(&self,payload:&Value)->Result<()> {
        if self.current_user_id()!=payload["user_id"].as_i64(){return Ok(());}
        match serde_json::from_value::<TimeWrite>(payload["change"].clone())? {
            TimeWrite::Board{project,..}=>{let mut row=self.board_project(project)?;for field in ["timeTrackingEnabled","showTimeTotals"]{row[field]=payload["before"][field].clone();}self.upsert("Project",&row)?;}
            TimeWrite::Start{task}|TimeWrite::Stop{task}=>self.kv_set(&key(payload["user_id"].as_i64().unwrap(),task),&payload["before"].to_string())?,
        }
        Ok(())
    }
}
