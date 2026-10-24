use crate::{Db, Result, Transport, http::Response};
use anyhow::{Context, ensure};
use serde_json::{Value,json};

pub fn cache_key(user:i64,kind:&str)->String {format!("board-sidebar:{user}:{kind}")}
pub fn load(transport:&mut impl Transport)->Result<(i64,Value,Value)> {
    let identity=transport.get("/user/context")?;
    ensure!(identity.status==200 && identity.body["success"]==true,"Unauthorized");
    let user=identity.body["user"]["id"].as_i64().filter(|id|*id>0).context("Unauthorized")?;
    let teams=transport.post("/api/teams/getAllSidebar",&json!({}),"sidebar-read")?;
    let favorites=transport.get("/api/favorites/getFavorites")?;
    let read=|r:Response|->Result<Value>{ensure!((200..300).contains(&r.status),"{}",r.body["message"].as_str().or(r.body["error"].as_str()).unwrap_or("Request failed"));ensure!(r.body.is_array(),"Invalid board metadata");Ok(r.body)};
    Ok((user,read(teams)?,read(favorites)?))
}
impl Db {
    pub fn cache_sidebar(&mut self,user:i64,teams:&Value,favorites:&Value)->Result<()> {
        ensure!(self.current_user_id()==Some(user),"Board metadata belongs to another account");
        ensure!(teams.is_array() && favorites.is_array(),"Invalid board metadata");
        self.transaction(|db|{
            for team in teams.as_array().unwrap() {
                let id=team["id"].as_i64().filter(|id|*id>0).context("Invalid team")?;
                for project in team["projects"].as_array().context("Missing team boards")? {
                    let mut project=project.clone();project["teamId"]=json!(id);db.upsert("Project",&project)?;
                }
            }
            db.kv_set(&cache_key(user,"teams"),&teams.to_string())?;
            db.kv_set(&cache_key(user,"favorites"),&favorites.to_string())?;
            db.reapply_pending()?; Ok(())
        })
    }
    pub fn sidebar_data(&self,kind:&str)->Value {
        self.current_user_id().and_then(|user|self.kv_get(&cache_key(user,kind)).ok().flatten()).and_then(|s|serde_json::from_str(&s).ok()).unwrap_or(json!([]))
    }
    pub(crate) fn prepare_favorites(&self,ids:&[i64])->Result<Value> {
        let user=self.current_user_id().context("Unauthorized")?;
        ensure!(ids.len()<=10 && ids.iter().all(|id|*id>0),"Too many favorites (max 10)");
        let mut seen=std::collections::BTreeSet::new();
        let favorites=ids.iter().enumerate().map(|(position,id)|{
            ensure!(seen.insert(*id),"Duplicate favorite");
            let project=self.raw("Project",&json!(id))?.context("Board is not cached")?;
            Ok(json!({"projectId":id,"project":project,"index":if position==9 {0}else{position+1}}))
        }).collect::<Result<Vec<_>>>()?;
        Ok(json!({"user_id":user,"before":self.sidebar_data("favorites"),"favorites":favorites,"body":{"favorites":ids.iter().map(|id|json!({"projectId":id})).collect::<Vec<_>>()}}))
    }
}
