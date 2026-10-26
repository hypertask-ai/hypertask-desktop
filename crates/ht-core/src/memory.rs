use crate::{Db, Result};
use anyhow::ensure;
use rusqlite::ffi;
use serde_json::{Value, json};

impl Db {
    pub fn memory_usage(&self) -> Result<Value> {
        let setting: i64 = self.conn.query_row("PRAGMA cache_size", [], |row| row.get(0))?;
        let mut usage = json!({"SqliteCacheSizeSetting": setting, "CompactTaskReads": cfg!(feature = "compact-task-reads"), "CompactInboxMetadata": cfg!(feature = "compact-inbox-metadata"), "PackedInboxMetadata": cfg!(feature = "packed-inbox-metadata")});
        for (key, operation) in [
            ("SqliteCacheBytes", ffi::SQLITE_DBSTATUS_CACHE_USED),
            ("SqliteSchemaBytes", ffi::SQLITE_DBSTATUS_SCHEMA_USED),
            ("SqliteStatementBytes", ffi::SQLITE_DBSTATUS_STMT_USED),
        ] {
            let mut current = 0;
            let mut high_water = 0;
            // The connection remains borrowed on its owning thread; reset=0 is
            // observational and does not evict cache pages or trim the heap.
            let status = unsafe {
                ffi::sqlite3_db_status(self.conn.handle(), operation, &mut current, &mut high_water, 0)
            };
            ensure!(status == ffi::SQLITE_OK, "SQLite memory status failed");
            usage[key] = json!(current);
        }
        for (key, sql) in [
            ("StoredTasks", "SELECT count(*) FROM Task"),
            ("StoredTaskJsonBytes", "SELECT coalesce(sum(length(CAST(raw_json AS BLOB))),0) FROM Task"),
            ("StoredDescriptionBytes", "SELECT coalesce(sum(length(CAST(description AS BLOB))),0) FROM Task"),
            ("StoredNotifications", "SELECT count(*) FROM Notification"),
            ("StoredNotificationJsonBytes", "SELECT coalesce(sum(length(CAST(raw_json AS BLOB))),0) FROM Notification"),
        ] {
            let bytes: i64 = self.conn.query_row(sql, [], |row| row.get(0))?;
            usage[key] = json!(bytes);
        }
        Ok(usage)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_receipt_observes_sqlite_without_changing_cached_tasks() {
        let mut db = Db::memory().unwrap();
        let fixture: Value = serde_json::from_str(include_str!("../../../fixtures/data-fidelity.json")).unwrap();
        db.apply_response(&fixture["detail"]).unwrap();
        let before = db.task(1).unwrap();
        let usage = db.memory_usage().unwrap();
        assert!(usage["SqliteCacheBytes"].as_i64().unwrap() > 0);
        assert!(usage["SqliteSchemaBytes"].as_i64().unwrap() > 0);
        assert!(usage["SqliteStatementBytes"].as_i64().unwrap() >= 0);
        assert_eq!(db.task(1).unwrap(), before);
    }
}
