use super::*;
use crate::command_center::ShellCommand;

impl Desktop {
    pub(in crate::ui) fn run_shell_command(
        &mut self,
        action: ShellCommand,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match action {
            ShellCommand::TaskWriter => self.open_compose(window, cx),
            ShellCommand::SummarizeTicket => {
                if self.screen != Screen::Task { return; }
                if let Some(sidebar) = &self.ai_sidebar {
                    sidebar.update(cx, |sidebar, cx| sidebar.summarize_ticket(window, cx));
                    self.shell.ai_open = sidebar.read(cx).is_open();
                }
            }
            ShellCommand::DeclineDuplicate => {
                self.show_modal(crate::ui::interactions::Modal::Relation(ht_core::RelationKind::Duplicate), window, cx);
                self.task_actions.decline_duplicate = true;
            }
            ShellCommand::Relation(kind) => self.show_modal(crate::ui::interactions::Modal::Relation(kind), window, cx),
            ShellCommand::Recurrence => self.show_modal(crate::ui::interactions::Modal::Recurrence, window, cx),
            ShellCommand::StartDate => self.show_start_date(window, cx),
            ShellCommand::WaitingOn => self.show_modal(crate::ui::interactions::Modal::WaitingOn, window, cx),
            ShellCommand::AssignMe => {
                if let Some(task) = self.selected_task() { self.queue_task_action(task, ht_core::TaskAction::AssignMe, window, cx); }
            }
            ShellCommand::NewFromTemplate => self.open_task_templates(window,cx),
            ShellCommand::AcceptTask => {
                if let Some(task) = self.selected_task() {
                    match self.db.accept_task_action(task) {
                        Ok(change) => self.queue_task_action(task, change, window, cx),
                        Err(error) => { self.error = Some(error.to_string()); cx.notify(); }
                    }
                }
            }
            ShellCommand::DeclineTask | ShellCommand::SaveTemplate => {
                if let Some(task) = self.selected_task() {
                    self.queue_task_action(task, if action == ShellCommand::DeclineTask { ht_core::TaskAction::Decline } else { ht_core::TaskAction::SaveTemplate }, window, cx);
                }
            }
            ShellCommand::TaskTimer => self.toggle_task_timer(window,cx),
            ShellCommand::BoardTime => self.toggle_board_time(window,cx),
            ShellCommand::AutoAssignColumn => { self.views.column_auto_select = true; self.open_view_panel(crate::ui::board_views::Panel::Columns,window,cx); },
            ShellCommand::SmartSplit => self.open_smart_split(None,window,cx),
            ShellCommand::DuplicateTask(pick_board)=>self.open_duplicate_task(pick_board,window,cx),
            ShellCommand::ArchivedOnBoard=>self.toggle_archived_on_board(window,cx),
            ShellCommand::HideColumn=>self.hide_current_column(window,cx),
            ShellCommand::Columns=>{self.views.column_auto_select=false;self.open_view_panel(crate::ui::board_views::Panel::Columns,window,cx);},
            ShellCommand::Subtasks=>self.open_view_panel(crate::ui::board_views::Panel::Subtasks,window,cx),
            ShellCommand::EmptyColumns=>self.toggle_empty_columns(window,cx),
            ShellCommand::CreateTask=>self.open_create_task(window,cx),
            ShellCommand::Sort=>self.open_view_panel(crate::ui::board_views::Panel::Sort,window,cx),
            ShellCommand::Filters=>self.open_view_panel(crate::ui::board_views::Panel::Filters,window,cx),
            ShellCommand::ValueMatch=>self.open_view_panel(crate::ui::board_views::Panel::ValueMatch,window,cx),
            ShellCommand::SortTimeInColumn|ShellCommand::SortLastComment=>self.command_sort(if action==ShellCommand::SortTimeInColumn{"TimeInColumn"}else{"TimeWithoutComment"},window,cx),
            ShellCommand::Views => self.open_view_panel(crate::ui::board_views::Panel::Manage,window,cx),
            ShellCommand::Favorites => self.open_favorites(window, cx),
            ShellCommand::Settings(section) => {
                self.open_settings(window, cx);
                self.select_settings(section, window, cx);
            }
            ShellCommand::Shortcuts => self.open_shortcuts(window, cx),
            ShellCommand::QuickTips => {
                self.footer = !self.footer;
                if let Err(error) = self.db.kv_set("showQuickTips", if self.footer { "true" } else { "false" }) {
                    self.error = Some(error.to_string());
                }
                cx.notify();
            }
            ShellCommand::HelpCenter => cx.open_url("https://hypertask.ai/help"),
            ShellCommand::Feedback => self.open_feedback(window, cx),
            ShellCommand::Invite => self.open_invite(window, cx),
            ShellCommand::Cli => {
                self.shell.cli = true;
                window.focus(&self.focus, cx);
                cx.notify();
            }
        }
    }

    pub(in crate::ui::shell) fn close_cli(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.shell.cli = false;
        window.focus(&self.focus, cx);
        cx.notify();
    }

    pub(in crate::ui::shell) fn cli_overlay(&self, cx: &mut Context<Self>) -> AnyElement {
        div().absolute().inset_0().bg(rgba(0x00000040))
            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| this.close_cli(window, cx)))
            .child(div().id("cli-install-modal").absolute().top(px(56.)).left(gpui::relative(0.5)).ml(px(-300.))
                .w(px(600.)).p(px(20.)).rounded(px(8.)).bg(color(self.colors.container))
                .flex().flex_col().gap(px(12.))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(div().text_size(px(18.)).font_weight(FontWeight::BOLD).child("Hypertask CLI")
                    .layout_key(&self.layout, "shell.cli.title", Some("Hypertask CLI".into())))
                .child("Manage your tasks from the terminal.")
                .child("1. Install").child("Run this in your terminal on macOS or Linux.")
                .child(self.code_row("install", INSTALL, cx))
                .child("2. Sign in").child("Opens your browser to authorize the CLI for your account.")
                .child(self.code_row("login", LOGIN, cx))
                .child("That's it. Run hypertask --help to see everything you can do.")
                .child(self.settings_link("See the docs", "https://docs.hypertask.ai"))
                .child(self.settings_action("cli-mcp", "Connect AI assistants via MCP →")
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.close_cli(window, cx);
                        this.run_shell_command(ShellCommand::Settings("mcp"), window, cx);
                    })))
                .child(self.settings_action("cli-close", "Close")
                    .on_click(cx.listener(|this, _, window, cx| this.close_cli(window, cx)))
                    .layout_key(&self.layout, "shell.cli.close", None))
                .layout_key(&self.layout, "shell.cli", None))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::shell::tests::{click, draw, measured, setup};

    fn command(view: gpui::WindowHandle<Desktop>, cx: &mut gpui::VisualTestContext, key: &str) {
        cx.simulate_keystrokes("ctrl-k");
        let rows = crate::command_center::commands(Screen::Inbox, false);
        let name = &rows.iter().find(|row| row.key == key).expect(key).name;
        cx.simulate_input(name);
        let index = view.read_with(cx, |this, cx| {
            assert_eq!(this.interaction.modal, Some(crate::ui::interactions::Modal::Commands), "{key}: Ctrl+K must open the palette");
            assert_eq!(this.interaction.query.read(cx).content.as_ref(), name, "{key}: query must reach the palette input");
            let rows = this.command_rows();
            let usage = this.db.kv_get("command-usage").unwrap().and_then(|value| serde_json::from_str::<Value>(&value).ok()).unwrap_or(Value::Null);
            crate::command_center::ranked(&rows, name, &usage, chrono::Utc::now().timestamp_millis()).iter().position(|row| row.key == key).expect("command must match its web name")
        }).unwrap();
        for _ in 0..index { cx.simulate_keystrokes("down"); }
        cx.simulate_keystrokes("enter");
        draw(view, cx);
    }

    #[gpui::test]
    fn shell_palette_summarize_ticket_opens_existing_chat_only_on_task_detail(cx: &mut gpui::TestAppContext) {
        cx.update(crate::input::init);
        let view = cx.open_window(gpui::size(px(1440.), px(900.)), |window, cx| crate::ui::task_fields::tests::desktop(window, cx));
        let mut cx = gpui::VisualTestContext::from_window(view.into(), cx);
        draw(view, &mut cx);
        cx.simulate_keystrokes("ctrl-k");
        cx.simulate_input("Summarize ticket");
        view.read_with(&cx, |this, _| assert!(this.command_rows().iter().any(|row| row.key == "summarizeTicket"))).unwrap();
        cx.simulate_keystrokes("enter");
        draw(view, &mut cx);
        view.read_with(&cx, |this, cx| {
            assert!(this.interaction.modal.is_none());
            assert!(this.ai_sidebar.as_ref().unwrap().read(cx).is_open());
            assert!(this.db.mutations().unwrap().is_empty());
        }).unwrap();
    }

    #[gpui::test]
    fn shell_palette_query_keeps_j_and_k_while_arrows_navigate(cx: &mut gpui::TestAppContext) {
        let (view, mut cx) = setup(cx);
        cx.simulate_keystrokes("ctrl-k");
        cx.simulate_input("j k");
        view.read_with(&cx, |this, cx| assert_eq!(this.interaction.query.read(cx).content.as_ref(), "j k")).unwrap();
        cx.simulate_keystrokes("ctrl-a backspace");
        cx.simulate_input("Settings");
        cx.simulate_keystrokes("down");
        view.read_with(&cx, |this, _| assert_eq!(this.interaction.choice, 1)).unwrap();
        cx.simulate_keystrokes("up");
        view.read_with(&cx, |this, _| assert_eq!(this.interaction.choice, 0)).unwrap();
    }

    #[gpui::test]
    fn shell_palette_favorites_opens_the_web_ten_slot_panel_without_a_write(cx: &mut gpui::TestAppContext) {
        let(view,mut cx)=setup(cx);
        command(view,&mut cx,"manageFavorites");
        measured(view,&mut cx,"shell.favorites.manager");
        view.read_with(&cx,|this,_|assert!(this.db.mutations().unwrap().is_empty())).unwrap();
        cx.simulate_keystrokes("escape");
        measured(view,&mut cx,"inbox.tabs");
    }

    #[gpui::test]
    fn shell_palette_settings_targets_open_without_toggling_controls(cx: &mut gpui::TestAppContext) {
        let (view, mut cx) = setup(cx);
        for (key, section) in [
            ("settingsCalendarShowWeekends", "calendar"),
            ("settingsLearnQuickTips", "learn"),
            ("boardSettings", "board-general"),
            ("settingsCli", "cli"),
            ("settingsShortcuts", "shortcuts"),
            ("settingsGeneral", "general"),
            ("settingsAppearance", "appearance"),
            ("settingsPlans", "plans"),
            ("settingsApiKeys", "apiKeys"),
            ("settingsApiKeysOpenaiApiKey", "apiKeys"),
            ("settingsApiKeysAnthropicApiKey", "apiKeys"),
            ("settingsApiKeysGoogleApiKey", "apiKeys"),
            ("settingsApiKeysDeepseekApiKey", "apiKeys"),
            ("settingsApiKeysMoonshotKimiApiKey", "apiKeys"),
            ("settingsApiKeysZAiGlmApiKey", "apiKeys"),
            ("settingsApiKeysAlibabaQwenApiKey", "apiKeys"),
            ("settingsApiKeysOpenrouterApiKey", "apiKeys"),
            ("settingsApiKeysVercelAiGatewayApiKey", "apiKeys"),
            ("settingsApiKeysCustomEndpointOperatesUnderAnEuUsDataAgreement", "apiKeys"),
            ("settingsSkills", "skills"),
            ("settingsBoardMembers", "board-members"),
            ("settingsBoardAi", "board-ai"),
            ("settingsBoardSkills", "board-skills"),
            ("settingsBoardFiles", "board-files"),
            ("settingsLearn", "learn"),
        ] {
            command(view, &mut cx, key);
            view.read_with(&cx, |this, _| {
                assert!(this.shell.settings, "{key} must open native settings");
                assert_eq!(this.shell.settings_state.section, section, "{key}");
                assert_eq!(this.shell.settings_state.board, 15);
                assert!(!this.shell.settings_state.calendar_weekends);
                assert!(this.footer);
                assert!(this.interaction.modal.is_none());
            }).unwrap();
            cx.simulate_keystrokes("escape");
            measured(view, &mut cx, "inbox.tabs");
        }
    }

    #[gpui::test]
    fn shell_palette_quick_tips_persists_and_help_opens_exact_web_url(cx: &mut gpui::TestAppContext) {
        let (view, mut cx) = setup(cx);
        for expected in [false, true] {
            command(view, &mut cx, "quickTips");
            view.read_with(&cx, |this, _| {
                assert_eq!(this.footer, expected);
                assert_eq!(this.db.kv_get("showQuickTips").unwrap(), Some(expected.to_string()));
            }).unwrap();
        }
        command(view, &mut cx, "helpCenter");
        assert_eq!(cx.opened_url().as_deref(), Some("https://hypertask.ai/help"));
        measured(view, &mut cx, "inbox.tabs");
    }

    #[gpui::test]
    fn shell_palette_feedback_and_invite_reuse_real_forms_and_preserve_route(cx: &mut gpui::TestAppContext) {
        let (view, mut cx) = setup(cx);
        command(view, &mut cx, "sendFeedback");
        measured(view, &mut cx, "shell.feedback.title");
        cx.simulate_keystrokes("escape");
        command(view, &mut cx, "inviteBoard");
        measured(view, &mut cx, "shell.invite.title");
        cx.simulate_keystrokes("escape");
        measured(view, &mut cx, "inbox.tabs");
        view.read_with(&cx, |this, _| assert!(this.db.mutations().unwrap().is_empty())).unwrap();
    }

    #[gpui::test]
    fn shell_palette_cli_is_a_modal_with_copy_close_escape_and_outside_dismissal(cx: &mut gpui::TestAppContext) {
        let (view, mut cx) = setup(cx);
        for dismissal in [0, 1, 2] {
            command(view, &mut cx, "cliInstall");
            measured(view, &mut cx, "shell.cli.title");
            click(view, &mut cx, "settings.copy.install");
            assert_eq!(cx.read_from_clipboard().unwrap().text().as_deref(), Some(INSTALL));
            click(view, &mut cx, "settings.copy.login");
            assert_eq!(cx.read_from_clipboard().unwrap().text().as_deref(), Some(LOGIN));
            match dismissal {
                0 => click(view, &mut cx, "shell.cli.close"),
                1 => cx.simulate_keystrokes("escape"),
                _ => cx.simulate_click(point(px(50.), px(800.)), Default::default()),
            }
            view.read_with(&cx, |this, _| { assert!(!this.shell.cli); assert_eq!(this.screen, Screen::Inbox); }).unwrap();
        }
    }
}
