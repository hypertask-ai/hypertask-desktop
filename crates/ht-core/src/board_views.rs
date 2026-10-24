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
        if self.route=="update-view" && self.body["updateMode"]=="personal-empty-sections"{
            let patch=|row:&mut Value|{if row["id"]==self.body["viewId"]{if row["ViewLastUsed"].as_array().is_none_or(|rows|rows.is_empty()){row["ViewLastUsed"]=json!([{}]);}row["ViewLastUsed"][0]["board_empty_sections"]=self.body["view_settings"]["board_empty_sections"].clone();}};
            for row in view["allViews"].as_array_mut().into_iter().flatten(){patch(row);}
            patch(&mut view["default_view"]);for row in view["user_project_views"].as_array_mut().into_iter().flatten(){patch(&mut row["appliedView"]);}return;
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
    let base=row.get("appliedView").filter(|view|view.is_object()).unwrap_or(&project["project_view"]["default_view"]);
    if let Some(personal)=std::iter::once(base).chain(project["project_view"]["allViews"].as_array().into_iter().flatten()).find(|view|view["id"]==base["id"] && matches!(view["ViewLastUsed"][0]["board_empty_sections"].as_str(),Some("Show"|"Hidden"))){settings["board_empty_sections"]=personal["ViewLastUsed"][0]["board_empty_sections"].clone();}
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
        let mut value=self.raw("Project", &json!(project))?.context("Board is unavailable")?;
        value["section"]=json!(self.json_rows("SELECT raw_json FROM Section WHERE projectId=? AND deleted=0 ORDER BY ranking,id",[project])?);
        Ok(value)
    }
}

pub(crate) fn filter_sort(tasks: &mut Vec<Value>, settings: &Value, project:&Value, running:&[Value]) {
    let now=chrono::Local::now();
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
                "CreatedBy" => values.is_empty() || values.iter().any(|v|v["id"].as_i64().is_some_and(|id|Some(id)==task["userId"].as_i64().or_else(||task["userId"].as_str().and_then(|value|value.parse().ok())))),
                "UpdatedBy" => values.is_empty() || values.iter().any(|v|if v.get("uid").is_some(){task["updatedByUserIds"].as_array().into_iter().flatten().any(|id|*id==v["id"])}else{task["agentId"]==v["id"]}),
                "BlockedByPerson" => !task["waitingOnUserId"].is_null() && values.iter().any(|v|v["id"]==0 || v["id"]==task["waitingOnUserId"]),
                "UpdatedRange"|"CreatedAt"|"DueDate" => {
                    let field=match filter["type"].as_str().unwrap(){"UpdatedRange"=>"updatedAt","CreatedAt"=>"createdAt",_=>"dueDate"};
                    let date=task.get(field).filter(|v|!v.is_null()).unwrap_or(if field=="updatedAt"{&task["createdAt"]}else{&Value::Null});
                    date_filter(date,&values.first().cloned().unwrap_or(Value::Null),field=="dueDate",now)
                }
                "NoRecentComment" => stale(task.get("lastCommentAt").filter(|v|!v.is_null()).unwrap_or(&task["createdAt"]),project,now.timestamp()),
                "StuckInColumn" => stale(task.get("sectionChangedAt").filter(|v|!v.is_null()).unwrap_or(&task["createdAt"]),project,now.timestamp()),
                "StaleOnBoard" => stale(&task["createdAt"],project,now.timestamp()),
                "NotStale" => !stale(task.get("lastCommentAt").filter(|v|!v.is_null()).unwrap_or(&task["createdAt"]),project,now.timestamp()) && !stale(task.get("sectionChangedAt").filter(|v|!v.is_null()).unwrap_or(&task["createdAt"]),project,now.timestamp()),
                "RunningTimer" => project["timeTrackingEnabled"]==true && running.iter().any(|entry|entry["taskId"]==task["id"]),
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
                _ => false,
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
    if levels[0].0!="Manual" {if let Some(stack)=settings["board_sorting_stack"].as_array(){for level in stack{
        if levels.len()==3{break;}
        if let(Some(mode),Some(order))=(level["mode"].as_str(),level["order"].as_str()){
            if SORT_MODES.iter().any(|(_,value)|*value==mode) && mode!="Manual" && matches!(order,"Ascending"|"Descending") && !levels.iter().any(|(current,_)|*current==mode){levels.push((mode,order));}
        }
    }}}
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
                "Assignee" => {
                    let name=|task:&Value|task["assignees"][0]["agent"]["displayName"].as_str().filter(|name|!name.is_empty()).or_else(||task["assignees"][0]["user"]["displayName"].as_str()).map(|name|json!(name)).unwrap_or(Value::Null);
                    compare(&name(a),&name(b))
                }
                "TimeOnBoard" => compare(&a["createdAt"],&b["createdAt"]).reverse(),
                "CreatedAt" => compare(&a["createdAt"], &b["createdAt"]),
                "UpdatedAt" => compare(&a["updatedAt"], &b["updatedAt"]),
                "SectionChangedAt" | "LastCommentAt" | "TimeInColumn" | "TimeWithoutComment" => {
                    let field = if matches!(*mode,"SectionChangedAt"|"TimeInColumn") {
                        "sectionChangedAt"
                    } else {
                        "lastCommentAt"
                    };
                    let result=compare(
                        a.get(field)
                            .filter(|v| !v.is_null())
                            .unwrap_or(&a["createdAt"]),
                        b.get(field)
                            .filter(|v| !v.is_null())
                            .unwrap_or(&b["createdAt"]),
                    );
                    if matches!(*mode,"TimeInColumn"|"TimeWithoutComment"){result.reverse()}else{result}
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

pub fn filter_people(transport:&mut impl Transport,project:i64)->Result<Value>{
    let response=transport.get(&format!("/api/members/getAllForAssignees?projectId={project}"))?;
    ensure!(response.status==200,"{}",response.body["message"].as_str().unwrap_or("Error getting Assignees and Members"));
    ensure!(response.body["id"]==project,"Wrong board members");
    Ok(response.body)
}
pub fn running_timers(transport:&mut impl Transport)->Result<Value>{
    let response=transport.get("/api/time/running")?;
    ensure!(response.status==200 && response.body["success"]==true,"{}",response.body["error"].as_str().unwrap_or("Could not load running timers"));
    ensure!(response.body["entries"].is_array(),"Could not load running timers");Ok(response.body["entries"].clone())
}

pub fn ordered_views(project:&Value)->Vec<Value>{
    let pv=&project["project_view"];let unsaved=&pv["user_project_views"][0]["unsavedView"]["id"];
    let mut rows:Vec<Value>=pv["allViews"].as_array().into_iter().flatten().filter(|view|view["id"]!=*unsaved).cloned().collect();
    let order=pv["user_project_views"][0]["view_order"].as_array().or_else(||pv["default_view_order"].as_array());
    rows.sort_by_key(|view|if view["id"]==pv["default_view_id"]{0}else{1+order.and_then(|order|order.iter().position(|id|*id==view["id"])).unwrap_or(usize::MAX-1)});
    rows
}


pub const SORT_MODES:&[(&str,&str)]=&[("Last Updated","UpdatedAt"),("Priority","Priority"),("Due Date","DueDate"),("Size","Size"),("Assignee","Assignee"),("Title","Title"),("Ticket number","TicketNumber"),("Time in column","TimeInColumn"),("Time on board","TimeOnBoard"),("Time without comment","TimeWithoutComment"),("Creation Date","CreatedAt"),("Column changed","SectionChangedAt"),("Last comment","LastCommentAt"),("Manual","Manual")];
fn timestamp(value:&Value)->Option<i64>{
    let value=value.as_str()?;
    chrono::DateTime::parse_from_rfc3339(value).ok().map(|date|date.timestamp()).or_else(||chrono::NaiveDate::parse_from_str(value,"%Y-%m-%d").ok().map(|date|date.and_hms_opt(0,0,0).unwrap().and_utc().timestamp()))
}
fn stale(value:&Value,project:&Value,now:i64)->bool{
    timestamp(value).is_some_and(|date|(now-date).max(0)/86400>=project["staleWarnDays"].as_i64().unwrap_or(7))
}
fn date_filter(value:&Value,payload:&Value,due:bool,now:chrono::DateTime<chrono::Local>)->bool {
    use chrono::{Datelike,Duration,TimeZone};
    let range=payload["dynamicRange"].as_str();
    if due && range==Some("NO_DUE_DATE"){return value.is_null();}
    if due && payload["condition"]=="ANY"{return !value.is_null();}
    if due && value.is_null(){return false;}
    let Some(date)=timestamp(value)else{return false;};
    if let Some(range)=range{
        let today=now.date_naive();let date=chrono::Local.timestamp_opt(date,0).single().unwrap().date_naive();
        return match range{
            "TODAY"=>date==today,"YESTERDAY"=>date==today-Duration::days(1),
            "LAST_7_DAYS"=>date>=today-Duration::days(6) && date<=today,
            "LAST_30_DAYS"=>date>=today-Duration::days(29) && date<=today,
            "OVERDUE"=>date<today,"NEXT_7_DAYS"=>date>=today && date<=today+Duration::days(6),
            "THIS_WEEK"=>{let monday=today-Duration::days(today.weekday().num_days_from_monday() as i64);date>=monday && date<=monday+Duration::days(6)},
            "THIS_MONTH"=>date.year()==today.year() && date.month()==today.month(),"ANY"=>true,_=>false,
        };
    }
    if let(Some(from),Some(to))=(timestamp(&payload["fromDate"]),timestamp(&payload["toDate"])){return date>=from && date<=to;}
    if let Some(selected)=timestamp(&payload["selectedDate"]){return if payload["condition"]=="BEFORE" || (payload["condition"].is_null() && selected>now.timestamp()){date<selected}else if payload["condition"]=="AFTER" || (payload["condition"].is_null() && selected<=now.timestamp()){date>selected}else{true};}
    true
}

#[cfg(test)]
mod advanced_tests {
    use super::*;
    #[test]
    fn dynamic_dates_custom_dates_and_staleness_follow_web_boundaries(){
        let now=chrono::DateTime::parse_from_rfc3339("2026-10-21T12:00:00Z").unwrap().with_timezone(&chrono::Local);
        let today=now.to_rfc3339();
        for range in ["TODAY","LAST_7_DAYS","LAST_30_DAYS","THIS_WEEK","THIS_MONTH","NEXT_7_DAYS","ANY"]{assert!(date_filter(&json!(today),&json!({"dynamicRange":range}),true,now),"{range}");}
        assert!(!date_filter(&json!(today),&json!({"dynamicRange":"OVERDUE"}),true,now));
        assert!(date_filter(&Value::Null,&json!({"dynamicRange":"NO_DUE_DATE"}),true,now));
        assert!(!date_filter(&Value::Null,&json!({"condition":"ANY"}),true,now));
        assert!(date_filter(&json!(today),&json!({"fromDate":"2026-10-01","toDate":"2026-10-31"}),false,now));
        assert!(!date_filter(&json!(today),&json!({"selectedDate":"2026-10-01","condition":"BEFORE"}),false,now));
        assert!(date_filter(&json!(today),&json!({"selectedDate":"2026-10-01","condition":"AFTER"}),false,now));
        assert!(stale(&json!("2026-10-14T12:00:00Z"),&json!({}),now.timestamp()));assert!(!stale(&json!("2026-10-14T12:00:00Z"),&json!({"staleWarnDays":8}),now.timestamp()));
        assert!(!stale(&Value::Null,&json!({}),now.timestamp()));
    }
    #[test]
    fn advanced_sort_comparators_stack_cap_and_unknown_filters_never_silently_match(){
        let source=vec![json!({"id":1,"title":"Same","createdAt":"2000-01-01","sectionChangedAt":"2000-01-01","lastCommentAt":"2000-01-01","assignees":[{"user":{"displayName":"Zed"}}]}),json!({"id":2,"title":"Same","createdAt":"2026-01-01","sectionChangedAt":"2026-01-01","lastCommentAt":"2026-01-01","assignees":[{"agent":{"displayName":"Amy"}}]})];
        for mode in ["TimeInColumn","TimeOnBoard","TimeWithoutComment","Assignee"]{
            let mut tasks=source.clone();filter_sort(&mut tasks,&json!({"board_sorting_mode":mode,"board_sorting_order":"Descending"}),&json!({}),&[]);assert_eq!(tasks[0]["id"],1,"{mode}");
        }
        let mut tasks=source.clone();filter_sort(&mut tasks,&json!({"board_sorting_mode":"Title","board_sorting_order":"Ascending","board_sorting_stack":[{"mode":"Title","order":"Descending"},{"mode":"Assignee","order":"Ascending"},{"mode":"CreatedAt","order":"Descending"}]}),&json!({}),&[]);assert_eq!(tasks[0]["id"],2);
        filter_sort(&mut tasks,&json!({"board_filters":{"addedFilters":[{"type":"not-a-web-filter"}],"matchFilters":"ALL"}}),&json!({}),&[]);assert!(tasks.is_empty());
    }
}

pub fn builtin_views(project:&Value,decisions:bool)->Vec<Value>{
    let(current,next)=cycle_window(project,chrono::Utc::now().timestamp());
    [("my-tasks","My Tasks"),("overdue","Overdue"),("blocked","Blocked"),("agents","Agents"),("current-cycle","Current cycle"),("next-cycle","Next cycle"),("decisions","Decisions")].into_iter().filter(|(id,_)|match *id{"current-cycle"=>project["cyclesEnabled"]==true && current.is_some(),"next-cycle"=>project["cyclesEnabled"]==true && next.is_some(),"decisions"=>decisions,_=>true}).map(|(id,title)|json!({"id":format!("builtin:{id}"),"title":title,"builtin":true})).collect()
}
fn cycle_window(project:&Value,now:i64)->(Option<i64>,Option<i64>){
    let mut cycles:Vec<_>=project["cycles"].as_array().into_iter().flatten().filter_map(|cycle|Some((timestamp(&cycle["startDate"])?/86400,timestamp(&cycle["endDate"])?/86400,cycle["id"].as_i64()?))).collect();cycles.sort_by_key(|cycle|cycle.0);let today=now.div_euclid(86400);let current=cycles.iter().find(|cycle|cycle.0<=today && today<cycle.1);let next=if let Some(current)=current{cycles.iter().find(|cycle|cycle.0==current.1)}else{cycles.iter().find(|cycle|cycle.0>today)};(current.map(|cycle|cycle.2),next.map(|cycle|cycle.2))
}
pub(crate) fn builtin_matches(id:&str,task:&Value,project:&Value,user:Option<i64>,now:i64)->bool{
    let section=project["section"].as_array().into_iter().flatten().find(|section|section["id"]==task["sectionId"]);
    let title=section.and_then(|section|section["section_title"].as_str()).or(task["section"].as_str()).unwrap_or("").trim().to_lowercase();
    match id{
        "builtin:my-tasks"=>user.is_some_and(|user|task["assignees"].as_array().into_iter().flatten().any(|assignee|assignee["userId"]==user && assignee["agentId"].is_null())),
        "builtin:overdue"=>task["status"]=="Normal" && timestamp(&task["dueDate"]).is_some_and(|date|date<now) && !section.and_then(|section|section["isDone"].as_bool()).unwrap_or(["done","complete","completed","shipped","closed","finished","live","released"].contains(&title.as_str())),
        "builtin:blocked"=>!task["waitingOnUserId"].is_null() || task["blockingTasks"].as_array().is_some_and(|tasks|!tasks.is_empty()),
        "builtin:agents"=>task["assignees"].as_array().into_iter().flatten().any(|assignee|!assignee["agentId"].is_null()),
        "builtin:current-cycle"|"builtin:next-cycle"=>{let(current,next)=cycle_window(project,now);project["cyclesEnabled"]==true && (if id=="builtin:current-cycle"{current}else{next}).is_some_and(|cycle|task["cycleId"]==cycle)},
        "builtin:decisions"=>{
            if title=="ux sign-off"{return true;}
            let name=project["members"].as_array().into_iter().flatten().find(|member|member["userId"].as_i64()==user && member["agentId"].is_null()).and_then(|member|member["user"]["displayName"].as_str()).or_else(||(project["owner"]["id"].as_i64()==user).then(||project["owner"]["displayName"].as_str()).flatten()).unwrap_or("").trim().to_lowercase();
            !name.is_empty() && (title==format!("{name} review") || title==format!("{} review",name.split_whitespace().next().unwrap()))
        }
        _=>false,
    }
}

#[cfg(test)]
mod builtin_tests{
    use super::*;
    #[test]
    fn builtin_cycle_utc_windows_done_override_and_decisions_names_match_web(){
        let mut project=json!({"cyclesEnabled":true,"cycles":[{"id":1,"startDate":"2026-10-01","endDate":"2026-10-15"},{"id":2,"startDate":"2026-10-15","endDate":"2026-10-29"}],"section":[{"id":1,"section_title":"Done","isDone":false}],"owner":{"id":42,"displayName":"Owner Name"}});
        let now=timestamp(&json!("2026-10-14T23:59:59Z")).unwrap();assert_eq!(cycle_window(&project,now),(Some(1),Some(2)));assert_eq!(cycle_window(&project,now+1),(Some(2),None));
        assert!(builtin_matches("builtin:overdue",&json!({"sectionId":1,"status":"Normal","dueDate":"2026-10-01"}),&project,Some(42),now));
        assert!(builtin_matches("builtin:decisions",&json!({"section":"Owner review"}),&project,Some(42),now));assert!(!builtin_matches("builtin:decisions",&json!({"section":"Other review"}),&project,Some(42),now));
        project["cyclesEnabled"]=json!(false);assert!(!builtin_views(&project,false).iter().any(|view|view["id"]=="builtin:decisions" || view["id"]=="builtin:current-cycle"));
    }
}
