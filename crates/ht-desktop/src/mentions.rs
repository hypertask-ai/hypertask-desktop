use super::*;
use gpui::MouseButton;
use serde_json::Value;
use std::ops::Range;

pub(super) struct Mentions {
    generation: u64,
    token: Option<(gpui::EntityId, i64, Range<usize>, String)>,
    input: Option<Entity<TextInput>>,
    rows: Vec<Value>,
    choice: usize,
    loading: bool,
    error: bool,
    dismissed: bool,
    scroll: ScrollHandle,
}
impl Mentions {
    pub fn new() -> Self {Self {generation:0,token:None,input:None,rows:vec![],choice:0,loading:false,error:false,dismissed:false,scroll:ScrollHandle::new()}}
    pub fn invalidate(&mut self) {self.generation+=1;self.token=None;self.input=None;self.rows.clear();self.loading=false;}
}
impl Desktop {
    pub(super) fn bind_mentions(&mut self, cx: &mut Context<Self>) {
        for input in [self.comment_input.clone(),self.comment_actions.input.clone(),self.interaction.description.clone()] {
            cx.observe(&input,|this,input,cx|this.observe_mention(input,cx)).detach();
        }
    }
    fn observe_mention(&mut self, input: Entity<TextInput>, cx: &mut Context<Self>) {
        if self.screen != Screen::Task || self.compose_open() {return;}
        let project=self.detail.as_ref().map_or(0,|d|d.task.project_id);
        let next=input.read(cx).mention_token().map(|(range,query)|(input.entity_id(),project,range,query));
        if self.mentions.token==next {return;}
        self.mentions.invalidate();
        self.mentions.token=next.clone();
        self.mentions.dismissed=false;
        self.mentions.choice=0;
        self.mentions.error=false;
        let Some((_,project,range,query))=next else{cx.notify();return;};
        self.mentions.input=Some(input.clone());
        self.mentions.loading=true;
        let generation=self.mentions.generation;
        let task=self.options.task;
        let user=self.db.current_user_id();
        let Some(token)=self.cli_token() else {self.mentions.loading=false;self.mentions.error=true;cx.notify();return;};
        cx.spawn(async move |view,cx| {
            cx.background_executor().timer(Duration::from_millis(180)).await;
            let current=view.update(cx,|this,_|this.mentions.generation==generation);
            if !matches!(current,Ok(true)) {return;}
            let (send,receive)=std::sync::mpsc::channel();
            std::thread::spawn(move|| {
                let result=(|| {
                    let http=ht_core::Http::new(token)?;
                    let request=ht_core::desktop_cli::mentions_request(project,&query);
                    let value=http.json_request("GET",&request.path,None)?;
                    anyhow::ensure!(value.is_array(),"invalid mention response");
                    Ok::<_,anyhow::Error>(value.as_array().unwrap().clone())
                })();
                let _=send.send(result.map_err(|_|()));
            });
            loop {
                match receive.try_recv() {
                    Ok(result)=> {
                        let _=view.update(cx,|this,cx| {
                            let valid=this.mentions.generation==generation && this.options.task==task && this.db.current_user_id()==user && this.cli_token().is_some() && input.read(cx).mention_token().is_some_and(|(r,_)|r==range);
                            if !valid {return;}
                            this.mentions.loading=false;
                            match result {Ok(rows)=>this.mentions.rows=rows,Err(())=>this.mentions.error=true}
                            this.mentions.choice=this.mention_selectable().first().copied().unwrap_or(0);
                            cx.notify();
                        });
                        break;
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty)=>cx.background_executor().timer(Duration::from_millis(50)).await,
                    Err(_)=>break,
                }
            }
        }).detach();
        cx.notify();
    }
    fn mention_selectable(&self) -> Vec<usize> {
        self.mentions.rows.iter().enumerate().filter(|(_,row)|ht_core::desktop_cli::mention_html(row).is_some()).map(|(i,_)|i).collect()
    }
    fn insert_mention_choice(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(input)=self.mentions.input.clone() else{return;};
        let Some((_,project,range,query))=self.mentions.token.clone() else{return;};
        if input.read(cx).mention_token()!=Some((range.clone(),query)) || self.detail.as_ref().is_none_or(|d|d.task.project_id!=project) {self.mentions.invalidate();return;}
        let Some(html)=self.mentions.rows.get(index).and_then(ht_core::desktop_cli::mention_html) else{return;};
        // Dismiss before the editor notification so insertion cannot reopen an old request.
        self.mentions.invalidate();
        input.update(cx,|input,cx|input.insert_mention(range,&html,window,cx));
        window.focus(&input.focus_handle(cx),cx);
        cx.notify();
    }
    pub(super) fn mention_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.mentions.dismissed || self.mentions.token.is_none() || !self.mentions.input.as_ref().is_some_and(|i|i.focus_handle(cx).is_focused(window)) {return false;}
        let key=&event.keystroke;
        if key.modifiers.control || key.modifiers.platform || key.modifiers.alt || key.modifiers.shift {return false;}
        match key.key.as_str() {
            "escape"=>self.mentions.dismissed=true,
            "up"|"down"=> {
                let rows=self.mention_selectable();
                if !rows.is_empty() {
                    let at=rows.iter().position(|i|*i==self.mentions.choice).unwrap_or(0);
                    self.mentions.choice=rows[if key.key=="down" {(at+1)%rows.len()}else{(at+rows.len()-1)%rows.len()}];
                    self.mentions.scroll.scroll_to_item(self.mentions.choice);
                }
            }
            "enter"=>self.insert_mention_choice(self.mentions.choice,window,cx),
            _=>return false,
        }
        cx.stop_propagation();cx.notify();true
    }
    pub(super) fn mention_overlay(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let Some(input)=&self.mentions.input else{return div().into_any_element();};
        if self.mentions.dismissed || self.mentions.token.is_none() || !input.focus_handle(cx).is_focused(window) || self.screen!=Screen::Task || self.compose_open() {return div().into_any_element();}
        let t=self.colors;
        let bounds=input.read(cx).input_bounds();
        let left=bounds.map_or(180.,|b|f32::from(b.left())).min((f32::from(window.viewport_size().width)-510.).max(0.));
        let top=bounds.map_or(100.,|b|(f32::from(b.top())-290.).max(0.));
        let mut menu=div().id("mention-menu").absolute().left(px(left)).top(px(top)).w(px(500.)).max_h(px(280.)).overflow_y_scroll().track_scroll(&self.mentions.scroll).bg(color(t.comment)).border_1().border_color(color(t.border)).rounded(px(5.)).shadow(shadow()).on_mouse_down(MouseButton::Left,cx.listener(|_,_,_,cx|cx.stop_propagation()));
        if self.mentions.loading {menu=menu.child(div().p(px(12.)).child("Loading mentions…"));}
        else if self.mentions.error {menu=menu.child(div().p(px(12.)).child("Error loading results"));}
        else if self.mention_selectable().is_empty() {menu=menu.child(div().p(px(12.)).child("No results found"));}
        else { for (i,row) in self.mentions.rows.iter().enumerate() {
            let selectable=ht_core::desktop_cli::mention_html(row).is_some();
            menu=menu.child(div().id(("mention-result",i)).px(px(12.)).py(px(8.)).when(!selectable,|d|d.text_size(px(11.)).text_color(color(t.muted))).when(selectable,|d|d.cursor_pointer().when(i==self.mentions.choice,|d|d.bg(color(t.hover))).on_mouse_down(MouseButton::Left,cx.listener(move|this,_,window,cx|{cx.stop_propagation();this.insert_mention_choice(i,window,cx);}))).child(row["name"].as_str().unwrap_or("").to_owned()).layout_key(&self.layout,format!("comment.mention.{i}"),None));
        }}
        menu.layout_key(&self.layout,"comment.mentions",None).into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn desktop(window:&mut Window,cx:&mut Context<Desktop>)->Desktop {
        let mut db=Db::memory().unwrap();db.kv_set("current-user-id","985").unwrap();
        db.apply_task(&json!({"id":1,"projectId":7283,"title":"QA","status":"Normal"})).unwrap();
        db.apply_comments(1,&json!({"success":true,"comments":[{"id":7,"creatorId":985,"text":"<p>Comment</p>","createdAt":"2026-01-01T00:00:00Z"}]})).unwrap();
        let options=Options::parse(["--offline","--screen","task","--task","1","--board","7283"].map(String::from)).unwrap();
        let view=Desktop::new(db,options,window,cx);window.focus(&view.focus,cx);view
    }
    fn results(this:&mut Desktop,cx:&mut Context<Desktop>) {
        let input=this.comment_input.clone();
        let (range,query)=input.read(cx).mention_token().unwrap();
        this.mentions.token=Some((input.entity_id(),7283,range,query));this.mentions.input=Some(input);
        this.mentions.loading=false;this.mentions.error=false;this.mentions.dismissed=false;this.mentions.choice=1;
        this.mentions.rows=vec![json!({"type":"peopleHeading","name":"People"}),json!({"type":"name","id":985,"name":"QA é🙂"}),json!({"type":"taskHeading","name":"Tasks"}),json!({"type":"task","id":5,"name":"Linked task","project_id":7283,"index":5,"ticketNumber":"QA-5"})];
        cx.notify();
    }
    #[gpui::test]
    fn comment_mentions_skip_headers_insert_rich_task_links_and_undo_without_sending(cx:&mut gpui::TestAppContext) {
        cx.update(crate::input::init);let(view,cx)=cx.add_window_view(desktop);
        cx.simulate_keystrokes("ctrl-m");cx.simulate_input("Before @QA");
        view.update_in(cx,|this,_,cx|results(this,cx));cx.simulate_keystrokes("down enter");
        view.read_with(cx,|this,cx|{
            let html=this.comment_input.read(cx).html();assert!(html.contains("Before "));assert!(html.contains("data-label=\"task\""));assert!(html.contains("href=\"https://app.hypertask.ai/detail/project-7283/5\""));assert!(!html.contains("@QA"));assert!(this.db.mutations().unwrap().is_empty());
        });
        cx.simulate_keystrokes("ctrl-z");view.read_with(cx,|this,cx|assert!(this.comment_input.read(cx).content.ends_with("@QA")));
    }
    #[gpui::test]
    fn mention_escape_empty_error_and_stale_selection_do_not_insert_or_send(cx:&mut gpui::TestAppContext) {
        cx.update(crate::input::init);let(view,cx)=cx.add_window_view(desktop);
        cx.simulate_keystrokes("ctrl-m");cx.simulate_input("@QA");view.update_in(cx,|this,_,cx|results(this,cx));
        cx.simulate_keystrokes("escape");view.read_with(cx,|this,cx|{assert!(this.mentions.dismissed);assert_eq!(this.comment_input.read(cx).content.as_ref(),"@QA");});
        view.update_in(cx,|this,window,cx|{
            results(this,cx);this.mentions.rows.clear();this.insert_mention_choice(0,window,cx);assert_eq!(this.comment_input.read(cx).content.as_ref(),"@QA");
            results(this,cx);this.options.task=2;this.detail=None;this.insert_mention_choice(1,window,cx);assert_eq!(this.comment_input.read(cx).content.as_ref(),"@QA");assert!(this.db.mutations().unwrap().is_empty());
        });
    }
    #[gpui::test]
    fn comment_star_pin_shortcuts_and_command_rows_target_focused_comment_only(cx:&mut gpui::TestAppContext) {
        cx.update(crate::input::init);let(view,cx)=cx.add_window_view(desktop);
        view.update_in(cx,|this,_,_|this.interaction.task_part=1);
        cx.simulate_keystrokes("ctrl-shift-s ctrl-shift-p");
        view.update_in(cx,|this,_,_|{let rows=this.db.mutations().unwrap();assert_eq!(rows.len(),2);assert_eq!(rows[0].payload_json["change"],json!({"action":"saved","public":false}));assert_eq!(rows[1].payload_json["change"],json!({"action":"saved","public":true}));assert_eq!(rows[1].payload_json["comment_id"],7);
            let mut commands=vec![];this.add_comment_commands(&mut commands);assert!(commands.iter().any(|c|c.key=="starComment"));assert!(commands.iter().any(|c|c.key=="pinComment"));this.interaction.task_part=0;
        });
        cx.simulate_keystrokes("ctrl-shift-s ctrl-shift-p");view.read_with(cx,|this,_|assert_eq!(this.db.mutations().unwrap().len(),2));
        cx.simulate_keystrokes("ctrl-m ctrl-shift-s ctrl-shift-p");view.read_with(cx,|this,_|assert_eq!(this.db.mutations().unwrap().len(),2));
    }
}
