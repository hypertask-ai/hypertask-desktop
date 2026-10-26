use super::*;

#[derive(Default)]
pub(super) struct TimerShortcut {
    pending: Option<String>,
}
impl Desktop {
    pub(super) fn toggle_task_timer(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.timer_shortcut.pending.is_some() { return; }
        let Some(task) = self.selected_task() else { return; };
        if !self.enqueue(Write::Board { change: ht_core::BoardWrite::ToggleTimer { task } }, cx) { return; }
        if let Ok(rows) = self.db.mutations()
            && let Some(row) = rows.last()
        {
            self.timer_shortcut.pending = Some(row.client_id.clone());

        }
        #[cfg(not(test))]
        cx.spawn_in(window, async move |view, cx| {
            loop {
                cx.background_executor().timer(Duration::from_millis(100)).await;
                if !matches!(view.update(cx, |this, cx| this.poll_timer_shortcut(cx)), Ok(true)) { break; }
            }
        }).detach();
        let _ = window;
    }
    pub(super) fn poll_timer_shortcut(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(id) = self.timer_shortcut.pending.clone() else { return false; };
        match self.db.take_board_result(&id) {
            Ok(Some(result)) => {
                self.timer_shortcut.pending = None;
                if let Some(error) = result["error"].as_str() {
                    self.error = Some(error.into());
                }
                cx.notify();
                false
            }
            Ok(None) => true,
            Err(error) => { self.timer_shortcut.pending = None; self.error = Some(error.to_string()); cx.notify(); false }
        }
    }
    pub(super) fn task_timer_control(&self, task: i64, cx: &mut Context<Self>) -> AnyElement {
        let state = self.db.kv_get(&format!("timer-state:{task}")).ok().flatten().and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok());
        let Some(running) = state.and_then(|v| v["timer_running"].as_bool()) else { return div().into_any_element(); };
        let text = if running { "Stop" } else { "Start" };
        div().flex().gap(px(16.)).py(px(8.)).child("Time")
            .child(div().id("task-timer-toggle").cursor_pointer().child(text)
                .on_click(cx.listener(|this, _, window, cx| this.toggle_task_timer(window, cx)))
                .layout_key(&self.layout, "task.timer.control", Some(text.into())))
            .into_any_element()
    }
}
