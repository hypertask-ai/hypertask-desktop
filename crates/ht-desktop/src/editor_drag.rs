use super::*;
use crate::layout::LayoutKey;

impl TextInput {
    pub(super) fn block_drag_handles(&self, cx: &mut Context<Self>) -> Vec<gpui::AnyElement> {
        if self.read_only {return vec![];}
        let (Some(editor),Some(layout))=(&self.editor,&self.last_layout) else{return vec![];};
        if layout.text!=self.content {return vec![];}
        let blocks=editor.document.drag_blocks();
        blocks.iter().enumerate().filter(|(i,(_,offset))| !blocks.iter().skip(i+1).any(|(_,other)|other==offset)).map(|(i,(path,offset))|{
            let row=layout.row((*offset).min(layout.text.len()));
            let path=path.clone();
            div().id(("block-drag",i)).absolute().left(px(-20.)).top(layout.rows[row].top).w(px(18.)).h(px(22.))
                .cursor(CursorStyle::OpenHand).text_size(px(14.)).text_color(gpui::white()).bg(crate::theme::color(0x4f5765)).rounded(px(3.)).child("⠿")
                .on_mouse_down(MouseButton::Left,cx.listener(move|this,_,window,cx|{
                    cx.stop_propagation();this.is_selecting=false;
                    window.focus(&this.focus_handle,cx);
                    this.editor.as_mut().unwrap().dragging=Some(path.clone());cx.notify();
                }))
                .layout_key(&editor.capture,format!("block-drag-{i}"),None).into_any_element()
        }).collect()
    }
    pub(super) fn finish_block_drag(&mut self, event: &MouseUpEvent, window: &mut Window, cx: &mut Context<Self>) {
        let Some(source)=self.editor.as_mut().and_then(|editor|editor.dragging.take()) else{return;};
        let (Some(layout),Some(bounds))=(&self.last_layout,self.last_bounds) else{return;};
        let y=event.position.y-bounds.top();
        let row=layout.row_for_y(y);
        let offset=layout.rows[row].range.start;
        let blocks=self.editor.as_ref().unwrap().document.drag_blocks();
        let Some((target,_))=blocks.iter().rev().find(|(_,at)|*at<=offset).or_else(||blocks.first()) else{return;};
        let after=y>=layout.rows[row].top+layout.rows[row].height/2.;
        let mut document=self.editor.as_ref().unwrap().document.clone();
        if !document.reorder_block(&source,target,after) {return;}
        self.remember_edit();
        self.content=document.text().into();
        self.editor.as_mut().unwrap().document=document;
        self.selected_range=0..0;self.last_layout=None;
        window.notify_text_input_changed(&self.focus_handle);cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[gpui::test]
    fn real_drag_handle_moves_paragraph_below_checklist_and_undo_restores_html(cx:&mut gpui::TestAppContext) {
        cx.update(init);
        // Padding leaves room for the web's left-of-block grip.
        let (host,cx)=cx.add_window_view(|_,cx|{
            struct Host(Entity<TextInput>);
            impl Render for Host {fn render(&mut self,_:&mut Window,_:&mut Context<Self>)->impl IntoElement{div().p(px(40.)).child(self.0.clone())}}
            Host(cx.new(|cx|TextInput::new(String::new(),"",gpui::black(),cx).comment_editor()))
        });
        let input=host.read_with(cx,|h,_|h.0.clone());
        let original="<p><strong>move é🙂</strong></p><ul data-type=\"taskList\"><li data-type=\"taskItem\" data-checked=\"true\"><div><p>done</p></div></li></ul><p>end</p>";
        input.update_in(cx,|input,window,cx|{input.set_html(original,cx);window.focus(&input.focus_handle,cx);});
        cx.run_until_parked();
        // The first layout paints text; the following render paints measured grips.
        input.update(cx,|_,cx|cx.notify());cx.run_until_parked();
        let (grip,target)=input.read_with(cx,|input,_|{
            let bounds=input.last_bounds.unwrap();let layout=input.last_layout.as_ref().unwrap();
            (point(bounds.left()-px(10.),bounds.top()+px(10.)),point(bounds.left()+px(30.),bounds.top()+layout.rows[1].top+layout.rows[1].height-px(1.)))
        });
        cx.simulate_mouse_down(grip,MouseButton::Left,gpui::Modifiers::default());
        input.read_with(cx,|input,_|assert!(input.editor.as_ref().unwrap().dragging.is_some()));
        cx.simulate_mouse_move(target,MouseButton::Left,gpui::Modifiers::default());
        cx.simulate_mouse_up(target,MouseButton::Left,gpui::Modifiers::default());
        input.read_with(cx,|input,_|{assert_eq!(input.content.as_ref(),"done\nmove é🙂\nend");assert!(input.html().contains("data-checked=\"true\""));});
        cx.simulate_keystrokes("ctrl-z");
        input.read_with(cx,|input,_|assert_eq!(input.html(),crate::editor_document::Document::parse(original).html()));
    }
}
