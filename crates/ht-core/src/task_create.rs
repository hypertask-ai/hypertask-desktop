use crate::{Db, Result};
use anyhow::{Context, ensure};
use serde_json::{Value, json};

pub fn priority(index: u8) -> Option<Value> {
    let (name, color) = *[
        ("No Priority", "transparent"),
        ("Urgent", "#F88F9C"),
        ("High", "#DE9D6E"),
        ("Medium", "#C2CFA5"),
        ("Low", "#95999E"),
    ]
    .get(index as usize)?;
    Some(json!({"priority_index":index,"Priority_Value":name,"colorCode":color}))
}
pub fn estimate(index: u8) -> Option<Value> {
    let (name, full, color) = match index {
        0 => ("No size", "-", "transparent"),
        2 => ("XS", "Extra small", "#DE9D6E"),
        3 => ("S", "Small", "#C2CFA5"),
        4 => ("M", "Medium", "#95999E"),
        5 => ("L", "Large", "#95999E"),
        6 => ("XL", "Extra large", "#95999E"),
        _ => return None,
    };
    Some(
        json!({"estimate_index":index,"estimate_value":name,"estimate_full_value":full,"colorCode":color}),
    )
}
pub fn view_defaults(project: &Value) -> Value {
    let view = &project["project_view"]["user_project_views"][0];
    let filters = [
        &view["unsavedView"]["board_filters"],
        &view["appliedView"]["board_filters"],
        &project["project_view"]["default_view"]["board_filters"],
    ]
    .into_iter()
    .find(|v| !v.is_null())
    .unwrap_or(&Value::Null);
    let values = |kind| {
        filters["addedFilters"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|f| {
                f["type"] == kind && f["match"].as_str().is_none_or(|m| m == "ANY" || m == "ALL")
            })
            .and_then(|f| f["searchPayload"].as_array())
            .cloned()
            .unwrap_or_default()
    };
    let uuid = |v: &Value| {
        v.as_str()
            .is_some_and(|id| uuid::Uuid::parse_str(id).is_ok())
    };
    let mut defaults = json!({"assignees":values("Assignees").into_iter().filter(|p| if p.get("uid").is_some() {p["id"].as_i64().is_some_and(|id|id>0)}else{uuid(&p["id"]) && p["revokedAt"].is_null()}).collect::<Vec<_>>()});
    let tags: Vec<_> = values("Labels")
        .into_iter()
        .filter(|l| uuid(&l["id"]))
        .collect();
    if !tags.is_empty() {
        defaults["tags"] = json!(tags);
    }
    if let Some(value) = values("Priority")
        .iter()
        .filter_map(|v| v["priority_index"].as_u64())
        .filter(|i| *i > 0 && *i <= 4)
        .find_map(|i| priority(i as u8))
    {
        defaults["priority"] = value;
    }
    if let Some(value) = values("Size")
        .iter()
        .filter_map(|v| v["estimate_index"].as_u64())
        .filter(|i| *i > 0 && *i <= 6)
        .find_map(|i| estimate(i as u8))
    {
        defaults["estimate"] = value;
    }
    defaults
}
impl Db {
    pub(crate) fn prepare_create(
        &self,
        project: i64,
        section: i64,
        title: &str,
        top: bool,
    ) -> Result<Value> {
        ensure!(!title.trim().is_empty(), "title must not be empty");
        let user = self
            .current_user_id()
            .context("sync your account before creating a task")?;
        let project_row = self
            .raw("Project", &json!(project))?
            .context("board is not cached")?;
        let columns = self.board(project)?;
        let column = columns
            .iter()
            .find(|c| c.id == section)
            .context("column is not visible in this board")?;
        let edge = if top {
            column.cards.first()
        } else {
            column.cards.last()
        }
        .and_then(|t| t.extra.get("ranking"))
        .and_then(Value::as_str);
        let ranking =
            crate::task_rank(if top { None } else { edge }, if top { edge } else { None })?;
        let defaults = view_defaults(&project_row);
        let global = defaults["tags"].as_array().is_some_and(|a| !a.is_empty())
            || defaults["assignees"]
                .as_array()
                .is_some_and(|a| !a.is_empty())
            || !defaults["priority"].is_null()
            || !defaults["estimate"].is_null();
        let mut body = json!({"title":title,"sectionId":section,"userId":user,"projectId":project,"ranking":ranking,"assignees":[]});
        if global {
            body.as_object_mut()
                .unwrap()
                .extend(defaults.as_object().unwrap().clone());
            body["fullScreenTask"] = json!(true);
            body["section_title"] = json!(column.title);
            body["projectIdentifier"] = project_row
                .get("uniqueIdentifier")
                .filter(|v| !v.is_null())
                .cloned()
                .unwrap_or(json!("TASK"));
            if body["priority"].is_null()
                && project_row["sorting_mode"] == "Priority"
                && top
                && !column.cards.is_empty()
            {
                body["priority"] = priority(1).unwrap();
            }
        } else {
            body["section"] = json!(column.title);
            body["index"] = json!(if project_row["sorting_mode"] == "Priority" && top {
                0
            } else {
                column.cards.len()
            });
        }
        Ok(json!({"user_id":user,"body":body,"global":global}))
    }
}
