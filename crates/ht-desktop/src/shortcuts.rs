use crate::ui::Screen;
use gpui::Keystroke;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Commands,
    Back,
    Open,
    Comment,
    Save { stay: bool, complete: bool },
    Next,
    Previous,
    NextColumn,
    PreviousColumn,
    First,
    Last,
    MoveLeft,
    MoveRight,
    Archive,
    ArchiveNotification,
    Rename,
    Description,
    Inbox,
    Board,
    Search,
    LocalSearch,
    Boards,
    CopyPageUrl,
    Select,
    ExtendNext,
    ExtendPrevious,
    SelectAll { all_splits: bool },
    BulkArchive,
    MoveColumn,
    Priority,
    Assign,
    Tags,
    Size,
    DueDate,
    Delete,
    CopyId,
    CopyTitleId,
    CopyUrl,
    CopyTitleUrl,
    CopyPublicUrl,
    CopyPublicTitleUrl,
    DiscardDraft,
}

/// Match exact modifiers: editor text and browser/OS chords must never fall through to J/K/E.
pub fn resolve(key: &Keystroke, screen: Screen, editor: bool) -> Option<Action> {
    let k = key.key.as_str();
    let m = key.modifiers;
    let ctrl = m.control || m.platform;
    if ctrl && !m.alt && !m.shift && k == "k" {
        return Some(Action::Commands);
    }
    if k == "escape" && !ctrl && !m.alt && !m.shift {
        return Some(Action::Back);
    }
    if editor {
        return match (k, ctrl, m.shift, m.alt) {
            ("enter", true, stay, complete) => Some(Action::Save { stay, complete }),
            (",", true, true, false) => Some(Action::DiscardDraft),
            _ => None,
        };
    }
    match (k, ctrl, m.shift, m.alt) {
        ("m", true, false, false) => Some(Action::Comment),
        ("e", true, false, false) => Some(Action::Archive),
        ("f", true, false, false) if screen != Screen::Task => Some(Action::LocalSearch),
        ("b", true, false, false) => Some(Action::Boards),
        ("r", true, false, false) if screen == Screen::Inbox => Some(Action::Comment),
        ("j" | "down", false, true, false) if screen == Screen::Inbox => Some(Action::ExtendNext),
        ("k" | "up", false, true, false) if screen == Screen::Inbox => Some(Action::ExtendPrevious),
        ("i", true, false, false) => Some(Action::CopyTitleId),
        ("i", true, true, false) => Some(Action::CopyId),
        (";" | ":", true, true, false) => Some(Action::CopyUrl),
        (";" | "," | ":", true, false, false) => Some(Action::CopyTitleUrl),
        (".", true, true, false) => Some(Action::CopyPublicUrl),
        (".", true, false, false) => Some(Action::CopyPublicTitleUrl),
        ("d", true, false, false) if screen == Screen::Task => Some(Action::Description),
        ("a", true, all_splits, false) if screen != Screen::Task => {
            Some(Action::SelectAll { all_splits })
        }
        ("e", false, true, false) if screen == Screen::Inbox => Some(Action::BulkArchive),
        ("h" | "left", false, true, false) => Some(Action::MoveLeft),
        ("l" | "right", false, true, false) => Some(Action::MoveRight),
        ("3" | "#", false, true, false) | ("#", false, false, false) => Some(Action::Delete),
        ("tab", false, false, false) => Some(Action::NextColumn),
        ("tab", false, true, false) => Some(Action::PreviousColumn),
        (_, true, _, _) | (_, _, true, _) | (_, _, _, true) => None,
        ("down", false, false, false) if screen == Screen::Task => Some(Action::NextColumn),
        ("up", false, false, false) if screen == Screen::Task => Some(Action::PreviousColumn),
        ("j" | "down", false, false, false) => Some(Action::Next),
        ("k" | "up", false, false, false) => Some(Action::Previous),
        ("h" | "left", false, false, false) if screen == Screen::Board => {
            Some(Action::PreviousColumn)
        }
        ("l" | "right", false, false, false) if screen == Screen::Board => Some(Action::NextColumn),
        ("enter", false, false, false) => Some(Action::Open),
        ("e", false, false, false) if screen == Screen::Board => Some(Action::Rename),
        ("e", false, false, false) => Some(Action::ArchiveNotification),
        ("f2", false, false, false) => Some(Action::Rename),
        ("m", false, false, false) => Some(Action::MoveColumn),
        ("a", false, false, false) => Some(Action::Assign),
        ("t", false, false, false) => Some(Action::Tags),
        ("p", false, false, false) => Some(Action::Priority),
        ("s", false, false, false) => Some(Action::Size),
        ("d", false, false, false) => Some(Action::DueDate),
        ("x", false, false, false) if screen != Screen::Task => Some(Action::Select),
        ("1", false, false, false) => Some(Action::Inbox),
        ("2", false, false, false) => Some(Action::Board),
        ("/" | "6", false, false, false) => Some(Action::Search),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn key(value: &str, screen: Screen, editor: bool) -> Option<Action> {
        resolve(&Keystroke::parse(value).unwrap(), screen, editor)
    }
    #[test]
    fn modifiers_do_not_trigger_plain_shortcuts() {
        for screen in [Screen::Inbox, Screen::Board, Screen::Task] {
            assert_eq!(key("ctrl-j", screen, false), None);
            assert_eq!(key("alt-j", screen, false), None);
            assert_eq!(key("ctrl-e", screen, false), Some(Action::Archive));
            assert_eq!(key("ctrl-k", screen, true), Some(Action::Commands));
            assert_eq!(key("cmd-k", screen, true), Some(Action::Commands));
            assert_eq!(key("j", screen, true), None);
            assert_eq!(
                key("ctrl-enter", screen, true),
                Some(Action::Save {
                    stay: false,
                    complete: false
                })
            );
            assert_eq!(
                key("ctrl-shift-enter", screen, true),
                Some(Action::Save {
                    stay: true,
                    complete: false
                })
            );
        }
    }
    #[test]
    fn contextual_bindings_and_aliases_match_web() {
        assert_eq!(key("e", Screen::Board, false), Some(Action::Rename));
        assert_eq!(
            key("e", Screen::Inbox, false),
            Some(Action::ArchiveNotification)
        );
        assert_eq!(
            key("shift-e", Screen::Inbox, false),
            Some(Action::BulkArchive)
        );
        for (left, right) in [
            ("j", "down"),
            ("k", "up"),
            ("shift-h", "shift-left"),
            ("shift-l", "shift-right"),
        ] {
            assert_eq!(
                key(left, Screen::Board, false),
                key(right, Screen::Board, false)
            );
        }
        assert_eq!(key("tab", Screen::Board, false), Some(Action::NextColumn));
        assert_eq!(
            key("shift-tab", Screen::Board, false),
            Some(Action::PreviousColumn)
        );
        assert_eq!(key("ctrl-m", Screen::Inbox, false), Some(Action::Comment));
    }
}
