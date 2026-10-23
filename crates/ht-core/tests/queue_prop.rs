use ht_core::{
    Db, Drain, Sync, Transport, Write,
    http::{Response, TransportError},
};
use proptest::prelude::*;
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Default)]
struct Server {
    tasks: BTreeMap<i64, Value>,
    notifications: BTreeMap<i64, Value>,
    comments: BTreeMap<i64, Value>,
    responses: HashMap<String, Response>,
    executions: Vec<String>,
    requests: Vec<String>,
    mode: u8,
    clock: i64,
}
impl Server {
    fn new() -> Self {
        let tasks = (1..=2).map(|id|(id,json!({"id":id,"title":"initial","projectId":15,"sectionId":1,"section":"Todo","description":"body","status":"Normal","updatedAt":"2026-01-01T00:00:00Z","commentCount":0}))).collect();
        let notifications = (1..=2).map(|id|(id,json!({"id":id,"status":"Normal","archivedAt":null,"taskId":id,"createdAt":"2026-01-01T00:00:00Z","seen":false,"task":{"title":"initial"}}))).collect();
        Self {
            tasks,
            notifications,
            clock: 1000,
            ..Self::default()
        }
    }
    fn apply(&mut self, path: &str, body: &Value) -> Response {
        self.clock += 1;
        let response = match path {
            "/tasks/update" => {
                let task = self
                    .tasks
                    .get_mut(&body["task_id"].as_i64().unwrap())
                    .unwrap();
                if let Some(title) = body.get("title") {
                    task["title"] = title.clone();
                }
                if let Some(section) = body.get("sectionId") {
                    task["sectionId"] = section.clone();
                    task["section"] = json!(if *section == 1 { "Todo" } else { "Doing" });
                }
                task["updatedAt"] = json!(
                    chrono::DateTime::from_timestamp(self.clock, 0)
                        .unwrap()
                        .to_rfc3339()
                );
                json!({"success":true,"tasks":[task]})
            }
            "/comments" => {
                let id = self.clock;
                let comment = json!({"id":id,"text":body["text"],"taskId":body["task_id"],"createdAt":"2026-01-01T00:00:00Z"});
                self.comments.insert(id, comment.clone());
                let task = self
                    .tasks
                    .get_mut(&body["task_id"].as_i64().unwrap())
                    .unwrap();
                task["commentCount"] = json!(task["commentCount"].as_i64().unwrap() + 1);
                json!({"success":true,"comment":comment})
            }
            "/inbox/archive" => {
                for id in body["notification_ids"].as_array().unwrap() {
                    let n = self.notifications.get_mut(&id.as_i64().unwrap()).unwrap();
                    n["status"] = json!("Archive");
                    n["archivedAt"] = json!("2026-01-01T00:00:00Z");
                }
                json!({"success":true,"archived_count":1})
            }
            _ => panic!("unexpected path"),
        };
        Response {
            status: 200,
            body: response,
        }
    }
}
impl Transport for Server {
    fn get(&mut self, path: &str) -> std::result::Result<Response, TransportError> {
        let url = url::Url::parse(&format!("https://test.invalid{path}")).unwrap();
        let q: HashMap<_, _> = url.query_pairs().into_owned().collect();
        let body = match url.path() {
            "/tasks" => {
                json!({"success":true,"tasks":[self.tasks[&q["task_id"].parse::<i64>().unwrap()].clone()]})
            }
            "/comments" => {
                let task_id: i64 = q["task_id"].parse().unwrap();
                let items: Vec<_> = self
                    .comments
                    .values()
                    .filter(|c| c["taskId"] == task_id)
                    .cloned()
                    .collect();
                json!({"success":true,"total":items.len(),"comments":items})
            }
            "/inbox/list" => {
                json!({"success":true,"user_notifications":self.notifications.values().filter(|n|n["status"]=="Normal").collect::<Vec<_>>()})
            }
            _ => panic!("unexpected refetch {path}"),
        };
        Ok(Response { status: 200, body })
    }
    fn post(
        &mut self,
        path: &str,
        body: &Value,
        client: &str,
    ) -> std::result::Result<Response, TransportError> {
        self.requests.push(client.to_owned());
        match self.mode {
            1 => {
                return Ok(Response {
                    status: 500,
                    body: Value::Null,
                });
            }
            2 => return Err(TransportError::Timeout),
            4 => {
                return Ok(Response {
                    status: 422,
                    body: Value::Null,
                });
            }
            5 => return Err(TransportError::Offline),
            _ => (),
        }
        let response = if let Some(response) = self.responses.get(client) {
            response.clone()
        } else {
            let response = self.apply(path, body);
            self.responses.insert(client.to_owned(), response.clone());
            self.executions.push(client.to_owned());
            response
        };
        if self.mode == 3 {
            Err(TransportError::Timeout)
        } else {
            Ok(response)
        }
    }
}

fn setup() -> Sync<Server> {
    let mut db = Db::memory().unwrap();
    let server = Server::new();
    for task in server.tasks.values() {
        db.apply_task(task).unwrap();
    }
    db.apply_response(&json!({"success":true,"sections":[
        {"id":1,"projectId":15,"section_title":"Todo","ranking":"A","deleted":false,"visibility":true},
        {"id":2,"projectId":15,"section_title":"Doing","ranking":"B","deleted":false,"visibility":true}
    ],"user_notifications":server.notifications.values().collect::<Vec<_>>()})).unwrap();
    Sync::new(db, server)
}

proptest! {
    #![proptest_config(ProptestConfig { cases:128, failure_persistence:Some(Box::new(proptest::test_runner::FileFailurePersistence::WithSource("proptest-regressions"))), ..ProptestConfig::default() })]
    #[test]
    fn prop_queue_interleavings_are_durable_ordered_and_convergent(operations in prop::collection::vec((0u8..14,1i64..3,any::<u16>()),1..100)) {
        let mut sync = setup();
        let mut now = 100;
        let mut enqueued = BTreeMap::new();
        let mut applied = HashSet::new();
        let mut failed = HashSet::new();
        for (op,entity,value) in operations {
            now += 1;
            let write = match op {
                0 => Some(Write::TaskTitle{task_id:entity,title:format!("edit {value}")}),
                1 => Some(Write::TaskSection{task_id:entity,section_id:1+i64::from(value%2)}),
                2 => Some(Write::TaskComment{task_id:entity,text:format!("comment {value}")}),
                3 => Some(Write::InboxArchive{notification_id:entity}),
                _ => None,
            };
            if let Some(write) = write {
                let id = sync.db.enqueue(write,now).unwrap();
                let row = sync.db.mutations().unwrap().into_iter().find(|r|r.id==id).unwrap();
                enqueued.insert(id,(row.client_id,row.entity));
            } else if op == 9 {
                sync.transport.tasks.get_mut(&entity).unwrap()["title"] = json!(format!("remote {value}"));
                sync.db.apply_event(&json!({"event":"task:changed","channel":format!("private-task-{entity}"),"data":json!({"originUserId":6}).to_string()}).to_string()).unwrap();
                if value%2==0 { sync.refetch_pending().unwrap(); }
            } else if op == 10 {
                sync.transport.clock += 1;
                let id = sync.transport.clock;
                let comment = json!({"id":id,"taskId":entity,"text":format!("remote {value}"),"createdAt":"2026-01-01T00:00:00Z"});
                sync.transport.comments.insert(id,comment.clone());
                sync.db.apply_event(&json!({"event":"comment:changed","channel":format!("private-task-{entity}"),"data":{"originUserId":6,"comment":comment}}).to_string()).unwrap();
            } else if op >= 12 {
                now += i64::from(value%100);
            } else {
                sync.transport.mode = match op { 4=>0,5=>1,6=>2,7=>3,8=>5,11=>4,_=>unreachable!() };
                let head = sync.db.mutations().unwrap().into_iter().find(|m|m.status!="failed");
                let result = sync.db.drain_once(&mut sync.transport,now).unwrap();
                match result {
                    Drain::Applied(id) => { prop_assert!(applied.insert(id)); prop_assert_eq!(Some(id),head.map(|r|r.id)); }
                    Drain::Failed(id) => { prop_assert!(failed.insert(id)); }
                    Drain::Retrying(id) => { prop_assert_eq!(Some(id),head.map(|r|r.id)); }
                    _ => (),
                }
            }
            let rows = sync.db.mutations().unwrap();
            let queued:HashSet<_> = rows.iter().map(|r|r.id).collect();
            prop_assert_eq!(queued.len()+applied.len(),enqueued.len());
            for (id,(client,_)) in &enqueued {
                prop_assert_eq!(queued.contains(id),!applied.contains(id));
                if let Some(row) = rows.iter().find(|r|r.id==*id) { prop_assert_eq!(&row.client_id,client); }
            }
            let executions:HashSet<_> = sync.transport.executions.iter().collect();
            prop_assert_eq!(executions.len(),sync.transport.executions.len());
            let positions:HashMap<_,_> = sync.transport.executions.iter().enumerate().map(|(i,c)|(c,i)).collect();
            for entity in ["task:1","task:2","notification:1","notification:2"] {
                let executed:Vec<_> = enqueued.values().filter(|(c,e)| e==entity && positions.contains_key(c)).map(|(c,_)|positions[c]).collect();
                prop_assert!(executed.windows(2).all(|p|p[0]<p[1]));
            }
        }
        sync.transport.mode = 0;
        loop {
            now += 1000;
            match sync.db.drain_once(&mut sync.transport,now).unwrap() {
                Drain::Empty => break,
                Drain::Applied(id) => { prop_assert!(applied.insert(id)); }
                other => prop_assert!(false,"unexpected final drain {other:?}"),
            }
        }
        sync.refetch_pending().unwrap();
        for id in 1..=2 {
            let local = sync.db.task(id).unwrap().unwrap();
            let server = &sync.transport.tasks[&id];
            prop_assert_eq!(local.title.as_str(),server["title"].as_str().unwrap());
            prop_assert_eq!(local.section_id,server["sectionId"].as_i64());
            let comments = sync.db.task_detail(id).unwrap().unwrap().comments;
            let ids:HashSet<_> = comments.iter().map(|c|c.id).collect();
            let expected:HashSet<_> = sync.transport.comments.iter().filter(|(_,c)|c["taskId"]==id).map(|(&id,_)|id).collect();
            prop_assert_eq!(comments.len(),ids.len());
            prop_assert_eq!(ids,expected);
        }
        let local:HashSet<_> = sync.db.inbox().unwrap().iter().map(|n|n.id).collect();
        let expected:HashSet<_> = sync.transport.notifications.iter().filter(|(_,n)|n["status"]=="Normal").map(|(&id,_)|id).collect();
        prop_assert_eq!(local,expected);
        prop_assert_eq!(sync.db.mutations().unwrap().iter().map(|m|m.id).collect::<HashSet<_>>(),failed);
        prop_assert_eq!(applied.len()+sync.db.mutations().unwrap().len(),enqueued.len());
        for id in applied { prop_assert!(sync.transport.responses.contains_key(&enqueued[&id].0)); }
        for client in &sync.transport.executions { prop_assert!(enqueued.values().any(|(c,_)|c==client)); }
    }

    #[test]
    fn prop_retry_backoff_never_reorders_edits(failures in 0usize..12,titles in prop::collection::vec("[a-z]{1,20}",1..20)) {
        let mut sync = setup();
        let mut now = 1;
        for title in &titles { sync.db.enqueue(Write::TaskTitle{task_id:1,title:title.clone()},now).unwrap(); }
        let first = sync.db.mutations().unwrap()[0].id;
        sync.transport.mode = 1;
        for attempt in 0..failures {
            prop_assert_eq!(sync.db.drain_once(&mut sync.transport,now).unwrap(),Drain::Retrying(first));
            let row = &sync.db.mutations().unwrap()[0];
            prop_assert_eq!(row.attempts,attempt as u32+1);
            prop_assert_eq!(row.next_attempt_at-now,ht_core::queue::backoff(attempt as u32));
            prop_assert_eq!(sync.db.drain_once(&mut sync.transport,now).unwrap(),Drain::Waiting(row.next_attempt_at));
            now = row.next_attempt_at;
        }
        sync.transport.mode = 0;
        while !matches!(sync.db.drain_once(&mut sync.transport,now).unwrap(),Drain::Empty) {}
        sync.refetch_pending().unwrap();
        prop_assert_eq!(sync.db.task(1).unwrap().unwrap().title,titles.last().unwrap().clone());
        prop_assert_eq!(sync.transport.tasks[&1]["title"].as_str().unwrap(),titles.last().unwrap());
        prop_assert_eq!(sync.transport.executions.len(),titles.len());
    }
}
