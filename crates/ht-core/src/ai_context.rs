use crate::{Result,Transport};
use anyhow::{Context,ensure};
use serde_json::{Value,json};

pub fn mention_path(query:&str,project:Option<i64>)->String {
    let mut params=url::form_urlencoded::Serializer::new(String::new());
    params.append_pair("param",if query.is_empty(){"all"}else{query});
    if let Some(project)=project.filter(|id|*id>0){params.append_pair("projectId",&project.to_string());}
    format!("/api/ai/chat/mention?{}",params.finish())
}
pub fn mentions(transport:&mut impl Transport,query:&str,project:Option<i64>)->Result<Vec<Value>> {
    let response=transport.get(&mention_path(query,project))?;
    ensure!(response.status==200,"Error loading results");
    let rows=response.body.as_array().context("Error loading results")?;
    Ok(if rows.is_empty(){vec![json!({"type":"no-results","name":"No results found"})]}else{rows.clone()})
}
pub fn scopes(transport:&mut impl Transport)->Result<(i64,Vec<Value>,bool)> {
    let identity=transport.get("/user/context")?;
    ensure!(identity.status==200 && identity.body["success"]==true,"Unauthorized");
    let user=identity.body["user"]["id"].as_i64().filter(|id|*id>0).context("Unauthorized")?;
    let response=transport.post("/api/teams/getAllSidebar",&json!({"userId":user}),"ai-scope-read")?;
    ensure!(response.status==200,"Could not load boards");
    let teams=response.body.as_array().context("Could not load boards")?.clone();
    let flags=transport.get("/api/flags")?;
    // Unknown flags never enable the optional ticket scope.
    let ticket=flags.status==200 && flags.body["flags"]["htpr-7316-chat-scope-ticket"]==true;
    Ok((user,teams,ticket))
}
pub fn sort_scopes(teams:&[Value],recent:&[i64])->Vec<Value> {
    let rank=|project:&Value|recent.iter().position(|id|Some(*id)==project["id"].as_i64()).unwrap_or(usize::MAX);
    let mut teams:Vec<_>=teams.iter().filter(|team|team["projects"].as_array().is_some_and(|p|!p.is_empty())).cloned().collect();
    for team in &mut teams {if let Some(projects)=team["projects"].as_array_mut(){projects.sort_by_key(rank);}}
    teams.sort_by_key(|team|team["projects"].as_array().into_iter().flatten().map(rank).min().unwrap_or(usize::MAX));
    teams
}
pub fn selectable(row:&Value)->bool {matches!(row["type"].as_str(),Some("task"|"project"|"agent"|"name"))}
