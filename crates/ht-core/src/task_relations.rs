use crate::{Result, Transport};
use anyhow::{Context, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationKind {
    RelatedTo,
    BlockedBy,
    BlockedTo,
    Duplicate,
}
impl RelationKind {
    pub fn title(self) -> &'static str {
        match self {
            Self::RelatedTo => "Add related task",
            Self::BlockedBy => "Mark blocked by",
            Self::BlockedTo => "Mark as blocking",
            Self::Duplicate => "Mark duplicate of",
        }
    }
    pub fn section(self, from: bool) -> &'static str {
        match (self, from) {
            (Self::BlockedBy, true) | (Self::BlockedTo, false) => "Blocked by",
            (Self::BlockedTo, true) | (Self::BlockedBy, false) => "Blocks",
            (Self::Duplicate, true) => "Duplicate of",
            (Self::Duplicate, false) => "Duplicate",
            _ => "Related",
        }
    }
}
pub fn search(transport: &mut impl Transport, task: i64, query: &str) -> Result<Vec<Value>> {
    let projects = transport.get("/api/projects/getAllMinimal?mode=ExtraMinimal")?;
    ensure!(projects.status == 200, "Unable to load tasks");
    let projects = projects.body.as_array().context("Unable to load tasks")?;
    let ids: Vec<_> = projects
        .iter()
        .filter_map(|project| project["id"].as_i64().filter(|id| *id > 0))
        .collect();
    if ids.is_empty() {
        return Ok(vec![]);
    }
    let mut body = json!({"projectIds":ids});
    if query.trim().is_empty() {
        body["mode"] = json!("recent");
        body["currentTaskId"] = json!(task);
    } else {
        body["searchQuery"] = json!(query.trim());
    }
    let response = transport.post("/api/tasks/searchAll", &body, "task-relation-search")?;
    ensure!(response.status == 200, "Unable to load tasks");
    let rows = response.body.as_array().context("Unable to load tasks")?;
    ensure!(
        rows.iter()
            .all(|row| row["id"].as_i64().is_some_and(|id| id > 0)
                && row["projectId"]
                    .as_i64()
                    .is_some_and(|id| ids.contains(&id))
                && row["uniqueIndex"].as_i64().is_some_and(|id| id > 0)
                && row["title"].is_string()),
        "Unable to load tasks"
    );
    Ok(rows
        .iter()
        .filter(|row| row["id"] != task)
        .cloned()
        .collect())
}

pub(crate) fn put(task: &mut Value, relation: Value) {
    let mut rows = task["relatedFromTasks"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    if let Some(row) = rows
        .iter_mut()
        .find(|row| row["targetTaskId"] == relation["targetTaskId"])
    {
        *row = relation;
    } else {
        rows.push(relation);
    }
    task["relatedFromTasks"] = json!(rows);
}

pub fn details(transport: &mut impl Transport, task: i64) -> Result<Value> {
    let response = transport.get(&format!("/api/tasks/single?id={task}"))?;
    ensure!(
        response.status == 200 && response.body["id"] == task,
        "Couldn't load task relations"
    );
    for (field, side) in [
        ("relatedFromTasks", "sourceTaskId"),
        ("relatedToTasks", "targetTaskId"),
    ] {
        ensure!(
            response.body[field].as_array().is_some_and(|rows| rows
                .iter()
                .all(|row| row["id"].as_i64().is_some_and(|id| id > 0) && row[side] == task)),
            "Couldn't load task relations"
        );
    }
    Ok(response.body)
}
impl crate::Db {
    pub fn cache_task_relations(&mut self, user: i64, task: i64, source: &Value) -> Result<()> {
        ensure!(
            self.current_user_id() == Some(user) && source["id"] == task,
            "Unauthorized"
        );
        let mut current = serde_json::to_value(self.task(task)?.context("Task is unavailable")?)?;
        for field in ["relatedFromTasks", "relatedToTasks"] {
            ensure!(source[field].is_array(), "Couldn't load task relations");
            current[field] = source[field].clone();
        }
        self.apply_task(&current)
    }
}
