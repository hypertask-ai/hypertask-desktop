use crate::{Db, Result, TaskView, db::task_view};
use serde_json::Value;

impl Db {
    pub fn search_tasks(&self, query: &str, project: Option<i64>) -> Result<Vec<TaskView>> {
        let tokens: Vec<_> = query.split_whitespace().map(str::to_lowercase).collect();
        self.json_rows(
            "SELECT raw_json FROM Task WHERE status='Normal' ORDER BY updatedAt DESC,id",
            [],
        )?
        .into_iter()
        .map(task_view)
        .filter_map(|task| match task {
            Ok(task)
                if project.is_none_or(|id| id == task.project_id)
                    && tokens.iter().all(|token| {
                        format!(
                            "{} {} {}",
                            task.title,
                            task.ticket_number.as_deref().unwrap_or(""),
                            task.description
                        )
                        .to_lowercase()
                        .contains(token)
                    }) =>
            {
                Some(Ok(task))
            }
            Err(error) => Some(Err(error)),
            _ => None,
        })
        .take(100)
        .collect()
    }

    pub fn projects(&self) -> Result<Vec<Value>> {
        self.json_rows("SELECT raw_json FROM Project ORDER BY title,id", [])
    }
}

impl Db {
    pub fn project_labels(&self, project: i64) -> Result<Vec<Value>> {
        self.json_rows(
            "SELECT raw_json FROM Label WHERE projectId=? ORDER BY value,id",
            [project],
        )
    }

    pub fn project_members(&self, project: i64) -> Result<Vec<Value>> {
        if let Some(json) = self.kv_get(&format!("members:{project}"))? {
            return Ok(serde_json::from_str(&json)?);
        }
        let mut members = std::collections::BTreeMap::new();
        for task in self.json_rows(
            "SELECT raw_json FROM Task WHERE projectId=? ORDER BY id",
            [project],
        )? {
            for member in task["assignees"].as_array().into_iter().flatten() {
                if let Some(id) = member["id"].as_i64() {
                    members.insert(id, member.clone());
                }
            }
        }
        Ok(members.into_values().collect())
    }
}

impl<T: crate::Transport> crate::Sync<T> {
    pub fn catalog(&mut self, project: i64) -> Result<()> {
        let members = crate::http::successful(
            self.transport
                .get(&format!("/projects/{project}/members"))?,
        )?;
        let labels =
            crate::http::successful(self.transport.get(&format!("/projects/{project}/labels"))?)?;
        anyhow::ensure!(
            members["projectId"] == project && labels["projectId"] == project,
            "catalog returned another board"
        );
        let members = crate::db::array(&members, "members")?;
        let labels = crate::db::array(&labels, "labels")?;
        self.db.transaction(|db| {
            db.kv_set(
                &format!("members:{project}"),
                &serde_json::to_string(members)?,
            )?;
            for label in labels {
                let mut label = label.clone();
                label["projectId"] = serde_json::json!(project);
                label["value"] = label["name"].clone();
                db.upsert("Label", &label)?;
            }
            Ok(())
        })
    }
}
