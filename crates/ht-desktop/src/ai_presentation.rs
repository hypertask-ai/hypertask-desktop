use super::*;
use gpui::{MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisplayMode {
    Sidebar,
    Floating,
    Fullscreen,
}

pub struct Presentation {
    pub mode: DisplayMode,
    pub pinned: bool,
    pub suppressed: bool,
    pub open: bool,
    pub width: f32,
    drag: Option<(f32, f32)>,
}

impl Default for Presentation {
    fn default() -> Self {
        Self {
            mode: DisplayMode::Sidebar,
            pinned: false,
            suppressed: false,
            open: true,
            width: 420.,
            drag: None,
        }
    }
}

impl Presentation {
    pub fn load(db: &Db) -> Self {
        let flag = |key| db.kv_get(key).ok().flatten().as_deref() == Some("true");
        Self {
            mode: if db.kv_get("isAiChatSidebarMode").ok().flatten().as_deref() == Some("false") {
                DisplayMode::Floating
            } else {
                DisplayMode::Sidebar
            },
            pinned: flag("aiChatPinned"),
            suppressed: flag("aiChatAutoOpenSuppressed"),
            open: flag("aiChatPinned")
                || (!flag("aiChatAutoOpenSuppressed")
                    && db.kv_get("openAiChatByDefault").ok().flatten().as_deref() != Some("false")),
            width: db
                .kv_get("aiChatSidebarWidthPx")
                .ok()
                .flatten()
                .and_then(|s| s.parse::<f32>().ok())
                .filter(|w| w.is_finite())
                .unwrap_or(420.),
            drag: None,
        }
    }

    pub fn save(&self, db: &Db) -> ht_core::Result<()> {
        let mut values = vec![
            ("aiChatPinned", self.pinned.to_string()),
            ("aiChatAutoOpenSuppressed", self.suppressed.to_string()),
            ("aiChatSidebarWidthPx", self.width.to_string()),
        ];
        if self.mode != DisplayMode::Fullscreen {
            values.push((
                "isAiChatSidebarMode",
                (self.mode == DisplayMode::Sidebar).to_string(),
            ));
        }
        for (key, value) in values {
            if db.kv_get(key)?.as_deref() != Some(&value) {
                db.kv_set(key, &value)?;
            }
        }
        Ok(())
    }

    pub fn clamp(width: f32, viewport: f32) -> f32 {
        width.min(900_f32.min((viewport * 0.85).floor())).max(340.)
    }
}

impl AiSidebar {
    pub fn restore_presentation(&mut self, db: &Db) {
        self.presentation = Presentation::load(db);
    }

    pub fn persist_presentation(&self, db: &Db) -> ht_core::Result<()> {
        self.presentation.save(db)
    }

    pub fn is_open(&self) -> bool {
        self.presentation.open
    }
    pub fn mode(&self) -> DisplayMode {
        self.presentation.mode
    }
    pub fn minimized(&self) -> bool {
        self.collapsed
    }

    pub fn set_open(&mut self, open: bool, cx: &mut Context<Self>) {
        self.presentation.open = open;
        self.presentation.suppressed = !open;
        if !open {
            self.presentation.pinned = false;
        }
        self.collapsed = false;
        cx.notify();
    }

    pub fn toggle_pin(&mut self, cx: &mut Context<Self>) {
        self.presentation.pinned = !self.presentation.pinned;
        if self.presentation.pinned {
            self.set_open(true, cx);
        }
        self.more_menu = false;
        cx.notify();
    }

    pub fn set_mode(&mut self, mode: DisplayMode, cx: &mut Context<Self>) {
        self.presentation.mode = mode;
        self.set_open(true, cx);
        self.more_menu = false;
    }

    pub fn resize_move(&mut self, event: &MouseMoveEvent, window: &Window, cx: &mut Context<Self>) {
        if let Some((start_x, start_width)) = self.presentation.drag {
            self.presentation.width = Presentation::clamp(
                start_width - (f32::from(event.position.x) - start_x),
                f32::from(window.viewport_size().width),
            );
            cx.notify();
        }
    }

    pub fn resize_end(&mut self, _: &MouseUpEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.presentation.drag.take().is_some() {
            cx.notify();
        }
    }

    pub fn resize_edge(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        div()
            .id("ai-resize-edge")
            .absolute()
            .left_0()
            .top_0()
            .w(px(4.))
            .h_full()
            .cursor(gpui::CursorStyle::ResizeLeftRight)
            .hover(|d| d.bg(color(self.colors.accent)))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, e: &MouseDownEvent, _, cx| {
                    this.presentation.drag =
                        Some((f32::from(e.position.x), this.presentation.width));
                    cx.stop_propagation();
                }),
            )
            .layout_key(&self.capture, "ai.resize", None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn web_resize_bounds_and_persisted_pin_suppression_and_mode() {
        assert_eq!(Presentation::clamp(100., 1440.), 340.);
        assert_eq!(Presentation::clamp(2000., 1440.), 900.);
        assert_eq!(Presentation::clamp(900., 800.), 680.);
        let db = Db::memory().unwrap();
        let mut p = Presentation::load(&db);
        assert_eq!(p.width, 420.);
        p.width = 577.;
        p.pinned = true;
        p.suppressed = true;
        p.mode = DisplayMode::Floating;
        p.save(&db).unwrap();
        let p = Presentation::load(&db);
        assert!(p.open && p.pinned);
        assert_eq!(p.width, 577.);
        assert_eq!(p.mode, DisplayMode::Floating);
    }

    #[gpui::test]
    fn pin_close_modes_preserve_conversation_and_draft(cx: &mut gpui::TestAppContext) {
        let view = cx.add_window(|window, cx| {
            let focus = cx.focus_handle();
            AiSidebar::new(crate::theme::THEMES[0], None, focus, window, cx)
        });
        view.update(cx, |this, _, cx| {
            let id = this.chats[0].id.clone();
            this.input
                .update(cx, |input, cx| input.set_content("Draft".into(), cx));
            this.toggle_pin(cx);
            assert!(this.presentation.pinned && this.is_open());
            for mode in [
                DisplayMode::Floating,
                DisplayMode::Fullscreen,
                DisplayMode::Sidebar,
            ] {
                this.set_mode(mode, cx);
                assert_eq!(this.mode(), mode);
                assert_eq!(this.chats[0].id, id);
                assert_eq!(this.input.read(cx).content.as_ref(), "Draft");
            }
            this.set_open(false, cx);
            assert!(!this.is_open() && !this.presentation.pinned && this.presentation.suppressed);
            this.set_open(true, cx);
            assert!(!this.presentation.suppressed);
        })
        .unwrap();
    }
}
