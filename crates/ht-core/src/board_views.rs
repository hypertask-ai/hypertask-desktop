use crate::{Db, Result, Transport};
use anyhow::{Context, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewWrite {
    pub project: i64,
    pub route: String,
    pub body: Value,
}
impl ViewWrite {
    pub(crate) fn validate(&self) -> Result<()> {
        ensure!(
            self.project > 0 && self.body["projectId"] == self.project,
            "Invalid board view"
        );
        ensure!(
            matches!(
                self.route.as_str(),
                "rename-view" | "delete-view" | "unsaved-view"
                    | "switch-view"
                    | "create-view"
                    | "update-view"
                    | "update-order"
                    | "set-default-order"
                    | "reset-order"
                    | "reset-to-default"
            ),
            "Unsupported view mutation"
        );
        Ok(())
    }
    pub fn request(&self)->Result<(&'static str,String,Value)> {
        self.validate()?;
        if self.route=="delete-view" {
            let id=self.body["viewId"].as_str().filter(|id|!id.is_empty()).context("Select a saved view")?;
            let query=url::form_urlencoded::Serializer::new(String::new()).append_pair("viewId",id).append_pair("projectId",&self.project.to_string()).finish();
            Ok(("DELETE",format!("/api/projects/views/delete-rename-view?{query}"),Value::Null))
        }else{Ok(("POST",format!("/api/projects/views/{}",if self.route=="rename-view"{"delete-rename-view"}else{&self.route}),self.body.clone()))}
    }
    pub(crate) fn optimistic(&self, project: &mut Value) {
        let view = &mut project["project_view"];
        if !view.is_object() {
            *view = json!({"projectId":self.project,"user_project_views":[{}]});
        }
        if view["user_project_views"]
            .as_array()
            .is_none_or(|rows| rows.is_empty())
        {
            view["user_project_views"] = json!([{}]);
        }
        match self.route.as_str() {
            "unsaved-view" => view["user_project_views"][0]["unsavedView"] = self.body.clone(),
            "rename-view"|"update-view"=>{
                let patch=if self.route=="rename-view"{json!({"title":self.body["title"]})}else{self.body["view_settings"].clone()};
                let mut update=|row:&mut Value|{if row["id"]==self.body["viewId"]{for(key,value)in patch.as_object().into_iter().flatten(){row[key]=value.clone();}}};
                for row in view["allViews"].as_array_mut().into_iter().flatten(){update(row);}
                update(&mut view["default_view"]);update(&mut view["user_project_views"][0]["appliedView"]);
                if self.route=="update-view"{view["user_project_views"][0]["unsavedView"]=Value::Null;}
            }
            "delete-view"=>{
                if let Some(rows)=view["allViews"].as_array_mut(){rows.retain(|row|row["id"]!=self.body["viewId"]);}
                if view["user_project_views"][0]["appliedView"]["id"]==self.body["viewId"]{view["user_project_views"][0]["appliedView"]=Value::Null;view["user_project_views"][0]["unsavedView"]=Value::Null;}
            }
            "switch-view" => {
                let selected = view["allViews"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .find(|v| v["id"] == self.body["newViewId"])
                    .cloned();
                if let Some(selected) = selected {
                    view["user_project_views"][0]["appliedView"] = selected;
                    view["user_project_views"][0]["unsavedView"] = Value::Null;
                }
            }
            "reset-to-default" => {
                view["user_project_views"][0]["unsavedView"] = Value::Null;
                if self.body["mode"] == "ResetToDefault" {
                    view["user_project_views"][0]["appliedView"] = Value::Null;
                }
            }
            "update-order" => {
                view["user_project_views"][0]["view_order"] = self.body["viewOrder"].clone()
            }
            "reset-order"=>view["user_project_views"][0]["view_order"]=Value::Null,
            "set-default-order"=>view["default_view_order"]=self.body["viewOrder"].clone(),
            _ => (),
        }
    }
}
pub fn settings(project: &Value) -> Value {
    let row = &project["project_view"]["user_project_views"][0];
    let sources = [
        &row["unsavedView"],
        &row["appliedView"],
        &project["project_view"]["default_view"],
    ];
    let mut settings = json!({"board_columns_view":project.get("section").cloned().unwrap_or(json!([])),"board_filters":{"addedFilters":[],"matchFilters":"ANY"},"board_sorting_mode":"Manual","board_sorting_order":"Descending","board_sorting_stack":[],"board_subtask_setting":"Flattened_Card","board_empty_sections":"Show","board_staleness":null,"board_show_archived":null,"table_sort_column":null,"table_sort_direction":null,"board_layout":null});
    for (key, value) in settings.as_object_mut().unwrap() {
        if let Some(v) = sources
            .iter()
            .filter_map(|source| source.get(key))
            .find(|v| !v.is_null())
        {
            *value = v.clone();
        }
    }
    settings
}
pub fn load(transport: &mut impl Transport, project: i64) -> Result<(i64, Value)> {
    let identity = transport.get("/user/context")?;
    ensure!(
        identity.status == 200 && identity.body["success"] == true,
        "Unauthorized"
    );
    let user = identity.body["user"]["id"]
        .as_i64()
        .context("Unauthorized")?;
    let response = transport.post(
        "/api/projects/boardTasks",
        &json!({"projectId":project,"userId":user}),
        "board-view-read",
    )?;
    ensure!(
        response.status == 200,
        "{}",
        response.body["message"]
            .as_str()
            .or(response.body["error"].as_str())
            .unwrap_or("Failed to load board")
    );
    ensure!(
        response.body["project"]["id"] == project,
        "Wrong board response"
    );
    Ok((user, response.body))
}
impl Db {
    pub fn apply_web_board(&mut self, user: i64, body: &Value) -> Result<()> {
        ensure!(
            self.current_user_id() == Some(user),
            "Board response belongs to another account"
        );
        self.transaction(|db| {
            let mut project = body["project"].clone();
            ensure!(
                project["id"].as_i64().is_some_and(|id| id > 0),
                "Invalid board"
            );
            if project["project_view"].is_object() {
                project["project_view"]["allViews"] = body["allViews"].clone();
            }
            db.upsert("Project", &project)?;
            for section in project["section"].as_array().into_iter().flatten() {
                db.upsert("Section", section)?;
            }
            for task in body["tasks"].as_array().into_iter().flatten() {
                db.upsert_task(task)?;
            }
            db.reapply_pending()?;
            Ok(())
        })
    }
    pub fn board_project(&self, project: i64) -> Result<Value> {
        self.raw("Project", &json!(project))?
            .context("Board is unavailable")
    }
}

pub(crate) fn filter_sort(tasks: &mut Vec<Value>, settings: &Value) {
    let filters = settings["board_filters"]["addedFilters"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    tasks.retain(|task| {
        let matches = |filter: &Value| {
            let values = filter["searchPayload"]
                .as_array()
                .cloned()
                .unwrap_or_default();
            match filter["type"].as_str().unwrap_or("") {
                "Priority" => values.iter().any(|v| {
                    v["priority_index"].as_i64().unwrap_or(0)
                        == task["priority"]["priority_index"].as_i64().unwrap_or(0)
                }),
                "Size" => values.iter().any(|v| {
                    v["estimate_index"].as_i64().unwrap_or(0)
                        == task["estimate"]["estimate_index"].as_i64().unwrap_or(0)
                }),
                "Labels" | "Assignees" => {
                    let has = |v: &Value| {
                        if filter["type"] == "Labels" {
                            task["taskLabels"]
                                .as_array()
                                .into_iter()
                                .flatten()
                                .any(|l| l["label"]["id"] == v["id"])
                                || task["labels"]
                                    .as_array()
                                    .into_iter()
                                    .flatten()
                                    .any(|l| l["id"] == v["id"])
                        } else {
                            task["assignees"].as_array().into_iter().flatten().any(|p| {
                                if v.get("uid").is_some() {
                                    p["agentId"].is_null()
                                        && (p["userId"] == v["id"] || p["id"] == v["id"])
                                } else {
                                    p["agentId"] == v["id"] || p["agent"]["id"] == v["id"]
                                }
                            })
                        }
                    };
                    values.is_empty()
                        || if filter["match"] == "ALL" {
                            values.iter().all(has)
                        } else {
                            values.iter().any(has)
                        }
                }
                "Status" => {
                    values.is_empty() || values.iter().any(|v| v["value"] == task["status"])
                }
                "Starred" => {
                    values.is_empty()
                        || task["savedContent"]
                            .as_array()
                            .is_some_and(|a| !a.is_empty())
                }
                "Inbox" => task["notifications"]
                    .as_array()
                    .is_some_and(|a| !a.is_empty()),
                "Unread" => task["notifications"][0]["seen"] == false,
                _ => true,
            }
        };
        filters.is_empty()
            || if settings["board_filters"]["matchFilters"] == "ALL" {
                filters.iter().all(matches)
            } else {
                filters.iter().any(matches)
            }
    });
    let mut levels = vec![(
        settings["board_sorting_mode"].as_str().unwrap_or("Manual"),
        settings["board_sorting_order"]
            .as_str()
            .unwrap_or("Descending"),
    )];
    if let Some(stack) = settings["board_sorting_stack"].as_array() {
        levels.extend(
            stack
                .iter()
                .filter_map(|s| Some((s["mode"].as_str()?, s["order"].as_str()?))),
        );
    }
    tasks.sort_by(|a, b| {
        use std::cmp::Ordering;
        for (mode, order) in &levels {
            let descending = *order == "Descending";
            let compare = |a: &Value, b: &Value| {
                if a.is_null() || b.is_null() {
                    return b.is_null().cmp(&a.is_null());
                }
                let cmp = if a.is_number() && b.is_number() {
                    a.as_f64().unwrap().total_cmp(&b.as_f64().unwrap())
                } else {
                    a.as_str()
                        .unwrap_or("")
                        .to_lowercase()
                        .cmp(&b.as_str().unwrap_or("").to_lowercase())
                };
                if descending { cmp.reverse() } else { cmp }
            };
            let cmp = match *mode {
                "Manual" => Ordering::Equal,
                "Priority" => {
                    if a["priority"].is_null() || b["priority"].is_null() {
                        a["priority"].is_null().cmp(&b["priority"].is_null())
                    } else {
                        compare(
                            &a["priority"]["priority_index"],
                            &b["priority"]["priority_index"],
                        )
                        .reverse()
                    }
                }
                "Size" => {
                    if a["estimate"].is_null() || b["estimate"].is_null() {
                        a["estimate"].is_null().cmp(&b["estimate"].is_null())
                    } else {
                        compare(
                            &a["estimate"]["estimate_index"],
                            &b["estimate"]["estimate_index"],
                        )
                    }
                }
                "Title" => compare(&a["title"], &b["title"]),
                "TicketNumber" => compare(&a["uniqueIndex"], &b["uniqueIndex"]),
                "DueDate" => {
                    if a["dueDate"].is_null() || b["dueDate"].is_null() {
                        a["dueDate"].is_null().cmp(&b["dueDate"].is_null())
                    } else {
                        compare(&a["dueDate"], &b["dueDate"])
                    }
                }
                "CreatedAt" => compare(&a["createdAt"], &b["createdAt"]),
                "UpdatedAt" => compare(&a["updatedAt"], &b["updatedAt"]),
                "SectionChangedAt" | "LastCommentAt" => {
                    let field = if *mode == "SectionChangedAt" {
                        "sectionChangedAt"
                    } else {
                        "lastCommentAt"
                    };
                    compare(
                        a.get(field)
                            .filter(|v| !v.is_null())
                            .unwrap_or(&a["createdAt"]),
                        b.get(field)
                            .filter(|v| !v.is_null())
                            .unwrap_or(&b["createdAt"]),
                    )
                }
                _ => Ordering::Equal,
            };
            if cmp != Ordering::Equal {
                return cmp;
            }
        }
        Ordering::Equal
    });
}

pub fn ordered_views(project:&Value)->Vec<Value>{
    let pv=&project["project_view"];let unsaved=&pv["user_project_views"][0]["unsavedView"]["id"];
    let mut rows:Vec<Value>=pv["allViews"].as_array().into_iter().flatten().filter(|view|view["id"]!=*unsaved).cloned().collect();
    let order=pv["user_project_views"][0]["view_order"].as_array().or_else(||pv["default_view_order"].as_array());
    rows.sort_by_key(|view|if view["id"]==pv["default_view_id"]{0}else{1+order.and_then(|order|order.iter().position(|id|*id==view["id"])).unwrap_or(usize::MAX-1)});
    rows
}
