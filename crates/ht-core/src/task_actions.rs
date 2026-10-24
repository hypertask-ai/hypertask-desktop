use crate::{
    Db, Result, Transport,
    http::{Response, TransportError},
    queue::Mutation,
};
use anyhow::{Context, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskAction {
    StartDate(Option<String>),
    WaitingOn(Option<i64>),
    AssignMe,
    Recurrence(Option<String>),
    RemoveRelation(i64),
    Relation {
        target: Value,
        relation: crate::RelationKind,
    },
}
impl TaskAction {
    pub fn request(&self, task: i64, user: i64) -> Result<(&'static str, Value)> {
        ensure!(task > 0 && user > 0, "Unauthorized");
        Ok(match self {
            Self::StartDate(date) => {
                if let Some(date) = date {
                    chrono::DateTime::parse_from_rfc3339(date)?;
                }
                (
                    "/api/tasks/setStartDate",
                    json!({"taskId":task,"startDate":date}),
                )
            }
            Self::WaitingOn(user_id) => {
                ensure!(user_id.is_none_or(|id| id > 0), "Invalid member");
                (
                    "/api/tasks/waiting-on",
                    json!({"taskId":task,"userId":user_id}),
                )
            }
            Self::RemoveRelation(id) => {
                ensure!(*id > 0, "Task relation is unavailable");
                ("/api/tasks/removeRelation", json!({"relationId":id}))
            }
            Self::Relation { target, relation } => {
                ensure!(
                    target["id"].as_i64().is_some_and(|id| id > 0 && id != task)
                        && target["projectId"].as_i64().is_some_and(|id| id > 0)
                        && target["uniqueIndex"].as_i64().is_some_and(|id| id > 0),
                    "Task is unavailable"
                );
                (
                    "/api/tasks/addRelations",
                    json!({"relations":{"currentTaskId":task,"relatedTasks":[{"projectId":target["projectId"],"uniqueIndex":target["uniqueIndex"],"relationType":relation}]}}),
                )
            }
            Self::Recurrence(rule) => {
                ensure!(
                    rule.as_deref().is_none_or(|rule| [
                        "Daily", "Weekdays", "Weekly", "Biweekly", "Monthly"
                    ]
                    .contains(&rule)),
                    "Invalid recurrence rule"
                );
                (
                    "/api/tasks/setRecurrence",
                    json!({"taskId":task,"recurrence":rule}),
                )
            }
            Self::AssignMe => (
                "/api/assignees/assign",
                json!({"taskId":task,"userId":user,"intent":"assign"}),
            ),
        })
    }
    fn fields(&self) -> &'static [&'static str] {
        match self {
            Self::StartDate(_) => &["startDate"],
            Self::Recurrence(_) => &["recurrence"],
            Self::Relation { .. } => &["relatedFromTasks"],
            Self::RemoveRelation(_) => &["relatedFromTasks", "relatedToTasks"],
            Self::WaitingOn(_) => &["waitingOnUserId", "waitingOnSetById", "waitingOnSetAt"],
            Self::AssignMe => &["assignees", "assigneeCount"],
        }
    }
    pub fn failure(&self) -> &'static str {
        match self {
            Self::StartDate(_) => "Could not update the start date. Try again.",
            Self::WaitingOn(_) => "Unable to update blocked by person",
            Self::AssignMe => "Unable to assign this task",
            Self::Recurrence(_) => "Could not update the repeat rule",
            Self::Relation { .. } => "Unable to add task relation",
            Self::RemoveRelation(_) => "Unable to remove task relation",
        }
    }
}
impl Db {
    pub(crate) fn prepare_task_action(&self, task: i64, change: &TaskAction) -> Result<Value> {
        let user = self.current_user_id().context("Unauthorized")?;
        change.request(task, user)?;
        let before = serde_json::to_value(self.task(task)?.context("Task is unavailable")?)?;
        ensure!(
            !self.mutations()?.iter().any(|row| row.status != "failed"
                && row.kind == "web_task_action"
                && row.payload_json["task_id"] == task),
            "Wait for the previous task update"
        );
        if let TaskAction::RemoveRelation(id) = change {
            ensure!(
                ["relatedFromTasks", "relatedToTasks"]
                    .iter()
                    .any(|field| before[*field]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .any(|row| row["id"] == *id)),
                "Task relation is unavailable"
            );
        }
        if let TaskAction::WaitingOn(Some(id)) = change {
            ensure!(
                self.project_members(
                    before["projectId"]
                        .as_i64()
                        .context("Board is unavailable")?
                )?
                .iter()
                .any(|member| member["id"] == *id),
                "User is not a member of this project"
            );
        }
        if matches!(change, TaskAction::AssignMe) {
            ensure!(
                self.project_members(
                    before["projectId"]
                        .as_i64()
                        .context("Board is unavailable")?
                )?
                .iter()
                .any(|member| member["id"] == user),
                "User is not a member of this project"
            );
        }
        Ok(json!({"user_id":user,"task_id":task,"change":change,"before":before,"attempted":false}))
    }
    pub(crate) fn optimistic_task_action(&self, payload: &Value, now: i64) -> Result<()> {
        if self.current_user_id() != payload["user_id"].as_i64() {
            return Ok(());
        }
        let mut task = self
            .raw("Task", &payload["task_id"])?
            .context("Task is unavailable")?;
        match serde_json::from_value::<TaskAction>(payload["change"].clone())? {
            TaskAction::RemoveRelation(id) => {
                for field in ["relatedFromTasks", "relatedToTasks"] {
                    let rows: Vec<_> = task[field]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter(|row| row["id"] != id)
                        .cloned()
                        .collect();
                    task[field] = json!(rows);
                }
            }
            TaskAction::Relation { target, relation } => {
                crate::task_relations::put(
                    &mut task,
                    json!({"id":-target["id"].as_i64().context("Task is unavailable")?,"sourceTaskId":payload["task_id"],"targetTaskId":target["id"],"relationType":relation,"targetTask":target,"pending":true}),
                );
            }
            TaskAction::StartDate(date) => task["startDate"] = json!(date),
            TaskAction::Recurrence(rule) => task["recurrence"] = json!(rule),
            TaskAction::WaitingOn(user) => {
                task["waitingOnUserId"] = json!(user);
                task["waitingOnSetById"] = if user.is_some() {
                    payload["user_id"].clone()
                } else {
                    Value::Null
                };
                task["waitingOnSetAt"] = if user.is_some() {
                    json!(
                        chrono::DateTime::from_timestamp(now, 0)
                            .context("Invalid date")?
                            .to_rfc3339()
                    )
                } else {
                    Value::Null
                };
            }
            TaskAction::AssignMe => {
                let mut rows = task["assignees"].as_array().cloned().unwrap_or_default();
                if !rows.iter().any(|row| {
                    row["agentId"].is_null()
                        && row["agent"].is_null()
                        && (row["userId"] == payload["user_id"] || row["id"] == payload["user_id"])
                }) {
                    let mut member = self
                        .project_members(
                            task["projectId"].as_i64().context("Board is unavailable")?,
                        )?
                        .into_iter()
                        .find(|member| member["id"] == payload["user_id"])
                        .context("User is not a member of this project")?;
                    member["userId"] = payload["user_id"].clone();
                    rows.push(member);
                }
                task["assigneeCount"] = json!(rows.len());
                task["assignees"] = json!(rows);
            }
        }
        self.upsert_task(&task)
    }
    pub(crate) fn send_task_action(
        &self,
        transport: &mut impl Transport,
        row: &Mutation,
    ) -> std::result::Result<Response, TransportError> {
        let payload = &row.payload_json;
        if payload["attempted"] == true {
            return Ok(Response {
                status: 409,
                body: json!({"success":false}),
            });
        }
        let identity = transport.get("/user/context")?;
        if identity.status != 200 {
            return Ok(identity);
        }
        if identity.body["success"] != true
            || identity.body["user"]["id"] != payload["user_id"]
            || self.current_user_id() != payload["user_id"].as_i64()
        {
            return Ok(Response {
                status: 403,
                body: json!({"success":false}),
            });
        }
        let change: TaskAction = serde_json::from_value(payload["change"].clone())
            .map_err(|_| TransportError::InvalidResponse)?;
        let (path, body) = change
            .request(
                payload["task_id"].as_i64().unwrap_or(0),
                payload["user_id"].as_i64().unwrap_or(0),
            )
            .map_err(|_| TransportError::InvalidResponse)?;
        let mut attempted = payload.clone();
        attempted["attempted"] = json!(true);
        self.conn
            .execute(
                "UPDATE mutation_queue SET payload_json=? WHERE id=?",
                rusqlite::params![attempted.to_string(), row.id],
            )
            .map_err(|_| TransportError::InvalidResponse)?;
        // These web routes create activities but do not deduplicate idempotency keys.
        Ok(match transport.post(path, &body, &row.client_id) {
            Ok(response) if response.status == 200 && change.valid_ack(payload, &response.body) => {
                Response {
                    status: 200,
                    body: json!({"success":true,"value":response.body}),
                }
            }
            Ok(response)
                if response.status == 200
                    && matches!(change, TaskAction::Relation { .. })
                    && response.body.as_array().is_some_and(Vec::is_empty) =>
            {
                Response {
                    status: 409,
                    body: json!({"message":"Task relation already exists or cannot be added"}),
                }
            }
            Ok(response) if (400..500).contains(&response.status) => response,
            _ => Response {
                status: 409,
                body: json!({"success":false}),
            },
        })
    }
    pub(crate) fn resolve_task_action(&self, payload: &Value, body: &Value) -> Result<()> {
        ensure!(
            self.current_user_id() == payload["user_id"].as_i64(),
            "Unauthorized"
        );
        let change: TaskAction = serde_json::from_value(payload["change"].clone())?;
        let value = &body["value"];
        ensure!(
            change.valid_ack(payload, value),
            "Task update was not acknowledged"
        );
        let mut task = self
            .raw("Task", &payload["task_id"])?
            .context("Task is unavailable")?;
        if matches!(change, TaskAction::RemoveRelation(_)) {
            // The deleted row is already absent optimistically; the server confirms its id.
        } else if matches!(change, TaskAction::Relation { .. }) {
            for relation in value
                .as_array()
                .context("Invalid relation acknowledgement")?
            {
                crate::task_relations::put(&mut task, relation.clone());
            }
        } else if matches!(change, TaskAction::AssignMe) {
            let rows: Vec<_> = value["body"]
                .as_array()
                .unwrap()
                .iter()
                .map(|row| {
                    let mut member = row["user"].clone();
                    if row["agentId"].is_null() && row["agent"].is_null() && member.is_object() {
                        member["userId"] = row["userId"].clone();
                        member
                    } else {
                        row.clone()
                    }
                })
                .collect();
            task["assigneeCount"] = json!(rows.len());
            task["assignees"] = json!(rows);
        } else {
            for field in change.fields() {
                task[*field] = value[*field].clone();
            }
        }
        self.upsert_task(&task)?;
        self.request_refetch(
            "task",
            payload["task_id"].as_i64().context("Task is unavailable")?,
        )
    }
    pub(crate) fn rollback_task_action(&self, payload: &Value) -> Result<()> {
        if self.current_user_id() != payload["user_id"].as_i64() {
            return Ok(());
        }
        let change: TaskAction = serde_json::from_value(payload["change"].clone())?;
        let mut task = self
            .raw("Task", &payload["task_id"])?
            .context("Task is unavailable")?;
        for field in change.fields() {
            task[*field] = payload["before"][*field].clone();
        }
        self.upsert_task(&task)
    }
}
impl TaskAction {
    fn valid_ack(&self, payload: &Value, value: &Value) -> bool {
        if value["success"] == false {
            return false;
        }
        match self {
            Self::RemoveRelation(id) => {
                value["status"] == 200
                    && value["json"]["id"] == *id
                    && ["relatedFromTasks", "relatedToTasks"].iter().any(|field| {
                        payload["before"][*field]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .any(|row| {
                                row["id"] == *id
                                    && row["sourceTaskId"] == value["json"]["sourceTaskId"]
                                    && row["targetTaskId"] == value["json"]["targetTaskId"]
                            })
                    })
            }
            Self::Relation { target, relation } => value.as_array().is_some_and(|rows| {
                rows.len() == 1
                    && rows.iter().all(|row| {
                        row["id"].as_i64().is_some_and(|id| id > 0)
                            && row["sourceTaskId"] == payload["task_id"]
                            && row["targetTaskId"] == target["id"]
                            && row["relationType"] == json!(relation)
                            && row["targetTask"]["id"] == target["id"]
                    })
            }),
            Self::Recurrence(rule) => {
                value["id"] == payload["task_id"] && value.get("recurrence") == Some(&json!(rule))
            }
            Self::StartDate(date) => {
                value["id"] == payload["task_id"]
                    && if let Some(date) = date {
                        value["startDate"]
                            .as_str()
                            .and_then(|ack| chrono::DateTime::parse_from_rfc3339(ack).ok())
                            .zip(chrono::DateTime::parse_from_rfc3339(date).ok())
                            .is_some_and(|(ack, date)| ack == date)
                    } else {
                        value.get("startDate") == Some(&Value::Null)
                    }
            }
            Self::WaitingOn(user) => {
                value["id"] == payload["task_id"]
                    && value.get("waitingOnUserId") == Some(&json!(user))
                    && if user.is_some() {
                        value["waitingOnSetById"] == payload["user_id"]
                            && value["waitingOnSetAt"].as_str().is_some_and(|date| {
                                chrono::DateTime::parse_from_rfc3339(date).is_ok()
                            })
                    } else {
                        value.get("waitingOnSetById") == Some(&Value::Null)
                            && value.get("waitingOnSetAt") == Some(&Value::Null)
                    }
            }
            Self::AssignMe => value["body"].as_array().is_some_and(|rows| {
                rows.iter().all(|row| row["taskId"] == payload["task_id"])
                    && rows
                        .iter()
                        .any(|row| row["userId"] == payload["user_id"] && row["agentId"].is_null())
            }),
        }
    }
}
