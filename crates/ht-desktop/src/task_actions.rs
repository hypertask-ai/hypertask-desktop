use super::*;
use ht_core::TaskAction;
use serde_json::Value;

#[derive(Default)]
pub(super) struct TaskActions {
    pending: Option<(String, i64, i64, TaskAction)>,
}
impl Desktop {
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
        match self.db.enqueue(
            Write::TaskAction {
                task_id: task,
                change: change.clone(),
            },
            chrono::Utc::now().timestamp(),
        ) {
            Ok(id) => {
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
            self.error = result["error"].as_str().map(str::to_owned);
            self.reload();
            if self.error.is_none() && self.selected_task() == Some(task) {
                match change {
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
            let (expected, request) = self.1.request(1, 42).unwrap();
            assert_eq!(path, expected);
            assert_eq!(*body, request);
            assert!(!key.is_empty());
            let reply = match &self.1 {
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
            (Some("Weekdays"), "Every weekday"),
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
}
