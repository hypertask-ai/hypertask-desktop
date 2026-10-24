use crate::{Db, Result, ViewWrite, board_views};
use anyhow::{Context, ensure};
use serde_json::{Value, json};

impl ViewWrite {
    pub(crate) fn column(&self) -> bool { self.route.starts_with("column-") }
    pub(crate) fn column_request(&self) -> Result<(&'static str, String, Value)> {
        let mut body = self.body.clone();
        let path = match self.route.as_str() {
            "column-create" => { ensure!(body["title"].as_str().is_some_and(|title| !title.trim().is_empty()), "Column name is required"); "/api/section/create" }
            "column-update" => {
                let fields = body["newSection"].as_object().context("Invalid column settings")?;
                ensure!(!fields.is_empty() && fields.keys().all(|key| matches!(key.as_str(), "section_title" | "ranking" | "isDone" | "deleted")), "Invalid column settings");
                for (key,value) in fields {
                    ensure!(match key.as_str() { "section_title" | "ranking" => value.as_str().is_some_and(|value| !value.trim().is_empty()), "isDone" => value.is_boolean() || value.is_null(), "deleted" => value.is_boolean(), _ => false }, "Invalid column setting");
                }
                ensure!(body["userId"].as_i64().is_some_and(|id| id > 0), "Unauthorized");
                "/api/section/update"
            }
            "column-visibility" => { ensure!(body["visible"].is_boolean(), "Invalid column visibility"); "/api/sections/view-visibility" }
            "column-autoassign" => {
                ensure!(body.get("autoAssignUserId").is_some() && body.get("autoAssignAgentId").is_some(), "Choose a member, agent or Nobody");
                ensure!(body["autoAssignUserId"].is_null() || body["autoAssignUserId"].as_i64().is_some_and(|id| id > 0), "Invalid auto-assign member");
                ensure!(body["autoAssignAgentId"].is_null() || body["autoAssignAgentId"].as_str().is_some_and(|id| !id.trim().is_empty()), "Invalid auto-assign agent");
                ensure!(body["autoAssignUserId"].is_null() || body["autoAssignAgentId"].is_null(), "Choose one auto-assign member or agent");
                "/api/sections/auto-assign"
            }
            _ => anyhow::bail!("Unsupported column mutation"),
        };
        if self.route != "column-create" {
            ensure!(body["sectionId"].as_i64().is_some_and(|id| id > 0), "Select a board column");
            body.as_object_mut().unwrap().remove("projectId");
        }
        Ok(("POST", path.into(), body))
    }
}

fn patch_views(project: &mut Value, section: &Value, fields: &Value, remove: bool, all_visibility: Option<bool>) {
    let id = &section["id"];
    let patch = |view: &mut Value| {
        if !view.is_object() { return; }
        if !view["board_columns_view"].is_array() {
            if all_visibility != Some(true) { return; }
            view["board_columns_view"] = json!([]);
        }
        let columns = view["board_columns_view"].as_array_mut().unwrap();
        if remove { columns.retain(|column| column.get("id").or_else(|| column.get("sectionId")) != Some(id)); return; }
        let found = columns.iter_mut().find(|column| column.get("id").or_else(|| column.get("sectionId")) == Some(id));
        if let Some(column) = found {
            for (key,value) in fields.as_object().into_iter().flatten() { column[key] = value.clone(); }
        } else if all_visibility == Some(true) { let mut column=section.clone(); column["visibility"]=json!(true); columns.push(column); }
    };
    let pv = &mut project["project_view"];
    if !pv.is_object() { return; }
    patch(&mut pv["default_view"]);
    for view in pv["allViews"].as_array_mut().into_iter().flatten() { patch(view); }
    for user in pv["user_project_views"].as_array_mut().into_iter().flatten() { patch(&mut user["appliedView"]); patch(&mut user["unsavedView"]); }
}

impl Db {
    pub(crate) fn column_snapshot(&self, change: &ViewWrite) -> Result<Value> {
        let project=self.board_project(change.project)?;
        if change.route == "column-create" { return Ok(json!({"section":null,"tasks":[]})); }
        let section=self.raw("Section",&change.body["sectionId"])?.context("Column is unavailable")?;
        ensure!(section["projectId"]==change.project && section["deleted"]!=true, "Column is unavailable");
        let tasks: Vec<_> = if change.body["newSection"]["deleted"] == true {
            self.json_rows("SELECT raw_json FROM Task WHERE projectId=? AND status='Normal'",[change.project])?.into_iter().filter(|task| task["sectionId"]==section["id"] || task["section"]==section["section_title"]).collect()
        } else if change.body["newSection"]["section_title"].is_string() {
            self.json_rows("SELECT raw_json FROM Task WHERE sectionId=?",[section["id"].as_i64().unwrap()])?
        } else { vec![] };
        if change.body["newSection"]["deleted"] == true {
            ensure!(tasks.is_empty() || project["section"].as_array().is_some_and(|sections| sections.iter().any(|other| other["id"]!=section["id"])), "This is the board's last column. Move or delete its cards first.");
        }
        let destination=project["section"].as_array().into_iter().flatten().find(|other|other["id"]!=section["id"]);
        Ok(json!({"section":section,"tasks":tasks,"destination":destination}))
    }
    pub(crate) fn optimistic_column(&self, change:&ViewWrite, mutation:i64, now:i64) -> Result<()> {
        if change.route == "column-create" {
            let project=self.board_project(change.project)?;
            let last=project["section"].as_array().into_iter().flatten().filter(|section|section["id"] != -mutation).last().and_then(|section|section["ranking"].as_str());
            // Temporary IDs are local-only; creation never guesses the server ID.
            let rank=change.body["ranking"].as_str().map(str::to_owned).unwrap_or_else(|| crate::task_rank(last,None).unwrap_or_else(|_| format!("{}z",last.unwrap_or(""))));
            self.upsert("Section",&json!({"id":-mutation,"projectId":change.project,"section_title":change.body["title"],"visibility":true,"deleted":false,"ranking":rank,"pending":true}))?;
            let section=self.raw("Section",&json!(-mutation))?.context("Missing pending column")?;
            let mut project=self.board_project(change.project)?;
            patch_views(&mut project,&section,&json!({"visibility":true}),false,Some(true)); self.upsert("Project",&project)?;
            return Ok(());
        }
        let mut section=self.raw("Section",&change.body["sectionId"])?.context("Column is unavailable")?;
        let fields=match change.route.as_str() {
            "column-update"=>change.body["newSection"].clone(),
            "column-visibility"=>json!({"visibility":change.body["visible"]}),
            _=>json!({"autoAssignUserId":change.body["autoAssignUserId"],"autoAssignAgentId":change.body["autoAssignAgentId"]}),
        };
        let deleting=fields["deleted"]==true;
        let mut project=self.board_project(change.project)?;
        if deleting {
            if let Some(first)=project["section"].as_array().into_iter().flatten().find(|other|other["id"]!=section["id"]) {
                for mut task in self.json_rows("SELECT raw_json FROM Task WHERE projectId=? AND status='Normal'",[change.project])?.into_iter().filter(|task|task["sectionId"]==section["id"] || task["section"]==section["section_title"]) {
                    task["sectionId"]=first["id"].clone(); task["section"]=first["section_title"].clone(); task["sectionChangedAt"]=json!(chrono::DateTime::from_timestamp(now,0).map(|date|date.to_rfc3339())); self.upsert_task(&task)?;
                }
            }
        }
        if let Some(title)=fields["section_title"].as_str() {
            for mut task in self.json_rows("SELECT raw_json FROM Task WHERE sectionId=?",[section["id"].as_i64().unwrap()])? { task["section"]=json!(title); self.upsert_task(&task)?; }
        }
        for (key,value) in fields.as_object().into_iter().flatten() {section[key]=value.clone();}
        if change.route != "column-visibility" { self.upsert("Section",&section)?; }
        patch_views(&mut project,&section,&fields,deleting,if change.route=="column-visibility" {fields["visibility"].as_bool()}else{None});
        self.upsert("Project",&project)?;
        Ok(())
    }
    pub(crate) fn resolve_column(&self, change:&ViewWrite, mutation:i64, response:&Value) -> Result<()> {
        if change.route == "column-create" {
            let section=&response["section"];
            ensure!(section["id"].as_i64().is_some_and(|id|id>0) && section["projectId"]==change.project && section["section_title"].is_string(), "Wrong column acknowledgement");
            let view=&response["project_view"];
            ensure!(view.is_null() || (view["projectId"]==change.project && view["id"].is_string()), "Wrong created-column view acknowledgement");
            self.conn.execute("DELETE FROM Section WHERE id=?",[-mutation])?;
            self.upsert("Section",section)?;
            let mut project=self.board_project(change.project)?; project["project_view"]=view.clone(); self.upsert("Project",&project)?;
        } else if change.route == "column-update" {
            if change.body["newSection"]["deleted"] != true || !response.is_null() {
                ensure!(response["id"]==change.body["sectionId"] && response["projectId"]==change.project, "Wrong column acknowledgement");
                for (key,value) in change.body["newSection"].as_object().into_iter().flatten() { ensure!(response[key]==*value,"Column setting was not acknowledged"); }
                self.upsert("Section",response)?;
            }
        } else {
            ensure!(response["success"]==true && response["sectionId"]==change.body["sectionId"], "Wrong column acknowledgement");
            let keys:&[&str]=if change.route=="column-visibility" {&["visible"]} else {&["autoAssignUserId","autoAssignAgentId"]};
            for key in keys { ensure!(response[*key]==change.body[*key],"Column setting was not acknowledged"); }
        }
        self.request_refetch("board",change.project)?;
        Ok(())
    }
    pub(crate) fn rollback_column(&self, change:&ViewWrite, mutation:i64, snapshot:&Value) -> Result<()> {
        if change.route == "column-create" { self.conn.execute("DELETE FROM Section WHERE id=?",[-mutation])?; return Ok(()); }
        let mut current=self.raw("Section",&change.body["sectionId"])?.context("Column is unavailable")?;
        let fields: Vec<&str> = match change.route.as_str() { "column-update"=>change.body["newSection"].as_object().into_iter().flatten().map(|(key,_)|key.as_str()).collect(), "column-visibility"=>vec!["visibility"], _=>vec!["autoAssignUserId","autoAssignAgentId"] };
        for field in fields {current[field]=snapshot["section"][field].clone();}
        if change.route != "column-visibility" { self.upsert("Section",&current)?; }
        for task in snapshot["tasks"].as_array().into_iter().flatten() {
            if let Some(mut current)=self.raw("Task",&task["id"])? {
                let deleting=change.body["newSection"]["deleted"]==true;
                // A later acknowledged move/rename owns its fields even if this older write fails.
                let still_optimistic=if deleting {current["sectionId"]==snapshot["destination"]["id"] && current["section"]==snapshot["destination"]["section_title"]}
                    else {current["sectionId"]==snapshot["section"]["id"] && current["section"]==change.body["newSection"]["section_title"]};
                if still_optimistic {
                    let fields:&[&str]=if deleting {&["sectionId","section","sectionChangedAt"]}else{&["section"]};
                    for field in fields {current[*field]=task[*field].clone();}self.upsert_task(&current)?;
                }
            }
        }
        Ok(())
    }
}

pub fn visible_counts(project:&Value, section:i64) -> (usize,usize) {
    let unsaved=&project["project_view"]["user_project_views"][0]["unsavedView"]["id"];
    let views: Vec<_>=project["project_view"]["allViews"].as_array().into_iter().flatten().filter(|view|view["id"]!=*unsaved).collect();
    (views.iter().filter(|view|view["board_columns_view"].as_array().into_iter().flatten().any(|column|column.get("id").or_else(||column.get("sectionId"))==Some(&json!(section)) && column["visibility"]==true)).count(),views.len())
}

pub fn reordered_settings(project:&Value) -> Value {
    let mut settings=board_views::settings(project);
    let rank=|column:&Value|project["section"].as_array().into_iter().flatten().find(|section|Some(&section["id"])==column.get("id").or_else(||column.get("sectionId"))).and_then(|section|section["ranking"].as_str()).unwrap_or("").to_owned();
    if let Some(columns)=settings["board_columns_view"].as_array_mut() { columns.sort_by_key(rank); }
    settings
}

impl Db {
    pub fn column_task_count(&self, project:i64, section:i64) -> Result<usize> {
        Ok(self.conn.query_row("SELECT COUNT(*) FROM Task WHERE projectId=? AND sectionId=? AND status='Normal'",rusqlite::params![project,section],|row|row.get(0))?)
    }
}
