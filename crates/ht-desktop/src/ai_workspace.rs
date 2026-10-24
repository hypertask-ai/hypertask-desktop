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

    pub(super) fn exit_ai_fullscreen(&mut self, cx: &mut Context<Self>) {
        if let Some(sidebar) = &self.ai_sidebar {
            if sidebar.read(cx).mode() == DisplayMode::Fullscreen {
                sidebar.update(cx, |sidebar, cx| sidebar.exit_fullscreen(cx));
            }
        }
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

#[cfg(test)]
mod tests {
    use super::super::shell::tests::{click, draw, measured, setup};
    use super::*;
    use gpui::MouseButton;

    #[gpui::test]
    fn ai_resize_mouse_drag_clamps_and_persists_after_reopen(cx: &mut gpui::TestAppContext) {
        let (view, mut cx) = setup(cx);
        let edge = measured(view, &mut cx, "ai.resize");
        let x = edge["x"].as_f64().unwrap() as f32 + 2.;
        let y = edge["y"].as_f64().unwrap() as f32 + 250.;
        cx.simulate_mouse_down(point(px(x), px(y)), MouseButton::Left, Default::default());
        cx.simulate_mouse_move(
            point(px(x - 180.), px(y)),
            MouseButton::Left,
            Default::default(),
        );
        cx.simulate_mouse_up(
            point(px(x - 180.), px(y)),
            MouseButton::Left,
            Default::default(),
        );
        assert_eq!(measured(view, &mut cx, "rightpanel")["w"], 600.);
        view.read_with(&mut cx, |this, _| {
            assert_eq!(
                this.db.kv_get("aiChatSidebarWidthPx").unwrap().as_deref(),
                Some("600")
            )
        })
        .unwrap();
        click(view, &mut cx, "rail.icon.6");
        click(view, &mut cx, "shell.ai.reopen");
        assert_eq!(measured(view, &mut cx, "rightpanel")["w"], 600.);
        let edge = measured(view, &mut cx, "ai.resize");
        let x = edge["x"].as_f64().unwrap() as f32 + 2.;
        cx.simulate_mouse_down(point(px(x), px(y)), MouseButton::Left, Default::default());
        cx.simulate_mouse_move(
            point(px(1430.), px(y)),
            MouseButton::Left,
            Default::default(),
        );
        cx.simulate_mouse_up(
            point(px(1430.), px(y)),
            MouseButton::Left,
            Default::default(),
        );
        assert_eq!(measured(view, &mut cx, "rightpanel")["w"], 340.);
    }

    #[gpui::test]
    fn ai_header_pin_modes_close_and_commands_are_real_mouse_paths(cx: &mut gpui::TestAppContext) {
        let (view, mut cx) = setup(cx);
        click(view, &mut cx, "ai.more");
        click(view, &mut cx, "ai.pin");
        view.read_with(&mut cx, |this, _| {
            assert_eq!(
                this.db.kv_get("aiChatPinned").unwrap().as_deref(),
                Some("true")
            )
        })
        .unwrap();
        cx.simulate_keystrokes("2");
        assert_eq!(measured(view, &mut cx, "rightpanel")["w"], 420.);
        click(view, &mut cx, "ai.more");
        click(view, &mut cx, "ai.mode");
        let floating = measured(view, &mut cx, "rightpanel");
        assert_eq!(floating["w"], 432.);
        assert_eq!(floating["h"], 630.);
        assert_eq!(measured(view, &mut cx, "main")["w"], 1440.);
        click(view, &mut cx, "ai.more");
        click(view, &mut cx, "ai.fullscreen");
        assert_eq!(measured(view, &mut cx, "rightpanel")["w"], 1440.);
        assert_eq!(measured(view, &mut cx, "rightpanel")["h"], 900.);
        click(view, &mut cx, "ai.fullscreen.exit");
        assert_eq!(measured(view, &mut cx, "rightpanel")["w"], 432.);
        click(view, &mut cx, "ai.more");
        click(view, &mut cx, "ai.mode");
        assert_eq!(measured(view, &mut cx, "rightpanel")["w"], 420.);
        click(view, &mut cx, "ai.close");
        draw(view, &mut cx);
        view.read_with(&mut cx, |this, _| {
            assert!(!this.shell.ai_open);
            assert_eq!(
                this.db.kv_get("aiChatPinned").unwrap().as_deref(),
                Some("false")
            );
            assert_eq!(
                this.db
                    .kv_get("aiChatAutoOpenSuppressed")
                    .unwrap()
                    .as_deref(),
                Some("true")
            );
        })
        .unwrap();
        cx.simulate_keystrokes("ctrl-k");
        cx.simulate_input("Pin AI chat open");
        cx.simulate_keystrokes("enter");
        assert_eq!(measured(view, &mut cx, "rightpanel")["w"], 420.);
        cx.simulate_keystrokes("ctrl-q");
        cx.simulate_keystrokes("ctrl-k");
        cx.simulate_input("Open AI chat full screen");
        cx.simulate_keystrokes("enter");
        assert_eq!(measured(view, &mut cx, "rightpanel")["w"], 1440.);
        let selected = view.read_with(&mut cx, |this, _| this.selected).unwrap();
        cx.simulate_keystrokes("j");
        view.read_with(&mut cx, |this, _| assert_eq!(this.selected, selected))
            .unwrap();
        cx.simulate_keystrokes("2");
        assert_eq!(measured(view, &mut cx, "rightpanel")["w"], 420.);
    }
}
