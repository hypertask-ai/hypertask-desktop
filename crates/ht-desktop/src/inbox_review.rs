use super::*;

pub(super) struct Review {
    token: std::time::Instant,
    rows: Vec<InboxItem>,
    decisions: Vec<bool>,
    origin: (Screen,Screen,i64,usize,usize),
}
impl Desktop {
    pub(super) fn start_unread_review(&mut self,window:&mut Window,cx:&mut Context<Self>) {
        let rows=self.db.inbox().unwrap_or_default().into_iter().filter(|n|!n.seen).collect();
        self.inbox_actions.review=Some(Review{token:std::time::Instant::now(),rows,decisions:vec![],origin:(self.screen,self.previous,self.options.task,self.split,self.selected)});
        self.show_review_task(cx);window.focus(&self.focus,cx);cx.notify();
    }
    fn show_review_task(&mut self,cx:&mut Context<Self>) {
        let Some(review)=&self.inbox_actions.review else{return;};
        if let Some(task)=review.rows.get(review.decisions.len()).and_then(|n|n.task_id) {
            if self.options.task!=task {self.comment_input.update(cx,|input,cx|input.set_content(String::new(),cx));}
            self.options.task=task;self.screen=Screen::Task;self.previous=Screen::Board;
            self.editing=Editing::None;self.reset_comment_actions();self.reload();self.request_sync();
        }
        cx.notify();
    }
    fn decide_unread(&mut self,archive:bool,window:&mut Window,cx:&mut Context<Self>) {
        let Some(review)=&mut self.inbox_actions.review else{return;};
        if review.decisions.len()>=review.rows.len() {return;}
        review.decisions.push(archive);
        let done=review.decisions.len()==review.rows.len();let count=review.decisions.len();let token=review.token;
        self.show_review_task(cx);window.focus(&self.focus,cx);
        if done {
            cx.spawn_in(window,async move|view,cx|{
                cx.background_executor().timer(Duration::from_millis(850)).await;
                let _=view.update_in(cx,|this,window,cx|{
                    if this.inbox_actions.review.as_ref().is_some_and(|r|r.token==token&&r.decisions.len()==count&&r.rows.len()==count){this.finish_unread_review(window,cx);}
                });
            }).detach();
        }
    }
    fn undo_unread(&mut self,window:&mut Window,cx:&mut Context<Self>) {
        if let Some(review)=&mut self.inbox_actions.review {review.decisions.pop();}
        self.show_review_task(cx);window.focus(&self.focus,cx);cx.notify();
    }
    fn finish_unread_review(&mut self,window:&mut Window,cx:&mut Context<Self>) {
        let Some(review)=self.inbox_actions.review.take() else{return;};
        let current=self.db.inbox().unwrap_or_default();
        let mut ids=std::collections::BTreeSet::new();
        for (row,archive) in review.rows.iter().zip(&review.decisions) {
            if !archive {continue;}
            ids.insert(row.id);
            if let Some(task)=row.task_id {ids.extend(current.iter().filter(|n|n.task_id==Some(task)).map(|n|n.id));}
        }
        for id in ids {if !self.enqueue(Write::InboxArchive{notification_id:id},cx) {break;}}
        (self.screen,self.previous,self.options.task,self.split,self.selected)=review.origin;
        self.editing=Editing::None;self.reset_comment_actions();self.reload();window.focus(&self.focus,cx);cx.notify();
    }
    pub(super) fn unread_review_key(&mut self,event:&KeyDownEvent,window:&mut Window,cx:&mut Context<Self>)->bool {
        if self.inbox_actions.review.is_none(){return false;}
        let key=&event.keystroke;let ctrl=key.modifiers.control||key.modifiers.platform;
        let editing=[&self.comment_input,&self.title_input,&self.comment_actions.input,&self.interaction.description].iter().any(|input|input.focus_handle(cx).is_focused(window));
        if key.key=="escape"&&editing {window.focus(&self.focus,cx);cx.stop_propagation();return true;}
        if key.key=="escape"||(ctrl&&key.key=="k"){self.finish_unread_review(window,cx);}
        else if editing {return false;}
        else if key.key=="backspace"||(ctrl&&key.key=="z"){self.undo_unread(window,cx);}
        else if !ctrl&&!key.modifiers.alt&&!key.modifiers.shift&&key.key=="right"{self.decide_unread(true,window,cx);}
        else if !ctrl&&!key.modifiers.alt&&!key.modifiers.shift&&key.key=="left"{self.decide_unread(false,window,cx);}
        else {return true;}
        cx.stop_propagation();true
    }
    pub(super) fn unread_review_screen(&self,cx:&mut Context<Self>)->AnyElement {
        let review=self.inbox_actions.review.as_ref().unwrap();let t=self.colors;
        let current=review.rows.get(review.decisions.len());
        div().id("unread-review").size_full().bg(color(t.page)).text_color(color(t.text)).font_family(self.options.theme.font()).text_size(px(14.)).flex().flex_col().items_center().track_focus(&self.focus)
            .capture_key_down(cx.listener(Self::key_down))
            .child(div().w_full().max_w(px(720.)).h(px(56.)).flex().items_center().justify_between()
                .child(div().id("review-close").cursor_pointer().child("Close").on_click(cx.listener(|this,_,window,cx|this.finish_unread_review(window,cx))).layout_key(&self.layout,"inbox.review.close",None))
                .child(format!("{} left",review.rows.len()-review.decisions.len()))
                .child(div().id("review-undo").cursor_pointer().child("Undo").on_click(cx.listener(|this,_,window,cx|this.undo_unread(window,cx))).layout_key(&self.layout,"inbox.review.undo",None)))
            .child(div().w_full().max_w(px(720.)).flex_1().min_h_0().flex().flex_col().child(if current.is_none(){div().size_full().flex().items_center().justify_center().child("All caught up").into_any_element()}else if current.unwrap().task_id.is_none(){div().child("This notification has no task to display").into_any_element()}else{self.task_screen(cx)}).layout_key(&self.layout,"inbox.review.task",None))
            .when(current.is_some(),|d|d.child(div().w_full().max_w(px(720.)).flex().gap(px(16.)).py(px(16.))
                .child(div().id("review-keep").flex_1().p(px(12.)).rounded_full().bg(color(t.hover)).cursor_pointer().child("← Keep in inbox").on_click(cx.listener(|this,_,window,cx|this.decide_unread(false,window,cx))).layout_key(&self.layout,"inbox.review.keep",None))
                .child(div().id("review-archive").flex_1().p(px(12.)).rounded_full().bg(color(t.accent)).cursor_pointer().child("Archive →").on_click(cx.listener(|this,_,window,cx|this.decide_unread(true,window,cx))).layout_key(&self.layout,"inbox.review.archive",None))))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[gpui::test]
    fn unread_review_keeps_archives_undoes_and_flushes_only_decisions_on_escape(cx:&mut gpui::TestAppContext) {
        cx.update(crate::input::init);
        let (view,cx)=cx.add_window_view(|window,cx|{
            let mut db=Db::memory().unwrap();db.apply_response(&json!({"success":true,"tasks":[{"id":1,"projectId":15,"title":"One","status":"Normal"},{"id":2,"projectId":15,"title":"Two","status":"Normal"}],"user_notifications":[{"id":101,"taskId":1,"projectId":15,"status":"Normal","seen":false,"createdAt":"2026-01-02T00:00:00Z"},{"id":102,"taskId":2,"projectId":15,"status":"Normal","seen":false,"createdAt":"2026-01-01T00:00:00Z"},{"id":103,"status":"Normal","seen":true,"createdAt":"2026-01-03T00:00:00Z"}]})).unwrap();
            let options=Options::parse(["--offline","--screen","inbox"].map(String::from)).unwrap();let mut v=Desktop::new(db,options,window,cx);v.start_unread_review(window,cx);v
        });
        view.read_with(cx,|this,_|{assert_eq!(this.inbox_actions.review.as_ref().unwrap().rows.len(),2);assert_eq!(this.options.task,1);assert!(this.db.mutations().unwrap().is_empty());});
        cx.simulate_keystrokes("right backspace left right ctrl-z");
        view.read_with(cx,|this,_|{assert_eq!(this.inbox_actions.review.as_ref().unwrap().decisions,vec![false]);assert!(this.db.mutations().unwrap().is_empty());});
        cx.simulate_keystrokes("right escape");
        view.read_with(cx,|this,_|{assert!(this.inbox_actions.review.is_none());assert_eq!(this.screen,Screen::Inbox);let rows=this.db.mutations().unwrap();assert_eq!(rows.len(),1);assert_eq!(rows[0].payload_json["notification_ids"],json!([102]));});
    }
}

#[cfg(test)]
mod layout_tests {
    use super::*;
    use crate::ui::shell::tests::{measured, setup};

    #[gpui::test]
    fn unread_embedded_task_columns_fit_the_review_card(cx: &mut gpui::TestAppContext) {
        let (view, mut cx) = setup(cx);
        view.update(&mut cx, |this, window, cx| {
            this.db.apply_response(&serde_json::json!({"success":true,"user_notifications":[
                {"id":77,"taskId":101,"projectId":15,"seen":false,"status":"Normal","createdAt":"2026-01-01T00:00:00Z"}
            ]})).unwrap();
            this.reload();
            this.start_unread_review(window, cx);
        }).unwrap();
        let card = measured(view, &mut cx, "inbox.review.task");
        assert_eq!(card["w"], 720.);
        let left = card["x"].as_f64().unwrap();
        let right = left + card["w"].as_f64().unwrap();
        for key in ["task.description", "task.composer", "task.meta.0", "task.meta.1", "task.meta.2", "task.meta.3", "task.meta.4", "task.meta.5"] {
            let field = measured(view, &mut cx, key);
            assert!(field["x"].as_f64().unwrap() >= left && field["x"].as_f64().unwrap() + field["w"].as_f64().unwrap() <= right,
                "{key} is horizontally clipped by unread review: {field}");
        }
    }

    #[gpui::test]
    fn unread_long_task_wheel_reaches_properties_inside_the_review_viewport(cx: &mut gpui::TestAppContext) {
        let (view, mut cx) = setup(cx);
        view.update(&mut cx, |this, window, cx| {
            let comments = (1..=24).map(|i| serde_json::json!({
                "id":i,"taskId":101,"creatorId":1,"text":format!("<p>Long review comment {i}</p>"),
                "createdAt":format!("2026-01-{i:02}T00:00:00Z")
            })).collect::<Vec<_>>();
            this.db.apply_comments(101, &serde_json::json!({"success":true,"comments":comments})).unwrap();
            this.db.apply_response(&serde_json::json!({"success":true,"user_notifications":[
                {"id":77,"taskId":101,"projectId":15,"seen":false,"status":"Normal","createdAt":"2026-01-02T00:00:00Z"},
                {"id":78,"taskId":102,"projectId":16,"seen":false,"status":"Normal","createdAt":"2026-01-01T00:00:00Z"}
            ]})).unwrap();
            this.reload();
            this.start_unread_review(window, cx);
        }).unwrap();
        let card = measured(view, &mut cx, "inbox.review.task");
        let top = card["y"].as_f64().unwrap();
        let bottom = top + card["h"].as_f64().unwrap();
        let before = measured(view, &mut cx, "task.meta.5");
        assert!(before["y"].as_f64().unwrap() > bottom, "long-thread fixture must require scrolling");
        cx.simulate_event(gpui::ScrollWheelEvent {
            position: gpui::point(px((card["x"].as_f64().unwrap() + 360.) as f32), px((top + 200.) as f32)),
            delta: gpui::ScrollDelta::Pixels(gpui::point(px(0.), px(-10000.))),
            touch_phase: gpui::TouchPhase::Moved,
            ..Default::default()
        });
        for i in 0..6 {
            let field = measured(view, &mut cx, &format!("task.meta.{i}"));
            let y = field["y"].as_f64().unwrap();
            assert!(y >= top && y + field["h"].as_f64().unwrap() <= bottom,
                "wheel did not reveal review property {i} inside the card: {field}");
        }
        cx.simulate_keystrokes("right backspace left escape");
        view.read_with(&cx, |this, _| {
            assert!(this.inbox_actions.review.is_none());
            assert!(this.db.mutations().unwrap().is_empty(), "undo then keep must not archive the reviewed task");
        }).unwrap();
    }

}
