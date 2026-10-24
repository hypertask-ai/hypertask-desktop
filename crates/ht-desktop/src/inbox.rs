use super::*;
use crate::shortcuts::Action;
use chrono::{Datelike, TimeZone, Timelike};
use gpui::{KeyDownEvent, MouseButton};

pub(super) struct InboxActions {
    search: Entity<TextInput>,
    pub search_text: String,
    searching: bool,
    split_ids: Option<Vec<std::collections::BTreeSet<i64>>>,
    pub review: Option<super::inbox_review::Review>,
    reminder_query: Entity<TextInput>,
    reminder: Option<Reminder>,
}
struct Reminder {
    ids: Vec<i64>,
    dates: Vec<(String, String)>,
    choice: usize,
}
impl InboxActions {
    pub fn new(cx: &mut Context<Desktop>) -> Self {
        let search = cx.new(|cx| TextInput::new(String::new(), "Find in view...", color(0x8e9093), cx));
        cx.observe(&search, |this, input, cx| {
            this.inbox_actions.search_text = input.read(cx).content.trim().to_lowercase();
            this.selected = 0;
            this.filter_inbox();
            cx.notify();
        }).detach();
        let reminder_query = cx.new(|cx| TextInput::new(String::new(), "Remind me at (ISO date/time)", color(0x8e9093), cx));
        cx.observe(&reminder_query, |this, _, cx| {
            if let Some(reminder) = &mut this.inbox_actions.reminder { reminder.choice = 0; }
            cx.notify();
        }).detach();
        Self { search, search_text: String::new(), searching: false, split_ids: None, review: None, reminder_query, reminder: None }
    }
}
pub(super) fn search_matches(n: &InboxItem, query: &str) -> bool {
    query.is_empty() || n.title.to_lowercase().contains(query) || n.ticket_number.as_deref().unwrap_or("").to_lowercase().contains(query)
}
fn reminder_dates(now: chrono::DateTime<chrono::Local>) -> Vec<(String, String)> {
    let today = now.date_naive();
    let hour = now.hour();
    let (label, day, hour) = if hour < 9 { ("Later today", today, 9) } else if hour < 17 { ("Later today", today, 17) } else if hour < 21 { ("Later today", today, 21) } else { ("Tonight", today + chrono::Duration::days(1), 0) };
    let monday = 7 - today.weekday().num_days_from_monday() as i64;
    [(label, day, hour), ("Tomorrow", today + chrono::Duration::days(1), 9), ("In 2 days", today + chrono::Duration::days(2), 9), ("Next week monday", today + chrono::Duration::days(monday), 9), ("In 2 weeks", today + chrono::Duration::days(14), 9)]
        .into_iter().filter_map(|(label, date, hour)| {
            chrono::Local.from_local_datetime(&date.and_hms_opt(hour, 0, 0)?).earliest()
                .map(|time| (label.to_owned(), time.to_rfc3339()))
        }).collect()
}

impl Desktop {
    pub(super) fn mark_opened_notification(&mut self, task_id: i64, cx: &mut Context<Self>) {
        if self.screen != Screen::Inbox && !(self.screen == Screen::Task && self.previous == Screen::Inbox) { return; }
        let notification = if self.screen == Screen::Inbox {
            self.filtered.get(self.selected).map(|&i| &self.inbox[i]).filter(|n| n.task_id == Some(task_id))
        } else { self.inbox.iter().find(|n| n.task_id == Some(task_id)) };
        if let Some(id) = notification.filter(|n| !n.seen).map(|n|n.id) {
            self.enqueue(Write::InboxSeen { notification_id:id }, cx);
        }
    }
    pub(super) fn reload_inbox_tabs(&mut self) {
        if let Ok(Some(tabs))=self.db.inbox_splits() {
            self.splits=tabs.iter().filter_map(|tab|tab["project"].as_str().map(str::to_owned)).collect();
            self.inbox_actions.split_ids=Some(tabs.iter().map(|tab|tab["ids"].as_array().into_iter().flatten().filter_map(|id|id.as_i64()).collect()).collect());
        } else {
            self.inbox_actions.split_ids=None;
            self.splits=vec!["Important".into(),"Agents".into()];
            for n in &self.inbox {
                if let Some(title)=n.raw_json.pointer("/project/title").and_then(|v|v.as_str()) {
                    if !self.splits.iter().any(|s|s==title) {self.splits.push(title.into());}
                }
            }
            self.splits[2..].sort_unstable();self.splits.push("All".into());
        }
        if self.splits.is_empty() {self.splits.push("All".into());self.inbox_actions.split_ids=None;}
        self.split=self.split.min(self.splits.len().saturating_sub(1));self.filter_inbox();
    }
    pub(super) fn inbox_split_matches(&self,n:&InboxItem,index:usize)->bool {
        if let Some(ids)=&self.inbox_actions.split_ids {return ids.get(index).is_some_and(|ids|ids.contains(&n.id));}
        self.splits.get(index).is_some_and(|name|Self::split_matches(n,name))
    }
    pub(super) fn task_inbox_ids(&self) -> Vec<i64> {
        self.db.inbox().unwrap_or_default().iter().filter(|n|n.task_id==Some(self.options.task)).map(|n|n.id).collect()
    }
    pub(super) fn show_inbox_reminder(&mut self, ids: Vec<i64>, window: &mut Window, cx: &mut Context<Self>) {
        if ids.is_empty() { return; }
        let mut dates = reminder_dates(chrono::Local::now());
        if let Ok(Some(last)) = self.db.kv_get("inbox-last-reminder") {
            if chrono::DateTime::parse_from_rfc3339(&last).is_ok_and(|at| at.timestamp() > chrono::Utc::now().timestamp()) {
                dates.insert(0, ("last used".into(), last));
            }
        }
        self.inbox_actions.reminder = Some(Reminder { ids, dates, choice:0 });
        self.inbox_actions.reminder_query.update(cx, |input,cx|input.set_content(String::new(),cx));
        window.focus(&self.inbox_actions.reminder_query.focus_handle(cx),cx);
        cx.notify();
    }
    fn reminder_choices(&self, cx: &App) -> Vec<(String,String)> {
        let Some(reminder) = &self.inbox_actions.reminder else { return vec![]; };
        let query = self.inbox_actions.reminder_query.read(cx).content.trim();
        if let Ok(time) = chrono::DateTime::parse_from_rfc3339(query) {
            return vec![(query.into(),time.to_rfc3339())];
        }
        reminder.dates.iter().filter(|(name,_)|name.to_lowercase().starts_with(&query.to_lowercase())).cloned().collect()
    }
    fn choose_inbox_reminder(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some((_,remind_at))=self.reminder_choices(cx).get(index).cloned() else {return;};
        let Some(reminder)=self.inbox_actions.reminder.as_ref() else {return;};
        let ids=reminder.ids.clone();
        for chunk in ids.chunks(100) {
            if !self.enqueue(Write::InboxRemind{notification_ids:chunk.to_vec(),remind_at:remind_at.clone()},cx) {return;}
            for id in chunk {self.interaction.bulk.remove(id);}
            if let Some(reminder)=&mut self.inbox_actions.reminder {reminder.ids.retain(|id|!chunk.contains(id));}
        }
        let _=self.db.kv_set("inbox-last-reminder",&remind_at);
        self.inbox_actions.reminder=None;
        window.focus(&self.focus,cx);
        if self.screen==Screen::Task && self.previous==Screen::Inbox {self.playlist_step(1,cx);}
    }
    pub(super) fn inbox_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let key=&event.keystroke;
        let ctrl=key.modifiers.control || key.modifiers.platform;
        if self.inbox_actions.reminder.is_some() {
            let count=self.reminder_choices(cx).len();
            let reminder=self.inbox_actions.reminder.as_mut().unwrap();
            match key.key.as_str() {
                "escape"=>{self.inbox_actions.reminder=None;window.focus(&self.focus,cx);},
                "down"=>reminder.choice=if count==0{0}else{(reminder.choice+1)%count},
                "up"=>reminder.choice=if count==0{0}else{(reminder.choice+count-1)%count},
                "enter"=>{let index=reminder.choice;self.choose_inbox_reminder(index,window,cx);},
                _=>return true,
            }
            cx.stop_propagation();cx.notify();return true;
        }
        if self.screen!=Screen::Inbox {return false;}
        if self.inbox_actions.search.focus_handle(cx).is_focused(window) {
            if key.key=="escape" {
                self.inbox_actions.searching=false;
                self.inbox_actions.search_text.clear();
                self.inbox_actions.search.update(cx,|input,cx|input.set_content(String::new(),cx));
                self.filter_inbox();window.focus(&self.focus,cx);cx.stop_propagation();cx.notify();
            } else if ctrl && key.key=="f" {cx.stop_propagation();}
            return true;
        }
        if ctrl && key.key=="f" && !key.modifiers.alt && !key.modifiers.shift && self.interaction.modal.is_none() {
            self.inbox_actions.searching=true;
            window.focus(&self.inbox_actions.search.focus_handle(cx),cx);cx.stop_propagation();cx.notify();return true;
        }
        false
    }
    fn inbox_selection_bar(&self, cx: &mut Context<Self>) -> AnyElement {
        let t=self.colors;
        div().flex().items_center().gap(px(16.)).px(px(24.)).py(px(8.))
            .child(format!("{} selected",self.interaction.bulk.len()))
            .child(div().id("inbox-bulk-archive").cursor_pointer().child("Archive ⇧E").on_click(cx.listener(|this,_,window,cx|this.run_shortcut(Action::BulkArchive,window,cx))).layout_key(&self.layout,"inbox.bulk.archive",None))
            .child(div().id("inbox-bulk-remind").cursor_pointer().child("Remind ⇧H").on_click(cx.listener(|this,_,window,cx|this.show_inbox_reminder(this.interaction.bulk.iter().copied().collect(),window,cx))).layout_key(&self.layout,"inbox.bulk.remind",None))
            .text_color(color(t.text)).layout_key(&self.layout,"inbox.bulk",None).into_any_element()
    }
    pub(super) fn inbox_reminder_overlay(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(reminder)=&self.inbox_actions.reminder else{return div().into_any_element();};
        let t=self.colors;
        div().id("inbox-reminder-backdrop").absolute().size_full().top_0().left_0().bg(rgba(0x00000080)).flex().items_center().justify_center()
            .on_mouse_down(MouseButton::Left,cx.listener(|_,_,_,cx|cx.stop_propagation()))
            .on_click(cx.listener(|this,_,window,cx|{this.inbox_actions.reminder=None;window.focus(&this.focus,cx);cx.notify();}))
            .child(div().id("inbox-reminder-dialog").w(px(430.)).p(px(16.)).flex().flex_col().gap(px(8.)).bg(color(t.comment)).text_color(color(t.text)).rounded(px(5.)).shadow(shadow())
                .on_click(cx.listener(|_,_,_,cx|cx.stop_propagation()))
                .child("Remind · Regardless")
                .child(self.inbox_actions.reminder_query.clone())
                .children(self.reminder_choices(cx).into_iter().enumerate().map(|(i,(label,date))|{
                    div().id(("reminder-time",i)).p(px(8.)).cursor_pointer().when(i==reminder.choice,|d|d.bg(color(t.hover)))
                        .child(label).child(div().text_size(px(12.)).text_color(color(t.muted)).child(date))
                        .on_click(cx.listener(move|this,_,window,cx|this.choose_inbox_reminder(i,window,cx)))
                        .layout_key(&self.layout,format!("inbox.reminder.{i}"),None)
                }))
                .layout_key(&self.layout,"inbox.reminder",None))
            .into_any_element()
    }
    pub(super) fn inbox_screen(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let t = self.colors;
        let tabs = self.splits.iter().enumerate().map(|(i, name)| {
            let count = self
                .inbox
                .iter()
                .filter(|n| self.inbox_split_matches(n, i))
                .count();
            div()
                .id(("split", i))
                .cursor_pointer()
                .flex()
                .h(px(32.))
                .text_size(px(14.))
                .pr(px(15.))
                .items_center()
                .gap(px(4.))
                .font_weight(FontWeight::NORMAL)
                .text_color(color(t.text))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.split = i;
                    this.selected = 0;
                    this.filter_inbox();
                    cx.notify();
                }))
                .child(
                    div()
                        .flex()
                        .items_baseline()
                        .gap(px(4.))
                        .font_weight(if i == self.split {
                            FontWeight::SEMIBOLD
                        } else {
                            FontWeight::NORMAL
                        })
                        .text_color(color(if i == self.split { 0xf3f4f6 } else { t.muted }))
                        .child(name.clone())
                        .child(
                            div()
                                .text_size(px(11.))
                                .font_weight(FontWeight::NORMAL)
                                .text_color(color(
                                    if matches!(
                                        self.options.theme,
                                        ThemeName::Graphite | ThemeName::Amoled
                                    ) {
                                        0xd1d5dc
                                    } else {
                                        0x4b5563
                                    },
                                ))
                                .child(count.to_string()),
                        ),
                )
                .when(i + 1 == self.splits.len(), |d| {
                    d.child(div().px(px(4.)).child(icon("settings", t.muted, 14.)))
                })
                .layout_key(
                    &self.layout,
                    format!("inbox.tab.{i}"),
                    Some(format!("{name} {count}")),
                )
        });
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .bg(color(t.inbox_page))
            .items_center()
            .child(
                div()
                    .w_full()
                    .max_w(px(1440.))
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .h(px(80.))
                            .flex_shrink_0()
                            .w_full()
                            .min_w_0()
                            .pl(px(64.))
                            .pr(px(32.))
                            .pt(px(24.))
                            .pb(px(24.))
                            .flex()
                            .gap(px(9.))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .h(px(32.))
                                    .flex()
                                    .items_center()
                                    .gap(px(9.))
                                    .children(tabs)
                                    .layout_key(&self.layout, "inbox.tabs", None),
                            )
                            .child(div()),
                    )
                    .child(
                        div()
                            .mx(px(12.))
                            .mb(px(12.))
                            .flex_1()
                            .min_h_0()
                            .bg(color(t.inbox))
                            .rounded(px(6.))
                            .when(self.options.theme != ThemeName::Amoled, |d| {
                                d.shadow(vec![
                                    gpui::BoxShadow {
                                        inset: false,
                                        color: rgba(0x0000001a).into(),
                                        offset: point(px(0.), px(4.)),
                                        blur_radius: px(6.),
                                        spread_radius: px(-1.),
                                    },
                                    gpui::BoxShadow {
                                        inset: false,
                                        color: rgba(0x0000001a).into(),
                                        offset: point(px(0.), px(2.)),
                                        blur_radius: px(4.),
                                        spread_radius: px(-2.),
                                    },
                                ])
                            })
                            .flex()
                            .flex_col()
                            .overflow_hidden()
                            .child(
                                div()
                                    .h(px(46.))
                                    .flex_shrink_0()
                                    .pl(px(52.))
                                    .pt(px(24.))
                                    .text_size(px(12.))
                                    .line_height(px(14.4))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(color(0x95999e))
                                    .when(self.options.theme == ThemeName::Dia, |d| {
                                        d.font_family("Newsreader").italic()
                                    })
                                    .child(
                                        div().flex().child(
                                            div()
                                                .child(inbox_date(
                                                    self.filtered.first().map(|&i| {
                                                        self.inbox[i].created_at.as_str()
                                                    }),
                                                ))
                                                .layout_key(
                                                    &self.layout,
                                                    "inbox.section",
                                                    Some(inbox_date(self.filtered.first().map(
                                                        |&i| self.inbox[i].created_at.as_str(),
                                                    ))),
                                                ),
                                        ),
                                    ),
                            )
                            .when(self.inbox_actions.searching, |d| d.child(div().px(px(24.)).py(px(8.)).child(self.inbox_actions.search.clone()).layout_key(&self.layout,"inbox.search",None)))
                            .when(!self.interaction.bulk.is_empty(), |d|d.child(self.inbox_selection_bar(cx)))
                            .when(self.filtered.is_empty(), |d| {
                                d.child(
                                    div()
                                        .flex_1()
                                        .flex()
                                        .flex_col()
                                        .items_center()
                                        .justify_center()
                                        .gap(px(12.))
                                        .text_color(color(t.muted))
                                        .child(icon("inbox", t.muted, 32.))
                                        .child(if self.inbox_actions.search_text.is_empty() { "You're all caught up" } else { "No results" }),
                                )
                            })
                            .child(
                                uniform_list(
                                    "notifications",
                                    self.filtered.len(),
                                    cx.processor(|this, range: std::ops::Range<usize>, _, cx| {
                                        range.map(|i| this.inbox_row(i, cx)).collect::<Vec<_>>()
                                    }),
                                )
                                .track_scroll(&self.inbox_scroll)
                                .flex_1()
                                .min_h_0(),
                            ),
                    ),
            )
    }

    fn inbox_row(&self, row: usize, cx: &mut Context<Self>) -> AnyElement {
        let n = &self.inbox[self.filtered[row]];
        let t = self.colors;
        let id = n.id;
        let kind = n
            .raw_json
            .pointer("/comment/text")
            .and_then(|v| v.as_str())
            .map(|html| RichText::parse(html, t).text.to_string())
            .filter(|text| !text.trim().is_empty())
            .unwrap_or_else(|| notification_label(n.raw_json["type"].as_str().unwrap_or("")));
        let metadata = inbox_metadata(&n.raw_json["task"]);
        let source = n.source.clone().unwrap_or_default();
        let source = match n.raw_json["unreadCount"]
            .as_u64()
            .filter(|count| *count > 0)
        {
            Some(count) => format!("{source} ({count})"),
            None => source,
        };
        div()
            .id(("notification", id as u64))
            .w_full()
            .h(px(36.))
            .flex()
            .items_center()
            .border_l(px(4.))
            .border_color(gpui::transparent_black())
            .pl(px(48.))
            .pr(px(48.))
            .cursor_pointer()
            .when(
                row == self.selected || self.interaction.bulk.contains(&id),
                |d| {
                    d.border_color(color(if self.options.theme == ThemeName::Porcelain {
                        0x9a9daa
                    } else {
                        t.accent
                    }))
                    .bg(color(
                        if self.options.theme == ThemeName::Porcelain {
                            0xe2e2e6
                        } else if self.options.theme == ThemeName::Dia {
                            t.active
                        } else {
                            t.footer
                        },
                    ))
                },
            )
            .hover(|d| d.bg(color(t.hover)))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.selected = row;
                if let Some(id) = this.selected_task() {
                    this.open_task(id, cx);
                }
            }))
            .child(
                div()
                    .w_full()
                    .h(px(36.))
                    .flex()
                    .items_center()
                    .gap(px(32.))
                    .child(
                        div()
                            .text_size(px(13.))
                            .line_height(px(19.5))
                            .w(gpui::relative(0.15))
                            .min_w_0()
                            .flex_shrink_0()
                            .flex()
                            .items_center()
                            .relative()
                            .child(
                                div()
                                    .absolute()
                                    .left(px(-8.))
                                    .size(px(7.))
                                    .rounded_full()
                                    .when(!n.seen, |d| d.bg(color(t.accent))),
                            )
                            .child(div().truncate().child(source.clone()).layout_key(
                                &self.layout,
                                format!("inbox.row.{row}.author"),
                                Some(source.clone()),
                            )),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .line_height(px(19.5))
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .items_center()
                            .gap(px(8.))
                            .overflow_hidden()
                            .child(
                                div()
                                    .flex()
                                    .items_baseline()
                                    .gap(px(4.))
                                    .min_w(px(180.))
                                    .child(
                                        div()
                                            .flex()
                                            .flex_shrink_0()
                                            .text_color(color(0x999a9d))
                                            .text_size(px(11.))
                                            .line_height(px(16.5))
                                            .child(n.ticket_number.clone().unwrap_or_default())
                                            .child(" ")
                                            .layout_key(
                                                &self.layout,
                                                format!("inbox.row.{row}.taskid"),
                                                n.ticket_number.clone(),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .min_w_0()
                                            .truncate()
                                            .child(n.title.clone())
                                            .layout_key(
                                                &self.layout,
                                                format!("inbox.row.{row}.title"),
                                                Some(n.title.clone()),
                                            ),
                                    ),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w(px(90.))
                                    .text_color(color(0x8e9093))
                                    .truncate()
                                    .child(kind.clone())
                                    .layout_key(
                                        &self.layout,
                                        format!("inbox.row.{row}.kind"),
                                        Some(kind),
                                    ),
                            )
                            .when(!metadata.is_empty(), |d| {
                                d.child(
                                    div()
                                        .flex_shrink_0()
                                        .max_w(px(180.))
                                        .overflow_hidden()
                                        .whitespace_nowrap()
                                        .text_size(px(11.))
                                        .text_color(color(t.muted))
                                        .child(metadata.clone())
                                        .layout_key(
                                            &self.layout,
                                            format!("inbox.row.{row}.meta"),
                                            Some(metadata),
                                        ),
                                )
                            }),
                    )
                    .child(
                        div()
                            .w(px(62.))
                            .flex_shrink_0()
                            .flex()
                            .items_center()
                            .justify_end()
                            .gap(px(10.))
                            .pr(px(4.))
                            .child(
                                div()
                                    .id(("archive-notification", id as u64))
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        cx.stop_propagation();
                                        this.enqueue(
                                            Write::InboxArchive {
                                                notification_id: id,
                                            },
                                            cx,
                                        );
                                    }))
                                    .child(icon("archive-x", 0x696b6e, 18.))
                                    .layout_key(&self.layout,format!("inbox.row.{row}.archive"),None),
                            )
                            .child(div().id(("remind-notification",id as u64)).on_click(cx.listener(move|this,_,window,cx|{
                                cx.stop_propagation();
                                this.show_inbox_reminder(vec![id],window,cx);
                            })).child(icon("clock",0x696b6e,18.)).layout_key(&self.layout,format!("inbox.row.{row}.remind"),None))
                            .layout_key(&self.layout, format!("inbox.row.{row}.actions"), None),
                    )
                    .layout_key(&self.layout, format!("inbox.row.{row}"), None),
            )
            .into_any_element()
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::shell::tests::{click,draw};
    use gpui::{VisualTestContext,WindowHandle};
    use serde_json::json;

    fn desktop(window: &mut Window, cx: &mut Context<Desktop>) -> Desktop {
        let mut db=Db::memory().unwrap();
        db.kv_set("current-user-id","42").unwrap();
        db.apply_task(&json!({"id":1,"projectId":15,"title":"Needle","ticketNumber":"HT-1","status":"Normal"})).unwrap();
        db.apply_task(&json!({"id":2,"projectId":15,"title":"Other","ticketNumber":"HT-2","status":"Normal"})).unwrap();
        let options=Options::parse(["--offline","--screen","inbox","--task","1","--board","15"].map(String::from)).unwrap();
        let view=Desktop::new(db,options,window,cx);window.focus(&view.focus,cx);view
    }
    fn seed(view: &mut Desktop) {
        view.db.apply_response(&json!({"success":true,"user_notifications":[
            {"id":101,"taskId":1,"projectId":15,"status":"Normal","seen":false,"createdAt":"2026-01-01T00:00:00Z","task":{"title":"Needle","ticketNumber":"HT-1"}},
            {"id":102,"taskId":2,"projectId":15,"status":"Normal","seen":false,"createdAt":"2026-01-02T00:00:00Z","task":{"title":"Other","ticketNumber":"HT-2"}}
        ]})).unwrap();
        view.screen=Screen::Inbox;view.reload();view.split=view.splits.iter().position(|s|s=="All").unwrap();view.filter_inbox();
    }
    #[gpui::test]
    fn inbox_metadata_shares_content_gap_without_an_empty_outer_column(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(crate::input::init);
        for priority in [String::new(), "High".to_owned(), "High priority ".repeat(30)] {
            let has_metadata = !priority.is_empty();
            let view: WindowHandle<Desktop> = cx.add_window(|window, cx| {
                let mut v = desktop(window, cx);
                seed(&mut v);
                v.db.apply_response(&json!({"success":true,"user_notifications":[{
                    "id":102,"taskId":2,"projectId":15,"status":"Normal","seen":true,
                    "createdAt":"2026-01-02T00:00:00Z","type":"TaskReminder",
                    "task":{"title":"Other","ticketNumber":"HT-2","priority":
                        if has_metadata { json!({"Priority_Value":priority}) } else { json!(null) }
                    }
                }]})).unwrap();
                v.reload();
                v.layout = Some(Rc::new(RefCell::new(LayoutDump::default())));
                v.ready_scheduled = true;
                v
            });
            let mut cx = VisualTestContext::from_window(view.into(), cx);
            draw(view, &mut cx);
            view.read_with(&mut cx, |this, _| {
                let elements = this.layout.as_ref().unwrap().borrow().interaction_elements();
                let kind = elements.iter().find(|e| e["key"] == "inbox.row.0.kind").unwrap();
                let actions = elements.iter().find(|e| e["key"] == "inbox.row.0.actions").unwrap();
                let meta = elements.iter().find(|e| e["key"] == "inbox.row.0.meta");
                let kind_right = kind["x"].as_f64().unwrap() + kind["w"].as_f64().unwrap();
                let content_right = if has_metadata {
                    let meta = meta.expect("nonempty metadata must be rendered");
                    assert_eq!(meta["text"], priority.chars().take(40).collect::<String>());
                    assert!(meta["w"].as_f64().unwrap() <= 180.);
                    assert!(meta["h"].as_f64().unwrap() <= 18., "metadata must stay on one line");
                    if priority.len() > 40 {
                        assert_eq!(meta["w"], 180.);
                    }
                    assert!((meta["x"].as_f64().unwrap() - kind_right - 8.).abs() <= 1.);
                    meta["x"].as_f64().unwrap() + meta["w"].as_f64().unwrap()
                } else {
                    assert!(meta.is_none(), "empty metadata must not consume a flex gap");
                    kind_right
                };
                assert!((actions["x"].as_f64().unwrap() - content_right - 32.).abs() <= 1.);
                assert!(this.db.mutations().unwrap().is_empty());
            }).unwrap();
        }
    }

    #[gpui::test]
    fn inbox_real_search_filters_title_ticket_and_escape_restores_split_without_navigation(cx: &mut gpui::TestAppContext) {
        cx.update(crate::input::init);
        let (view,cx)=cx.add_window_view(|window,cx|{let mut v=desktop(window,cx);seed(&mut v);v});
        view.update_in(cx,|this,window,cx|window.focus(&this.focus,cx));
        cx.simulate_keystrokes("ctrl-f");cx.run_until_parked();cx.simulate_input("HT-2");cx.run_until_parked();
        view.read_with(cx,|this,cx|{assert_eq!(this.inbox_actions.search.read(cx).content.as_ref(),"HT-2");assert_eq!(this.inbox_actions.search_text,"ht-2");assert_eq!(this.filtered.len(),1);assert_eq!(this.inbox[this.filtered[0]].id,102);assert!(this.interaction.modal.is_none());});
        cx.simulate_keystrokes("escape");
        view.read_with(cx,|this,_|{assert_eq!(this.screen,Screen::Inbox);assert_eq!(this.splits[this.split],"All");assert_eq!(this.filtered.len(),2);assert!(!this.inbox_actions.searching);assert!(this.db.mutations().unwrap().is_empty());});
        cx.simulate_keystrokes("ctrl-f");cx.run_until_parked();cx.simulate_input("nEeDlE");cx.run_until_parked();
        view.read_with(cx,|this,_|{assert_eq!(this.filtered.len(),1);assert_eq!(this.inbox[this.filtered[0]].id,101);});
    }
    #[gpui::test]
    fn inbox_open_marks_only_target_seen_and_j_k_e_and_tabs_keep_keyboard_flow(cx: &mut gpui::TestAppContext) {
        cx.update(crate::input::init);
        let (view,cx)=cx.add_window_view(|window,cx|{let mut v=desktop(window,cx);seed(&mut v);v});
        view.update_in(cx,|this,window,cx|window.focus(&this.focus,cx));
        cx.simulate_keystrokes("j");
        view.read_with(cx,|this,_|assert_eq!(this.selected,1));
        cx.simulate_keystrokes("k");
        view.read_with(cx,|this,_|assert_eq!(this.selected,0));
        cx.simulate_keystrokes("shift-tab tab");
        view.read_with(cx,|this,_|assert_eq!(this.splits[this.split],"All"));
        cx.simulate_keystrokes("enter");
        view.read_with(cx,|this,_|{assert_eq!(this.screen,Screen::Task);let rows=this.db.mutations().unwrap();assert_eq!(rows.len(),1);assert_eq!(rows[0].kind,"inbox_seen");assert_eq!(rows[0].payload_json["notification_ids"],json!([102]));});
    }
    #[gpui::test]
    fn inbox_row_reminder_targets_clicked_row_cancel_is_not_a_write_and_bulk_is_captured(cx: &mut gpui::TestAppContext) {
        cx.update(crate::input::init);
        let view:WindowHandle<Desktop>=cx.add_window(|window,cx|{let mut v=desktop(window,cx);seed(&mut v);v.layout=Some(Rc::new(RefCell::new(LayoutDump::default())));v.ready_scheduled=true;v});
        let mut cx=VisualTestContext::from_window(view.into(),cx);draw(view,&mut cx);
        click(view,&mut cx,"inbox.row.1.remind");
        view.read_with(&mut cx,|this,_|{assert_eq!(this.inbox_actions.reminder.as_ref().unwrap().ids,vec![101]);assert_eq!(this.screen,Screen::Inbox);assert!(this.db.mutations().unwrap().is_empty());}).unwrap();
        cx.simulate_keystrokes("escape");
        view.read_with(&mut cx,|this,_|assert!(this.db.mutations().unwrap().is_empty())).unwrap();
        cx.simulate_keystrokes("x j x shift-h");
        view.read_with(&mut cx,|this,_|assert_eq!(this.inbox_actions.reminder.as_ref().unwrap().ids,vec![101,102])).unwrap();
        cx.simulate_keystrokes("enter");
        view.read_with(&mut cx,|this,_|{let rows=this.db.mutations().unwrap();assert_eq!(rows.len(),1);assert_eq!(rows[0].kind,"inbox_remind");assert_eq!(rows[0].payload_json["notification_ids"],json!([101,102]));assert!(this.interaction.bulk.is_empty());assert!(this.inbox.is_empty());}).unwrap();
    }
    #[gpui::test]
    fn inbox_row_archive_targets_clicked_row_and_reply_shortcut_uses_remaining_focused_task(cx:&mut gpui::TestAppContext) {
        cx.update(crate::input::init);
        let view:WindowHandle<Desktop>=cx.add_window(|window,cx|{let mut v=desktop(window,cx);seed(&mut v);v.layout=Some(Rc::new(RefCell::new(LayoutDump::default())));v.ready_scheduled=true;v});
        let mut cx=VisualTestContext::from_window(view.into(),cx);draw(view,&mut cx);
        click(view,&mut cx,"inbox.row.1.archive");
        view.read_with(&mut cx,|this,_|{assert_eq!(this.screen,Screen::Inbox);assert_eq!(this.inbox.len(),1);assert_eq!(this.inbox[0].id,102);let rows=this.db.mutations().unwrap();assert_eq!(rows.len(),1);assert_eq!(rows[0].payload_json["notification_ids"],json!([101]));}).unwrap();
        cx.simulate_keystrokes("ctrl-r");
        view.read_with(&mut cx,|this,cx|{assert_eq!(this.options.task,2);assert!(this.editing==Editing::Comment);assert_eq!(this.db.mutations().unwrap()[1].kind,"inbox_seen");assert!(!this.comment_input.read(cx).content.contains("101"));}).unwrap();
    }
    #[gpui::test]
    fn bulk_remind_over_one_hundred_rows_queues_bounded_requests_without_dropping_selection(cx:&mut gpui::TestAppContext) {
        cx.update(crate::input::init);
        let (view,cx)=cx.add_window_view(|window,cx|{let mut v=desktop(window,cx);seed(&mut v);v});
        view.update_in(cx,|this,window,cx|{
            let notifications=(1..=101).map(|id|json!({"id":id,"taskId":id+1000,"projectId":15,"status":"Normal","seen":false,"createdAt":"2026-01-01T00:00:00Z"})).collect::<Vec<_>>();
            this.db.apply_response(&json!({"success":true,"user_notifications":notifications})).unwrap();this.reload();
            this.interaction.bulk=(1..=101).collect();this.run_shortcut(Action::BulkRemind,window,cx);
        });
        cx.simulate_keystrokes("enter");
        view.read_with(cx,|this,_|{let rows=this.db.mutations().unwrap();assert_eq!(rows.len(),2);assert_eq!(rows.iter().map(|m|m.payload_json["notification_ids"].as_array().unwrap().len()).collect::<Vec<_>>(),vec![100,1]);assert!(this.interaction.bulk.is_empty());assert!(this.inbox_actions.reminder.is_none());});
    }
    #[test]
    fn reminder_presets_match_web_local_hours_week_boundary_and_two_week_nine_am() {
        for hour in [0,8,9,16,17,20,21,23] {
            let now=chrono::Local.with_ymd_and_hms(2026,1,4,hour,30,0).single().unwrap();
            let dates=reminder_dates(now);
            assert_eq!(dates.len(),5);
            let first=chrono::DateTime::parse_from_rfc3339(&dates[0].1).unwrap();
            assert!(first.timestamp()>now.timestamp());
            assert_eq!(first.hour(),if hour<9{9}else if hour<17{17}else if hour<21{21}else{0});
            assert_eq!(dates[0].0,if hour<21{"Later today"}else{"Tonight"});
            let monday=chrono::DateTime::parse_from_rfc3339(&dates[3].1).unwrap();
            assert_eq!(monday.weekday(),chrono::Weekday::Mon);assert_eq!(monday.hour(),9);
            let weeks=chrono::DateTime::parse_from_rfc3339(&dates[4].1).unwrap();
            assert_eq!((weeks.date_naive()-now.date_naive()).num_days(),14);assert_eq!(weeks.hour(),9);
        }
    }
}

#[cfg(test)]
mod composer_tests {
    use super::*;
    use crate::ui::shell::tests::draw;
    use gpui::{VisualTestContext,WindowHandle};
    use serde_json::json;
    #[gpui::test]
    fn desktop_composer_omits_phone_only_actions_but_task_shortcuts_keep_working(cx:&mut gpui::TestAppContext) {
        cx.update(crate::input::init);
        for snooze in [true,false] {
            let view:WindowHandle<Desktop>=cx.add_window(|window,cx|{
                let mut db=Db::memory().unwrap();db.kv_set("current-user-id","42").unwrap();
                db.apply_response(&json!({"success":true,"tasks":[{"id":1,"projectId":15,"title":"First","status":"Normal"},{"id":2,"projectId":15,"title":"Next","status":"Normal"}],"user_notifications":[{"id":101,"taskId":1,"projectId":15,"status":"Normal","seen":true,"createdAt":"2026-01-02T00:00:00Z"},{"id":102,"taskId":2,"projectId":15,"status":"Normal","seen":true,"createdAt":"2026-01-01T00:00:00Z"}]})).unwrap();
                let options=Options::parse(["--offline","--screen","inbox","--task","1","--board","15"].map(String::from)).unwrap();
                let mut v=Desktop::new(db,options,window,cx);v.split=v.splits.iter().position(|s|s=="All").unwrap();v.filter_inbox();v.open_task(1,cx);
                v.layout=Some(Rc::new(RefCell::new(LayoutDump::default())));v.ready_scheduled=true;v
            });
            let mut cx=VisualTestContext::from_window(view.into(),cx);draw(view,&mut cx);
            view.read_with(&mut cx,|this,_|{
                // NewCommentComponent.tsx:447-485 renders the desktop composer;
                // TaskOptions is only mounted in its phone branch at line 527.
                let elements=this.layout.as_ref().unwrap().borrow().interaction_elements();
                assert!(elements.iter().any(|e|e["key"]=="task.composer"));
                for key in ["task.composer.snooze","task.composer.remove-notification"] {
                    assert!(!elements.iter().any(|e|e["key"]==key), "phone-only action {key}");
                }
            }).unwrap();
            view.update(&mut cx,|this,window,cx|window.focus(&this.focus,cx)).unwrap();
            cx.simulate_keystrokes(if snooze{"h"}else{"e"});
            if snooze {
                view.read_with(&mut cx,|this,_|{assert_eq!(this.inbox_actions.reminder.as_ref().unwrap().ids,vec![101]);assert!(this.db.mutations().unwrap().is_empty());}).unwrap();
                cx.simulate_keystrokes("enter");
            }
            view.read_with(&mut cx,|this,_|{assert_eq!(this.options.task,2);let rows=this.db.mutations().unwrap();assert_eq!(rows.len(),1);assert_eq!(rows[0].kind,if snooze{"inbox_remind"}else{"inbox_archive"});assert_eq!(rows[0].payload_json["notification_ids"],json!([101]));}).unwrap();
        }
    }
}

#[cfg(test)]
mod split_tests {
    use super::*;
    use serde_json::json;
    #[gpui::test]
    fn inbox_tabs_use_server_memberships_counts_and_same_name_project_identity(cx:&mut gpui::TestAppContext) {
        cx.update(crate::input::init);
        let (view,cx)=cx.add_window_view(|window,cx|{
            let mut db=Db::memory().unwrap();db.kv_set("current-user-id","42").unwrap();
            db.apply_response(&json!({"success":true,"user_notifications":[{"id":1,"status":"Normal","seen":false,"createdAt":"2026-01-01T00:00:00Z"},{"id":2,"status":"Normal","seen":false,"createdAt":"2026-01-02T00:00:00Z"}],"structuredData":{"tabs":[{"project":"Same","projectId":15},{"project":"Same","projectId":16}],"data":[[0],[1]]}})).unwrap();
            let options=Options::parse(["--offline","--screen","inbox"].map(String::from)).unwrap();
            let v=Desktop::new(db,options,window,cx);window.focus(&v.focus,cx);v
        });
        view.read_with(cx,|this,_|{assert_eq!(this.splits,vec!["Same","Same"]);assert_eq!(this.filtered.len(),1);assert_eq!(this.inbox[this.filtered[0]].id,1);});
        cx.simulate_keystrokes("tab");
        view.read_with(cx,|this,_|assert_eq!(this.inbox[this.filtered[0]].id,2));
        view.update_in(cx,|this,_,cx|{this.enqueue(Write::InboxArchive{notification_id:2},cx);assert!(this.filtered.is_empty());});
        cx.simulate_keystrokes("shift-tab");
        view.read_with(cx,|this,_|assert_eq!(this.inbox[this.filtered[0]].id,1));
    }
}
