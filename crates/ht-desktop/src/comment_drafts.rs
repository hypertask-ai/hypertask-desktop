use super::*;

#[derive(Default)]
pub(super) struct CommentDrafts {
    task: Option<i64>,
    user: Option<i64>,
    text: String,
    html: String,
}

impl Desktop {
    pub(super) fn restore_comment_draft(&mut self, cx: &mut Context<Self>) {
        if self.screen != Screen::Task {
            return;
        }
        let task = self.options.task;
        let user = self.db.current_user_id();
        let html = self
            .db
            .comment_draft(task)
            .ok()
            .flatten()
            .unwrap_or_default();
        if self.comment_drafts.task == Some(task)
            && self.comment_drafts.user == user
            && self.comment_drafts.html == html
        {
            return;
        }
        let text = RichText::parse(&html, self.colors).text;
        self.comment_drafts.task = Some(task);
        self.comment_drafts.user = user;
        self.comment_drafts.text = text.clone();
        self.comment_drafts.html = html.clone();
        self.comment_input
            .update(cx, |input, cx| input.set_html(&html, cx));
    }

    pub(super) fn comment_draft_body(&self, cx: &App) -> String {
        self.comment_input.read(cx).html()
    }

    pub(super) fn observe_comment_draft(&mut self, cx: &mut Context<Self>) {
        if self.screen != Screen::Task || self.comment_drafts.task != Some(self.options.task) {
            return;
        }
        let text = self.comment_input.read(cx).content.to_string();
        let html = if text.is_empty() {
            String::new()
        } else {
            self.comment_body_html(cx)
        };
        if html == self.comment_drafts.html {
            return;
        }
        self.comment_drafts.text = text;
        self.comment_drafts.html = html.clone();
        let task = self.options.task;
        let user = self.db.current_user_id();
        if let Err(error) = self.db.save_local_comment_draft(task, &html) {
            self.error = Some(error.to_string());
        }
        cx.spawn(async move |view, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(750))
                .await;
            let _ = view.update(cx, |this, cx| {
                // Freeze ownership/task at edit time; navigation must not retarget an autosave.
                if this.db.current_user_id() != user || user.is_none() {
                    return;
                }
                // A successful send/discard or a later edit may have replaced this snapshot.
                if this.db.comment_draft(task).ok().flatten().as_deref() != Some(html.as_str()) {
                    return;
                }
                this.enqueue(
                    Write::CommentDraft {
                        task_id: task,
                        text: (!html.is_empty()).then_some(html),
                    },
                    cx,
                );
            });
        })
        .detach();
        cx.notify();
    }

    pub(super) fn comment_draft_sent(&mut self) {
        self.comment_drafts.text.clear();
        self.comment_drafts.html.clear();
    }

    pub(super) fn discard_comment_draft(&mut self, cx: &mut Context<Self>) -> bool {
        self.comment_draft_sent();
        if self.db.current_user_id().is_some() {
            return self.enqueue(
                Write::CommentDraft {
                    task_id: self.options.task,
                    text: None,
                },
                cx,
            );
        } else if let Err(error) = self.db.save_local_comment_draft(self.options.task, "") {
            self.error = Some(error.to_string());
            return false;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[gpui::test]
    fn comment_autosave_debounces_and_freezes_task_identity_before_navigation(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(crate::input::init);
        let (view, cx) = cx.add_window_view(|window, cx| {
            let mut db = Db::memory().unwrap();
            db.kv_set("current-user-id", "42").unwrap();
            for id in [1, 2] {
                db.apply_task(&json!({"id":id,"projectId":15,"title":"Task","status":"Normal"}))
                    .unwrap();
            }
            Desktop::new(
                db,
                Options::parse(["--offline", "--screen", "task", "--task", "1"].map(String::from))
                    .unwrap(),
                window,
                cx,
            )
        });
        cx.run_until_parked();
        view.update_in(cx, |this, window, cx| this.edit_comment(window, cx));
        cx.simulate_input("First");
        cx.executor().advance_clock(Duration::from_millis(500));
        cx.run_until_parked();
        view.read_with(cx, |this, _| {
            assert!(this.db.mutations().unwrap().is_empty())
        });
        cx.simulate_input(" latest");
        view.update_in(cx, |this, _, cx| {
            this.open_task(2, cx);
            this.restore_comment_draft(cx);
        });
        cx.executor().advance_clock(Duration::from_millis(250));
        cx.run_until_parked();
        view.read_with(cx, |this, _| {
            assert!(this.db.mutations().unwrap().is_empty())
        });
        cx.executor().advance_clock(Duration::from_millis(500));
        cx.run_until_parked();
        view.read_with(cx, |this, cx| {
            let rows = this.db.mutations().unwrap();
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].kind, "comment_draft");
            assert_eq!(rows[0].payload_json["task_id"], 1);
            assert!(
                rows[0].payload_json["text"]
                    .as_str()
                    .unwrap()
                    .contains("First latest")
            );
            assert!(this.comment_input.read(cx).content.is_empty());
        });
    }

    #[gpui::test]
    fn comment_drafts_restore_per_task_and_do_not_leak_between_users(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(crate::input::init);
        let (view, cx) = cx.add_window_view(|window, cx| {
            let mut db = Db::memory().unwrap();
            db.kv_set("current-user-id", "42").unwrap();
            for id in [1, 2] {
                db.apply_task(&json!({"id":id,"projectId":15,"title":"Task","status":"Normal"}))
                    .unwrap();
            }
            let options =
                Options::parse(["--offline", "--screen", "task", "--task", "1"].map(String::from))
                    .unwrap();
            Desktop::new(db, options, window, cx)
        });
        cx.run_until_parked();
        view.update_in(cx, |this, window, cx| this.edit_comment(window, cx));
        cx.simulate_input("Draft é🙂");
        cx.run_until_parked();
        view.update_in(cx, |this, _, cx| {
            assert!(
                this.db
                    .comment_draft(1)
                    .unwrap()
                    .unwrap()
                    .contains("Draft é🙂")
            );
            this.open_task(2, cx);
            this.restore_comment_draft(cx);
            assert!(this.comment_input.read(cx).content.is_empty());
            this.open_task(1, cx);
            this.restore_comment_draft(cx);
            assert_eq!(this.comment_input.read(cx).content.trim(), "Draft é🙂");
            this.db.kv_set("current-user-id", "43").unwrap();
            this.restore_comment_draft(cx);
            assert!(this.comment_input.read(cx).content.is_empty());
            this.db.kv_set("current-user-id", "42").unwrap();
            this.restore_comment_draft(cx);
            assert_eq!(this.comment_input.read(cx).content.trim(), "Draft é🙂");
        });
        view.update_in(cx, |this, window, cx| {
            this.run_shortcut(crate::shortcuts::Action::DiscardDraft, window, cx);
            assert!(this.comment_input.read(cx).content.is_empty());
            assert_eq!(this.db.comment_draft(1).unwrap().as_deref(), Some(""));
            assert_eq!(
                this.db.mutations().unwrap().last().unwrap().kind,
                "comment_draft"
            );
        });
    }
}

#[cfg(test)]
mod rich_draft_tests {
    use super::*;
    use serde_json::json;

    #[gpui::test]
    fn mark_only_edits_autosave_and_restore_as_rich_html(cx: &mut gpui::TestAppContext) {
        cx.update(crate::input::init);
        let (view, cx) = cx.add_window_view(|window, cx| {
            let mut db = Db::memory().unwrap();
            db.kv_set("current-user-id", "42").unwrap();
            for id in [1, 2] {
                db.apply_task(&json!({"id":id,"projectId":15,"title":"Task","status":"Normal"})).unwrap();
            }
            db.save_local_comment_draft(1, "<p>Draft é🙂</p>").unwrap();
            Desktop::new(db, Options::parse(["--offline", "--screen", "task", "--task", "1"].map(String::from)).unwrap(), window, cx)
        });
        cx.run_until_parked();
        view.update_in(cx, |this, window, cx| this.edit_comment(window, cx));
        cx.simulate_keystrokes("ctrl-a ctrl-b");
        cx.run_until_parked();
        view.read_with(cx, |this, cx| {
            assert_eq!(this.comment_input.read(cx).html(), "<p><strong>Draft é🙂</strong></p>");
            assert_eq!(this.db.comment_draft(1).unwrap().as_deref(), Some("<p><strong>Draft é🙂</strong></p>"));
        });
        cx.executor().advance_clock(Duration::from_millis(750));
        cx.run_until_parked();
        view.update_in(cx, |this, _, cx| {
            let rows = this.db.mutations().unwrap();
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].kind, "comment_draft");
            assert_eq!(rows[0].payload_json["text"], "<p><strong>Draft é🙂</strong></p>");
            this.open_task(2, cx);
            this.restore_comment_draft(cx);
            assert!(this.comment_input.read(cx).is_empty());
            this.open_task(1, cx);
            this.restore_comment_draft(cx);
            assert_eq!(this.comment_input.read(cx).html(), "<p><strong>Draft é🙂</strong></p>");
        });
    }
}
