#![cfg(feature = "compact-task-reads")]

use ht_core::Db;
use serde_json::json;

#[test]
fn compact_cards_preserve_every_display_field_and_full_detail_search_and_store() {
    let mut db = Db::memory().unwrap();
    let sections = json!({"success":true,"sections":[
        {"id":1,"projectId":15,"section_title":"Todo","ranking":"a","visibility":true,"deleted":false},
        {"id":2,"projectId":15,"section_title":"Doing","ranking":"b","visibility":true,"deleted":false}
    ]});
    db.apply_response(&sections).unwrap();
    let description = format!("<p>searchable-unabridged-description {}</p>", "rich text ".repeat(6000));
    for id in 1..=220 {
        db.apply_task(&json!({"id":id,"title":format!("Card {id}"),"projectId":15,
            "sectionId":1 + id % 2,"status":"Normal","description":description,
            "ticketNumber":format!("MEM-{id}"),"dueDate":"2026-10-20T00:00:00Z",
            "assignees":[{"id":id,"displayName":format!("Person {id}")}],
            "labels":[{"id":format!("label-{id}"),"name":"customer"}],
            "priority":{"Priority_Value":"High"},"hasBoardDraft":true,
            "summary":"Summary remains available", "attachments":[{"url":"https://example.org/image.png"}],
            "pullRequests":[{"title":"detail-only PR"}], "savedContent":{"body":"detail only"},
            "customFieldValues":[{"value":"detail only"}], "followers":[{"id":7}], "sub_tasks":[{"id":99}], "updatedAt":"2026-10-20T00:00:00Z"})).unwrap();
    }
    let full = db.board(15).unwrap();
    let compact = db.board_cards(15).unwrap();
    assert_eq!(compact.iter().map(|c| c.cards.len()).sum::<usize>(), 220);
    for (full, compact) in full.iter().zip(&compact) {
        assert_eq!((&full.id, &full.title, &full.ranking), (&compact.id, &compact.title, &compact.ranking));
        for (full, compact) in full.cards.iter().zip(&compact.cards) {
            assert_eq!(full.description, description);
            let mut expected = full.clone();
            expected.description.clear();
            for key in ["attachments", "pullRequests", "savedContent", "customFieldValues", "followers", "sub_tasks"] {
                expected.extra.remove(key);
            }
            assert_eq!(&expected, compact);
            assert_eq!(db.task_detail(compact.id).unwrap().unwrap().task, *full);
        }
    }
    assert!(!db.search_tasks("searchable-unabridged-description", Some(15)).unwrap().is_empty());
    assert_eq!(db.task(1).unwrap().unwrap().description, description);
    assert!(serde_json::to_vec(&compact).unwrap().len() * 10 < serde_json::to_vec(&full).unwrap().len());
}

#[test]
fn compact_board_keeps_duplicate_legacy_section_membership_and_order() {
    let mut db = Db::memory().unwrap();
    db.apply_response(&json!({"success":true,"sections":[
        {"id":1,"projectId":15,"section_title":"Todo","ranking":"a","visibility":true,"deleted":false},
        {"id":2,"projectId":15,"section_title":"Todo","ranking":"b","visibility":true,"deleted":false}
    ],"tasks":[
        {"id":1,"projectId":15,"title":"Legacy","status":"Normal","section":"Todo","description":"retain"},
        {"id":2,"projectId":15,"title":"Modern","status":"Normal","sectionId":2,"description":"retain"}
    ]})).unwrap();
    let full = db.board(15).unwrap();
    let compact = db.board_cards(15).unwrap();
    for (full, compact) in full.iter().zip(compact) {
        assert_eq!(full.cards.iter().map(|t|t.id).collect::<Vec<_>>(), compact.cards.iter().map(|t|t.id).collect::<Vec<_>>());
    }
    assert_eq!(full[0].cards.len(), 1);
    assert_eq!(full[1].cards.len(), 2);
}

#[test]
fn compact_inbox_uses_current_title_and_ticket_without_materializing_description() {
    let mut db = Db::memory().unwrap();
    db.apply_response(&json!({"success":true,"tasks":[
        {"id":1,"projectId":15,"title":"Current title","ticketNumber":"MEM-1","status":"Normal","description":"large ".repeat(10000)}
    ],"user_notifications":[
        {"id":1,"taskId":1,"status":"Normal","createdAt":"2026-10-20T00:00:00Z", "task":{"title":"Stale title","ticketNumber":"STALE"}},
        {"id":2,"taskId":2,"status":"Normal","createdAt":"2026-10-20T00:00:00Z", "task":{"title":"Uncached title","ticketNumber":"MEM-2"}},
        {"id":3,"status":"Normal","createdAt":"2026-10-20T00:00:00Z", "message":"Invitation"}
    ]})).unwrap();
    let items = db.inbox().unwrap();
    let task = items.iter().find(|n|n.id == 1).unwrap();
    assert_eq!(task.title, "Current title");
    assert_eq!(task.ticket_number.as_deref(), Some("MEM-1"));
    let uncached = items.iter().find(|n|n.id == 2).unwrap();
    assert_eq!(uncached.title, "Uncached title");
    assert_eq!(uncached.ticket_number.as_deref(), Some("MEM-2"));
    assert_eq!(items.iter().find(|n|n.id == 3).unwrap().title, "Invitation");
    assert_eq!(db.task(1).unwrap().unwrap().description.len(), 60000);
}

#[test]
fn compact_inbox_omits_only_hidden_task_bodies_and_preserves_full_store() {
    let mut db = Db::memory().unwrap();
    let task = json!({"id":1,"projectId":15,"title":"Current","ticketNumber":"MEM-1","status":"Normal",
        "description":"large ".repeat(10000),"attachments":[{"url":"https://example.org/large.png"}],
        "pullRequests":[{"title":"PR"}],"savedContent":{"text":"saved"},"customFieldValues":[1],
        "followers":[{"id":2}],"sub_tasks":[{"id":3}],"dueDate":"2026-10-20T00:00:00Z",
        "priority":{"Priority_Value":"High"},"estimate":{"estimate_value":"3"},
        "taskLabels":[{"label":{"value":"Customer"}}]});
    db.apply_response(&json!({"success":true,"tasks":[task],"user_notifications":[{
        "id":1,"taskId":1,"projectId":15,"status":"Normal","createdAt":"2026-10-20T00:00:00Z",
        "task":task,"comment":{"text":"<p>Keep this preview</p>"},"directReply":true,
        "fromUser":{"displayName":"Colleague","photoURL":"https://example.org/avatar.png"}
    }]})).unwrap();
    let full = db.inbox().unwrap();
    let rows = db.inbox_rows().unwrap();
    let mut expected = serde_json::to_value(&full).unwrap();
    for key in ["description", "attachments", "pullRequests", "savedContent", "customFieldValues", "followers", "sub_tasks"] {
        expected[0]["rawJson"]["task"].as_object_mut().unwrap().remove(key);
    }
    assert_eq!(serde_json::to_value(&rows).unwrap(), expected);
    assert_eq!(db.inbox().unwrap()[0].raw_json, full[0].raw_json);
    assert_eq!(db.task(1).unwrap().unwrap().description, task["description"].as_str().unwrap());
    assert!(serde_json::to_vec(&rows).unwrap().len() * 20 < serde_json::to_vec(&full).unwrap().len());
}
