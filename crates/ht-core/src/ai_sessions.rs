use crate::{Db, Result, Transport, http::{Response, TransportError}};
use anyhow::{Context, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AiSessionWrite {
    Create { task: Option<i64>, project: Option<i64> },
    Rename { id: String, title: String },
    Delete { id: String },
    DeleteAll,
}
impl AiSessionWrite {
    pub fn request(&self) -> Result<(&'static str, String, Value)> {
        let valid_id = |id: &str| !id.is_empty() && id != "ALL" && !id.starts_with("local:");
        Ok(match self {
            Self::Create { task, project } => {
                ensure!(task.is_none_or(|v|v>0) && project.is_none_or(|v|v>0), "Invalid chat scope");
                let mut body=json!({});
                if let Some(id)=task {body["taskId"]=json!(id);}
                if let Some(id)=project {body["projectId"]=json!(id);}
                ("POST", "/api/ai-chat/create-session".into(), body)
            }
            Self::Rename { id, title } => {
                ensure!(valid_id(id) && !title.trim().is_empty(), "Select a chat and enter a title");
                ("POST", "/api/ai-chat/update-session".into(), json!({"sessionId":id,"title":title.trim()}))
            }
            Self::Delete { id } => {
                ensure!(valid_id(id), "Select a saved chat");
                ("DELETE", format!("/api/ai-chat/delete-session?{}",url::form_urlencoded::Serializer::new(String::new()).append_pair("delete",id).finish()), json!({}))
            }
            Self::DeleteAll => ("DELETE", "/api/ai-chat/delete-session?delete=ALL".into(), json!({})),
        })
    }
    pub(crate) fn optimistic(&self, rows: &mut Vec<Value>) {
        match self {
            Self::Rename { id, title } => {for row in rows {if row["id"]==*id {row["title"]=json!(title.trim());}}}
            Self::Delete { id } => rows.retain(|row|row["id"]!=*id),
            Self::DeleteAll => rows.clear(),
            Self::Create { .. } => (),
        }
    }
}
fn cache_key(user:i64)->String {format!("ai-sessions:{user}")}
impl Db {
    pub fn enqueue_ai_session(&mut self,change:AiSessionWrite,sessions:&[Value],continuation:&Value,now:i64)->Result<String>{
        self.transaction(|db|{
            db.cache_ai_sessions(sessions)?;
            let id=db.enqueue_in_transaction(crate::Write::AiSession{change:change.clone()},now)?;
            let row=db.mutations()?.into_iter().find(|row|row.id==id).context("Missing chat mutation")?;
            let user=db.current_user_id().context("Unauthorized")?;
            db.kv_set(&format!("ai-session-pending:{user}"),&json!({"key":row.client_id,"change":change,"continuation":continuation}).to_string())?;
            Ok(row.client_id)
        })
    }
    pub fn ai_sessions(&self)->Result<Vec<Value>> {
        let user=self.current_user_id().context("Sign in to load chats")?;
        Ok(self.kv_get(&cache_key(user))?.map(|v|serde_json::from_str(&v)).transpose()?.unwrap_or_default())
    }
    pub fn cache_ai_sessions(&self, rows:&[Value])->Result<()> {
        let user=self.current_user_id().context("Sign in to load chats")?;
        self.kv_set(&cache_key(user),&serde_json::to_string(rows)?)
    }
    pub(crate) fn optimistic_ai_session(&self,payload:&Value)->Result<()> {
        if self.current_user_id()!=payload["user_id"].as_i64(){return Ok(());}
        let change:AiSessionWrite=serde_json::from_value(payload["change"].clone())?;
        let mut rows=self.ai_sessions()?;change.optimistic(&mut rows);self.cache_ai_sessions(&rows)
    }
    pub(crate) fn resolve_ai_session(&self,payload:&Value,body:&Value)->Result<()> {
        ensure!(self.current_user_id()==payload["user_id"].as_i64(),"Chat belongs to another account");
        let change:AiSessionWrite=serde_json::from_value(payload["change"].clone())?;
        let mut rows=self.ai_sessions()?;
        match &change {
            AiSessionWrite::Create { .. } | AiSessionWrite::Rename { .. } => {
                let session=&body["session"];
                let id=session["id"].as_str().filter(|s|!s.is_empty()).context("Missing chat acknowledgement")?;
                if let AiSessionWrite::Rename { id:expected, .. }=&change {ensure!(id==expected,"Wrong chat acknowledgement");}
                if let Some(user)=session["userId"].as_i64(){ensure!(Some(user)==self.current_user_id(),"Wrong chat owner");}
                if matches!(change,AiSessionWrite::Create { .. }) {ensure!(session["messages"].is_array(),"Missing chat messages");}
                if let Some(row)=rows.iter_mut().find(|row|row["id"]==id) {
                    for(key,value)in session.as_object().context("Invalid chat")?{row[key]=value.clone();}
                }else{rows.insert(0,session.clone());}
            }
            _ => {ensure!(body["message"].is_string(),"Missing deletion acknowledgement");change.optimistic(&mut rows);}
        }
        self.cache_ai_sessions(&rows)
    }
    pub(crate) fn rollback_ai_session(&self,payload:&Value)->Result<()> {
        let user=payload["user_id"].as_i64().context("Missing chat owner")?;
        self.kv_set(&cache_key(user),&payload["before"].to_string())
    }
    pub(crate) fn send_ai_session(&mut self,transport:&mut impl Transport,row:&crate::queue::Mutation)->std::result::Result<Response,TransportError> {
        let payload=&row.payload_json;
        let change:AiSessionWrite=serde_json::from_value(payload["change"].clone()).map_err(|_|TransportError::InvalidResponse)?;
        let(method,path,body)=change.request().map_err(|_|TransportError::InvalidResponse)?;
        // The web create route has no idempotency contract; a lost acknowledgement must not create a second session.
        if matches!(change,AiSessionWrite::Create { .. }) && payload["attempted"]==true {
            return Ok(Response{status:409,body:json!({"error":"Chat creation result uncertain; refresh history before trying again"})});
        }
        let identity=transport.get("/user/context")?;
        if !(200..300).contains(&identity.status){return Ok(identity);}
        if identity.body["success"]!=true || identity.body["user"]["id"]!=payload["user_id"] || self.current_user_id()!=payload["user_id"].as_i64(){return Ok(Response{status:403,body:json!({"error":"Forbidden"})});}
        if matches!(change,AiSessionWrite::Create { .. }) {
            let mut attempted=payload.clone();attempted["attempted"]=json!(true);
            self.conn.execute("UPDATE mutation_queue SET payload_json=? WHERE id=?",rusqlite::params![attempted.to_string(),row.id]).map_err(|_|TransportError::InvalidResponse)?;
        }
        transport.mutate(method,&path,&body,&row.client_id).map(|reply| {
            if (200..300).contains(&reply.status) && !matches!(change,AiSessionWrite::Create{..}|AiSessionWrite::Rename{..}) && reply.body["message"].is_string() {
                Response{status:reply.status,body:json!({"success":true,"message":reply.body["message"]})}
            }else{reply}
        })
    }
}
