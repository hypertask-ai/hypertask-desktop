use super::*;

impl Desktop {
    pub(super) fn comment_composer(&self, cx: &mut Context<Self>) -> AnyElement {
        let t = self.colors;
        div().id("comment-composer").anchor_scroll(Some(self.composer_anchor.clone())).flex_shrink_0().h(px(if self.options.theme == ThemeName::Porcelain { 152. } else { 154. })).pt(px(19.)).pb(px(if matches!(self.options.theme, ThemeName::Graphite | ThemeName::Amoled) { 27. } else { 25. })).pl(px(19.)).pr(px(16.)).rounded(px(if self.options.theme == ThemeName::Dia { 10. } else { 5. })).bg(color(t.comment)).border_1().border_color(color(t.border)).shadow_md().flex().flex_col()
                        .on_click(cx.listener(|this, _, window, cx| this.edit_comment(window, cx)))
                        .child(div().text_size(px(14.)).line_height(px(21.)).child(self.comment_input.clone()).layout_key(&self.layout, "task.composer.input", None))
                        .child(div().flex_1())
                        .child(div().flex().items_center().text_size(px(14.)).line_height(px(21.)).text_color(color(0x95999e)).font_weight(FontWeight::SEMIBOLD)
                            .child(div().id("send-comment-advance").cursor_pointer().flex().items_center().on_click(cx.listener(|this, _, window, cx| { cx.stop_propagation(); this.editing = Editing::Comment; this.run_shortcut(crate::shortcuts::Action::Save { stay: false, complete: false }, window, cx); })).child("Send + ").child(icon("send-archive", 0x696b6e, 14.)).layout_key(&self.layout, "task.composer.send-advance", None))
                            .child(div().id("send-comment").ml(px(24.)).cursor_pointer().on_click(cx.listener(|this, _, window, cx| { cx.stop_propagation(); this.editing = Editing::Comment; this.run_shortcut(crate::shortcuts::Action::Save { stay: true, complete: false }, window, cx); })).child("Send").layout_key(&self.layout, "task.composer.send", None))
                            .child(div().flex_1()).child(div().text_size(px(16.)).font_weight(FontWeight::NORMAL).text_color(color(0xc668ff)).child("ai"))
                            .child(div().ml(px(8.)).child(icon("paperclip", 0x999a9d, 16.))).child(div().ml(px(10.)).child(icon("mic", 0x999a9d, 18.))).child(div().ml(px(3.)).child(icon("chevron-down", 0x999a9d, 14.)))).layout_key(&self.layout, "task.composer", None)
    }
}
