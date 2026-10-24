use crate::ui::Screen;
use gpui::Keystroke;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Commands,
    Compose,
    CreateTask,
    SortView,
    FilterView,
    ArchivedOnBoard,
    StarComment,
    PinComment,
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
    Remind,
    BulkRemind,
    ReviewUnread,
    MoveColumn,
    MoveBoard,
    Priority,
    Assign,
    Tags,
    Size,
    DueDate,
    WaitingOn,
    Delete,
    CopyId,
    CopyTitleId,
    CopyUrl,
    CopyTitleUrl,
    DiscardDraft,
    PinAiChat,
    FullscreenAiChat,
    CopyCommentContent,
    CopyCommentLink,
    CopyCommentToAiChat,
    BranchCommentToAiChat,
    DeleteAllAiChats,
    ReplyComment,
    EditComment,
    DeleteComment,
    ReactComment,
    LikeComment,
    ExpandComment,
    ExpandComments,
    ExpandSummary,
    ToggleTimer,
    TimeLog,
}

// GPUI consumes Shift for OEM/digit keys on Windows, leaving the shifted character.
pub fn shifted(key: &Keystroke) -> bool {
    key.modifiers.shift || matches!(key.key.as_str(), "#" | ":" | "<" | ">")
}

/// Match exact modifiers: editor text and browser/OS chords must never fall through to J/K/E.
pub fn resolve(key: &Keystroke, screen: Screen, editor: bool) -> Option<Action> {
    let k = match key.key.as_str() {
        "#" => "3",
        ":" => ";",
        "<" => ",",
        ">" => ".",
        k => k,
    };
    let shift = shifted(key);
    let m = key.modifiers;
    let ctrl = m.control || m.platform;
    if ctrl && !m.alt && !shift && k == "k" {
        return Some(Action::Commands);
    }
    if k == "escape" && !ctrl && !m.alt && !shift {
        return Some(Action::Back);
    }
    if ctrl && !m.alt && !shift && k == "j" { return Some(Action::Compose); }
    if editor {
        return match (k, ctrl, shift, m.alt) {
            ("enter", true, true, true) => Some(Action::Save {
                stay: true,
                complete: true,
            }),
            ("enter", true, stay, false) => Some(Action::Save {
                stay,
                complete: false,
            }),
            (",", true, true, false) => Some(Action::DiscardDraft),
            _ => None,
        };
    }
    match (k, ctrl, shift, m.alt) {
        ("b", false, true, false) => Some(Action::WaitingOn),
        ("b", false, false, false) if screen==Screen::Task=>Some(Action::TimeLog),
        ("w", false, false, false) if screen==Screen::Task=>Some(Action::ToggleTimer),
        ("s", false, true, false) if screen!=Screen::Inbox=>Some(Action::SortView),
        ("f", false, true, false) if screen!=Screen::Inbox=>Some(Action::FilterView),
        ("c", false, false, false)=>Some(Action::CreateTask),
        ("s", true, true, false) if screen == Screen::Task => Some(Action::StarComment),
        ("p", true, true, false) if screen == Screen::Task => Some(Action::PinComment),
        ("3", true, true, false) if screen == Screen::Task => Some(Action::DeleteComment),
        ("i", false, false, false) if screen == Screen::Task => Some(Action::ExpandSummary),
        ("o", false, false, false) if screen == Screen::Task => Some(Action::ExpandComment),
        ("o", false, true, false) if screen == Screen::Task => Some(Action::ExpandComments),
        ("r", false, false, false) if screen == Screen::Task => Some(Action::ReactComment),
        ("l", false, false, false) if screen == Screen::Task => Some(Action::LikeComment),
        ("+" | "=", true, true, false) => Some(Action::Commands),
        ("+", true, false, false) => Some(Action::Commands),
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
        ("d", true, false, false) if screen == Screen::Task && m.control => {
            Some(Action::Description)
        }
        ("a", true, all_splits, false) if screen != Screen::Task => {
            Some(Action::SelectAll { all_splits })
        }
        ("e", false, true, false) if screen == Screen::Inbox => Some(Action::BulkArchive),
        ("m", false, true, false) => Some(Action::MoveBoard),
        ("h", false, true, false) if screen == Screen::Inbox => Some(Action::BulkRemind),
        ("h", false, false, false) if screen != Screen::Board => Some(Action::Remind),
        ("h" | "left", false, true, false) if screen != Screen::Inbox => Some(Action::MoveLeft),
        ("l" | "right", false, true, false) if screen != Screen::Inbox => Some(Action::MoveRight),
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

pub fn sequence(key: &str, shifted: bool, screen: Screen) -> Option<Action> {
    match key {
        "g" if screen != Screen::Task => Some(if shifted { Action::Last } else { Action::First }),
        "i" => Some(Action::Inbox),
        "b" => Some(Action::Board),
        "x" if screen != Screen::Inbox => Some(Action::ArchivedOnBoard),
        "t" if matches!(screen, Screen::Inbox | Screen::Task) => Some(Action::Board),
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
            assert_eq!(key("ctrl-j", screen, false), Some(Action::Compose));
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
        assert_eq!(key("e", Screen::Board, false), Some(Action::ArchiveNotification));
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
    #[test]
    fn complete_supported_key_map_and_contexts() {
        use Action::*;
        for screen in [Screen::Inbox, Screen::Board, Screen::Task] {
            for (chord, action) in [
                ("ctrl-m", Comment),
                ("ctrl-e", Archive),
                ("ctrl-b", Boards),
                ("ctrl-i", CopyTitleId),
                ("ctrl-shift-i", CopyId),
                ("ctrl-;", CopyTitleUrl),
                ("ctrl-,", CopyTitleUrl),
                ("ctrl-shift-;", CopyUrl),
                ("shift-h", MoveLeft),
                ("shift-left", MoveLeft),
                ("shift-l", MoveRight),
                ("shift-right", MoveRight),
                ("shift-3", Delete),
                ("tab", NextColumn),
                ("shift-tab", PreviousColumn),
                ("enter", Open),
                ("f2", Rename),
                ("m", MoveColumn),
                ("p", Priority),
                ("a", Assign),
                ("t", Tags),
                ("s", Size),
                ("d", DueDate),
                ("1", Inbox),
                ("2", Board),
                ("/", Search),
                ("6", Search),
            ] {
                assert_eq!(
                    key(chord, screen, false),
                    if screen == Screen::Inbox && chord == "shift-h" {
                        Some(BulkRemind)
                    } else if screen == Screen::Inbox && matches!(action, MoveLeft | MoveRight) {
                        None
                    } else {
                        Some(action)
                    },
                    "{chord} {screen:?}"
                );
                assert_eq!(key(chord, screen, true), None, "editor must own {chord}");
            }
            assert_eq!(
                sequence("g", false, screen),
                (screen != Screen::Task).then_some(First)
            );
            assert_eq!(
                sequence("g", true, screen),
                (screen != Screen::Task).then_some(Last)
            );
            assert_eq!(sequence("i", false, screen), Some(Inbox));
            assert_eq!(sequence("b", false, screen), Some(Board));
            assert_eq!(sequence("x", false, screen), if screen == Screen::Inbox { None } else { Some(ArchivedOnBoard) });
            assert_eq!(key("ctrl-shift-,", screen, true), Some(DiscardDraft));
            assert_eq!(
                key("ctrl-alt-shift-enter", screen, true),
                Some(Save {
                    stay: true,
                    complete: true
                })
            );
        }
        assert_eq!(key("down", Screen::Task, false), Some(NextColumn));
        assert_eq!(key("up", Screen::Task, false), Some(PreviousColumn));
        assert_eq!(key("j", Screen::Task, false), Some(Next));
        assert_eq!(key("k", Screen::Task, false), Some(Previous));
        assert_eq!(key("shift-j", Screen::Inbox, false), Some(ExtendNext));
        assert_eq!(key("shift-up", Screen::Inbox, false), Some(ExtendPrevious));
        assert_eq!(key("ctrl-f", Screen::Board, false), Some(LocalSearch));
        assert_eq!(key("ctrl-r", Screen::Inbox, false), Some(Comment));
        assert_eq!(key("ctrl-d", Screen::Task, false), Some(Description));
        assert_eq!(key("x", Screen::Board, false), Some(Select));
        assert_eq!(
            key("ctrl-a", Screen::Inbox, false),
            Some(SelectAll { all_splits: false })
        );
        assert_eq!(
            key("ctrl-shift-a", Screen::Inbox, false),
            Some(SelectAll { all_splits: true })
        );
    }
    #[test]
    fn windows_shifted_punctuation_retains_physical_chord() {
        for screen in [Screen::Inbox, Screen::Board, Screen::Task] {
            assert_eq!(key("#", screen, false), Some(Action::Delete));
            assert_eq!(key("ctrl-:", screen, false), Some(Action::CopyUrl));
            assert_eq!(key("ctrl-<", screen, true), Some(Action::DiscardDraft));
            assert_eq!(key("ctrl-;", screen, false), Some(Action::CopyTitleUrl));
            assert_eq!(key("ctrl-:", screen, true), None);
            assert_eq!(key("ctrl-alt-enter", screen, true), None);
        }
        assert_eq!(key("cmd-d", Screen::Task, false), None);
        assert_eq!(key("shift-h", Screen::Inbox, false), Some(Action::BulkRemind));
    }
}

#[cfg(test)]
mod create_view_shortcut_tests{
    use super::*;
    #[test]
    fn create_sort_and_filter_keys_never_capture_editor_typing(){
        for screen in [Screen::Board,Screen::Task]{for(chord,action)in [("c",Action::CreateTask),("shift-s",Action::SortView),("shift-f",Action::FilterView)]{let key=Keystroke::parse(chord).unwrap();assert_eq!(resolve(&key,screen,false),Some(action));assert_eq!(resolve(&key,screen,true),None);}}
    }
}
