use crate::{Db, Result};
use anyhow::ensure;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Only the shell's existing personal-bearer MCP contracts may enter the queue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellWrite {
    pub(crate) user_id: i64,
    method: String,
    path: String,
    body: Value,
}

impl ShellWrite {
    pub fn new(user_id: i64, method: &str, path: &str, body: Option<Value>) -> Result<Self> {
        let write = Self { user_id, method: method.into(), path: path.into(), body: body.unwrap_or(Value::Null) };
        write.validate()?;
        Ok(write)
    }

    pub(crate) fn validate(&self) -> Result<()> {
        ensure!(self.user_id > 0, "sync your account before changing settings");
        let positive = |id: &str| id.parse::<i64>().is_ok_and(|id| id > 0);
        let opaque = |id: &str| !id.is_empty() && id.bytes().all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_'));
        let parts: Vec<_> = self.path.split('/').collect();
        let allowed = match (self.method.as_str(), parts.as_slice()) {
            ("PATCH", ["", "user", "profile"]) => true,
            ("POST", ["", "projects", "archive"]) => true,
            ("PUT", ["", "projects", id, "instructions"]) => positive(id),
            ("POST", ["", "projects", id, "members"]) => positive(id),
            ("POST", ["", "skills"]) | ("POST", ["", "skills", "import"]) => true,
            ("PATCH" | "DELETE", ["", "skills", id]) => positive(id),
            ("POST", ["", "webhooks"]) => true,
            ("POST", ["", "webhooks", id, "test"]) => opaque(id),
            ("DELETE", ["", query]) => query.strip_prefix("webhooks?id=").is_some_and(opaque),
            _ => false,
        };
        ensure!(allowed, "unsupported shell bearer mutation");
        ensure!(self.body.is_object() || (self.method == "DELETE" && self.body.is_null()), "shell mutation requires an object body");
        Ok(())
    }

    pub(crate) fn request(&self) -> (&str, &str, &Value) {
        (&self.method, &self.path, &self.body)
    }
}

impl Db {
    pub fn take_shell_result(&mut self, client_id: &str) -> Result<Option<Value>> {
        self.transaction(|db| {
            let key = format!("shell-result:{client_id}");
            let value = db.kv_get(&key)?.map(|value| serde_json::from_str(&value)).transpose()?;
            db.conn.execute("DELETE FROM kv WHERE key=?", [&key])?;
            Ok(value)
        })
    }
}
