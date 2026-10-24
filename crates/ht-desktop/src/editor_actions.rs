use super::*;
use crate::input::EditorRequest;
use ht_core::DraftType;

impl Desktop {
    pub(super) fn bind_editors(&mut self, cx: &mut Context<Self>) {
        for input in [
            self.comment_input.clone(),
            self.comment_actions.input.clone(),
            self.interaction.description.clone(),
        ] {
            cx.subscribe(&input, |this, input, event, cx| {
                let EditorRequest::Improve { command, html } = event;
                if this.screen != Screen::Task {
                    return;
                }
                let task_id = this.options.task;
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64;
                match this.db.enqueue(
                    Write::EditorImprove {
                        task_id,
                        command: *command,
                        text: html.clone(),
                    },
                    now,
                ) {
                    Ok(id) => {
                        input.update(cx, |input, cx| input.set_editor_job(id, task_id, cx));
                        let key = this.editor_job_key(&input);
                        let _ = this.db.kv_set(
                            &key,
                            &serde_json::json!({"id":id,"source":html}).to_string(),
                        );
                        if let Some(worker) = &this.worker {
                            let _ = worker.handle().wake();
                        }
                    }
                    Err(error) => this.error = Some(error.to_string()),
                }
                cx.notify();
            })
            .detach();
        }
    }
    fn editor_job_key(&self, input: &Entity<TextInput>) -> String {
        let surface = if input == &self.comment_input {
            "comment".into()
        } else if input == &self.interaction.description {
            "description".into()
        } else {
            format!("edit-{}", self.comment_actions.edit_id.unwrap_or(0))
        };
        format!(
            "editor-job:{}:{}:{surface}",
            self.db.current_user_id().unwrap_or(0),
            self.options.task
        )
    }
    pub(super) fn restore_editor(
        &mut self,
        input: Entity<TextInput>,
        draft: Option<DraftType>,
        cx: &mut Context<Self>,
    ) {
        let key = self.editor_job_key(&input);
        if let Some(job) = self
            .db
            .kv_get(&key)
            .ok()
            .flatten()
            .and_then(|v| serde_json::from_str::<serde_json::Value>(&v).ok())
        {
            if let (Some(id), Some(source)) = (job["id"].as_i64(), job["source"].as_str()) {
                input.update(cx, |input, cx| {
                    input.set_html(source, cx);
                    input.set_editor_job(id, self.options.task, cx);
                });
                return;
            }
        }
        if let Some(kind) = draft {
            if let Some(html) = self
                .db
                .editor_drafts(self.options.task)
                .ok()
                .into_iter()
                .flatten()
                .find(|d| d["draftType"] == kind.as_str())
                .and_then(|d| d["text"].as_str().map(str::to_owned))
            {
                input.update(cx, |input, cx| input.set_html(&html, cx));
            }
        }
    }
    pub(super) fn poll_editor_results(&mut self, cx: &mut Context<Self>) {
        for input in [
            self.comment_input.clone(),
            self.comment_actions.input.clone(),
            self.interaction.description.clone(),
        ] {
            let Some((id, task)) = input.read(cx).editor_job() else {
                continue;
            };
            if task != self.options.task {
                continue;
            }
            if let Ok(Some(result)) = self.db.editor_result(id) {
                let key = self.editor_job_key(&input);
                if let Err(error) =
                    input.update(cx, |input, cx| input.apply_editor_result(result, cx))
                {
                    self.error = Some(error);
                }
                let _ = self.db.kv_set(&key, "null");
                let _ = self.db.clear_editor_result(id);
                cx.notify();
            }
        }
    }
    pub(super) fn finish_editor_job(&mut self, cx: &mut Context<Self>) {
        let input = match self.editing {
            Editing::Comment => self.comment_input.clone(),
            Editing::Description => self.interaction.description.clone(),
            Editing::ExistingComment => self.comment_actions.input.clone(),
            _ => return,
        };
        let _ = self.db.kv_set(&self.editor_job_key(&input), "null");
        if let Some((id, _)) = input.read(cx).editor_job() {
            let _ = self.db.cancel_editor_result(id);
        }
        input.update(cx, |input, _| input.clear_editor_job());
    }
    pub(super) fn discard_editor_draft(&mut self, cx: &mut Context<Self>) -> bool {
        let (kind, input) = match self.editing {
            Editing::Comment => (DraftType::Comment, self.comment_input.clone()),
            Editing::Description => (DraftType::Description, self.interaction.description.clone()),
            // The web keyboard handler maps every mode except create-comment to Description.
            Editing::ExistingComment => {
                (DraftType::Description, self.comment_actions.input.clone())
            }
            _ => return false,
        };
        if kind == DraftType::Comment {
            if !self.discard_comment_draft(cx) { return false; }
        } else if !self.enqueue(
            Write::DiscardDraft {
                task_id: self.options.task,
                draft_type: kind,
            },
            cx,
        ) {
            return false;
        }
        let _ = self.db.kv_set(&self.editor_job_key(&input), "null");
        if let Some((id, _)) = input.read(cx).editor_job() {
            let _ = self.db.cancel_editor_result(id);
        }
        input.update(cx, |input, _| input.clear_editor_job());
        true
    }
}
