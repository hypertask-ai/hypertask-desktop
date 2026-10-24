use super::*;
use crate::{ai_sidebar::DisplayMode, shortcuts::Action};

impl Desktop {
    pub(super) fn ai_panel(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let Some(sidebar) = self.ai_sidebar.clone().filter(|_| self.shell.ai_open) else {
            return div().into_any_element();
        };
        let state = sidebar.read(cx);
        let mode = state.mode();
        let width = state.width();
        let minimized = state.minimized();
        let mut panel = div().id("ai-panel-host").flex_shrink_0();
        panel = match mode {
            DisplayMode::Sidebar => panel.w(px(width)).h_full(),
            DisplayMode::Floating => panel
                .absolute()
                .right(px(8.))
                .bottom(px(if self.footer { 40. } else { 20. }))
                .w(px(if minimized {
                    200.
                } else {
                    (f32::from(window.viewport_size().width) * 0.3)
                        .max(360.)
                        .min(f32::from(window.viewport_size().width) - 16.)
                }))
                .h(px(if minimized {
                    40.
                } else {
                    (f32::from(window.viewport_size().height) * 0.7)
                        .min(f32::from(window.viewport_size().height) - 16.)
                }))
                .shadow(shadow()),
            DisplayMode::Fullscreen => panel.absolute().top_0().left_0().size_full(),
        };
        panel
            .child(sidebar)
            .layout_key(&self.layout, "rightpanel", None)
            .into_any_element()
    }

    pub(super) fn ai_command(&mut self, action: Action, cx: &mut Context<Self>) {
        if let Some(sidebar) = &self.ai_sidebar {
            sidebar.update(cx, |sidebar, cx| match action {
                Action::PinAiChat => sidebar.toggle_pin(cx),
                Action::FullscreenAiChat => sidebar.set_mode(DisplayMode::Fullscreen, cx),
                _ => (),
            });
            self.shell.ai_open = sidebar.read(cx).is_open();
        }
        cx.notify();
    }
}
