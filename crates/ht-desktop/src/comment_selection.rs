use super::*;
use crate::input::QuoteSelection;

impl Desktop {
    pub(super) fn prepare_comment_readers(&mut self,window:&mut Window,cx:&mut Context<Self>) {
        if self.screen!=Screen::Task {return;}
        let Some(detail)=&self.detail else{return;};
        self.comment_actions.readers.retain(|id,_|detail.comments.iter().any(|c|c.id==*id));
        for comment in &detail.comments {
            if self.comment_actions.readers.get(&comment.id).is_some_and(|(html,_)|html==&comment.text) {continue;}
            let html=comment.text.clone();let id=comment.id;
            let reader=cx.new(|cx|TextInput::new(String::new(),"",color(self.colors.muted),cx).read_only_comment(&html,cx));
            cx.observe(&reader,|_,_,cx|cx.notify()).detach();
            cx.subscribe_in(&reader,window,move|this,_,event:&QuoteSelection,window,cx| {
                this.quote_selected_comment(id,&event.html,window,cx);
            }).detach();
            self.comment_actions.readers.insert(id,(html,reader));
        }
    }
    fn quote_selected_comment(&mut self,id:i64,selection:&str,window:&mut Window,cx:&mut Context<Self>) {
        if selection.is_empty() {return;}
        let Some((i,comment))=self.detail.as_ref().and_then(|d|d.comments.iter().enumerate().find(|(_,c)|c.id==id)).map(|(i,c)|(i,c.clone())) else{return;};
        self.interaction.task_part=i+1;
        let name=comment.extra.get("creator").and_then(|v|v.get("displayName")).and_then(|v|v.as_str()).unwrap_or("");
        let label=comment.creator_id.filter(|id|*id!=332).map(|id|format!("name-{id}")).unwrap_or_default();
        let name=name.replace('&',"&amp;").replace('<',"&lt;").replace('>',"&gt;").replace('"',"&quot;").replace('\'',"&#39;");
        let html=format!("<span data-type=\"mention\" class=\"mention\" data-id=\"{name}\" data-label=\"{label}\" uniqueindex=\"\" projectid=\"\" contenteditable=\"false\">{name}</span> said <blockquote data-quote-reply=\"true\">{selection}</blockquote>");
        self.comment_input.update(cx,|input,cx|input.append_html(&html,window,cx));
        self.edit_comment(window,cx);
    }
    pub(super) fn comment_reader_key(&mut self,event:&KeyDownEvent,window:&mut Window,cx:&mut Context<Self>)->bool {
        if self.screen!=Screen::Task {return false;}
        let Some(reader)=self.comment_actions.readers.values().map(|(_,r)|r).find(|r|r.focus_handle(cx).is_focused(window)).cloned() else{return false;};
        let key=&event.keystroke;
        if key.key=="k" && (key.modifiers.control||key.modifiers.platform) {return false;}
        if key.key=="escape" {window.focus(&self.focus,cx);cx.stop_propagation();cx.notify();}
        if !reader.read(cx).has_selection() && key.key!="escape" {return false;}
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::EntityInputHandler;
    use serde_json::json;
    #[gpui::test]
    fn selected_comment_quote_preserves_only_selected_html_and_readonly_input_cannot_mutate(cx:&mut gpui::TestAppContext) {
        cx.update(crate::input::init);
        let (view,cx)=cx.add_window_view(|window,cx|{
            let mut db=Db::memory().unwrap();db.kv_set("current-user-id","42").unwrap();
            db.apply_task(&json!({"id":1,"title":"Task","projectId":15,"status":"Normal"})).unwrap();
            db.apply_comments(1,&json!({"success":true,"comments":[{"id":7,"creatorId":43,"creator":{"id":43,"displayName":"Other"},"text":"<p>before <strong>é🙂 selected</strong> after</p>","createdAt":"2026-01-01T00:00:00Z"}]})).unwrap();
            let options=Options::parse(["--offline","--screen","task","--task","1"].map(String::from)).unwrap();
            Desktop::new(db,options,window,cx)
        });
        cx.run_until_parked();
        let reader=view.read_with(cx,|this,_|this.comment_actions.readers[&7].1.clone());
        reader.update_in(cx,|input,window,cx|{window.focus(&input.focus_handle(cx),cx);input.set_selected_text_range(7..19,window,cx);});
        cx.simulate_input("NOT EDITABLE");cx.simulate_keystrokes("backspace ctrl-b");
        reader.read_with(cx,|input,_|assert_eq!(input.html(),"<p>before <strong>é🙂 selected</strong> after</p>"));
        reader.update_in(cx,|input,window,cx|input.set_selected_text_range(7..19,window,cx));
        cx.simulate_keystrokes("enter");
        view.read_with(cx,|this,cx|{let html=this.comment_input.read(cx).html();assert!(html.contains("<strong>é🙂 selected</strong>"),"{html}");assert!(!html.contains("before"));assert!(!html.contains("after"));assert!(html.contains("data-quote-reply=\"true\""));assert!(html.contains("Other"));assert!(this.db.mutations().unwrap().is_empty());});
        cx.simulate_keystrokes("ctrl-shift-enter");
        view.read_with(cx,|this,_|{let rows=this.db.mutations().unwrap();assert_eq!(rows.len(),1);assert_eq!(rows[0].kind,"task_comment");assert!(rows[0].payload_json["text"].as_str().unwrap().contains("<strong>é🙂 selected</strong>"));});
    }
}
