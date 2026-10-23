use super::*;

impl Desktop {
    pub(in crate::ui) fn open_invite(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self
            .shell
            .settings_state
            .pending
            .as_ref()
            .is_some_and(|r| r.method != "GET")
        {
            return;
        }
        self.shell.invite = true;
        self.shell.settings_state.board = self.options.board;
        self.shell.settings_state.error = None;
        self.shell.invite_emails.clear();
        self.shell
            .invite_input
            .update(cx, |input, cx| input.set_content(String::new(), cx));
        window.focus(&self.shell.invite_input.focus_handle(cx), cx);
        cx.notify();
    }

    pub(in crate::ui) fn send_invites(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self
            .shell
            .settings_state
            .pending
            .as_ref()
            .is_some_and(|r| r.method != "GET")
        {
            return;
        }
        self.shell.invite_emails = self
            .shell
            .invite_input
            .read(cx)
            .content
            .split([',', ' ', '\n'])
            .filter_map(|s| {
                let s = s.trim();
                let (name, domain) = s.split_once('@')?;
                (!name.is_empty()
                    && domain.contains('.')
                    && !domain.starts_with('.')
                    && !domain.ends_with('.'))
                .then(|| s.to_owned())
            })
            .collect();
        if self.shell.invite_emails.is_empty() {
            return;
        }
        self.invite_next(window, cx);
    }

    pub(super) fn invite_next(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(email) = self.shell.invite_emails.pop_front() {
            self.settings_request(
                "POST",
                format!("/projects/{}/members", self.shell.settings_state.board),
                Some(serde_json::json!({"userToAdd": email})),
                window,
                cx,
            );
        } else {
            self.shell.invite = false;
            window.focus(&self.focus, cx);
            cx.notify();
        }
    }

    pub(in crate::ui) fn invite_overlay(&self, cx: &mut Context<Self>) -> AnyElement {
        div()
            .absolute()
            .inset_0()
            .bg(rgba(0x00000040))
            .flex()
            .justify_center()
            .child(
                div()
                    .id("invite-member-modal")
                    .absolute()
                    .top(px(216.))
                    .w(px(560.))
                    .rounded(px(5.))
                    .bg(color(self.colors.container))
                    .shadow(shadow())
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .px(px(20.))
                            .py(px(12.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Invite to Board (share)")
                            .layout_key(
                                &self.layout,
                                "shell.invite.title",
                                Some("Invite to Board (share)".into()),
                            ),
                    )
                    .child(
                        div()
                            .p(px(16.))
                            .child(self.shell.invite_input.clone())
                            .layout_key(&self.layout, "shell.invite.input", None),
                    )
                    .when_some(self.shell.settings_state.error.clone(), |d, error| {
                        d.child(
                            div()
                                .p(px(16.))
                                .text_color(color(0xcf222e))
                                .child(error.clone())
                                .layout_key(&self.layout, "shell.invite.error", Some(error)),
                        )
                    })
                    .layout_key(&self.layout, "shell.invite", None),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::reply;
    use super::*;
    use crate::ui::shell::tests::{click, measured, setup};
    #[gpui::test]
    fn rail_invite_enter_sends_each_email_to_current_board_and_escape_closes(
        cx: &mut gpui::TestAppContext,
    ) {
        let (view, mut cx) = setup(cx);
        click(view, &mut cx, "rail.icon.9");
        assert_eq!(
            measured(view, &mut cx, "shell.invite.title")["text"],
            "Invite to Board (share)"
        );
        cx.simulate_input("one@example.com, two@example.com");
        cx.simulate_keystrokes("enter");
        let first = reply(view, &mut cx, serde_json::json!({"success": true}));
        assert_eq!(first.path, "/projects/15/members");
        assert_eq!(first.body.unwrap()["userToAdd"], "one@example.com");
        let second = reply(view, &mut cx, serde_json::json!({"success": true}));
        assert_eq!(second.body.unwrap()["userToAdd"], "two@example.com");
        measured(view, &mut cx, "inbox.tabs");
        click(view, &mut cx, "rail.icon.9");
        cx.simulate_keystrokes("escape");
        measured(view, &mut cx, "inbox.tabs");
    }
}
