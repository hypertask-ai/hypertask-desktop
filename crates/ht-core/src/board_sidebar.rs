use crate::{Db, Result, Transport, http::Response};
use anyhow::{Context, ensure};
use serde_json::{Value, json};

pub fn cache_key(user: i64, kind: &str) -> String {
    format!("board-sidebar:{user}:{kind}")
}
pub fn load(transport: &mut impl Transport) -> Result<(i64, Value, Value)> {
    let identity = transport.get("/user/context")?;
    ensure!(
        identity.status == 200 && identity.body["success"] == true,
        "Unauthorized"
    );
    let user = identity.body["user"]["id"]
        .as_i64()
        .filter(|id| *id > 0)
        .context("Unauthorized")?;
    let teams = transport.post(
        "/api/teams/getAllSidebar",
        &json!({"userId":user}),
        "sidebar-read",
    );
    let favorites = transport.get("/api/favorites/getFavorites");
    let read = |r: Response| -> Result<Value> {
        ensure!(
            (200..300).contains(&r.status),
            "{}",
            r.body["message"]
                .as_str()
                .or(r.body["error"].as_str())
                .unwrap_or("Request failed")
        );
        ensure!(r.body.is_array(), "Invalid board metadata");
        Ok(r.body)
    };
    // These are independent web queries: one refused read must not discard the other.
    Ok((
        user,
        teams.ok().and_then(|r| read(r).ok()).unwrap_or(Value::Null),
        favorites
            .ok()
            .and_then(|r| read(r).ok())
            .unwrap_or(Value::Null),
    ))
}
impl Db {
    pub fn cache_sidebar(&mut self, user: i64, teams: &Value, favorites: &Value) -> Result<()> {
        ensure!(
            self.current_user_id() == Some(user),
            "Board metadata belongs to another account"
        );
        ensure!(
            (teams.is_array() || teams.is_null()) && (favorites.is_array() || favorites.is_null()),
            "Invalid board metadata"
        );
        self.transaction(|db| {
            for team in teams.as_array().into_iter().flatten() {
                let id = team["id"]
                    .as_str()
                    .filter(|id| !id.is_empty())
                    .context("Invalid team")?;
                for project in team["projects"].as_array().context("Missing team boards")? {
                    let mut project = project.clone();
                    project["teamId"] = json!(id);
                    db.upsert("Project", &project)?;
                }
            }
            if teams.is_array() {
                db.kv_set(&cache_key(user, "teams"), &teams.to_string())?;
            }
            if favorites.is_array() {
                db.kv_set(&cache_key(user, "favorites"), &favorites.to_string())?;
            }
            db.reapply_pending()?;
            Ok(())
        })
    }
    pub fn sidebar_data(&self, kind: &str) -> Value {
        self.current_user_id()
            .and_then(|user| self.kv_get(&cache_key(user, kind)).ok().flatten())
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or(json!([]))
    }
    pub(crate) fn prepare_favorites(&self, ids: &[i64]) -> Result<Value> {
        let user = self.current_user_id().context("Unauthorized")?;
        ensure!(
            ids.len() <= 10 && ids.iter().all(|id| *id > 0),
            "Too many favorites (max 10)"
        );
        let mut seen = std::collections::BTreeSet::new();
        let favorites=ids.iter().enumerate().map(|(position,id)|{
            ensure!(seen.insert(*id),"Duplicate favorite");
            let project=self.raw("Project",&json!(id))?.context("Board is not cached")?;
            Ok(json!({"projectId":id,"project":project,"index":if position==9 {0}else{position+1}}))
        }).collect::<Result<Vec<_>>>()?;
        Ok(
            json!({"user_id":user,"before":self.sidebar_data("favorites"),"favorites":favorites,"body":{"favorites":ids.iter().map(|id|json!({"projectId":id})).collect::<Vec<_>>()}}),
        )
    }
}

impl Db {
    pub fn cache_sidebar_profile(&self, user: i64, profile: &Value) -> Result<()> {
        ensure!(self.current_user_id() == Some(user) && profile["id"] == user, "Wrong account profile");
        let setting = profile["UserSettingId"].as_i64().filter(|id| *id > 0).context("Missing user settings")?;
        self.kv_set(&cache_key(user, "setting"), &setting.to_string())
    }
    pub(crate) fn prepare_favorite_slot(&self, index: u8, project: Option<i64>) -> Result<Value> {
        ensure!(index <= 9, "Invalid favorite slot");
        let user = self.current_user_id().context("Unauthorized")?;
        let setting: i64 = self.kv_get(&cache_key(user, "setting"))?.context("User settings are unavailable")?.parse()?;
        let before = self.sidebar_data("favorites");
        let exists = before.as_array().into_iter().flatten().any(|row| row["index"] == index);
        let mut rows = before.as_array().cloned().unwrap_or_default();
        rows.retain(|row| row["index"] != index && project.is_none_or(|id| row["projectId"] != id));
        if let Some(id) = project {
            ensure!(id > 0, "Invalid favorite board");
            let board = self.raw("Project", &json!(id))?.context("Board is unavailable")?;
            rows.push(json!({"projectId":id,"project":board,"index":index,"userSettingId":setting}));
        }
        Ok(json!({"user_id":user,"before":before,"favorites":rows,"method":if exists || project.is_none(){"PUT"}else{"POST"},"body":{"userSettingId":setting,"index":index,"projectId":project.unwrap_or(-1)}}))
    }
}
