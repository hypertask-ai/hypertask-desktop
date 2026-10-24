use super::*;

pub(super) struct Webhooks {
    url: Entity<TextInput>,
    events: std::collections::BTreeSet<String>,
    delete: Option<String>,
    pub(super) secret: Option<String>,
}
const EVENTS: &[&str] = &[
    "task.created",
    "task.updated",
    "task.assigned",
    "task.unassigned",
    "comment.created",
    "comment.mention",
];
impl Webhooks {
    pub(super) fn new(cx: &mut Context<Desktop>) -> Self {
        let url = cx.new(|cx| {
            TextInput::new(
                String::new(),
                "https://example.com/hypertask",
                color(0x8e9093),
                cx,
            )
        });
        cx.observe(&url, |_, _, cx| cx.notify()).detach();
        Self {
            url,
            events: EVENTS.iter().map(|s| s.to_string()).collect(),
            delete: None,
            secret: None,
        }
    }
}

impl Desktop {
    pub(super) fn webhook_settings(&self, cx: &mut Context<Self>) -> AnyElement {
        let s = &self.shell.settings_state;
        let state = &s.webhooks;
        let subscriptions = self.settings_value()["subscriptions"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        div().flex().flex_col().gap(px(24.))
            .child("OUTGOING WEBHOOKS")
            .child("Hypertask sends a signed request to your own URL the moment something happens in this team. No agent needed. Each endpoint has its own secret and delivery log.")
            .child(self.settings_link("How webhooks work", "https://docs.hypertask.ai/mcp/agent-webhooks/"))
            .when_some(state.secret.clone(), |d, secret| d.child(div().flex().flex_col().gap(px(8.))
                .child("NEW SIGNING SECRET")
                .child("Copy it now, it won't be shown again. Use it to verify the X-Hypertask-Signature header.")
                .child(self.code_row("webhook-secret", secret, cx))))
            .child(div().flex().flex_col().gap(px(8.)).child("ENDPOINTS")
                .when(subscriptions.is_empty() && s.pending.is_none(), |d| d.child("No endpoints yet"))
                .when(subscriptions.is_empty() && s.pending.is_some(), |d| d.child("Loading webhooks"))
                .children(subscriptions.into_iter().map(|endpoint| {
                    let id = endpoint["id"].as_str().unwrap_or("").to_owned();
                    let url = endpoint["url"].as_str().unwrap_or("").to_owned();
                    let test_id = id.clone(); let delete_id = id.clone(); let confirm_id = id.clone();
                    div().id(SharedString::from(id.clone())).py(px(12.)).border_b_1().border_color(color(self.colors.border)).flex().flex_col().gap(px(8.))
                        .child(url.clone())
                        .child(format!("{} events · Secret {}", endpoint["events"].as_array().map_or(0, Vec::len), endpoint["secretHint"].as_str().unwrap_or("")))
                        .child(div().flex().gap(px(8.))
                            .child(self.settings_action("webhook-test", "Send test").on_click(cx.listener(move |this, _, window, cx| {
                                this.settings_request("POST", format!("/webhooks/{test_id}/test"), Some(serde_json::json!({})), window, cx);
                            })).layout_key(&self.layout, format!("settings.webhook.{id}.test"), None))
                            .child(self.settings_action("webhook-delete", "Delete").on_click(cx.listener(move |this, _, _, cx| { this.shell.settings_state.webhooks.delete = Some(delete_id.clone()); cx.notify(); }))
                                .layout_key(&self.layout, format!("settings.webhook.{id}.delete"), None)))
                        .when(state.delete.as_deref() == Some(id.as_str()), |d| d.child(div().flex().flex_col().gap(px(8.)).child("Delete this endpoint and its delivery history?")
                            .child(self.settings_action("webhook-confirm-delete", "Delete").on_click(cx.listener(move |this, _, window, cx| {
                                this.settings_request("DELETE", format!("/webhooks?id={confirm_id}"), None, window, cx);
                            })).layout_key(&self.layout, "settings.webhook.confirm-delete", None))
                            .child(self.settings_action("webhook-cancel-delete", "Cancel").on_click(cx.listener(|this, _, _, cx| { this.shell.settings_state.webhooks.delete = None; cx.notify(); }))
                                .layout_key(&self.layout, "settings.webhook.cancel-delete", None))))
                        .layout_key(&self.layout, format!("settings.webhook.{id}"), Some(url))
                })))
            .child(div().flex().flex_col().gap(px(12.)).child("ADD ENDPOINT")
                .child("Endpoint URL")
                .child(div().h(px(40.)).p(px(8.)).bg(color(self.colors.comment)).child(state.url.clone()).layout_key(&self.layout, "settings.webhook.url", None))
                .child("Must be HTTPS. We POST JSON here and sign every request.")
                .child("Board scope").child(self.settings_board_picker(cx))
                .child("Events").children(EVENTS.iter().map(|&event| div().id(event).p(px(8.)).rounded(px(4.)).cursor_pointer()
                    .when(state.events.contains(event), |d| d.bg(color(self.colors.active)))
                    .child(event).on_click(cx.listener(move |this, _, _, cx| {
                        if !this.shell.settings_state.webhooks.events.remove(event) { this.shell.settings_state.webhooks.events.insert(event.into()); } cx.notify();
                    })).layout_key(&self.layout, format!("settings.webhook.event.{event}"), Some(event.into()))))
                .child(self.settings_action("webhook-add", "Add endpoint").on_click(cx.listener(|this, _, window, cx| {
                    let state = &this.shell.settings_state.webhooks;
                    let url = state.url.read(cx).content.trim();
                    if url.is_empty() || state.events.is_empty() { return; }
                    let body = serde_json::json!({"project_id": this.shell.settings_state.board, "url": url, "events": state.events});
                    this.settings_request("POST", "/webhooks".into(), Some(body), window, cx);
                })).layout_key(&self.layout, "settings.webhook.add", None)))
            .into_any_element()
    }
}
