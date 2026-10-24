use super::*;
use ht_core::TaskAction;
use serde_json::Value;

#[derive(Default)]
pub(super) struct TaskActions {
    pending: Option<(String, i64, i64, TaskAction)>,
    pub relations: Vec<Value>,
    pub search_loading: bool,
    pub search_failed: bool,
    search_generation: u64,
    loaded: Option<(i64, i64)>,
    load_generation: u64,
    clear_waiting: bool,
    waiting_ticking: bool,
}
fn waiting_age(value: Option<&str>, now: i64) -> String {
    let Some(start) = value.and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
    else {
        return String::new();
    };
    let elapsed = (now - start.timestamp()).max(0);
    if elapsed < 3600 {
        format!("{}m", (elapsed / 60).max(1))
    } else if elapsed < 86400 {
        format!("{}h", elapsed / 3600)
    } else {
        format!("{}d", elapsed / 86400)
    }
}
impl Desktop {
    pub(super) fn tick_waiting_on(&mut self, cx: &mut Context<Self>) {
        if self.task_actions.waiting_ticking
            || self.screen != Screen::Task
            || self.detail.as_ref().is_none_or(|detail| {
                detail
                    .task
                    .extra
                    .get("waitingOnUserId")
                    .and_then(Value::as_i64)
                    .is_none()
            })
        {
            return;
        }
        self.task_actions.waiting_ticking = true;
        cx.spawn(async move |view, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_secs(60))
                    .await;
                if !view
                    .update(cx, |this, cx| {
                        let active = this.screen == Screen::Task
                            && this.detail.as_ref().is_some_and(|detail| {
                                detail
                                    .task
                                    .extra
                                    .get("waitingOnUserId")
                                    .and_then(Value::as_i64)
                                    .is_some()
                            });
                        if !active {
                            this.task_actions.waiting_ticking = false;
                        }
                        cx.notify();
                        active
                    })
                    .unwrap_or(false)
                {
                    break;
                }
            }
        })
        .detach();
    }

    pub(super) fn waiting_on_row(&self, task: &TaskView, cx: &mut Context<Self>) -> AnyElement {
        let Some(user) = task
            .extra
            .get("waitingOnUserId")
            .and_then(Value::as_i64)
            .filter(|id| *id > 0)
        else {
            return div().into_any_element();
        };
        let name = self
            .db
            .project_members(task.project_id)
            .unwrap_or_default()
            .iter()
            .find(|member| member["id"] == user)
            .and_then(|member| member["displayName"].as_str())
            .unwrap_or("Unknown user")
            .to_owned();
        let started = task.extra.get("waitingOnSetAt").and_then(Value::as_str);
        let now = chrono::Utc::now().timestamp();
        let age = waiting_age(started, now);
        let overdue = started
            .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
            .is_some_and(|date| now - date.timestamp() > 86400);
        let task_id = task.id;
        div()
            .mt(px(24.))
            .flex()
            .items_start()
            .gap(px(20.))
            .child(div().w(px(260. / 3.)).flex_shrink_0().child("Waiting on"))
            .child(
                div()
                    .id("task-waiting-on")
                    .flex_1()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.show_modal(interactions::Modal::WaitingOn, window, cx)
                    }))
                    .child(div().child(name.clone()).layout_key(
                        &self.layout,
                        "task.waiting-on.user",
                        Some(name),
                    ))
                    .when(!age.is_empty(), |d| {
                        d.child(
                            div()
                                .text_color(color(if overdue {
                                    0xcf222e
                                } else {
                                    self.colors.muted
                                }))
                                .child(format!("· {age}")),
                        )
                    })
                    .child(
                        div()
                            .id("clear-task-waiting-on")
                            .cursor_pointer()
                            .child("✕")
                            .on_click(cx.listener(move |this, _, window, cx| {
                                cx.stop_propagation();
                                if this.task_actions.pending.is_some() {
                                    return;
                                }
                                this.queue_task_action(
                                    task_id,
                                    TaskAction::WaitingOn(None),
                                    window,
                                    cx,
                                );
                                this.task_actions.clear_waiting =
                                    this.task_actions.pending.is_some();
                            }))
                            .layout_key(&self.layout, "task.waiting-on.clear", None),
                    ),
            )
            .into_any_element()
    }

    pub(super) fn load_task_relations(&mut self, cx: &mut Context<Self>) {
        if self.options.offline
            || self.screen != Screen::Task
            || self.task_actions.pending.is_some()
        {
            return;
        }
        let (Some(task), Some(user), Some(token)) = (
            self.selected_task(),
            self.db.current_user_id(),
            self.cli_token(),
        ) else {
            return;
        };
        if self.task_actions.loaded == Some((user, task)) {
            return;
        }
        self.task_actions.loaded = Some((user, task));
        self.task_actions.load_generation += 1;
        let generation = self.task_actions.load_generation;
        let expected = token.clone();
        let (send, receive) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let result = ht_core::Http::new(token).and_then(|mut http| {
                let identity = ht_core::Transport::get(&mut http, "/user/context")?;
                anyhow::ensure!(
                    identity.status == 200 && identity.body["user"]["id"] == user,
                    "Unauthorized"
                );
                ht_core::task_relations::details(&mut http, task)
            });
            let _ = send.send(result);
        });
        cx.spawn(async move |view, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(50))
                    .await;
                match receive.try_recv() {
                    Ok(result) => {
                        let _ = view.update(cx, |this, cx| {
                            if this.cli_token().as_deref() != Some(expected.as_str()) {
                                return;
                            }
                            this.apply_task_action_details(
                                generation,
                                user,
                                task,
                                result.map_err(|error| error.to_string()),
                                cx,
                            );
                        });
                        break;
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => (),
                    _ => break,
                }
            }
        })
        .detach();
    }
    fn apply_task_action_details(
        &mut self,
        generation: u64,
        user: i64,
        task: i64,
        result: Result<Value, String>,
        cx: &mut Context<Self>,
    ) {
        if self.task_actions.load_generation != generation
            || self.db.current_user_id() != Some(user)
        {
            return;
        }
        match result.and_then(|source| {
            self.db
                .cache_task_relations(user, task, &source)
                .map_err(|error| error.to_string())
        }) {
            Ok(()) => self.reload(),
            Err(error) => self.error = Some(error),
        }
        cx.notify();
    }
    pub(super) fn task_relations_rows(
        &self,
        task: &TaskView,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let mut groups = std::collections::BTreeMap::<&str, Vec<(Value, Value)>>::new();
        for (field, from, other) in [
            ("relatedFromTasks", true, "targetTask"),
            ("relatedToTasks", false, "sourceTask"),
        ] {
            for relation in task
                .extra
                .get(field)
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                let Some(kind) = serde_json::from_value::<ht_core::RelationKind>(
                    relation["relationType"].clone(),
                )
                .ok() else {
                    continue;
                };
                if relation[other].is_object() {
                    groups
                        .entry(kind.section(from))
                        .or_default()
                        .push((relation.clone(), relation[other].clone()));
                }
            }
        }
        let mut content = div().flex().flex_col().gap(px(8.));
        for title in [
            "Blocked by",
            "Blocks",
            "Duplicate of",
            "Duplicate",
            "Related",
        ] {
            let Some(rows) = groups.remove(title) else {
                continue;
            };
            let mut group = div().mt(px(16.)).flex().flex_col().gap(px(8.)).child(title);
            for (relation, other) in rows {
                let id = relation["id"].as_i64().unwrap_or(0);
                let task_id = task.id;
                let target = other["id"].as_i64().unwrap_or(0);
                group = group.child(
                    div()
                        .group("task-relation")
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .min_h(px(20.))
                        .child(
                            div()
                                .id(SharedString::from(format!("task-relation-{id}")))
                                .flex_1()
                                .cursor_pointer()
                                .child(format!(
                                    "{} {}",
                                    other["ticketNumber"].as_str().unwrap_or("").to_uppercase(),
                                    other["title"].as_str().unwrap_or("")
                                ))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.open_task(target, cx);
                                }))
                                .layout_key(&self.layout, format!("task.relation.{id}"), None),
                        )
                        .when(id > 0, |d| {
                            d.child(
                                div()
                                    .id(SharedString::from(format!("remove-relation-{id}")))
                                    .cursor_pointer()
                                    .opacity(0.)
                                    .group_hover("task-relation", |d| d.opacity(1.))
                                    .child("×")
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        cx.stop_propagation();
                                        this.queue_task_action(
                                            task_id,
                                            TaskAction::RemoveRelation(id),
                                            window,
                                            cx,
                                        );
                                    }))
                                    .layout_key(
                                        &self.layout,
                                        format!("task.relation.remove.{id}"),
                                        None,
                                    ),
                            )
                        }),
                );
            }
            content = content.child(group);
        }
        content.into_any_element()
    }

    pub(super) fn search_relation_tasks(&mut self, cx: &mut Context<Self>) {
        self.task_actions.search_generation += 1;
        let generation = self.task_actions.search_generation;
        self.task_actions.relations.clear();
        self.task_actions.search_failed = false;
        self.task_actions.search_loading = false;
        let Some(task) = self.selected_task() else {
            return;
        };
        let query = self.interaction.query.read(cx).content.trim().to_owned();
        if self.options.offline {
            self.task_actions.relations = self
                .db
                .search_tasks(&query, None)
                .unwrap_or_default()
                .into_iter()
                .filter(|row| row.id != task)
                .filter_map(|row| serde_json::to_value(row).ok())
                .collect();
            return;
        }
        let Some(user) = self.db.current_user_id() else {
            self.task_actions.search_failed = true;
            return;
        };
        let Some(token) = self.cli_token() else {
            self.task_actions.search_failed = true;
            return;
        };
        self.task_actions.search_loading = true;
        cx.spawn(async move |view, cx| {
            if !query.is_empty() {
                cx.background_executor()
                    .timer(Duration::from_millis(150))
                    .await;
            }
            if !view
                .update(cx, |this, _| {
                    this.task_actions.search_generation == generation
                        && matches!(
                            this.interaction.modal,
                            Some(interactions::Modal::Relation(_))
                        )
                })
                .unwrap_or(false)
            {
                return;
            }
            let expected = token.clone();
            let (send, receive) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                let result = ht_core::Http::new(token).and_then(|mut http| {
                    let identity = ht_core::Transport::get(&mut http, "/user/context")?;
                    anyhow::ensure!(
                        identity.status == 200 && identity.body["user"]["id"] == user,
                        "Unauthorized"
                    );
                    ht_core::task_relations::search(&mut http, task, &query)
                });
                let _ = send.send(result);
            });
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(50))
                    .await;
                match receive.try_recv() {
                    Ok(result) => {
                        let _ = view.update(cx, |this, cx| {
                            if this.task_actions.search_generation != generation
                                || !matches!(
                                    this.interaction.modal,
                                    Some(interactions::Modal::Relation(_))
                                )
                                || this.selected_task() != Some(task)
                                || this.db.current_user_id() != Some(user)
                                || this.cli_token().as_deref() != Some(expected.as_str())
                            {
                                return;
                            }
                            this.task_actions.search_loading = false;
                            match result {
                                Ok(rows) => this.task_actions.relations = rows,
                                Err(_) => this.task_actions.search_failed = true,
                            }
                            cx.notify();
                        });
                        break;
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => (),
                    _ => break,
                }
            }
        })
        .detach();
    }
    pub(super) fn queue_task_action(
        &mut self,
        task: i64,
        change: TaskAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.task_actions.pending.is_some() {
            return;
        }
        self.task_actions.clear_waiting = false;
        match self.db.enqueue(
            Write::TaskAction {
                task_id: task,
                change: change.clone(),
            },
            chrono::Utc::now().timestamp(),
        ) {
            Ok(id) => {
                // An older detail read must not replace this write after its queue row resolves.
                self.task_actions.load_generation += 1;
                self.task_actions.loaded = None;
                let row = self
                    .db
                    .mutations()
                    .unwrap()
                    .into_iter()
                    .find(|row| row.id == id)
                    .unwrap();
                self.task_actions.pending = Some((
                    row.client_id,
                    self.db.current_user_id().unwrap_or(0),
                    task,
                    change,
                ));
                self.error = None;
                self.reload();
                self.request_sync();
                if !self.options.offline {
                    cx.spawn_in(window, async move |view, cx| {
                        loop {
                            cx.background_executor()
                                .timer(Duration::from_millis(100))
                                .await;
                            if !view
                                .update_in(cx, |this, window, cx| this.poll_task_action(window, cx))
                                .unwrap_or(false)
                            {
                                break;
                            }
                        }
                    })
                    .detach();
                }
            }
            Err(error) => self.error = Some(error.to_string()),
        }
        cx.notify();
    }
    pub(super) fn poll_task_action(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let Some((key, user, task, change)) = self.task_actions.pending.clone() else {
            return false;
        };
        let Some(result) = self
            .db
            .kv_get(&format!("task-action-result:{key}"))
            .ok()
            .flatten()
            .and_then(|value| serde_json::from_str::<Value>(&value).ok())
        else {
            return true;
        };
        self.task_actions.pending = None;
        if self.db.current_user_id() == Some(user) {
            self.error = result["error"].as_str().map(|error| {
                if self.task_actions.clear_waiting {
                    "Unable to clear blocked by person".to_owned()
                } else {
                    error.to_owned()
                }
            });
            self.task_actions.clear_waiting = false;
            self.reload();
            if self.error.is_none() && self.selected_task() == Some(task) {
                match change {
                    TaskAction::Relation { relation, .. }
                        if self.interaction.modal
                            == Some(interactions::Modal::Relation(relation)) =>
                    {
                        self.close_modal(window, cx)
                    }
                    TaskAction::StartDate(_) => self.finish_start_date(task, window, cx),
                    TaskAction::WaitingOn(_)
                        if self.interaction.modal == Some(interactions::Modal::WaitingOn) =>
                    {
                        self.close_modal(window, cx)
                    }
                    _ => (),
                }
            }
        }
        cx.notify();
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::{
        shell::tests::{click, draw, measured},
        task_fields::tests::desktop,
    };
    use serde_json::json;

    struct Server(u16, TaskAction);
    impl ht_core::Transport for Server {
        fn get(
            &mut self,
            path: &str,
        ) -> Result<ht_core::http::Response, ht_core::http::TransportError> {
            assert_eq!(path, "/user/context");
            Ok(ht_core::http::Response {
                status: 200,
                body: json!({"success":true,"user":{"id":42}}),
            })
        }
        fn post(
            &mut self,
            path: &str,
            body: &Value,
            key: &str,
        ) -> Result<ht_core::http::Response, ht_core::http::TransportError> {
            self.mutate("POST", path, body, key)
        }
        fn mutate(
            &mut self,
            method: &str,
            path: &str,
            body: &Value,
            key: &str,
        ) -> Result<ht_core::http::Response, ht_core::http::TransportError> {
            assert_eq!(
                method,
                if matches!(self.1, TaskAction::Accept { .. }) {
                    "PUT"
                } else {
                    "POST"
                }
            );
            let (expected, request) = self.1.request(1, 42).unwrap();
            assert_eq!(path, expected);
            assert_eq!(*body, request);
            assert!(!key.is_empty());
            let reply = match &self.1 {
                TaskAction::Relation { target, relation } => {
                    json!([{"id":88,"sourceTaskId":1,"targetTaskId":target["id"],"targetTask":target,"relationType":relation}])
                }
                TaskAction::RemoveRelation(id) => {
                    json!({"status":200,"json":{"id":id,"sourceTaskId":1,"targetTaskId":2}})
                }
                TaskAction::Accept {
                    project,
                    section,
                    title,
                } => {
                    json!({"id":1,"projectId":project,"sectionId":section,"section":title,"ranking":"z"})
                }
                TaskAction::Decline => {
                    json!({"id":1,"status":"Archive","dueDate":null,"archivedAt":"2026-10-01T09:00:00Z"})
                }
                TaskAction::SaveTemplate => {
                    json!({"success":true,"template":{"id":81,"name":"Task"}})
                }
                TaskAction::Recurrence(rule) => json!({"id":1,"recurrence":rule}),
                TaskAction::StartDate(date) => json!({"id":1,"startDate":date}),
                TaskAction::WaitingOn(user) => {
                    json!({"id":1,"waitingOnUserId":user,"waitingOnSetById":user.map(|_|42),"waitingOnSetAt":user.map(|_|"2026-10-01T09:00:00Z")})
                }
                TaskAction::AssignMe => {
                    json!({"body":[{"taskId":1,"userId":42,"user":{"id":42,"displayName":"Me"}}]})
                }
            };
            Ok(ht_core::http::Response {
                status: self.0,
                body: if self.0 == 200 {
                    reply
                } else {
                    json!({"message":"Forbidden"})
                },
            })
        }
    }
    fn setup(
        cx: &mut gpui::TestAppContext,
    ) -> (gpui::WindowHandle<Desktop>, gpui::VisualTestContext) {
        cx.update(crate::input::init);
        let view = cx.open_window(gpui::size(px(1440.), px(900.)), desktop);
        let mut cx = gpui::VisualTestContext::from_window(view.into(), cx);
        view.update(&mut cx,|this,_,_|{this.db.kv_set("members:15",&json!([{"id":42,"displayName":"Me"},{"id":7,"displayName":"Alex"},{"id":"agent","displayName":"Bot"}]).to_string()).unwrap();}).unwrap();
        draw(view, &mut cx);
        (view, cx)
    }
    fn palette(cx: &mut gpui::VisualTestContext, name: &str) {
        cx.simulate_keystrokes("ctrl-k");
        cx.simulate_input(name);
        cx.simulate_keystrokes("enter");
    }
    #[gpui::test]
    fn start_date_palette_sets_and_resets_through_web_queue_and_keeps_refused_editor(
        cx: &mut gpui::TestAppContext,
    ) {
        let (view, mut cx) = setup(cx);
        for status in [401, 403, 200] {
            palette(&mut cx, "Set start date");
            cx.simulate_input("2030-12-31T14:30:00+02:00");
            cx.simulate_keystrokes("enter enter");
            view.update(&mut cx, |this, window, cx| {
                let change = TaskAction::StartDate(Some("2030-12-31T14:30:00+02:00".into()));
                assert_eq!(
                    this.db.task(1).unwrap().unwrap().extra["startDate"],
                    "2030-12-31T14:30:00+02:00"
                );
                assert_eq!(
                    this.db
                        .mutations()
                        .unwrap()
                        .iter()
                        .filter(|row| row.status != "failed")
                        .count(),
                    1
                );
                this.db
                    .drain_once(
                        &mut Server(status, change.clone()),
                        chrono::Utc::now().timestamp() + 1,
                    )
                    .unwrap();
                assert!(!this.poll_task_action(window, cx));
                if status != 200 {
                    assert_eq!(this.error.as_deref(), Some(change.failure()));
                    assert!(this.db.task(1).unwrap().unwrap().extra["startDate"].is_null());
                }
            })
            .unwrap();
            if status != 200 {
                measured(view, &mut cx, "task.due.dialog");
                cx.simulate_keystrokes("escape");
            }
        }
        palette(&mut cx, "Set start date");
        click(view, &mut cx, "due.choice.0");
        view.update(&mut cx, |this, window, cx| {
            this.db
                .drain_once(
                    &mut Server(200, TaskAction::StartDate(None)),
                    chrono::Utc::now().timestamp() + 1,
                )
                .unwrap();
            this.poll_task_action(window, cx);
            assert!(this.db.task(1).unwrap().unwrap().extra["startDate"].is_null());
        })
        .unwrap();
    }
    #[gpui::test]
    fn blocked_by_person_keyboard_and_palette_offer_only_people_and_clear_and_restore_refusal(
        cx: &mut gpui::TestAppContext,
    ) {
        let (view, mut cx) = setup(cx);
        for status in [401, 403, 200] {
            if status == 401 {
                cx.simulate_keystrokes("shift-b");
            } else {
                palette(&mut cx, "Blocked by person");
            }
            cx.simulate_input("Alex");
            cx.simulate_keystrokes("enter enter");
            view.update(&mut cx, |this, window, cx| {
                assert_eq!(
                    this.db.task(1).unwrap().unwrap().extra["waitingOnUserId"],
                    7
                );
                let change = TaskAction::WaitingOn(Some(7));
                this.db
                    .drain_once(
                        &mut Server(status, change.clone()),
                        chrono::Utc::now().timestamp() + 1,
                    )
                    .unwrap();
                this.poll_task_action(window, cx);
                if status != 200 {
                    assert_eq!(this.interaction.modal, Some(interactions::Modal::WaitingOn));
                    assert_eq!(this.error.as_deref(), Some(change.failure()));
                    assert!(this.db.task(1).unwrap().unwrap().extra["waitingOnUserId"].is_null());
                } else {
                    assert!(this.interaction.modal.is_none());
                }
            })
            .unwrap();
            if status != 200 {
                cx.simulate_keystrokes("escape");
            }
        }
        cx.simulate_keystrokes("shift-b");
        cx.simulate_input("Bot");
        cx.simulate_keystrokes("enter");
        view.read_with(&cx, |this, _| {
            assert!(
                this.db
                    .mutations()
                    .unwrap()
                    .iter()
                    .all(|row| row.status == "failed")
            )
        })
        .unwrap();
        cx.simulate_keystrokes("ctrl-a backspace");
        click(view, &mut cx, "command.choice.0");
        view.update(&mut cx, |this, window, cx| {
            this.db
                .drain_once(
                    &mut Server(200, TaskAction::WaitingOn(None)),
                    chrono::Utc::now().timestamp() + 1,
                )
                .unwrap();
            this.poll_task_action(window, cx);
            assert!(this.db.task(1).unwrap().unwrap().extra["waitingOnUserId"].is_null());
        })
        .unwrap();
    }
    #[gpui::test]
    fn assign_to_me_palette_preserves_other_people_and_refuses_without_a_false_success(
        cx: &mut gpui::TestAppContext,
    ) {
        let (view, mut cx) = setup(cx);
        for status in [401, 403, 200] {
            palette(&mut cx, "Assign to me");
            view.update(&mut cx, |this, window, cx| {
                assert_eq!(this.db.task(1).unwrap().unwrap().assignee_count, 1);
                this.db
                    .drain_once(
                        &mut Server(status, TaskAction::AssignMe),
                        chrono::Utc::now().timestamp() + 1,
                    )
                    .unwrap();
                this.poll_task_action(window, cx);
                assert_eq!(
                    this.db.task(1).unwrap().unwrap().assignee_count,
                    if status == 200 { 1 } else { 0 }
                );
                if status != 200 {
                    assert_eq!(this.error.as_deref(), Some("Unable to assign this task"));
                }
            })
            .unwrap();
        }
    }
    #[gpui::test]
    fn repeat_task_palette_matches_all_web_rules_and_clear_with_accept_and_refusal(
        cx: &mut gpui::TestAppContext,
    ) {
        let (view, mut cx) = setup(cx);
        view.update(&mut cx,|this,_,_|{this.db.apply_task(&json!({"id":1,"title":"Needle","projectId":15,"status":"Normal","recurrence":"Monthly"})).unwrap();}).unwrap();
        for (rule, label) in [
            (Some("Daily"), "Every day"),
            (Some("Weekdays"), "Every weekday (Mon–Fri)"),
            (Some("Weekly"), "Every week"),
            (Some("Biweekly"), "Every 2 weeks"),
            (Some("Monthly"), "Every month"),
            (None, "Don't repeat"),
        ] {
            for status in [401, 403, 200] {
                let before = view
                    .read_with(&cx, |this, _| {
                        this.db.task(1).unwrap().unwrap().extra["recurrence"].clone()
                    })
                    .unwrap();
                palette(&mut cx, "Repeat task");
                cx.simulate_input(label);
                let index = view
                    .read_with(&cx, |this, cx| {
                        let rows = this.choices(cx);
                        if rule == Some("Weekly") {
                            assert_eq!(rows[0].name, "Every weekday (Mon–Fri)");
                            assert_eq!(rows[1].name, "Every week");
                        }
                        rows.iter()
                            .position(|row| row.name == label)
                            .expect("exact web recurrence label must be selectable")
                    })
                    .unwrap();
                for _ in 0..index {
                    cx.simulate_keystrokes("down");
                }
                cx.simulate_keystrokes("enter");
                view.update(&mut cx, |this, window, cx| {
                    let change = TaskAction::Recurrence(rule.map(str::to_owned));
                    assert_eq!(
                        this.db.task(1).unwrap().unwrap().extra["recurrence"],
                        json!(rule)
                    );
                    assert!(this.interaction.modal.is_none());
                    this.db
                        .drain_once(
                            &mut Server(status, change.clone()),
                            chrono::Utc::now().timestamp() + 1,
                        )
                        .unwrap();
                    this.poll_task_action(window, cx);
                    assert_eq!(
                        this.db.task(1).unwrap().unwrap().extra["recurrence"],
                        if status == 200 {
                            json!(rule)
                        } else {
                            before.clone()
                        }
                    );
                    if status != 200 {
                        assert_eq!(this.error.as_deref(), Some(change.failure()));
                    }
                })
                .unwrap();
            }
        }
    }
    #[gpui::test]
    fn relation_command_families_search_other_tasks_queue_once_and_restore_both_refusals(
        cx: &mut gpui::TestAppContext,
    ) {
        let (view, mut cx) = setup(cx);
        view.update(&mut cx,|this,_,_|{this.db.apply_task(&json!({"id":2,"projectId":15,"sectionId":10,"title":"Other","ticketNumber":"HT-2","uniqueIndex":2,"status":"Normal"})).unwrap();}).unwrap();
        for kind in [
            ht_core::RelationKind::RelatedTo,
            ht_core::RelationKind::BlockedBy,
            ht_core::RelationKind::BlockedTo,
            ht_core::RelationKind::Duplicate,
        ] {
            for status in [401, 403, 200] {
                view.update(&mut cx,|this,_,_|{this.db.apply_task(&json!({"id":1,"projectId":15,"title":"Needle","status":"Normal","relatedFromTasks":[],"relatedToTasks":[]})).unwrap();}).unwrap();
                palette(&mut cx, kind.title());
                cx.simulate_input("Other");
                cx.simulate_keystrokes("enter enter");
                view.update(&mut cx,|this,window,cx|{
                    let rows=this.db.mutations().unwrap();let pending:Vec<_>=rows.iter().filter(|row|row.status!="failed").collect();assert_eq!(pending.len(),1);
                    let change:TaskAction=serde_json::from_value(pending[0].payload_json["change"].clone()).unwrap();
                    assert_eq!(this.db.task(1).unwrap().unwrap().extra["relatedFromTasks"][0]["targetTaskId"],2);
                    this.db.drain_once(&mut Server(status,change),chrono::Utc::now().timestamp()+1).unwrap();this.poll_task_action(window,cx);
                    if status!=200 {assert_eq!(this.error.as_deref(),Some("Forbidden"));assert_eq!(this.interaction.modal,Some(interactions::Modal::Relation(kind)));assert!(this.db.task(1).unwrap().unwrap().extra["relatedFromTasks"].as_array().unwrap().is_empty());}
                    else {assert!(this.interaction.modal.is_none());assert_eq!(this.db.task(1).unwrap().unwrap().extra["relatedFromTasks"][0]["id"],88);}
                }).unwrap();
                if status != 200 {
                    cx.simulate_keystrokes("escape");
                } else {
                    measured(view, &mut cx, "task.relation.88");
                }
            }
        }
        for status in [401, 403, 200] {
            click(view, &mut cx, "task.relation.remove.88");
            view.update(&mut cx, |this, window, cx| {
                assert!(
                    this.db.task(1).unwrap().unwrap().extra["relatedFromTasks"]
                        .as_array()
                        .unwrap()
                        .is_empty()
                );
                this.db
                    .drain_once(
                        &mut Server(status, TaskAction::RemoveRelation(88)),
                        chrono::Utc::now().timestamp() + 1,
                    )
                    .unwrap();
                this.poll_task_action(window, cx);
                assert_eq!(
                    this.db.task(1).unwrap().unwrap().extra["relatedFromTasks"]
                        .as_array()
                        .unwrap()
                        .is_empty(),
                    status == 200
                );
            })
            .unwrap();
        }
        palette(&mut cx, "Add related task");
        cx.simulate_input("does not exist");
        measured(view, &mut cx, "task.relations.search-status");
        cx.simulate_keystrokes("escape");
    }
    #[test]
    fn waiting_person_age_matches_web_minutes_hours_days_and_future_clamping() {
        for (elapsed, expected) in [
            (-1, "1m"),
            (0, "1m"),
            (60, "1m"),
            (3599, "59m"),
            (3600, "1h"),
            (86399, "23h"),
            (86400, "1d"),
            (172800, "2d"),
        ] {
            assert_eq!(
                waiting_age(
                    Some("2026-10-01T00:00:00Z"),
                    chrono::DateTime::parse_from_rfc3339("2026-10-01T00:00:00Z")
                        .unwrap()
                        .timestamp()
                        + elapsed
                ),
                expected
            );
        }
        assert_eq!(waiting_age(None, 0), "");
        assert_eq!(waiting_age(Some("invalid"), 0), "");
    }
    #[gpui::test]
    fn waiting_person_rail_opens_picker_and_clear_queues_exact_web_write_with_refusal_rollback(
        cx: &mut gpui::TestAppContext,
    ) {
        let (view, mut cx) = setup(cx);
        view.update(&mut cx,|this,_,_|{this.db.apply_task(&json!({"id":1,"projectId":15,"title":"Needle","status":"Normal","waitingOnUserId":7,"waitingOnSetById":42,"waitingOnSetAt":"2026-10-01T09:00:00Z"})).unwrap();this.reload();}).unwrap();
        click(view, &mut cx, "task.waiting-on.user");
        cx.simulate_keystrokes("escape");
        for status in [401, 403, 200] {
            click(view, &mut cx, "task.waiting-on.clear");
            view.update(&mut cx, |this, window, cx| {
                assert!(this.db.task(1).unwrap().unwrap().extra["waitingOnUserId"].is_null());
                this.db
                    .drain_once(
                        &mut Server(status, TaskAction::WaitingOn(None)),
                        chrono::Utc::now().timestamp() + 1,
                    )
                    .unwrap();
                this.poll_task_action(window, cx);
                if status == 200 {
                    assert!(this.db.task(1).unwrap().unwrap().extra["waitingOnUserId"].is_null());
                } else {
                    assert_eq!(
                        this.db.task(1).unwrap().unwrap().extra["waitingOnUserId"],
                        7
                    );
                    assert_eq!(
                        this.error.as_deref(),
                        Some("Unable to clear blocked by person")
                    );
                }
            })
            .unwrap();
        }
    }
    #[gpui::test]
    fn prewrite_task_detail_replies_cannot_overwrite_accepted_or_refused_commands(
        cx: &mut gpui::TestAppContext,
    ) {
        let (view, mut cx) = setup(cx);
        for status in [200, 401, 403] {
            view.update(&mut cx,|this,window,cx|{
                this.db.apply_task(&json!({"id":1,"projectId":15,"title":"Needle","status":"Normal","waitingOnUserId":null})).unwrap();
                let old_generation=this.task_actions.load_generation;
                let change=TaskAction::WaitingOn(Some(7));this.queue_task_action(1,change.clone(),window,cx);
                assert!(this.task_actions.load_generation>old_generation);assert!(this.task_actions.loaded.is_none());
                this.db.drain_once(&mut Server(status,change),chrono::Utc::now().timestamp()+1).unwrap();this.poll_task_action(window,cx);
                let before=this.db.task(1).unwrap().unwrap();let error=this.error.clone();
                this.apply_task_action_details(old_generation,42,1,Ok(json!({"id":1,"relatedFromTasks":[],"relatedToTasks":[],"waitingOnUserId":99})),cx);
                this.apply_task_action_details(old_generation,42,1,Err("Old read refused".into()),cx);
                assert_eq!(this.db.task(1).unwrap().unwrap(),before);assert_eq!(this.error,error);
                this.apply_task_action_details(this.task_actions.load_generation,42,1,Ok(json!({"id":1,"relatedFromTasks":[],"relatedToTasks":[],"recurrence":"Daily"})),cx);
                assert_eq!(this.db.task(1).unwrap().unwrap().extra["recurrence"],"Daily","current-generation read is the positive control");
            }).unwrap();
        }
    }
    #[gpui::test]
    fn triage_and_template_palette_commands_accept_or_restore_refused_web_writes(
        cx: &mut gpui::TestAppContext,
    ) {
        let (view, mut cx) = setup(cx);
        for (name, change) in [
            (
                "Accept task",
                TaskAction::Accept {
                    project: 15,
                    section: 11,
                    title: "Ready".into(),
                },
            ),
            ("Decline task", TaskAction::Decline),
            ("Save task as template", TaskAction::SaveTemplate),
        ] {
            for status in [401, 403, 200] {
                view.update(&mut cx, |this, _, _| {
                    this.db.apply_response(&json!({"success":true,"projects":[{"id":15,"title":"Board"}],"sections":[
                        {"id":10,"projectId":15,"section_title":"Triage","visibility":true,"deleted":false,"ranking":"a"},
                        {"id":11,"projectId":15,"section_title":"Ready","visibility":true,"deleted":false,"ranking":"b"}
                    ]})).unwrap();
                    this.db.apply_task(&json!({"id":1,"projectId":15,"title":"Task","sectionId":10,"section":"Triage","ranking":"a","status":"Normal","dueDate":"2030-01-01T09:00:00Z"})).unwrap();
                    this.reload();
                }).unwrap();
                draw(view, &mut cx);
                palette(&mut cx, name);
                view.update(&mut cx, |this, window, cx| {
                    assert_eq!(
                        this.db
                            .mutations()
                            .unwrap()
                            .iter()
                            .filter(|row| row.status != "failed")
                            .count(),
                        1,
                        "{name}"
                    );
                    let optimistic = this.db.task(1).unwrap().unwrap();
                    if matches!(change, TaskAction::Accept { .. }) {
                        assert_eq!(optimistic.section_id, Some(11));
                    }
                    if matches!(change, TaskAction::Decline) {
                        assert_eq!(optimistic.status, Status::Archive);
                        assert!(optimistic.due_date.is_none());
                    }
                    this.db
                        .drain_once(
                            &mut Server(status, change.clone()),
                            chrono::Utc::now().timestamp() + 1,
                        )
                        .unwrap();
                    assert!(!this.poll_task_action(window, cx));
                    let task = this.db.task(1).unwrap().unwrap();
                    if status != 200 {
                        let expected =
                            if matches!(change, TaskAction::Accept { .. } | TaskAction::Decline) {
                                "Forbidden"
                            } else {
                                change.failure()
                            };
                        assert_eq!(this.error.as_deref(), Some(expected));
                        assert_eq!(task.section_id, Some(10));
                        assert_eq!(task.status, Status::Normal);
                        assert!(task.due_date.is_some());
                    } else {
                        assert!(this.error.is_none());
                        if matches!(change, TaskAction::Accept { .. }) {
                            assert_eq!(task.section_id, Some(11));
                            assert_eq!(task.extra["ranking"], "z");
                        }
                        if matches!(change, TaskAction::Decline) {
                            assert_eq!(task.status, Status::Archive);
                            assert!(
                                !this
                                    .command_rows()
                                    .iter()
                                    .any(|row| row.key == "acceptTask" || row.key == "declineTask")
                            );
                        }
                    }
                })
                .unwrap();
            }
        }
    }
}
