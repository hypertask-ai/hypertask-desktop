use crate::{shortcuts::Action, theme::ThemeName, ui::Screen};

#[derive(Clone, Debug)]
pub struct Command {
    pub key: &'static str,
    pub name: &'static str,
    pub group: &'static str,
    pub keys: &'static str,
    pub keywords: &'static str,
    pub action: CommandAction,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandAction { Shortcut(Action), Theme(ThemeName) }

// Web declaration order, with unsupported commands omitted rather than inert rows.
pub fn commands(screen: Screen, task: bool) -> Vec<Command> {
    use Action::*;
    let mut rows = Vec::new();
    let mut add = |key, name, group, keys, keywords, action| rows.push(Command { key, name, group, keys, keywords, action: CommandAction::Shortcut(action) });
    if task && screen == Screen::Task {
        task_commands(&mut add);
    }
    add("copyCurrentPageUrl", "Copy current page URL", "Navigate", "", "copy current page URL link board view task address clipboard", CopyUrl);
    add("GotoInbox", "Go to inbox", "Navigate", "G  I", "navigate open inbox notifications activity mentions assignments updates feed", Inbox);
    add("searchTask", "Search", "Navigate", "/", "find search lookup locate query tasks tickets issues content", Search);
    add("appShellInbox", "App shell: Inbox", "App shell", "1", "inbox", Inbox);
    add("appShellBoard", "App shell: Board", "App shell", "2", "board kanban", Board);
    if task && screen != Screen::Task { task_commands(&mut add); }
    for (key, name, keywords, theme) in [
        ("lightTheme", "Light · Porcelain", "light white day bright porcelain theme appearance", ThemeName::Porcelain),
        ("darkTheme", "Dark · Graphite", "dark night grey gray graphite theme appearance", ThemeName::Graphite),
        ("amoledTheme", "OLED black · AMOLED", "oled amoled true black terminal cyan turquoise theme appearance", ThemeName::Amoled),
        ("paperTheme", "Paper · Dia", "paper editorial serif warm light elegant dia theme appearance", ThemeName::Dia),
    ] { rows.push(Command { key, name, keywords, group: "Appearance", keys: "", action: CommandAction::Theme(theme) }); }
    rows
}
fn task_commands(add: &mut impl FnMut(&'static str, &'static str, &'static str, &'static str, &'static str, Action)) {
    use Action::*;
    for (key, name, keys, keywords, action) in [
        ("setDueDate", "Set due date", "D", "set due date deadline scheduled calendar when date time", DueDate),
        ("estimateModal", "Set task size", "S", "task size estimate effort points complexity", Size),
        ("priorityModal", "Set priority", "P", "priority urgency important high low medium", Priority),
        ("moveToColumn", "Move task to column", "M", "move column section status kanban", MoveColumn),
        ("renameTask", "Rename task", "F2", "rename edit change task title", Rename),
        ("archiveTask", "Archive task", "CTRL E", "archive unarchive done complete task", Archive),
        ("deleteTask", "Delete task", "SHIFT 3", "delete trash remove task", Delete),
        ("copyformattedTaskLink", "Copy private task link (title + URL)", "CTRL ;", "copy private task link title URL clipboard", CopyTitleUrl),
        ("copyLink", "Copy private task URL", "CTRL SHIFT ;", "copy private task URL link clipboard", CopyUrl),
        ("copyTaskId", "Copy task ID", "CTRL SHIFT I", "copy task id ticket number clipboard", CopyId),
        ("copyTaskTitleId", "Copy task title and ID", "CTRL I", "copy task title id ticket number clipboard", CopyTitleId),
    ] { add(key, name, "Task", keys, keywords, action); }
}

fn edit_distance_one(left: &str, right: &str) -> bool {
    let a: Vec<_> = left.chars().collect();
    let b: Vec<_> = right.chars().collect();
    if a.len().abs_diff(b.len()) > 1 { return false; }
    if a.len() == b.len() {
        let mismatch: Vec<_> = (0..a.len()).filter(|&i| a[i] != b[i]).collect();
        return mismatch.len() <= 1 || (mismatch.len() == 2 && mismatch[1] == mismatch[0] + 1 && a[mismatch[0]] == b[mismatch[1]] && a[mismatch[1]] == b[mismatch[0]]);
    }
    let (short, long) = if a.len() < b.len() { (&a, &b) } else { (&b, &a) };
    let (mut i, mut j, mut edits) = (0, 0, 0);
    while i < short.len() && j < long.len() {
        if short[i] == long[j] { i += 1; } else { edits += 1; if edits > 1 { return false; } }
        j += 1;
    }
    true
}
fn words(value: &str) -> impl Iterator<Item = &str> { value.split(|c: char| !c.is_ascii_alphanumeric()).filter(|s| !s.is_empty()) }
fn score(command: &Command, token: &str) -> u32 {
    let name = command.name.to_lowercase();
    let keywords = command.keywords.to_lowercase();
    if name.starts_with(token) { 100 }
    else if words(&name).any(|w| w.starts_with(token)) { 80 }
    else if name.contains(token) { 60 }
    else if words(&keywords).any(|w| w.starts_with(token)) { 40 }
    else if keywords.contains(token) { 20 }
    else if token.chars().count() < 4 { 0 }
    else if words(&name).any(|w| edit_distance_one(token, w)) { 10 }
    else if words(&keywords).any(|w| edit_distance_one(token, w)) { 5 }
    else { 0 }
}
/// Port of useHTC.tsx: all tokens must match; stable score order, typo/transposition tolerance, 30 rows.
pub fn filter<'a>(commands: &'a [Command], query: &str) -> Vec<&'a Command> {
    let query = query.to_lowercase();
    let tokens: Vec<_> = query.split_whitespace().collect();
    if tokens.is_empty() { return commands.iter().collect(); }
    let mut ranked: Vec<_> = commands.iter().filter_map(|command| {
        let scores: Vec<_> = tokens.iter().map(|token| score(command, token)).collect();
        (!scores.contains(&0)).then_some((command, scores.iter().sum::<u32>()))
    }).collect();
    ranked.sort_by_key(|(_, score)| std::cmp::Reverse(*score));
    ranked.into_iter().take(30).map(|(command, _)| command).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn filter_matches_web_tokens_keywords_typos_and_stable_order() {
        let rows = commands(Screen::Task, true);
        assert_eq!(filter(&rows, "").len(), rows.len());
        assert_eq!(filter(&rows, "  ARCHVIE task ")[0].key, "archiveTask");
        assert_eq!(filter(&rows, "deadline")[0].key, "setDueDate");
        assert_eq!(filter(&rows, "graphte")[0].key, "darkTheme");
        assert!(filter(&rows, "arch").iter().any(|r| r.key == "archiveTask"));
        assert!(filter(&rows, "arcive unknown-no-match").is_empty());
        assert!(filter(&rows, "zz").is_empty());
        assert_eq!(filter(&rows, "copy")[0].key, "copyformattedTaskLink");
    }
    #[test]
    fn no_task_commands_without_target_and_no_hidden_noops() {
        let rows = commands(Screen::Inbox, false);
        assert!(rows.iter().all(|r| r.group != "Task"));
        assert!(!rows.iter().any(|r| ["calendarSettings", "settings", "openAiTaskWriter"].contains(&r.key)));
        let rows = commands(Screen::Task, true);
        assert_eq!(rows[0].group, "Task");
        assert!(edit_distance_one("archive", "archvie"));
        assert!(edit_distance_one("graphte", "graphite"));
        assert!(!edit_distance_one("ab", "cd"));
    }
}
