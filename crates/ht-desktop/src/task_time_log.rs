use super::*;
use chrono::{Datelike, Local, NaiveDate, TimeZone};
use gpui::MouseButton;
use ht_core::{TimeEntryChange, TimeWrite};
use serde_json::{Value, json};

struct Form {
    date: Entity<TextInput>,
    hours: Entity<TextInput>,
    minutes: Entity<TextInput>,
    note: Entity<TextInput>,
}
impl Form {
    fn new(cx: &mut Context<Desktop>) -> Self {
        let date = cx.new(|cx| {
            TextInput::new(
                Local::now().format("%Y-%m-%d").to_string(),
                "YYYY-MM-DD",
                color(0x8e9093),
                cx,
            )
        });
        let hours = cx.new(|cx| TextInput::new(String::new(), "0", color(0x8e9093), cx));
        let minutes = cx.new(|cx| TextInput::new(String::new(), "0", color(0x8e9093), cx));
        let note = cx.new(|cx| {
            TextInput::new(
                String::new(),
                "What did you do? (optional)",
                color(0x8e9093),
                cx,
            )
            .multiline()
        });
        for input in [&date, &hours, &minutes, &note] {
            cx.observe(input, |_, _, cx| cx.notify()).detach();
        }
        Self {
            date,
            hours,
            minutes,
            note,
        }
    }
    fn values(&self, cx: &App) -> anyhow::Result<(String, u16, i32, String)> {
        let date = self.date.read(cx).content.trim().to_owned();
        let day = NaiveDate::parse_from_str(&date, "%Y-%m-%d")
            .map_err(|_| anyhow::anyhow!("Pick a date"))?;
        let duration = |input: &Entity<TextInput>| {
            input
                .read(cx)
                .content
                .trim()
                .parse::<u16>()
                .or_else(|_| {
                    if input.read(cx).content.trim().is_empty() {
                        Ok(0)
                    } else {
                        Err(())
                    }
                })
                .map_err(|_| anyhow::anyhow!("Enter a duration from 1 minute to 24 hours"))
        };
        let hours = duration(&self.hours)?;
        let minutes = duration(&self.minutes)?;
        anyhow::ensure!(minutes <= 59, "Enter a duration from 1 minute to 24 hours");
        let total = hours
            .checked_mul(60)
            .and_then(|h| h.checked_add(minutes))
            .filter(|n| (1..=1440).contains(n))
            .ok_or_else(|| anyhow::anyhow!("Enter a duration from 1 minute to 24 hours"))?;
        let noon = Local
            .from_local_datetime(&day.and_hms_opt(12, 0, 0).unwrap())
            .earliest()
            .ok_or_else(|| anyhow::anyhow!("Pick a date"))?;
        let note = self.note.read(cx).content.to_string();
        anyhow::ensure!(
            note.encode_utf16().count() <= 500,
            "Note must be at most 500 characters"
        );
        Ok((date, total, -noon.offset().local_minus_utc() / 60, note))
    }
}
pub(super) struct TimeLog {
    open: bool,
    task: i64,
    user: i64,
    generation: u64,
    loading: bool,
    error: Option<String>,
    add: Form,
    edit: Form,
    editing: Option<i64>,
    pending: Option<TimeEntryChange>,
    range: usize,
    range_open: bool,
    users_open: bool,
    users: std::collections::BTreeSet<i64>,
}
impl TimeLog {
    pub fn new(cx: &mut Context<Desktop>) -> Self {
        Self {
            open: false,
            task: 0,
            user: 0,
            generation: 0,
            loading: false,
            error: None,
            add: Form::new(cx),
            edit: Form::new(cx),
            editing: None,
            pending: None,
            range: 2,
            range_open: false,
            users_open: false,
            users: Default::default(),
        }
    }
}
fn seconds(row: &Value, now: i64) -> i64 {
    if row["endedAt"].is_null() && row["pausedAt"].is_null() {
        ht_core::task_time::elapsed(row, now)
    } else {
        row["seconds"].as_i64().unwrap_or(0)
    }
}
fn day(row: &Value) -> Option<NaiveDate> {
    Some(
        chrono::DateTime::parse_from_rfc3339(row["startedAt"].as_str()?)
            .ok()?
            .with_timezone(&Local)
            .date_naive(),
    )
}
impl Desktop {
    pub(super) fn invalidate_time_log(&mut self, cx: &mut Context<Self>) {
        let generation = self.time_log.generation + 1;
        self.time_log = TimeLog::new(cx);
        self.time_log.generation = generation;
    }
    pub(super) fn open_time_log(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.screen != Screen::Task {
            return;
        }
        let Some(task) = self.selected_task() else {
            return;
        };
        if self
            .db
            .task_time(task)
            .ok()
            .flatten()
            .is_none_or(|s| s["enabled"] != true)
        {
            return;
        }
        if self.time_log.pending.is_some() {
            return;
        }
        let generation = self.time_log.generation + 1;
        self.time_log = TimeLog::new(cx);
        self.time_log.generation = generation;
        self.time_log.open = true;
        self.time_log.task = task;
        self.time_log.user = self.db.current_user_id().unwrap_or(0);
        window.focus(&self.time_log.add.hours.focus_handle(cx), cx);
        self.load_time_log(cx);
        cx.notify();
    }
    fn load_time_log(&mut self, cx: &mut Context<Self>) {
        if !self.time_log.open || self.options.offline {
            return;
        }
        let Some(token) = self.cli_token() else {
            self.time_log.error = Some("Unauthorized".into());
            return;
        };
        self.time_log.generation += 1;
        self.time_log.loading = true;
        let (generation, user, task) = (
            self.time_log.generation,
            self.time_log.user,
            self.time_log.task,
        );
        let expected = token.clone();
        let (send, receive) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let result = ht_core::Http::new(token)
                .and_then(|mut http| {
                    let identity = ht_core::Transport::get(&mut http, "/user/context")?;
                    anyhow::ensure!(
                        identity.status == 200 && identity.body["user"]["id"] == user,
                        "Unauthorized"
                    );
                    ht_core::time_entries::load(&mut http, task)
                })
                .map_err(|e| e.to_string());
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
                            if this.time_log.generation != generation
                                || this.time_log.pending.is_some()
                                || this.db.current_user_id() != Some(user)
                                || this.cli_token().as_deref() != Some(expected.as_str())
                            {
                                return;
                            }
                            this.time_log.loading = false;
                            match result.and_then(|value| {
                                this.db
                                    .cache_time_entries(user, task, &value)
                                    .map_err(|e| e.to_string())
                            }) {
                                Ok(()) => this.time_log.error = None,
                                Err(error) => this.time_log.error = Some(error),
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
    fn close_time_log(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.time_log.open = false;
        self.time_log.generation += 1;
        window.focus(&self.focus, cx);
        cx.notify();
    }
    fn submit_time_entry(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.time_log.pending.is_some()
            || self.time_log.user != self.db.current_user_id().unwrap_or(0)
        {
            return;
        }
        let result = (if self.time_log.editing.is_some() {
            &self.time_log.edit
        } else {
            &self.time_log.add
        })
        .values(cx);
        let (date, minutes, offset, note) = match result {
            Ok(v) => v,
            Err(e) => {
                self.time_log.error = Some(e.to_string());
                cx.notify();
                return;
            }
        };
        let change = if let Some(entry) = self.time_log.editing {
            let original = self
                .db
                .time_entries(self.time_log.task)
                .ok()
                .flatten()
                .and_then(|value| {
                    value["entries"]
                        .as_array()?
                        .iter()
                        .find(|row| row["id"] == entry)
                        .and_then(day)
                });
            TimeEntryChange::Update {
                entry,
                date: if original.map(|d| d.to_string()).as_deref() == Some(date.as_str()) {
                    None
                } else {
                    Some(date)
                },
                minutes,
                offset,
                note,
            }
        } else {
            TimeEntryChange::Create {
                date,
                minutes,
                offset,
                note,
            }
        };
        self.write_time_entry(change, window, cx);
    }
    fn write_time_entry(
        &mut self,
        change: TimeEntryChange,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.time_log.pending.is_some() || self.task_time.pending.is_some() {
            return;
        }
        self.queue_time(
            TimeWrite::Entry {
                task: self.time_log.task,
                change: change.clone(),
            },
            window,
            cx,
        );
        if self.task_time.pending.is_some() {
            self.time_log.pending = Some(change);
            self.time_log.generation += 1;
            self.time_log.loading = false;
            self.time_log.error = None;
        } else {
            self.time_log.error = self.error.clone();
        }
        cx.notify();
    }
    pub(super) fn finish_time_log(&mut self, result: &Value, cx: &mut Context<Self>) {
        let Some(change) = self.time_log.pending.take() else {
            return;
        };
        self.time_log.error = result["error"].as_str().map(str::to_owned);
        if self.time_log.error.is_none() {
            match change {
                TimeEntryChange::Create { .. } => {
                    for input in [
                        &self.time_log.add.hours,
                        &self.time_log.add.minutes,
                        &self.time_log.add.note,
                    ] {
                        input.update(cx, |input, cx| input.set_content(String::new(), cx));
                    }
                }
                TimeEntryChange::Update { .. } => self.time_log.editing = None,
                _ => (),
            }
            self.load_time_log(cx);
        }
    }
    fn edit_time_entry(&mut self, row: Value, window: &mut Window, cx: &mut Context<Self>) {
        if self.time_log.pending.is_some() {
            return;
        }
        self.time_log.editing = row["id"].as_i64();
        let minutes = ((row["seconds"].as_i64().unwrap_or(0) + 30) / 60).max(1);
        for (input, text) in [
            (
                &self.time_log.edit.date,
                day(&row).map(|d| d.to_string()).unwrap_or_default(),
            ),
            (&self.time_log.edit.hours, (minutes / 60).to_string()),
            (&self.time_log.edit.minutes, (minutes % 60).to_string()),
            (
                &self.time_log.edit.note,
                row["note"].as_str().unwrap_or("").to_owned(),
            ),
        ] {
            input.update(cx, |input, cx| input.set_content(text, cx));
        }
        window.focus(&self.time_log.edit.hours.focus_handle(cx), cx);
        cx.notify();
    }
    pub(super) fn time_log_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.time_log.user != self.db.current_user_id().unwrap_or(0) {
            self.invalidate_time_log(cx);
        }
        if !self.time_log.open {
            return false;
        }
        let key = &event.keystroke;
        let ctrl = key.modifiers.control || key.modifiers.platform;
        match key.key.as_str() {
            "escape" => {
                if self.time_log.editing.is_some() {
                    self.time_log.editing = None;
                    window.focus(&self.time_log.add.hours.focus_handle(cx), cx);
                    cx.notify();
                } else {
                    self.close_time_log(window, cx)
                }
            }
            "enter"
                if ctrl
                    || self.time_log.editing.is_some()
                        && !self.time_log.edit.note.focus_handle(cx).is_focused(window) =>
            {
                self.submit_time_entry(window, cx)
            }
            "tab" => {
                let form = if self.time_log.editing.is_some() {
                    &self.time_log.edit
                } else {
                    &self.time_log.add
                };
                let inputs = [&form.date, &form.hours, &form.minutes, &form.note];
                let at = inputs
                    .iter()
                    .position(|input| input.focus_handle(cx).is_focused(window))
                    .unwrap_or(0);
                let next = (at + if key.modifiers.shift { 3 } else { 1 }) % 4;
                window.focus(&inputs[next].focus_handle(cx), cx);
            }
            _ => return true,
        }
        cx.stop_propagation();
        true
    }
    fn time_entry_form(&self, editing: bool, cx: &mut Context<Self>) -> AnyElement {
        let form = if editing {
            &self.time_log.edit
        } else {
            &self.time_log.add
        };
        let prefix = if editing { "edit" } else { "add" };
        let mut fields = div().flex().items_center().gap(px(8.));
        for (name, input, width) in [
            ("date", &form.date, 130.),
            ("hours", &form.hours, 48.),
            ("minutes", &form.minutes, 48.),
        ] {
            fields = fields.child(
                div()
                    .w(px(width))
                    .border_b_1()
                    .border_color(color(self.colors.border))
                    .child(input.clone())
                    .layout_key(&self.layout, format!("time-log.{prefix}.{name}"), None),
            );
            if name != "date" {
                fields = fields.child(if name == "hours" { "h" } else { "m" });
            }
        }
        div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .child(fields)
            .child(div().min_h(px(54.)).child(form.note.clone()).layout_key(
                &self.layout,
                format!("time-log.{prefix}.note"),
                None,
            ))
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap(px(12.))
                    .child(
                        div()
                            .id(SharedString::from(format!("time-log-{prefix}-save")))
                            .cursor_pointer()
                            .child(if editing { "Save" } else { "Add entry" })
                            .on_click(
                                cx.listener(|this, _, window, cx| {
                                    this.submit_time_entry(window, cx)
                                }),
                            )
                            .layout_key(&self.layout, format!("time-log.{prefix}.save"), None),
                    )
                    .when(editing, |d| {
                        d.child(
                            div()
                                .id("time-log-cancel-edit")
                                .cursor_pointer()
                                .child("Cancel")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.time_log.editing = None;
                                    cx.notify();
                                }))
                                .layout_key(&self.layout, "time-log.edit.cancel", None),
                        )
                    }),
            )
            .when(self.time_log.pending.is_some(), |d| d.opacity(0.5))
            .into_any_element()
    }
    pub(super) fn time_log_overlay(&self, cx: &mut Context<Self>) -> AnyElement {
        if !self.time_log.open || self.time_log.user != self.db.current_user_id().unwrap_or(0) {
            return div().into_any_element();
        }
        let value = self
            .db
            .time_entries(self.time_log.task)
            .ok()
            .flatten()
            .unwrap_or(json!({"entries":[]}));
        let today = Local::now().date_naive();
        let now = chrono::Utc::now().timestamp();
        let rows: Vec<_> = value["entries"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|row| {
                let date = day(row);
                let date_matches = match self.time_log.range {
                    0 => date == Some(today),
                    1 => date.is_some_and(|d| {
                        d >= today
                            - chrono::Duration::days(i64::from(
                                today.weekday().num_days_from_monday(),
                            ))
                            && d < today
                                + chrono::Duration::days(
                                    7 - i64::from(today.weekday().num_days_from_monday()),
                                )
                    }),
                    2 => {
                        date.is_some_and(|d| d.year() == today.year() && d.month() == today.month())
                    }
                    _ => true,
                };
                date_matches
                    && (self.time_log.users.is_empty()
                        || row["userId"]
                            .as_i64()
                            .is_some_and(|id| self.time_log.users.contains(&id)))
            })
            .cloned()
            .collect();
        let total: i64 = rows.iter().map(|row| seconds(row, now)).sum();
        let task = self.db.task(self.time_log.task).ok().flatten();
        let title = task
            .as_ref()
            .map(|t| {
                format!(
                    "Time log — {} | {}",
                    t.ticket_number.as_deref().unwrap_or(""),
                    t.title
                )
            })
            .unwrap_or_else(|| "Time log".into());
        let mut filters = div().flex().items_center().gap(px(16.)).child(format!(
            "Total tracked  {}",
            ht_core::task_time::format_elapsed(total)
        ));
        filters = filters.child(
            div()
                .id("time-log-range")
                .cursor_pointer()
                .child(["Today", "This week", "This month", "All"][self.time_log.range])
                .on_click(cx.listener(|this, _, _, cx| {
                    this.time_log.range_open = !this.time_log.range_open;
                    this.time_log.users_open = false;
                    cx.notify();
                }))
                .layout_key(&self.layout, "time-log.range", None),
        );
        filters = filters.child(
            div()
                .id("time-log-users")
                .cursor_pointer()
                .child("Assignee")
                .on_click(cx.listener(|this, _, _, cx| {
                    this.time_log.users_open = !this.time_log.users_open;
                    this.time_log.range_open = false;
                    cx.notify();
                }))
                .layout_key(&self.layout, "time-log.users", None),
        );
        let mut content = div()
            .flex()
            .flex_col()
            .gap(px(12.))
            .child(
                div().flex().justify_between().child(title).child(
                    div()
                        .id("time-log-close")
                        .cursor_pointer()
                        .child("×")
                        .on_click(
                            cx.listener(|this, _, window, cx| this.close_time_log(window, cx)),
                        )
                        .layout_key(&self.layout, "time-log.close", None),
                ),
            )
            .child(filters);
        if self.time_log.range_open {
            for (index, label) in ["Today", "This week", "This month", "All"]
                .into_iter()
                .enumerate()
            {
                content = content.child(
                    div()
                        .id(SharedString::from(format!("time-log-range-{index}")))
                        .cursor_pointer()
                        .child(label)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.time_log.range = index;
                            this.time_log.range_open = false;
                            cx.notify();
                        }))
                        .layout_key(&self.layout, format!("time-log.range.{index}"), None),
                );
            }
        }
        if self.time_log.users_open {
            let project = task.as_ref().map_or(0, |t| t.project_id);
            for member in self.db.project_members(project).unwrap_or_default() {
                if let Some(user) = member["id"].as_i64() {
                    let selected = self.time_log.users.contains(&user);
                    content = content.child(
                        div()
                            .id(SharedString::from(format!("time-log-user-{user}")))
                            .cursor_pointer()
                            .child(format!(
                                "{} {}",
                                if selected { "☑" } else { "☐" },
                                member["displayName"].as_str().unwrap_or("")
                            ))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if !this.time_log.users.remove(&user) {
                                    this.time_log.users.insert(user);
                                }
                                cx.notify();
                            }))
                            .layout_key(&self.layout, format!("time-log.user.{user}"), None),
                    );
                }
            }
        }
        let mut list = div()
            .id("time-log-entries")
            .flex_1()
            .min_h(px(60.))
            .max_h(px(330.))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(px(8.));
        if self.time_log.loading {
            list = list.child("Loading…");
        } else if rows.is_empty() {
            list = list.child("No time entries.");
        }
        let mut groups = std::collections::BTreeMap::<NaiveDate, Vec<Value>>::new();
        for row in rows {
            if let Some(date) = day(&row) {
                groups.entry(date).or_default().push(row);
            }
        }
        for (date, entries) in groups.into_iter().rev() {
            let subtotal: i64 = entries.iter().map(|row| seconds(row, now)).sum();
            list = list.child(format!(
                "{}    {}",
                date.format("%A %-d %b"),
                ht_core::task_time::format_elapsed(subtotal)
            ));
            for row in entries {
                let id = row["id"].as_i64().unwrap_or(0);
                if self.time_log.editing == Some(id) {
                    list = list.child(self.time_entry_form(true, cx));
                    continue;
                }
                let can_manage = row["userId"] == self.time_log.user || value["canManage"] == true;
                let mut line = div()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .child(row["userName"].as_str().unwrap_or("").to_owned())
                    .child(row["note"].as_str().unwrap_or("").to_owned())
                    .child(ht_core::task_time::format_elapsed(seconds(&row, now)));
                if can_manage && id > 0 && self.time_log.pending.is_none() {
                    if row["endedAt"].is_string() {
                        let edit = row.clone();
                        line = line.child(
                            div()
                                .id(SharedString::from(format!("edit-time-entry-{id}")))
                                .cursor_pointer()
                                .child("✎")
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.edit_time_entry(edit.clone(), window, cx)
                                }))
                                .layout_key(&self.layout, format!("time-log.edit.{id}"), None),
                        );
                    }
                    line = line.child(
                        div()
                            .id(SharedString::from(format!("delete-time-entry-{id}")))
                            .cursor_pointer()
                            .child("⌫")
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.write_time_entry(
                                    TimeEntryChange::Delete { entry: id },
                                    window,
                                    cx,
                                )
                            }))
                            .layout_key(&self.layout, format!("time-log.delete.{id}"), None),
                    );
                }
                list = list.child(line);
            }
        }
        content = content
            .child(list)
            .when_some(self.time_log.error.clone(), |d, error| {
                d.child(div().text_color(color(0xcf222e)).child(error))
            })
            .child(self.time_entry_form(false, cx));
        div()
            .absolute()
            .inset_0()
            .bg(rgba(0x00000080))
            .flex()
            .justify_center()
            .items_center()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, window, cx| this.close_time_log(window, cx)),
            )
            .child(
                div()
                    .id("time-log-modal")
                    .w(px(680.))
                    .max_h(px(740.))
                    .p(px(16.))
                    .rounded(px(5.))
                    .bg(color(self.colors.container))
                    .text_size(px(14.))
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child(content)
                    .layout_key(&self.layout, "time-log.modal", None),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::{
        shell::tests::{click, draw, measured},
        task_fields::tests::desktop,
    };
    struct Server {
        status: u16,
        change: TimeEntryChange,
        reply: Value,
    }
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
            let (expected, request) = self.change.request(1).unwrap();
            assert_eq!(method, expected);
            assert_eq!(path, "/api/time/entries");
            assert_eq!(*body, request);
            assert!(!key.is_empty());
            Ok(ht_core::http::Response {
                status: self.status,
                body: if self.status == 200 {
                    self.reply.clone()
                } else {
                    json!({"error":"Forbidden"})
                },
            })
        }
    }
    fn entry() -> Value {
        let start = Local::now().date_naive().and_hms_opt(12, 0, 0).unwrap();
        let start = Local.from_local_datetime(&start).earliest().unwrap();
        json!({"id":5,"userId":42,"userName":"Me","note":"Before","startedAt":start.to_rfc3339(),"endedAt":(start+chrono::Duration::hours(1)).to_rfc3339(),"pausedAt":null,"createdAt":(start+chrono::Duration::days(1)).to_rfc3339(),"seconds":3600})
    }
    fn seed(this: &mut Desktop) {
        this.db.cache_task_time(42,1,json!({"enabled":true,"runningEntry":null,"activeRunningEntryCount":0,"taskTotalSeconds":3600,"otherEntriesSeconds":3600}),chrono::Utc::now().timestamp()).unwrap();
        this.db
            .cache_time_entries(
                42,
                1,
                &json!({"success":true,"canManage":false,"entries":[entry()]}),
            )
            .unwrap();
        this.reload();
    }
    #[gpui::test]
    fn time_log_b_and_native_crud_controls_queue_once_preserve_refused_edits_and_accept_server_rows(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(crate::input::init);
        let view = cx.open_window(gpui::size(px(1440.), px(900.)), desktop);
        let mut cx = gpui::VisualTestContext::from_window(view.into(), cx);
        for operation in ["create", "update", "delete"] {
            for status in [401, 403, 200] {
                view.update(&mut cx, |this, _, cx| {
                    this.invalidate_time_log(cx);
                    seed(this);
                })
                .unwrap();
                draw(view, &mut cx);
                if operation == "create" {
                    cx.simulate_keystrokes("b");
                    cx.simulate_input("1");
                    cx.simulate_keystrokes("ctrl-enter");
                } else {
                    click(view, &mut cx, "task.time.log");
                    measured(view, &mut cx, "time-log.modal");
                    if operation == "update" {
                        click(view, &mut cx, "time-log.edit.5");
                        cx.simulate_keystrokes("ctrl-a");
                        cx.simulate_input("2");
                        click(view, &mut cx, "time-log.edit.save");
                    } else {
                        click(view, &mut cx, "time-log.delete.5");
                    }
                }
                view.update(&mut cx, |this, _, cx| {
                    assert!(this.time_log.open);
                    let change = this
                        .time_log
                        .pending
                        .clone()
                        .expect("real control must queue a write");
                    assert_eq!(
                        this.db
                            .mutations()
                            .unwrap()
                            .iter()
                            .filter(|row| row.status != "failed")
                            .count(),
                        1
                    );
                    let value = this.db.time_entries(1).unwrap().unwrap();
                    let mut ack = value["entries"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .find(|row| row["pending"] == true)
                        .cloned()
                        .unwrap_or(Value::Null);
                    if operation == "create" {
                        assert!(ack["id"].as_i64().unwrap() < 0);
                        ack["id"] = json!(8);
                        ack["taskId"] = json!(1);
                    } else if operation == "update" {
                        assert_eq!(ack["seconds"], 7200);
                    } else {
                        assert!(value["entries"].as_array().unwrap().is_empty());
                    }
                    this.db
                        .drain_once(
                            &mut Server {
                                status,
                                change,
                                reply: json!({"success":true,"entry":ack}),
                            },
                            chrono::Utc::now().timestamp() + 1,
                        )
                        .unwrap();
                    assert!(!this.poll_time(cx));
                    let value = this.db.time_entries(1).unwrap().unwrap();
                    if status != 200 {
                        assert_eq!(this.time_log.error.as_deref(), Some("Forbidden"));
                        assert_eq!(value["entries"][0]["id"], 5);
                        assert_eq!(value["entries"][0]["seconds"], 3600);
                        if operation == "update" {
                            assert_eq!(this.time_log.editing, Some(5));
                            assert_eq!(this.time_log.edit.hours.read(cx).content, "2");
                        }
                        if operation == "create" {
                            assert_eq!(this.time_log.add.hours.read(cx).content, "1");
                        }
                    } else {
                        assert!(this.time_log.error.is_none());
                        if operation == "create" {
                            assert!(this.time_log.add.hours.read(cx).content.is_empty());
                            assert_eq!(value["entries"][0]["id"], 8);
                        }
                        if operation == "update" {
                            assert!(this.time_log.editing.is_none());
                            assert_eq!(value["entries"][0]["seconds"], 7200);
                        }
                        if operation == "delete" {
                            assert!(value["entries"].as_array().unwrap().is_empty());
                        }
                    }
                })
                .unwrap();
                let editing = view
                    .read_with(&mut cx, |this, _| this.time_log.editing.is_some())
                    .unwrap();
                if editing {
                    cx.simulate_keystrokes("escape");
                }
                cx.simulate_keystrokes("escape");
                view.read_with(&mut cx, |this, _| {
                    assert!(!this.time_log.open);
                    assert_eq!(this.screen, Screen::Task);
                })
                .unwrap();
            }
        }
    }
    #[gpui::test]
    fn time_log_range_user_filters_permissions_cancel_validation_and_account_fence(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(crate::input::init);
        let view = cx.open_window(gpui::size(px(1440.), px(900.)), desktop);
        let mut cx = gpui::VisualTestContext::from_window(view.into(), cx);
        view.update(&mut cx, |this, _, _| {
            seed(this);
            this.db
                .kv_set(
                    "members:15",
                    &json!([{"id":42,"displayName":"Me"},{"id":7,"displayName":"Other"}])
                        .to_string(),
                )
                .unwrap();
            let mut other = entry();
            other["id"] = json!(6);
            other["userId"] = json!(7);
            other["userName"] = json!("Other");
            this.db
                .cache_time_entries(
                    42,
                    1,
                    &json!({"success":true,"canManage":false,"entries":[entry(),other]}),
                )
                .unwrap();
        })
        .unwrap();
        draw(view, &mut cx);
        cx.simulate_keystrokes("b");
        click(view, &mut cx, "time-log.range");
        click(view, &mut cx, "time-log.range.3");
        click(view, &mut cx, "time-log.users");
        click(view, &mut cx, "time-log.user.42");
        view.update(&mut cx, |this, _, _| {
            assert_eq!(this.time_log.range, 3);
            assert!(this.time_log.users.contains(&42));
        })
        .unwrap();
        click(view, &mut cx, "time-log.users");
        click(view, &mut cx, "time-log.edit.5");
        cx.simulate_keystrokes("escape");
        view.update(&mut cx, |this, _, _| {
            assert!(this.time_log.editing.is_none())
        })
        .unwrap();
        cx.simulate_input("25");
        cx.simulate_keystrokes("ctrl-enter");
        view.update(&mut cx, |this, _, _| {
            assert!(this.db.mutations().unwrap().is_empty());
            assert_eq!(
                this.time_log.error.as_deref(),
                Some("Enter a duration from 1 minute to 24 hours")
            );
        })
        .unwrap();
        view.update(&mut cx, |this, _, _| {
            this.db.kv_set("current-user-id", "99").unwrap()
        })
        .unwrap();
        cx.simulate_keystrokes("escape");
        view.update(&mut cx, |this, _, _| {
            assert!(!this.time_log.open);
            assert!(this.db.mutations().unwrap().is_empty());
        })
        .unwrap();
    }
    #[gpui::test]
    fn time_log_does_not_attach_a_refused_entry_to_an_existing_timer_write(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(crate::input::init);
        let view = cx.open_window(gpui::size(px(1440.), px(900.)), desktop);
        let mut cx = gpui::VisualTestContext::from_window(view.into(), cx);
        view.update(&mut cx, |this, window, cx| {
            seed(this);
            this.queue_time(TimeWrite::Start { task: 1 }, window, cx);
            let pending = this.task_time.pending.clone();
            assert!(pending.is_some());
            this.open_time_log(window, cx);
            this.time_log
                .add
                .hours
                .update(cx, |input, cx| input.set_content("1".into(), cx));
            this.submit_time_entry(window, cx);
            assert!(
                this.time_log.pending.is_none(),
                "the timer acknowledgement must never count as entry success"
            );
            assert_eq!(this.task_time.pending, pending);
            assert_eq!(this.db.mutations().unwrap().len(), 1);
            assert_eq!(this.time_log.add.hours.read(cx).content, "1");
        })
        .unwrap();
    }
}
