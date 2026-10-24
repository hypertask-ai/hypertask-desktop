use crate::ui::Screen;

// src/components/Global/ConstantsQuickTips.ts:29,64,136.
pub fn hints(screen: Screen) -> &'static [(&'static str, &'static str)] {
    match screen {
        Screen::Inbox => &[
            ("CTRL K", "command center"),
            ("J", "down"),
            ("K", "up"),
            ("Enter", "open task"),
            ("TAB", "cycle inbox splits"),
            ("E", "Archive"),
            ("CTRL E", "Mark task as done"),
        ],
        Screen::Board => &[
            ("CTRL K", "command center"),
            ("J", "down"),
            ("K", "up"),
            ("TAB", "next column"),
            ("C", "add task"),
            ("N / Shift+C", "quick add"),
            ("CTRL E", "archive"),
            ("CTRL B", "Change Board"),
            ("/", "search"),
        ],
        Screen::Task => &[
            ("CTRL K", "command center"),
            ("A", "assign"),
            ("M", "change status(move)"),
            ("P", "priority"),
            ("S", "task size"),
            ("T", "tags"),
            ("CTRL E", "archive"),
            ("ESC", "back"),
            ("TAB", "cycle"),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn board_tips_include_the_web_quick_add_hint_in_declaration_order() {
        let tips = hints(Screen::Board);
        assert_eq!(tips.len(), 9);
        assert_eq!(&tips[4..7], &[("C", "add task"), ("N / Shift+C", "quick add"), ("CTRL E", "archive")]);
        for screen in [Screen::Inbox, Screen::Board, Screen::Task] {
            assert_eq!(hints(screen)[0], ("CTRL K", "command center"));
        }
    }
}
