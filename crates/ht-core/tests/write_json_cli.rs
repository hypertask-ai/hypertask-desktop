use ht_core::Db;
use serde_json::json;
use std::process::Command;

fn run(db: &std::path::Path, payload: &str) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_ht-core-cli"))
        .env_remove("HT_TOKEN")
        .args(["--db", db.to_str().unwrap(), "write-json", payload])
        .output()
        .unwrap()
}

fn seeded(path: &std::path::Path) {
    let mut db = Db::open(path).unwrap();
    db.apply_response(&json!({"success":true,"projects":[{"id":7049,"title":"QA Sandbox","status":"Normal"}]})).unwrap();
    db.apply_task(&json!({"id":501,"projectId":7049,"title":"before","status":"Normal"})).unwrap();
}

#[test]
fn write_json_queues_any_write_kind_and_survives_reopening() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("local.db");
    seeded(&path);
    let queued = run(&path, r#"{"kind":"task_patch","task_id":501,"patch":{"Priority":2}}"#);
    assert!(queued.status.success(), "{}", String::from_utf8_lossy(&queued.stderr));
    assert!(String::from_utf8_lossy(&queued.stdout).contains("QUEUED"));
    let reopened = Db::open(&path).unwrap();
    let pending = reopened.mutations().unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].kind, "task_patch");
    assert_eq!(pending[0].payload_json["priority"], 2);
    assert_eq!(pending[0].payload_json["task_id"], 501);
}

#[test]
fn write_json_rejects_unknown_kinds_without_queueing() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("local.db");
    seeded(&path);
    let refused = run(&path, r#"{"kind":"nope","task_id":501}"#);
    assert!(!refused.status.success());
    assert!(Db::open(&path).unwrap().mutations().unwrap().is_empty());
}
