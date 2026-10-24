use crate::{shortcuts::Action, theme::ThemeName, ui::Screen};

#[derive(Clone, Debug)]
pub struct Command {
    pub key: String,
    pub name: String,
    pub group: &'static str,
    pub keys: String,
    pub keywords: &'static str,
    pub action: CommandAction,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandAction {
    Shortcut(Action),
    Theme(ThemeName),
    Board(i64),
    BoardForm(BoardAction),
    Shell(ShellCommand),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoardAction { AddColumn, RenameColumn, DeleteColumn, RenameBoard, ArchiveBoard, MoveTask }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShellCommand {
    Settings(&'static str),
    Shortcuts,
    QuickTips,
    HelpCenter,
    Feedback,
    Invite,
    Cli,
    Favorites,
    Views,
    CreateTask,
    Sort,
    Filters,
    ValueMatch,
    SortTimeInColumn,
    SortLastComment,
    Columns,
    Subtasks,
    EmptyColumns,
    HideColumn,
    ArchivedOnBoard,
    SmartSplit,
    AutoAssignColumn,
    TaskTimer,
    StartDate,
    WaitingOn,
    AssignMe,
    BoardTime,
    DuplicateTask(bool),
}

// Web declaration order, with unsupported commands omitted rather than inert rows.
pub fn commands(screen: Screen, task: bool) -> Vec<Command> {
    use Action::*;
    let mut rows = Vec::new();
    let mut add = |key: &str, name: &str, group, keys: &str, keywords, action| {
        rows.push(Command {
            key: key.into(),
            name: name.into(),
            group,
            keys: keys.into(),
            keywords,
            action: CommandAction::Shortcut(action),
        })
    };
    if task && screen == Screen::Task {
        task_commands(&mut add);
    }
    if screen == Screen::Inbox {
        add("swipeThroughUnread","Swipe through unread","Inbox","","review unread notifications",ReviewUnread);
    }
    add("composeTask", "New Task", "Navigate", "CTRL J", "compose create task AI writer", Compose);
    add(
        "copyCurrentPageUrl",
        "Copy current page URL",
        "Navigate",
        "",
        "copy current page URL link board view task address clipboard",
        CopyPageUrl,
    );
    add(
        "GotoInbox",
        "Go to inbox",
        "Navigate",
        "G  I",
        "navigate open inbox notifications activity mentions assignments updates feed",
        Inbox,
    );
    add(
        "searchTask",
        "Search",
        "Navigate",
        "/",
        "find search lookup locate query tasks tickets issues content",
        Search,
    );
    add(
        "appShellInbox",
        "App shell: Inbox",
        "App shell",
        "1",
        "inbox",
        Inbox,
    );
    add(
        "appShellBoard",
        "App shell: Board",
        "App shell",
        "2",
        "board kanban",
        Board,
    );
    if task && screen != Screen::Task {
        task_commands(&mut add);
    }
    if task {
        for (key, name, keys, keywords, action) in [
            ("setStartDate", "Set start date", "", "start begin kickoff scheduled from date when planned", ShellCommand::StartDate),
            ("blockedByPerson", "Blocked by person…", "SHIFT B", "blocked waiting blocker block person stuck", ShellCommand::WaitingOn),
            ("assignToMe", "Assign to me", "", "assign me self take claim mine my task i'll do it", ShellCommand::AssignMe),
        ] { rows.push(Command { key:key.into(), name:name.into(), group:"Task", keys:keys.into(), keywords, action:CommandAction::Shell(action) }); }
    }
    if task {
        for (key, name, pick_board) in [("duplicateTask", "Duplicate task", false), ("duplicateTaskToBoard", "Duplicate task to board", true)] {
            rows.push(Command { key:key.into(), name:name.into(), group:"Task", keys:String::new(), keywords:"duplicate copy clone task ticket board", action:CommandAction::Shell(ShellCommand::DuplicateTask(pick_board)) });
        }
    }
    for (key, name, keywords, theme) in [
        (
            "lightTheme",
            "Light · Porcelain",
            "light white day bright porcelain theme appearance",
            ThemeName::Porcelain,
        ),
        (
            "darkTheme",
            "Dark · Graphite",
            "dark night grey gray graphite theme appearance",
            ThemeName::Graphite,
        ),
        (
            "amoledTheme",
            "OLED black · AMOLED",
            "oled amoled true black terminal cyan turquoise theme appearance",
            ThemeName::Amoled,
        ),
        (
            "paperTheme",
            "Paper · Dia",
            "paper editorial serif warm light elegant dia theme appearance",
            ThemeName::Dia,
        ),
    ] {
        rows.push(Command {
            key: key.into(),
            name: name.into(),
            keywords,
            group: "Appearance",
            keys: String::new(),
            action: CommandAction::Theme(theme),
        });
    }
    shell_commands(&mut rows);
    for(key,name,group,keys,keywords,action)in [
        ("createSmartSplit","Add smart split","Board","","add create smart split ai prompt automatic view label tag",ShellCommand::SmartSplit),
        ("toggleArchivedOnBoard","Show archived tasks on board","Board","G  X","show hide toggle archive archived completed tasks board visibility",ShellCommand::ArchivedOnBoard),
        ("createTask","Create task","Board","C","create add new task ticket issue todo work item",ShellCommand::CreateTask),
        ("hideColumn","Hide board column","Board","","hide conceal column status section board kanban visibility remove from view",ShellCommand::HideColumn),
        ("manageColumns","Manage board columns","Board","","columns visibility show hide",ShellCommand::Columns),
        ("subtaskSettings","Sub-task Settings","Board","","subtask checklist settings",ShellCommand::Subtasks),
        ("toggleEmptyColumns","Hide empty board columns","Board","","empty columns sections hide show toggle blank empty column visibility collapse",ShellCommand::EmptyColumns),
        ("sortBoard","Sort board","Board","SHIFT S","sort order arrange board kanban tasks priority date size",ShellCommand::Sort),
        ("ShowFilterHtc","Filters","Navigate","SHIFT F","filter narrow refine search view rules conditions matching tasks",ShellCommand::Filters),
        ("toggleFilterValueMatch","Filter values: match any or all","Board","","filter values match any all labels tags assignees both either",ShellCommand::ValueMatch),
        ("sortByTimeInColumn","Sort by time in column","Board","","sort staleness age time status column oldest",ShellCommand::SortTimeInColumn),
        ("sortByLastComment","Sort by last comment","Board","","sort staleness age comment discussion oldest",ShellCommand::SortLastComment),
    ]{
        if screen!=Screen::Inbox || action==ShellCommand::CreateTask{rows.push(Command{key:key.into(),name:name.into(),group,keys:keys.into(),keywords,action:CommandAction::Shell(action)});}
    }
    if screen == Screen::Board {
        rows.push(Command { key:"autoAssignColumn".into(), name:"Auto-assign for this column".into(), group:"Board", keys:String::new(), keywords:"auto assign column section member owner default workflow", action:CommandAction::Shell(ShellCommand::AutoAssignColumn) });
    }
    rows.sort_by_key(|row| match row.group {
        "Task" if screen == Screen::Task => 0,
        "Board" if screen == Screen::Board => 0,
        "Navigate" => 1,
        "App shell" => 2,
        "Task" => 3,
        "Board" => 4,
        "Appearance" => 5,
        "Settings" => 6,
        "Help" => 7,
        _ => 8,
    });
    rows
}

fn shell_commands(rows: &mut Vec<Command>) {
    use ShellCommand::*;
    for (key, name, group, keys, keywords, action) in [
        ("manageFavorites", "Manage favorites", "Navigate", "", "manage favorites favourites starred pinned sidebar boards reorder remove organize", Favorites),
        ("sendFeedback", "Send feedback", "Board", "", "send feedback bug idea issue report", Feedback),
        ("inviteBoard", "Invite people", "Board", "", "invite people members email board team", Invite),
        ("settingsCalendarShowWeekends", "Settings: Show weekends", "Settings", "", "settings show weekends profile", Settings("calendar")),
        ("settingsLearnQuickTips", "Settings: Quick tips", "Settings", "", "settings quick tips learning help", Settings("learn")),
        ("settingsGeneral", "Settings: General", "Settings", "", "settings general profile personal preferences application", Settings("general")),
        ("settingsAppearance", "Settings: Appearance", "Settings", "", "settings appearance display color scheme personal profile", Settings("appearance")),
        ("settingsPlans", "Settings: Plans", "Settings", "", "settings plans pricing upgrade downgrade subscription team", Settings("plans")),
        ("settingsApiKeys", "Settings: Bring your own key", "Settings", "", "settings bring your own key byok ai api credentials provider team", Settings("apiKeys")),
        ("settingsSkills", "Settings: Skills", "Settings", "", "settings skills prompts slash commands library team ai", Settings("skills")),
        ("settingsBoardMembers", "Settings: Members (board)", "Settings", "", "settings board members people permissions access project", Settings("board-members")),
        ("settingsBoardAi", "Settings: Custom instructions", "Settings", "", "settings board custom instructions prompt behavior configure", Settings("board-ai")),
        ("settingsBoardSkills", "Settings: Skills (board)", "Settings", "", "settings board skills prompts slash commands library ai", Settings("board-skills")),
        ("settingsBoardFiles", "Settings: Files", "Settings", "", "settings board files uploads custom instructions context", Settings("board-files")),
        ("settingsCli", "Settings: CLI", "Settings", "", "settings cli terminal command line shell install connect", Settings("cli")),
        ("settingsShortcuts", "Settings: Shortcuts", "Settings", "", "settings shortcuts keyboard hotkeys keybindings cheatsheet help", Settings("shortcuts")),
        ("settingsLearn", "Settings: Learn Hypertask", "Settings", "", "settings learn hypertask onboarding tutorial guide docs help", Settings("learn")),
        ("shortcuts", "Keyboard shortcuts", "Help", "?", "keyboard shortcuts hotkeys keybindings commands keys cheatsheet help reference", Shortcuts),
        ("settings", "Settings", "Help", "\\", "settings preferences configuration options account application customize setup", Settings("general")),
        ("boardSettings", "Board settings", "Help", "", "board settings preferences configuration options project customize setup", Settings("board-general")),
        ("helpCenter", "Help center", "Help", "", "help center support docs documentation guide questions answers contact", HelpCenter),
        ("quickTips", "Quick tips", "Help", "", "quick tips shortcuts help hints guidance learn keyboard productivity suggestions", QuickTips),
        ("cliInstall", "CLI", "Help", "", "cli terminal command line install npm shell hypertask login", Cli),
    ] {
        rows.push(Command { key: key.into(), name: name.into(), group, keys: keys.into(), keywords, action: CommandAction::Shell(action) });
    }
    for (key, name) in [
        ("settingsApiKeysOpenaiApiKey", "Settings: OpenAI API key"),
        ("settingsApiKeysAnthropicApiKey", "Settings: Anthropic API key"),
        ("settingsApiKeysGoogleApiKey", "Settings: Google API key"),
        ("settingsApiKeysDeepseekApiKey", "Settings: DeepSeek API key"),
        ("settingsApiKeysMoonshotKimiApiKey", "Settings: Moonshot (Kimi) API key"),
        ("settingsApiKeysZAiGlmApiKey", "Settings: Z.ai (GLM) API key"),
        ("settingsApiKeysAlibabaQwenApiKey", "Settings: Alibaba (Qwen) API key"),
        ("settingsApiKeysOpenrouterApiKey", "Settings: OpenRouter API key"),
        ("settingsApiKeysVercelAiGatewayApiKey", "Settings: Vercel AI Gateway API key"),
        ("settingsApiKeysCustomEndpointOperatesUnderAnEuUsDataAgreement", "Settings: Custom endpoint EU/US agreement"),
    ] { rows.push(Command { key:key.into(), name:name.into(), group:"Settings", keys:String::new(), keywords:"settings api key use byok", action:CommandAction::Shell(Settings("apiKeys")) }); }
}
fn task_commands(
    add: &mut impl FnMut(&'static str, &'static str, &'static str, &'static str, &'static str, Action),
) {
    use Action::*;
    for (key, name, keys, keywords, action) in [
        (
            "setDueDate",
            "Set due date",
            "D",
            "set due date deadline scheduled calendar when date time",
            DueDate,
        ),
        (
            "assignUser",
            "Assign task",
            "A",
            "assign user person member responsibility",
            Assign,
        ),
        (
            "labelmodal",
            "Set tags",
            "T",
            "tags labels categories",
            Tags,
        ),
        (
            "estimateModal",
            "Set task size",
            "S",
            "task size estimate effort points complexity",
            Size,
        ),
        (
            "priorityModal",
            "Set priority",
            "P",
            "priority urgency important high low medium",
            Priority,
        ),
        (
            "moveToColumn",
            "Move task to column",
            "M",
            "move column section status kanban",
            MoveColumn,
        ),
        (
            "renameTask",
            "Rename task",
            "F2",
            "rename edit change task title",
            Rename,
        ),
        (
            "archiveTask",
            "Archive task",
            "CTRL E",
            "archive unarchive done complete task",
            Archive,
        ),
        (
            "deleteTask",
            "Delete task",
            "SHIFT 3",
            "delete trash remove task",
            Delete,
        ),
        (
            "copyformattedTaskLink",
            "Copy private task link (title + URL)",
            "CTRL ;",
            "copy private task link title URL clipboard",
            CopyTitleUrl,
        ),
        (
            "copyLink",
            "Copy private task URL",
            "CTRL SHIFT ;",
            "copy private task URL link clipboard",
            CopyUrl,
        ),
        (
            "copyTaskId",
            "Copy task ID",
            "CTRL SHIFT I",
            "copy task id ticket number clipboard",
            CopyId,
        ),
        (
            "copyTaskTitleId",
            "Copy task title and ID",
            "CTRL I",
            "copy task title id ticket number clipboard",
            CopyTitleId,
        ),
    ] {
        add(key, name, "Task", keys, keywords, action);
    }
}

fn edit_distance_one(left: &str, right: &str) -> bool {
    let a: Vec<_> = left.chars().collect();
    let b: Vec<_> = right.chars().collect();
    if a.len().abs_diff(b.len()) > 1 {
        return false;
    }
    if a.len() == b.len() {
        let mismatch: Vec<_> = (0..a.len()).filter(|&i| a[i] != b[i]).collect();
        return mismatch.len() <= 1
            || (mismatch.len() == 2
                && mismatch[1] == mismatch[0] + 1
                && a[mismatch[0]] == b[mismatch[1]]
                && a[mismatch[1]] == b[mismatch[0]]);
    }
    let (short, long) = if a.len() < b.len() {
        (&a, &b)
    } else {
        (&b, &a)
    };
    let (mut i, mut j, mut edits) = (0, 0, 0);
    while i < short.len() && j < long.len() {
        if short[i] == long[j] {
            i += 1;
        } else {
            edits += 1;
            if edits > 1 {
                return false;
            }
        }
        j += 1;
    }
    true
}
fn words(value: &str) -> impl Iterator<Item = &str> {
    value
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|s| !s.is_empty())
}
fn score(command: &Command, token: &str) -> u32 {
    let name = command.name.to_lowercase();
    let keywords = command.keywords.to_lowercase();
    if name.starts_with(token) {
        100
    } else if words(&name).any(|w| w.starts_with(token)) {
        80
    } else if name.contains(token) {
        60
    } else if words(&keywords).any(|w| w.starts_with(token)) {
        40
    } else if keywords.contains(token) {
        20
    } else if token.chars().count() < 4 {
        0
    } else if words(&name).any(|w| edit_distance_one(token, w)) {
        10
    } else if words(&keywords).any(|w| edit_distance_one(token, w)) {
        5
    } else {
        0
    }
}
/// Port of useHTC.tsx: all tokens must match; stable score order, typo/transposition tolerance, 30 rows.
pub fn filter<'a>(commands: &'a [Command], query: &str) -> Vec<&'a Command> {
    ranked(commands, query, &serde_json::Value::Null, 0)
}

pub fn frecency(frequency: u64, last_used: i64, now: i64) -> f64 {
    (frequency as f64).ln_1p()
        * 0.5_f64.powf((now - last_used).max(0) as f64 / (14. * 86400. * 1000.))
}

pub fn ranked<'a>(
    commands: &'a [Command],
    query: &str,
    usage: &serde_json::Value,
    now: i64,
) -> Vec<&'a Command> {
    let query = query.to_lowercase();
    let tokens: Vec<_> = query.split_whitespace().collect();
    if tokens.is_empty() {
        return commands.iter().collect();
    }
    let mut seen = std::collections::BTreeSet::new();
    let mut ranked: Vec<_> = commands
        .iter()
        .filter(|c| c.group != "Frequently used" && seen.insert((&c.key, &c.name)))
        .filter_map(|command| {
            let scores: Vec<_> = tokens.iter().map(|token| score(command, token)).collect();
            let used = &usage[&command.key];
            let boost = frecency(
                used["frequency"].as_u64().unwrap_or(0),
                used["lastUsedAt"].as_i64().unwrap_or(now),
                now,
            );
            (!scores.contains(&0)).then_some((command, scores.iter().sum::<u32>() as f64 + boost))
        })
        .collect();
    ranked.sort_by(|(_, a), (_, b)| b.total_cmp(a));
    ranked
        .into_iter()
        .take(30)
        .map(|(command, _)| command)
        .collect()
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
        assert!(!rows.iter().any(|r| {
            ["calendarSettings", "openAiTaskWriter", "settingsAiModelsGdprSafeMode", "startKanbanTutorial"].contains(&r.key.as_str())
        }));
        assert_eq!(rows.iter().find(|row|row.key=="settingsApiKeysOpenaiApiKey").unwrap().action, CommandAction::Shell(ShellCommand::Settings("apiKeys")));
        let rows = commands(Screen::Task, true);
        assert_eq!(rows[0].group, "Task");
        assert!(edit_distance_one("archive", "archvie"));
        assert!(edit_distance_one("graphte", "graphite"));
        assert!(!edit_distance_one("ab", "cd"));
    }
    #[test]
    fn shell_command_groups_follow_web_context_order_and_settings_payloads() {
        for (screen, first) in [(Screen::Inbox, "Navigate"), (Screen::Board, "Board"), (Screen::Task, "Task")] {
            let rows = commands(screen, true);
            assert_eq!(rows[0].group, first);
            let position = |group| rows.iter().position(|row| row.group == group).unwrap();
            assert!(position("Appearance") < position("Settings"));
            assert!(position("Settings") < position("Help"));
            assert_eq!(rows.iter().filter(|row| row.key == "boardSettings").count(), 1);
            for (key, section) in [("settingsCalendarShowWeekends", "calendar"), ("settingsLearnQuickTips", "learn"), ("boardSettings", "board-general")] {
                assert_eq!(rows.iter().find(|row| row.key == key).unwrap().action, CommandAction::Shell(ShellCommand::Settings(section)));
            }
        }
    }
    #[test]
    fn usage_matches_web_decay_and_never_duplicates_frequent_results() {
        assert!((frecency(1, 0, 0) - 2_f64.ln()).abs() < 0.000001);
        assert!((frecency(1, 0, 14 * 86400 * 1000) - 2_f64.ln() / 2.).abs() < 0.000001);
        assert_eq!(frecency(0, 0, 0), 0.);
        let mut rows = commands(Screen::Task, true);
        let usage = serde_json::json!({"copyTaskId":{"frequency":100,"lastUsedAt":1000}});
        let mut frequent = rows.iter().find(|r| r.key == "copyTaskId").unwrap().clone();
        frequent.group = "Frequently used";
        rows.insert(0, frequent);
        let matched = ranked(&rows, "copy", &usage, 1000);
        assert_eq!(matched.iter().filter(|r| r.key == "copyTaskId").count(), 1);
    }
}

#[cfg(test)]
mod composer2_contract_tests {
    use super::*;
    #[test]
    fn auth_blocked_inbox_and_agent_actions_are_not_fake_palette_commands() {
        let rows=commands(Screen::Inbox,true);
        assert!(rows.iter().any(|row|row.key=="swipeThroughUnread"));
        for blocked in ["clearInboxToZero","archiveAllReadNotifications","archiveReactionNotifications","starTask","nextAgent","previousAgent","sendAgentMessage"] {
            assert!(!rows.iter().any(|row|row.key==blocked),"blocked command {blocked} must not pretend to work");
        }
    }
}

#[cfg(test)]
mod advanced_commands_tests{
    use super::*;
    #[test]
    fn advanced_view_and_expanded_create_commands_have_real_native_dispatch(){
        let rows=commands(Screen::Board,true);
        for(key,action)in [("createTask",ShellCommand::CreateTask),("sortBoard",ShellCommand::Sort),("ShowFilterHtc",ShellCommand::Filters),("toggleFilterValueMatch",ShellCommand::ValueMatch),("sortByTimeInColumn",ShellCommand::SortTimeInColumn),("sortByLastComment",ShellCommand::SortLastComment)]{
            assert_eq!(rows.iter().find(|row|row.key==key).unwrap().action,CommandAction::Shell(action));
        }
    }
}
