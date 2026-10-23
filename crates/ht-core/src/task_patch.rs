use crate::{Result, generated::Status};
use anyhow::ensure;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// Only fields supported by the authenticated /tasks/update contract.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskPatch {
    Status(Status),
    Description(String),
    Priority(u8),
    Estimate(u8),
    DueDate(Option<String>),
}
impl TaskPatch {
    pub(crate) fn fields(&self) -> Result<Value> {
        Ok(match self {
            Self::Status(status) => json!({"status": status}),
            Self::Description(text) => json!({"description": text, "content_type": "html"}),
            Self::Priority(index) => {
                ensure!(*index <= 4, "priority must be 0–4");
                json!({"priority": index})
            }
            Self::Estimate(index) => {
                ensure!([0, 2, 3, 4, 5, 6].contains(index), "unknown task size");
                json!({"estimate": index})
            }
            Self::DueDate(date) => {
                if let Some(date) = date { chrono::DateTime::parse_from_rfc3339(date)?; }
                json!({"due_date": date})
            }
        })
    }
}

pub(crate) fn optimistic(task: &mut Value, fields: &Value) {
    for key in ["status", "description"] {
        if let Some(value) = fields.get(key) { task[key] = value.clone(); }
    }
    if let Some(value) = fields.get("due_date") { task["dueDate"] = value.clone(); }
    if let Some(index) = fields["priority"].as_u64() {
        let name = ["No Priority", "Urgent", "High", "Medium", "Low"][index as usize];
        task["priority"] = json!({"priority_index": index, "Priority_Value": name});
    }
    if let Some(index) = fields["estimate"].as_u64() {
        let name = match index { 2 => "XS", 3 => "S", 4 => "M", 5 => "L", 6 => "XL", _ => "No size" };
        task["estimate"] = json!({"estimate_index": index, "estimate_value": name});
    }
}
