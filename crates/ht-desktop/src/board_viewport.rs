use super::*;

impl Desktop {
    pub(super) fn board_column_visible(&self, index: usize, width: f32, board_width: f32, window: &Window, cx: &App) -> bool {
        if !cfg!(feature = "visible-board-columns") { return true; }
        // Keep headers/column boxes laid out for scroll_to_item, but do not shape cards
        // behind the horizontal clip. The selected target must exist before autoscroll.
        let sidebar_width = self.ai_sidebar.as_ref()
            .filter(|sidebar| self.shell.ai_open && sidebar.read(cx).mode() == crate::ai_sidebar::DisplayMode::Sidebar)
            .map_or(0., |sidebar| sidebar.read(cx).width());
        let viewport = f32::from(window.viewport_size().width) - self.shell.width() - sidebar_width;
        let left = board_width * 0.015 + index as f32 * (width + 16.)
            + f32::from(self.board_scroll.offset().x);
        index == self.column || (left < viewport && left + width > 0.)
    }
}

#[cfg(all(test, feature = "visible-board-columns"))]
mod tests {
    use super::*;
    use crate::ui::shell::tests::draw;
    use crate::ui::task_fields::tests::desktop;
    use gpui::{VisualTestContext, WindowHandle};
    use serde_json::json;

    #[gpui::test]
    fn wide_board_culls_only_offscreen_cards_and_restores_them_after_scroll(cx: &mut gpui::TestAppContext) {
        cx.update(crate::input::init);
        let view: WindowHandle<Desktop> = cx.open_window(gpui::size(px(1440.), px(900.)), |window, cx| {
            let mut view = desktop(window, cx);
            let sections = (0..13).map(|i| json!({"id":100+i,"projectId":16,"section_title":format!("Column {i}"),"ranking":format!("{i:02}"),"visibility":true,"deleted":false})).collect::<Vec<_>>();
            view.db.apply_response(&json!({"success":true,"sections":sections})).unwrap();
            for i in 0..13 {
                view.db.apply_task(&json!({"id":100+i,"title":format!("Card {i}"),"projectId":16,"sectionId":100+i,"status":"Normal"})).unwrap();
            }
            view.options.board = 16;
            view.screen = Screen::Board;
            view.reload();
            view
        });
        let mut cx = VisualTestContext::from_window(view.into(), cx);
        draw(view, &mut cx);
        view.read_with(&mut cx, |this, _| {
            let elements = this.layout.as_ref().unwrap().borrow().interaction_elements();
            assert!(elements.iter().any(|e| e["key"] == "board.card.0.title"));
            assert!(!elements.iter().any(|e| e["key"] == "board.card.12.title"));
            assert_eq!(this.columns.iter().map(|c| c.cards.len()).sum::<usize>(), 13);
        }).unwrap();
        view.update(&mut cx, |this, _, cx| {
            this.column = 12;
            this.board_scroll.scroll_to_item(12);
            cx.notify();
        }).unwrap();
        draw(view, &mut cx);
        draw(view, &mut cx);
        view.read_with(&mut cx, |this, _| {
            let elements = this.layout.as_ref().unwrap().borrow().interaction_elements();
            let card = elements.iter().find(|e| e["key"] == "board.card.12.title").unwrap();
            assert!(card["x"].as_f64().unwrap() < 1440.);
            assert!(!elements.iter().any(|e| e["key"] == "board.card.0.title"));
        }).unwrap();
        // Exercise an offset-only (wheel/trackpad) move independently of selection.
        view.update(&mut cx, |this, _, cx| {
            this.board_scroll.set_offset(point(px(0.), px(0.)));
            cx.notify();
        }).unwrap();
        draw(view, &mut cx);
        view.read_with(&mut cx, |this, _| {
            let elements = this.layout.as_ref().unwrap().borrow().interaction_elements();
            assert!(elements.iter().any(|e| e["key"] == "board.card.0.title"));
            assert_eq!(this.columns.iter().map(|c| c.cards.len()).sum::<usize>(), 13);
            assert!(this.db.mutations().unwrap().is_empty());
        }).unwrap();
        view.update(&mut cx, |this, _, cx| {
            this.ai_sidebar.as_ref().unwrap().update(cx, |sidebar, cx| {
                sidebar.set_mode(crate::ai_sidebar::DisplayMode::Floating, cx);
            });
            cx.notify();
        }).unwrap();
        draw(view, &mut cx);
        view.read_with(&mut cx, |this, _| {
            let elements = this.layout.as_ref().unwrap().borrow().interaction_elements();
            assert!(elements.iter().any(|e| e["key"] == "board.card.4.title"), "floating AI must not shrink the board's clip");
        }).unwrap();
    }
}
