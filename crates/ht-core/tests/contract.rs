use ht_core::{Db, generated::Status};
use serde_json::{Value, json};
use std::{collections::HashSet, path::PathBuf};

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/demo")
}
fn load(name: &str) -> Value {
    serde_json::from_slice(&std::fs::read(fixtures().join(name)).unwrap()).unwrap()
}

#[test]
fn contract_every_live_response() {
    let mut files: Vec<_> = std::fs::read_dir(fixtures())
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .collect();
    files.sort();
    assert!(!files.is_empty());
    for path in files {
        let response: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        let mut db = Db::memory().unwrap();
        let key = [
            "projects",
            "sections",
            "tasks",
            "user_notifications",
            "comments",
        ]
        .into_iter()
        .find(|k| response[*k].is_array())
        .unwrap();
        let model = match key {
            "projects" => "Project",
            "sections" => "Section",
            "tasks" => "Task",
            "user_notifications" => "Notification",
            "comments" => "Comment",
            _ => unreachable!(),
        };
        if key == "comments" {
            db.apply_comments(58006, &response).unwrap();
        } else {
            db.apply_response(&response).unwrap();
        }
        let dump = db.dump().unwrap();
        let expected = response[key].as_array().unwrap();
        let actual = dump[model].as_array().unwrap();
        let unique: HashSet<_> = expected.iter().map(|v| v["id"].to_string()).collect();
        assert_eq!(actual.len(), unique.len(), "{}", path.display());
        for row in expected {
            let stored = actual.iter().find(|v| v["id"] == row["id"]).unwrap();
            for field in [
                "id",
                "title",
                "text",
                "description",
                "status",
                "projectId",
                "sectionId",
                "section_title",
                "ranking",
                "name",
                "type",
                "seen",
                "createdAt",
                "updatedAt",
                "commentCount",
                "totalComments",
                "assignees",
                "labels",
                "task",
            ] {
                if let Some(value) = row.get(field) {
                    assert_eq!(stored[field], *value, "{} {field}", path.display());
                }
            }
            if model == "Task" {
                let view = db.task(row["id"].as_i64().unwrap()).unwrap().unwrap();
                assert_eq!(view.title, row["title"]);
                assert_eq!(view.description, row["description"]);
                assert_eq!(
                    view.comment_count,
                    row.get("commentCount")
                        .or_else(|| row.get("totalComments"))
                        .unwrap()
                        .as_i64()
                        .unwrap()
                );
                assert_eq!(
                    view.label_count,
                    row["labelCount"]
                        .as_i64()
                        .unwrap_or_else(|| row["labels"].as_array().map_or(0, Vec::len) as i64)
                );
            }
        }
        db.apply_response(&json!({"success":true,"tasks":[]}))
            .unwrap();
        assert_eq!(
            db.dump().unwrap()[model].as_array().unwrap().len(),
            unique.len()
        );
    }
}

#[test]
fn contract_board_order_and_offline_detail() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let mut db = Db::open(file.path()).unwrap();
    db.apply_response(&load("sections-15.json")).unwrap();
    db.apply_response(&load("tasks-15-0.json")).unwrap();
    db.apply_response(&load("task-58006.json")).unwrap();
    db.apply_comments(58006, &load("comments-58006.json"))
        .unwrap();
    drop(db);
    let db = Db::open(file.path()).unwrap();
    let board = db.board(15).unwrap();
    assert_eq!(
        board.len(),
        load("sections-15.json")["sections"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|s| s["visibility"] == true && s["deleted"] == false)
            .count()
    );
    assert!(board.windows(2).all(|c| c[0].ranking <= c[1].ranking));
    for column in &board {
        assert!(
            column
                .cards
                .iter()
                .all(|t| t.status == Status::Normal && t.section_id == Some(column.id))
        );
        assert!(
            column
                .cards
                .windows(2)
                .all(|c| c[0].updated_at >= c[1].updated_at)
        );
    }
    let detail = db.task_detail(58006).unwrap().unwrap();
    assert_eq!(detail.task.ticket_number.as_deref(), Some("DEMO-7041"));
    assert_eq!(detail.task.comment_count, 58);
    assert_eq!(detail.task.assignee_count, 1);
    assert_eq!(detail.task.label_count, 1);
    assert_eq!(detail.comments.len(), 50);
    assert_eq!(detail.comments[0].id, 286362);
    assert_eq!(detail.comments[0].creator_id, Some(6));
    assert_eq!(
        detail.comments[0].extra["agent_display_name"],
        "Sam Example"
    );
}

#[test]
fn contract_inbox_fields_and_soft_status_filtering() {
    let response = load("inbox.json");
    let mut db = Db::memory().unwrap();
    db.apply_response(&response).unwrap();
    let inbox = db.inbox().unwrap();
    let expected: Vec<_> = response["user_notifications"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|n| n["status"] == "Normal" && n["archivedAt"].is_null())
        .collect();
    assert_eq!(inbox.len(), expected.len());
    for item in &inbox {
        let row = expected.iter().find(|n| n["id"] == item.id).unwrap();
        assert_eq!(
            item.title,
            row["task"]["title"]
                .as_str()
                .unwrap_or_else(|| row["message"].as_str().unwrap_or(""))
        );
        assert_eq!(item.seen, row["seen"].as_bool().unwrap());
        assert_eq!(item.task_id, row["taskId"].as_i64());
        assert_eq!(&item.raw_json, *row);
    }
    assert_eq!(inbox[0].id, 322068);
    assert_eq!(inbox[0].created_at, "2026-10-19T21:39:00.711Z");
    let first = &inbox[0];
    if let Some(task_id) = first.task_id {
        db.apply_task(&json!({"id":task_id,"title":"hidden","projectId":8893,"status":"Archive"}))
            .unwrap();
        assert!(
            !db.inbox()
                .unwrap()
                .iter()
                .any(|n| n.task_id == Some(task_id))
        );
    }
}

#[test]
fn contract_list_and_detail_count_aliases_are_not_stale() {
    let mut db = Db::memory().unwrap();
    db.apply_response(&load("tasks-15-0.json")).unwrap();
    db.apply_response(&load("task-58006.json")).unwrap();
    let mut task = load("task-58006.json")["tasks"][0].clone();
    task["totalComments"] = json!(71);
    db.apply_task(&task).unwrap();
    assert_eq!(db.task(58006).unwrap().unwrap().comment_count, 71);
    assert_eq!(
        db.dump().unwrap()["Label"].as_array().unwrap().len() >= 1,
        true
    );
}

#[test]
fn contract_missing_section_id_uses_title_without_stale_column_id() {
    let mut db = Db::memory().unwrap();
    db.apply_response(&load("sections-15.json")).unwrap();
    db.apply_response(&load("task-58006.json")).unwrap();
    let section = load("sections-15.json")["sections"][0].clone();
    let mut task = load("task-58006.json")["tasks"][0].clone();
    task.as_object_mut().unwrap().remove("sectionId");
    task["section"] = section["section_title"].clone();
    db.apply_task(&task).unwrap();
    assert!(db.task(58006).unwrap().unwrap().section_id.is_none());
    let board = db.board(15).unwrap();
    let column = board.iter().find(|c| json!(c.id) == section["id"]).unwrap();
    assert!(column.cards.iter().any(|t| t.id == 58006));
}

#[test]
fn contract_inbox_reads_optimistic_task_title_from_sqlite() {
    let mut db = Db::memory().unwrap();
    db.apply_response(&load("inbox.json")).unwrap();
    let item = db
        .inbox()
        .unwrap()
        .into_iter()
        .find(|n| n.task_id.is_some())
        .unwrap();
    let task_id = item.task_id.unwrap();
    db.apply_task(&json!({"id":task_id,"title":item.title,"projectId":15,"status":"Normal"}))
        .unwrap();
    db.enqueue(
        ht_core::Write::TaskTitle {
            task_id,
            title: "optimistic inbox title".into(),
        },
        1,
    )
    .unwrap();
    assert_eq!(
        db.inbox()
            .unwrap()
            .iter()
            .find(|n| n.id == item.id)
            .unwrap()
            .title,
        "optimistic inbox title"
    );
}
