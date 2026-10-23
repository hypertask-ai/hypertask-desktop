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
