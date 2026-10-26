use super::*;
use crate::shortcuts::Action;
use chrono::{Datelike, Local, NaiveDate, TimeZone, Timelike};
use gpui::MouseButton;
use ht_core::TaskPatch;

pub(super) struct TaskFields {
    query: Entity<TextInput>,
    due: Option<DuePicker>,
    pub summary_open: bool,
    summary_bounds: Rc<std::cell::Cell<Option<gpui::Bounds<gpui::Pixels>>>>,
}
struct DuePicker {
    task: i64,
    custom: bool,
    month: NaiveDate,
    selected: NaiveDate,
    choice: usize,
}
impl TaskFields {
    pub fn new(cx: &mut Context<Desktop>) -> Self {
        let query = cx.new(|cx| {
            TextInput::new(
                String::new(),
                "e.g. tomorrow, 2026-12-31 14:30 ...",
                color(0x8e9093),
                cx,
            )
        });
        cx.observe(&query, |this, _, cx| {
            if let Some(due) = &mut this.task_fields.due {
                due.choice = 0;
            }
            cx.notify();
        })
        .detach();
        Self {
            query,
            due: None,
            summary_open: false,
            summary_bounds: Rc::new(std::cell::Cell::new(None)),
        }
    }
}
fn at_local(date: NaiveDate, hour: u32, minute: u32) -> Option<String> {
    Local
        .from_local_datetime(&date.and_hms_opt(hour, minute, 0)?)
        .earliest()
        .map(|date| date.to_rfc3339())
}
fn parse_due(query: &str, today: NaiveDate) -> Option<String> {
    if let Ok(date) = chrono::DateTime::parse_from_rfc3339(query) {
        return Some(date.to_rfc3339());
    }
    if let Ok(date) = chrono::NaiveDateTime::parse_from_str(query, "%Y-%m-%d %H:%M") {
        return Local
            .from_local_datetime(&date)
            .earliest()
            .map(|date| date.to_rfc3339());
    }
    let date = match query.to_lowercase().as_str() {
        "today" => today,
        "tomorrow" => today + chrono::Duration::days(1),
        "next week" | "next week monday" => {
            today + chrono::Duration::days(7 - today.weekday().num_days_from_monday() as i64)
        }
        _ => NaiveDate::parse_from_str(query, "%Y-%m-%d").ok()?,
    };
    at_local(date, 9, 0)
}
fn due_presets(now: chrono::DateTime<Local>) -> Vec<(String, Option<String>)> {
    let today = now.date_naive();
    let (name, date, hour) = match now.hour() {
        0..=8 => ("Today", today, 9),
        9..=16 => ("Today", today, 17),
        17..=20 => ("Today", today, 21),
        _ => ("Tonight", today + chrono::Duration::days(1), 0),
    };
    [
        (name, date, hour),
        ("Tomorrow", today + chrono::Duration::days(1), 9),
        (
            "Next week monday",
            today + chrono::Duration::days(7 - today.weekday().num_days_from_monday() as i64),
            9,
        ),
    ]
    .into_iter()
    .filter_map(|(name, date, hour)| Some((name.into(), Some(at_local(date, hour, 0)?))))
    .collect()
}
pub(super) struct TicketTooltip;
impl Render for TicketTooltip {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let modifier = if cfg!(target_os = "macos") {
            "CMD"
        } else {
            "CTRL"
        };
        div()
            .p(px(8.))
            .bg(color(0x202124))
            .text_color(color(0xffffff))
            .rounded(px(4.))
            .text_size(px(12.))
            .flex()
            .flex_col()
            .child(format!("Copy task title & ID  {modifier}+I"))
            .child(format!("Copy task ID  {modifier}+SHIFT+I"))
    }
}
impl Desktop {
    pub(super) fn open_task_field(
        &mut self,
        label: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let action = match label {
            "Assignees" | "Agents" => Action::Assign,
            "Priority" => Action::Priority,
            "Due date" => Action::DueDate,
            "Tags" => Action::Tags,
            "Task size" => Action::Size,
            "ID" => Action::CopyId,
            _ => return,
        };
        self.run_shortcut(action, window, cx);
        self.interaction.agents_only = label == "Agents";
    }
    pub(super) fn has_next_status(&self, task: &TaskView) -> bool {
        self.columns
            .iter()
            .position(|c| Some(c.id) == task.section_id)
            .is_some_and(|i| i + 1 < self.columns.len())
    }
    pub(super) fn next_status_button(&self, cx: &mut Context<Self>) -> AnyElement {
        div()
            .id("task-next-status")
            .cursor_pointer()
            .child(icon("arrow-right", self.colors.muted, 11.))
            .on_click(cx.listener(|this, _, window, cx| {
                cx.stop_propagation();
                this.run_shortcut(Action::MoveRight, window, cx);
            }))
            .layout_key(&self.layout, "task.status.next", None)
    }
    pub(super) fn task_board_button(&self, cx: &mut Context<Self>) -> AnyElement {
        div()
            .id("task-board-link")
            .cursor_pointer()
            .child(icon("corner-left-up", self.colors.muted, 11.))
            .on_click(cx.listener(|this, _, window, cx| {
                cx.stop_propagation();
                this.run_shortcut(Action::Board, window, cx);
            }))
            .layout_key(&self.layout, "task.project.goto", None)
    }
    pub(super) fn task_summary_view(&self, cx: &mut Context<Self>) -> AnyElement {
        let t = self.colors;
        let anchor = self.task_fields.summary_bounds.clone();
        div()
            .id("task-summary")
            .pt(px(8.))
            .relative()
            .text_size(px(13.))
            .line_height(px(19.5))
            .text_color(color(t.muted))
            .on_click(cx.listener(|_, _, _, cx| cx.stop_propagation()))
            .child(
                div()
                    .id("task-summary-toggle")
                    .cursor_pointer()
                    .child(if self.task_fields.summary_open {
                        SharedString::from("Summary")
                    } else {
                        self.summary.clone()
                    })
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.task_fields.summary_open = !this.task_fields.summary_open;
                        cx.notify();
                    }))
                    .layout_key(&self.layout, "task.summary.toggle", Some(if self.task_fields.summary_open {
                        "Summary".into()
                    } else {
                        self.summary.to_string()
                    })),
            )
            .child(
                gpui::canvas(
                    move |bounds, _, _| anchor.set(Some(bounds)),
                    |_, _, _, _| {},
                )
                .absolute()
                .size_full(),
            )
            .into_any_element()
    }
    pub(super) fn task_summary_overlay(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(bounds) = self
            .task_fields
            .summary_bounds
            .get()
            .filter(|_| self.screen == Screen::Task && self.task_fields.summary_open)
        else {
            return div().into_any_element();
        };
        let markdown = self
            .detail
            .as_ref()
            .map_or("", |detail| full_task_summary(&detail.task.extra));
        let mut html = String::new();
        pulldown_cmark::html::push_html(
            &mut html,
            pulldown_cmark::Parser::new_ext(
                markdown,
                pulldown_cmark::Options::ENABLE_STRIKETHROUGH
                    | pulldown_cmark::Options::ENABLE_TABLES,
            ),
        );
        div()
            .id("task-summary-full")
            .absolute()
            .left(bounds.origin.x)
            .top(bounds.origin.y + px(28.))
            .w(bounds.size.width)
            .max_h((window.viewport_size().height - bounds.origin.y - px(40.)).max(px(80.)))
            .overflow_y_scroll()
            .p(px(16.))
            .bg(color(self.colors.detail))
            .shadow(shadow())
            .on_click(cx.listener(|_, _, _, cx| cx.stop_propagation()))
            .child(RichText::parse(&html, self.colors).element())
            .layout_key(&self.layout, "task.summary.full", Some(markdown.to_owned()))
    }
    pub(super) fn show_due_date(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.task_fields.summary_open = false;
        let Some(task) = self
            .selected_task()
            .and_then(|id| self.db.task(id).ok().flatten())
        else {
            return;
        };
        let selected = task
            .due_date
            .as_deref()
            .and_then(|date| chrono::DateTime::parse_from_rfc3339(date).ok())
            .map(|date| date.with_timezone(&Local).date_naive())
            .unwrap_or_else(|| Local::now().date_naive());
        self.task_fields.due = Some(DuePicker {
            task: task.id,
            custom: false,
            month: selected.with_day(1).unwrap(),
            selected,
            choice: 0,
        });
        self.task_fields
            .query
            .update(cx, |input, cx| input.set_content(String::new(), cx));
        window.focus(&self.task_fields.query.focus_handle(cx), cx);
        cx.notify();
    }
    fn due_choices(&self, cx: &App) -> Vec<(String, Option<String>)> {
        let Some(due) = &self.task_fields.due else {
            return vec![];
        };
        let query = self.task_fields.query.read(cx).content.trim();
        let mut choices = if query.is_empty() {
            let mut choices = vec![];
            if self
                .db
                .task(due.task)
                .ok()
                .flatten()
                .is_some_and(|task| task.due_date.is_some())
            {
                choices.push(("Reset".into(), None));
            }
            if let Ok(Some(last)) = self.db.kv_get(&format!(
                "due-last:{}",
                self.db.current_user_id().unwrap_or(0)
            )) {
                if chrono::DateTime::parse_from_rfc3339(&last).is_ok() {
                    choices.push(("last used".into(), Some(last)));
                }
            }
            choices.extend(due_presets(Local::now()));
            choices
        } else {
            parse_due(query, Local::now().date_naive())
                .map(|date| vec![(query.to_owned(), Some(date))])
                .unwrap_or_default()
        };
        choices.push(("Pick a custom date".into(), None));
        choices
    }
    fn close_due(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.task_fields.due = None;
        window.focus(&self.focus, cx);
        cx.notify();
    }
    fn save_due(&mut self, date: Option<String>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(due) = &self.task_fields.due else {
            return;
        };
        if !self.enqueue(
            Write::TaskPatch {
                task_id: due.task,
                patch: TaskPatch::DueDate(date.clone()),
            },
            cx,
        ) {
            return;
        }
        if let Some(date) = date {
            let _ = self.db.kv_set(
                &format!("due-last:{}", self.db.current_user_id().unwrap_or(0)),
                &date,
            );
        }
        self.close_due(window, cx);
    }
    fn choose_due(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some((name, date)) = self.due_choices(cx).get(index).cloned() else {
            return;
        };
        if name == "Pick a custom date" {
            if let Some(due) = &mut self.task_fields.due {
                due.custom = true;
            }
            window.focus(&self.focus, cx);
            cx.notify();
        } else {
            self.save_due(date, window, cx);
        }
    }
    pub(super) fn task_fields_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.task_fields.due.is_none() {
            if self.screen == Screen::Task
                && self.task_fields.summary_open
                && event.keystroke.key == "escape"
            {
                self.task_fields.summary_open = false;
                cx.notify();
                cx.stop_propagation();
                return true;
            }
            return false;
        }
        let key = &event.keystroke;
        let ctrl = key.modifiers.control || key.modifiers.platform;
        if key.key == "escape" {
            if key.modifiers.shift && self.task_fields.due.as_ref().unwrap().custom {
                self.task_fields.due.as_mut().unwrap().custom = false;
                window.focus(&self.task_fields.query.focus_handle(cx), cx);
            } else {
                self.close_due(window, cx);
            }
        } else if self.task_fields.due.as_ref().unwrap().custom {
            if key.key == "enter" && ctrl {
                let date = at_local(self.task_fields.due.as_ref().unwrap().selected, 9, 0);
                if date.is_some() {
                    self.save_due(date, window, cx);
                }
            } else {
                cx.stop_propagation();
                return true;
            }
        } else {
            let count = self.due_choices(cx).len();
            let due = self.task_fields.due.as_mut().unwrap();
            match key.key.as_str() {
                "down" if !ctrl => due.choice = (due.choice + 1).min(count.saturating_sub(1)),
                "up" if !ctrl => due.choice = due.choice.saturating_sub(1),
                "enter" => {
                    let choice = due.choice;
                    self.choose_due(choice, window, cx);
                }
                _ => return true,
            }
        }
        cx.notify();
        cx.stop_propagation();
        true
    }
    pub(super) fn due_date_overlay(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(due) = &self.task_fields.due else {
            return div().into_any_element();
        };
        let t = self.colors;
        let mut content = div().flex().flex_col().gap(px(8.)).child("Due date");
        if due.custom {
            let month = due.month;
            content = content.child(
                div()
                    .flex()
                    .justify_between()
                    .child(
                        div()
                            .id("due-prev-month")
                            .cursor_pointer()
                            .child("‹")
                            .on_click(cx.listener(|this, _, _, cx| {
                                let due = this.task_fields.due.as_mut().unwrap();
                                due.month = due
                                    .month
                                    .checked_sub_months(chrono::Months::new(1))
                                    .unwrap_or(due.month);
                                cx.notify();
                            }))
                            .layout_key(&self.layout, "due.previous", None),
                    )
                    .child(month.format("%B %Y").to_string())
                    .child(
                        div()
                            .id("due-next-month")
                            .cursor_pointer()
                            .child("›")
                            .on_click(cx.listener(|this, _, _, cx| {
                                let due = this.task_fields.due.as_mut().unwrap();
                                due.month = due
                                    .month
                                    .checked_add_months(chrono::Months::new(1))
                                    .unwrap_or(due.month);
                                cx.notify();
                            }))
                            .layout_key(&self.layout, "due.next", None),
                    ),
            );
            let start =
                month - chrono::Duration::days(month.weekday().num_days_from_sunday() as i64);
            for week in 0..6 {
                let mut row = div().flex();
                for day in 0..7 {
                    let date = start + chrono::Duration::days(week * 7 + day);
                    row = row.child(
                        div()
                            .id(SharedString::from(format!("due-{date}")))
                            .w(px(48.))
                            .h(px(32.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .when(date == due.selected, |d| d.bg(color(t.active)))
                            .when(date.month() != month.month(), |d| {
                                d.text_color(color(t.muted))
                            })
                            .child(date.day().to_string())
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.task_fields.due.as_mut().unwrap().selected = date;
                                cx.notify();
                            }))
                            .layout_key(&self.layout, format!("due.day.{date}"), None),
                    );
                }
                content = content.child(row);
            }
            content = content.child(
                div()
                    .id("due-save")
                    .cursor_pointer()
                    .child("Save")
                    .on_click(cx.listener(|this, _, window, cx| {
                        if let Some(date) = this
                            .task_fields
                            .due
                            .as_ref()
                            .and_then(|due| at_local(due.selected, 9, 0))
                        {
                            this.save_due(Some(date), window, cx);
                        }
                    }))
                    .layout_key(&self.layout, "due.save", None),
            );
        } else {
            content = content.child(self.task_fields.query.clone()).children(
                self.due_choices(cx)
                    .into_iter()
                    .enumerate()
                    .map(|(i, (name, date))| {
                        div()
                            .id(("due-choice", i))
                            .p(px(8.))
                            .cursor_pointer()
                            .when(i == due.choice, |d| d.bg(color(t.hover)))
                            .child(name)
                            .when_some(date, |d, date| {
                                d.child(
                                    div()
                                        .text_size(px(11.))
                                        .text_color(color(t.muted))
                                        .child(date),
                                )
                            })
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.choose_due(i, window, cx)
                            }))
                            .layout_key(&self.layout, format!("due.choice.{i}"), None)
                    }),
            );
        }
        div()
            .id("due-backdrop")
            .absolute()
            .size_full()
            .top_0()
            .left_0()
            .bg(rgba(0x00000080))
            .flex()
            .items_center()
            .justify_center()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|_, _, _, cx| cx.stop_propagation()),
            )
            .on_click(cx.listener(|this, _, window, cx| this.close_due(window, cx)))
            .child(
                div()
                    .id("due-dialog")
                    .w(px(400.))
                    .p(px(24.))
                    .bg(color(t.comment))
                    .rounded(px(5.))
                    .shadow(shadow())
                    .on_click(cx.listener(|_, _, _, cx| cx.stop_propagation()))
                    .child(content)
                    .layout_key(&self.layout, "task.due.dialog", None),
            )
            .into_any_element()
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use crate::ui::shell::tests::{click, draw};
    use gpui::{VisualTestContext, WindowHandle};
    use serde_json::json;

    pub(in super::super) fn desktop(window: &mut Window, cx: &mut Context<Desktop>) -> Desktop {
        let mut db = Db::memory().unwrap();
        db.kv_set("current-user-id", "42").unwrap();
        db.apply_response(&json!({"success":true,"projects":[{"id":15,"title":"Board"}],"sections":[
            {"id":10,"projectId":15,"section_title":"Todo","visibility":true,"deleted":false,"ranking":"a"},
            {"id":11,"projectId":15,"section_title":"Doing","visibility":true,"deleted":false,"ranking":"b"}
        ],"tasks":[
            {"id":1,"projectId":15,"sectionId":10,"title":"Needle","ticketNumber":"HT-1","status":"Normal","description":"<p>Hidden</p>","priority":{"priority_index":0,"Priority_Value":"No Priority"}},
            {"id":2,"projectId":15,"sectionId":10,"title":"Other","ticketNumber":"HT-2","status":"Normal"},
            {"id":3,"projectId":15,"sectionId":11,"title":"Third","ticketNumber":"HT-3","status":"Normal","labels":[{"id":"bug","name":"Bug"}]}
        ]})).unwrap();
        db.kv_set(
            "members:15",
            &json!([{"id":7,"displayName":"Alex"},{"id":8,"displayName":"Bea"}]).to_string(),
        )
        .unwrap();
        let options = Options::parse(
            [
                "--offline",
                "--screen",
                "task",
                "--task",
                "1",
                "--board",
                "15",
            ]
            .map(String::from),
        )
        .unwrap();
        let mut view = Desktop::new(db, options, window, cx);
        view.layout = Some(Rc::new(RefCell::new(LayoutDump::default())));
        view.ready_scheduled = true;
        window.focus(&view.focus, cx);
        view
    }
    fn visual(cx: &mut gpui::TestAppContext) -> (WindowHandle<Desktop>, VisualTestContext) {
        cx.update(crate::input::init);
        let view = cx.open_window(gpui::size(px(1440.), px(900.)), desktop);
        let mut cx = VisualTestContext::from_window(view.into(), cx);
        draw(view, &mut cx);
        (view, cx)
    }
    #[gpui::test]
    fn inline_priority_click_sets_and_clears_without_navigation(cx: &mut gpui::TestAppContext) {
        let (view, mut cx) = visual(cx);
        click(view, &mut cx, "task.meta.6");
        click(view, &mut cx, "command.choice.2");
        view.read_with(&mut cx, |this, _| {
            assert_eq!(this.screen, Screen::Task);
            assert_eq!(
                this.db.task(1).unwrap().unwrap().extra["priority"]["priority_index"],
                2
            );
            assert_eq!(
                this.db.mutations().unwrap()[0].payload_json,
                json!({"task_id":1,"priority":2})
            );
        })
        .unwrap();
        click(view, &mut cx, "task.meta.6");
        click(view, &mut cx, "command.choice.0");
        view.read_with(&mut cx, |this, _| {
            assert_eq!(
                this.db.task(1).unwrap().unwrap().extra["priority"]["priority_index"],
                0
            )
        })
        .unwrap();
    }
    #[gpui::test]
    fn inline_people_tags_and_size_use_searchable_existing_queue_pickers(
        cx: &mut gpui::TestAppContext,
    ) {
        let (view, mut cx) = visual(cx);
        for (field, query) in [(0, "Alex"), (8, "Bug"), (7, "XS")] {
            click(view, &mut cx, &format!("task.meta.{field}"));
            cx.simulate_input(query);
            cx.run_until_parked();
            click(view, &mut cx, "command.choice.0");
            if field != 7 {
                cx.simulate_keystrokes("escape");
            }
        }
        view.read_with(&mut cx, |this, _| {
            let task = this.db.task(1).unwrap().unwrap();
            assert_eq!(task.extra["assignees"][0]["id"], 7);
            assert_eq!(task.extra["labels"][0]["id"], "bug");
            assert_eq!(task.extra["estimate"]["estimate_index"], 2);
            assert_eq!(this.db.mutations().unwrap().len(), 3);
        })
        .unwrap();
        for field in [0, 8] {
            click(view, &mut cx, &format!("task.meta.{field}"));
            click(view, &mut cx, "command.choice.0");
            cx.simulate_keystrokes("escape");
        }
        view.read_with(&mut cx, |this, _| {
            let task = this.db.task(1).unwrap().unwrap();
            assert_eq!(task.assignee_count, 0);
            assert_eq!(task.label_count, 0);
        })
        .unwrap();
    }
    #[gpui::test]
    fn inline_due_arbitrary_datetime_cancel_reset_and_calendar_commit(
        cx: &mut gpui::TestAppContext,
    ) {
        let (view, mut cx) = visual(cx);
        click(view, &mut cx, "task.meta.4");
        cx.simulate_input("2030-12-31 14:30");
        cx.run_until_parked();
        cx.simulate_keystrokes("escape");
        view.read_with(&mut cx, |this, _| {
            assert!(this.db.mutations().unwrap().is_empty())
        })
        .unwrap();
        click(view, &mut cx, "task.meta.4");
        cx.simulate_input("2030-12-31T14:30:00+02:00");
        cx.run_until_parked();
        cx.simulate_keystrokes("enter");
        view.read_with(&mut cx, |this, _| {
            assert_eq!(
                this.db.mutations().unwrap()[0].payload_json["due_date"],
                "2030-12-31T14:30:00+02:00"
            )
        })
        .unwrap();
        click(view, &mut cx, "task.meta.4");
        click(view, &mut cx, "due.choice.0");
        view.read_with(&mut cx, |this, _| {
            assert!(this.db.task(1).unwrap().unwrap().due_date.is_none())
        })
        .unwrap();
        click(view, &mut cx, "task.meta.4");
        let last = view
            .read_with(&mut cx, |this, cx| this.due_choices(cx).len() - 1)
            .unwrap();
        click(view, &mut cx, &format!("due.choice.{last}"));
        click(view, &mut cx, "due.next");
        let date = view
            .read_with(&mut cx, |this, _| {
                this.task_fields
                    .due
                    .as_ref()
                    .unwrap()
                    .month
                    .with_day(15)
                    .unwrap()
            })
            .unwrap();
        click(view, &mut cx, &format!("due.day.{date}"));
        click(view, &mut cx, "due.save");
        view.read_with(&mut cx, |this, _| {
            let actual = chrono::DateTime::parse_from_rfc3339(
                this.db
                    .task(1)
                    .unwrap()
                    .unwrap()
                    .due_date
                    .as_deref()
                    .unwrap(),
            )
            .unwrap()
            .with_timezone(&Local);
            assert_eq!(actual.date_naive(), date);
            assert_eq!(actual.hour(), 9);
            assert_eq!(this.db.mutations().unwrap().len(), 3);
            assert!(this.task_fields.due.is_none());
        })
        .unwrap();
    }
    #[gpui::test]
    fn inline_status_arrow_does_not_open_picker_and_project_arrow_never_moves_task(
        cx: &mut gpui::TestAppContext,
    ) {
        let (view, mut cx) = visual(cx);
        click(view, &mut cx, "task.status.next");
        view.read_with(&mut cx, |this, _| {
            assert_eq!(this.db.task(1).unwrap().unwrap().section_id, Some(11));
            assert!(this.interaction.modal.is_none());
        })
        .unwrap();
        click(view, &mut cx, "task.meta.3");
        cx.simulate_keystrokes("escape");
        click(view, &mut cx, "task.id");
        view.read_with(&mut cx, |_, cx| {
            assert_eq!(cx.read_from_clipboard().unwrap().text().unwrap(), "HT-1")
        })
        .unwrap();
        click(view, &mut cx, "task.project.goto");
        view.read_with(&mut cx, |this, _| {
            assert_eq!(this.screen, Screen::Board);
            assert_eq!(this.options.board, 15);
            assert_eq!(this.db.mutations().unwrap().len(), 1);
        })
        .unwrap();
    }
    #[test]
    fn due_parser_preserves_offsets_rejects_invalid_dates_and_uses_web_local_nine() {
        let today = NaiveDate::from_ymd_opt(2030, 3, 1).unwrap();
        assert_eq!(
            parse_due("2030-03-05T12:45:00-04:00", today).unwrap(),
            "2030-03-05T12:45:00-04:00"
        );
        for query in ["2030-02-30", "nonsense", "2030-03-05 25:00"] {
            assert!(parse_due(query, today).is_none());
        }
        let date = chrono::DateTime::parse_from_rfc3339(&parse_due("tomorrow", today).unwrap())
            .unwrap()
            .with_timezone(&Local);
        assert_eq!(date.date_naive(), today + chrono::Duration::days(1));
        assert_eq!(date.hour(), 9);
    }
}

#[cfg(test)]
mod summary_tests {
    use super::*;
    use crate::ui::shell::tests::{click, draw, measured};
    use crate::ui::task_fields::tests::desktop;
    use gpui::VisualTestContext;
    use serde_json::json;
    #[gpui::test]
    fn task_header_without_summary_keeps_web_bottom_spacing(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(crate::input::init);
        let view = cx.open_window(gpui::size(px(1920.), px(1009.)), desktop);
        let mut cx = VisualTestContext::from_window(view.into(), cx);
        for title in ["Needle".to_owned(), "A wrapping task title ".repeat(12)] {
            view.update(&mut cx, |this, _, cx| {
                this.db.apply_task(&json!({
                    "id": 1, "projectId": 15, "title": title,
                    "status": "Normal", "Task_Summary": []
                })).unwrap();
                this.reload();
                cx.notify();
            }).unwrap();
            draw(view, &mut cx);
            let title = measured(view, &mut cx, "task.title");
            let header = measured(view, &mut cx, "task.header");
            // BaseTitleContainer.tsx:44-46 imports Bootstrap's pb-3 via tailwind-entry.css:7-8.
            let expected = 36. + title["h"].as_f64().unwrap().max(37.) + 16.;
            assert_eq!(header["h"].as_f64().unwrap(), expected, "{header}");
        }
    }
    #[gpui::test]
    fn task_summary_i_click_escape_preserve_full_markdown_without_requesting_ai(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(crate::input::init);
        let view=cx.open_window(gpui::size(px(1440.),px(900.)),|window,cx|{
            let mut v=desktop(window,cx);v.db.apply_task(&json!({"id":1,"projectId":15,"title":"Needle","status":"Normal","Task_Summary":[{"content":"## Overview\n- **First** item\n- Second é🙂"}]})).unwrap();v.reload();v
        });
        let mut cx = VisualTestContext::from_window(view.into(), cx);
        draw(view, &mut cx);
        cx.simulate_keystrokes("i");
        assert_eq!(
            measured(view, &mut cx, "task.summary.full")["text"],
            "## Overview\n- **First** item\n- Second é🙂"
        );
        cx.simulate_keystrokes("escape");
        click(view, &mut cx, "task.summary.toggle");
        view.read_with(&mut cx, |this, _| {
            assert!(this.task_fields.summary_open);
            assert!(this.db.mutations().unwrap().is_empty());
        })
        .unwrap();
        click(view, &mut cx, "task.summary.toggle");
        view.read_with(&mut cx, |this, _| assert!(!this.task_fields.summary_open))
            .unwrap();
    }
}

#[cfg(test)]
mod assignee_tests {
    use super::*;
    use crate::ui::shell::tests::{click, draw};
    use crate::ui::task_fields::tests::desktop;
    use gpui::VisualTestContext;
    use serde_json::json;
    #[gpui::test]
    fn inline_people_and_agent_pickers_stay_open_and_toggle_distinct_id_types(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(crate::input::init);
        let view =
            cx.open_window(gpui::size(px(1440.), px(900.)), |window, cx| {
                let v = desktop(window, cx);
                v.db.kv_set("members:15",&json!([
                {"id":7,"displayName":"Alex"},{"id":8,"displayName":"Bea"},
                {"id":"agent-a","displayName":"Runner","owner":{"id":42,"displayName":"Owner"}}
            ]).to_string()).unwrap();
                v
            });
        let mut cx = VisualTestContext::from_window(view.into(), cx);
        draw(view, &mut cx);
        click(view, &mut cx, "task.meta.0");
        click(view, &mut cx, "command.choice.0");
        click(view, &mut cx, "command.choice.1");
        view.read_with(&mut cx, |this, _| {
            assert_eq!(this.db.task(1).unwrap().unwrap().assignee_count, 2);
            assert_eq!(
                this.interaction.modal,
                Some(super::super::interactions::Modal::Assign)
            );
        })
        .unwrap();
        cx.simulate_keystrokes("escape");
        click(view, &mut cx, "task.meta.1");
        view.read_with(&mut cx, |this, cx| assert_eq!(this.choices(cx).len(), 1))
            .unwrap();
        click(view, &mut cx, "command.choice.0");
        view.read_with(&mut cx, |this, _| {
            let task = this.db.task(1).unwrap().unwrap();
            assert_eq!(task.assignee_count, 3);
            assert_eq!(task.extra["assignees"][2]["agent"]["id"], "agent-a");
        })
        .unwrap();
        click(view, &mut cx, "command.choice.0");
        view.read_with(&mut cx, |this, _| {
            assert_eq!(this.db.task(1).unwrap().unwrap().assignee_count, 2);
            assert_eq!(
                this.db.mutations().unwrap()[3].payload_json["change"]["AssignAgent"]["active"],
                false
            );
        })
        .unwrap();
    }
}

#[cfg(test)]
mod plain_paste_tests {
    use super::*;
    use crate::ui::task_fields::tests::desktop;
    #[gpui::test]
    fn plain_paste_key_replaces_selection_without_link_wrapping_and_keeps_unicode_and_newlines(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(crate::input::init);
        let (view, cx) = cx.add_window_view(desktop);
        cx.simulate_keystrokes("ctrl-m");
        cx.simulate_input("Selected");
        cx.simulate_keystrokes("ctrl-a");
        view.update_in(cx, |_, _, cx| {
            cx.write_to_clipboard(gpui::ClipboardItem::new_string(
                "https://example.org\r\né🙂".into(),
            ))
        });
        cx.simulate_keystrokes("ctrl-shift-v");
        view.read_with(cx, |this, cx| {
            let input = this.comment_input.read(cx);
            assert_eq!(input.content.as_ref(), "https://example.org\né🙂");
            assert!(input.html().contains("https://example.org"));
            assert!(!input.html().contains("Selected"));
            assert!(this.db.mutations().unwrap().is_empty());
        });
        cx.simulate_keystrokes("ctrl-z");
        view.read_with(cx, |this, cx| {
            assert_eq!(this.comment_input.read(cx).content.as_ref(), "Selected")
        });
    }
}
