use super::*;
use ht_core::team_settings::TeamSettings;
use serde_json::json;

pub(super) struct Billing {
    team: Option<String>,
    snapshot: Option<TeamSettings>,
    loading: bool,
    generation: u64,
    bound: bool,
    applying: bool,
    yearly: bool,
    confirm_free: bool,
    ensured: bool,
    inputs: BTreeMap<String, Entity<TextInput>>,
    revisions: BTreeMap<String, u64>,
    previous: Option<Value>,
    tests: BTreeMap<String, Value>,
    custom_gdpr: bool,
    observed: BTreeMap<String, String>,
}
fn providers() -> Vec<Value> {
    serde_json::from_str(include_str!("byok_providers.json")).unwrap()
}
impl Billing {
    pub fn new(cx: &mut Context<Desktop>) -> Self {
        let mut inputs = BTreeMap::new();
        for row in providers() {
            let source = row["byokKey"].as_str().unwrap().to_owned();
            let placeholder = row["keyPlaceholder"].as_str().unwrap().to_owned();
            inputs.insert(
                source,
                cx.new(|cx| {
                    TextInput::new(String::new(), &placeholder, color(0x8e9093), cx).secret()
                }),
            );
        }
        for (id, placeholder) in [
            ("custom", "API key"),
            ("baseUrl", "https://api.example.com/v1"),
            ("modelId", "Model id, e.g. llama-4-70b"),
        ] {
            inputs.insert(
                id.into(),
                cx.new(|cx| {
                    let input = TextInput::new(String::new(), placeholder, color(0x8e9093), cx);
                    if id == "custom" {
                        input.secret()
                    } else {
                        input
                    }
                }),
            );
        }
        let observed = inputs
            .keys()
            .map(|field| (field.clone(), String::new()))
            .collect();
        Self {
            team: None,
            snapshot: None,
            loading: false,
            generation: 0,
            bound: false,
            applying: false,
            yearly: false,
            confirm_free: false,
            ensured: false,
            inputs,
            revisions: BTreeMap::new(),
            previous: None,
            tests: BTreeMap::new(),
            custom_gdpr: false,
            observed,
        }
    }
}
impl Desktop {
    pub(in crate::ui) fn open_model_settings(
        &mut self,
        team: String,
        section: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self
            .shell
            .settings_state
            .pending
            .as_ref()
            .is_some_and(|request| request.method != "GET")
        {
            return;
        }
        if self.shell.settings_state.billing.team.as_deref() != Some(team.as_str()) {
            self.clear_billing(cx);
            self.shell.settings_state.billing.team = Some(team.clone());
        }
        if let Err(error) = self.db.kv_set("selectedSettingsTeamId", &team) {
            self.error = Some(error.to_string());
            return;
        }
        self.open_settings(window, cx);
        self.select_settings(section, window, cx);
    }
    pub(super) fn clear_billing(&mut self, cx: &mut Context<Self>) {
        let b = &mut self.shell.settings_state.billing;
        b.generation += 1;
        b.snapshot = None;
        b.team = None;
        b.loading = false;
        b.ensured = false;
        b.tests.clear();
        b.previous = None;
        b.revisions.clear();
        b.custom_gdpr = false;
        b.observed = b
            .inputs
            .keys()
            .map(|field| (field.clone(), String::new()))
            .collect();
        b.applying = true;
        for input in b.inputs.values() {
            input.update(cx, |input, cx| input.set_content(String::new(), cx));
        }
        b.applying = false;
    }
    pub(super) fn load_billing(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.shell.settings_state.billing.bound {
            self.shell.settings_state.billing.bound = true;
            for (field, input) in self.shell.settings_state.billing.inputs.clone() {
                cx.observe_in(&input, window, move |this, input, window, cx| {
                    let content = input.read(cx).content.to_string();
                    if this.shell.settings_state.billing.observed.get(&field) == Some(&content) {
                        return;
                    }
                    this.shell
                        .settings_state
                        .billing
                        .observed
                        .insert(field.clone(), content);
                    if !this.shell.settings_state.billing.applying {
                        this.schedule_billing_key(
                            if matches!(field.as_str(), "baseUrl" | "modelId") {
                                "custom"
                            } else {
                                &field
                            },
                            window,
                            cx,
                        );
                    }
                })
                .detach();
            }
        }
        if self.shell.settings_state.billing.team.is_none() {
            self.shell.settings_state.billing.team = self
                .db
                .kv_get("selectedSettingsTeamId")
                .ok()
                .flatten()
                .or_else(|| {
                    self.db
                        .board_project(self.options.board)
                        .ok()
                        .and_then(|project| project["teamId"].as_str().map(str::to_owned))
                });
        }
        if self.options.offline {
            return;
        }
        let team = self.shell.settings_state.billing.team.clone();
        let current = self
            .db
            .board_project(self.options.board)
            .ok()
            .and_then(|project| project["teamId"].as_str().map(str::to_owned));
        let previous = self
            .db
            .kv_get("last_board")
            .ok()
            .flatten()
            .and_then(|id| id.parse::<i64>().ok())
            .and_then(|id| self.db.board_project(id).ok())
            .and_then(|project| project["teamId"].as_str().map(str::to_owned));
        let Some(token) = self.cli_token() else {
            return;
        };
        let b = &mut self.shell.settings_state.billing;
        b.generation += 1;
        let generation = b.generation;
        b.loading = true;
        let keys = self.shell.settings_state.section == "apiKeys";
        let expected = token.clone();
        let (send, receive) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let result = ht_core::Http::new(token)
                .and_then(|mut http| {
                    ht_core::team_settings::load_for_scope(
                        &mut http,
                        team.as_deref(),
                        current.as_deref(),
                        previous.as_deref(),
                        keys,
                    )
                })
                .map_err(|error| error.to_string());
            let _ = send.send(result);
        });
        cx.spawn_in(window, async move |view, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(50))
                    .await;
                match receive.try_recv() {
                    Ok(result) => {
                        let _ = view.update_in(cx, |this, window, cx| {
                            if this.shell.settings_state.billing.generation != generation
                                || this.cli_token().as_deref() != Some(expected.as_str())
                            {
                                return;
                            }
                            this.apply_billing(result, window, cx);
                        });
                        break;
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => (),
                    _ => break,
                }
            }
        })
        .detach();
        cx.notify();
    }
    fn apply_billing(
        &mut self,
        result: Result<TeamSettings, String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.shell.settings_state.billing.loading = false;
        match result {
            Err(error) => {
                self.shell.settings_state.billing.snapshot = None;
                self.shell.settings_state.error = Some(error);
            }
            Ok(snapshot) => {
                if snapshot.user["id"].as_i64() != self.db.current_user_id() {
                    return;
                }
                let team = snapshot.team["id"].as_str().map(str::to_owned);
                if let Some(team) = &team {
                    let _ = self.db.kv_set("selectedSettingsTeamId", team);
                }
                let b = &mut self.shell.settings_state.billing;
                b.team = team;
                for input in b.inputs.values() {
                    input.update(cx, |input, cx| {
                        input.set_read_only(!snapshot.manage_keys(), cx)
                    });
                }
                if b.snapshot.as_ref().is_none_or(|previous| {
                    previous.billing_interval() != snapshot.billing_interval()
                }) {
                    b.yearly = snapshot.yearly();
                }
                b.applying = true;
                for provider in providers().into_iter().chain(std::iter::once(
                    json!({"byokKey":"custom","keyPlaceholder":"API key"}),
                )) {
                    let source = provider["byokKey"].as_str().unwrap();
                    let has_secret = snapshot.keys["keys"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .any(|row| row["provider"] == source && row["hasSecret"] == true);
                    let placeholder = if has_secret {
                        "Replace key…"
                    } else {
                        provider["keyPlaceholder"].as_str().unwrap_or("API key")
                    };
                    b.inputs[source]
                        .update(cx, |input, cx| input.configure(placeholder, false, cx));
                }
                let custom = snapshot.keys["keys"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .find(|row| row["provider"] == "custom");
                for key in ["baseUrl", "modelId"] {
                    if b.inputs[key].read(cx).is_empty() {
                        let value = custom
                            .and_then(|row| row[key].as_str())
                            .unwrap_or("")
                            .to_owned();
                        b.observed.insert(key.into(), value.clone());
                        b.inputs[key].update(cx, |input, cx| input.set_content(value, cx));
                    }
                }
                if b.snapshot.is_none() || !b.revisions.contains_key("custom") {
                    b.custom_gdpr = custom.is_some_and(|row| row["gdprCompliant"] == true);
                }
                b.applying = false;
                let ensure = self.shell.settings_state.section == "plans"
                    && snapshot.owner()
                    && snapshot.team["stripe_customer_id"].is_null()
                    && !b.ensured;
                let team = snapshot.team["id"].clone();
                b.snapshot = Some(snapshot);
                if ensure {
                    self.shell.settings_state.billing.ensured = true;
                    self.billing_write(
                        "/api/stripe/ensure-customer",
                        json!({"teamId":team}),
                        window,
                        cx,
                    );
                }
            }
        }
        cx.notify();
    }
    fn schedule_billing_key(&mut self, source: &str, window: &mut Window, cx: &mut Context<Self>) {
        if !self.shell.settings
            || self.shell.settings_state.section != "apiKeys"
            || !self
                .shell
                .settings_state
                .billing
                .snapshot
                .as_ref()
                .is_some_and(TeamSettings::manage_keys)
        {
            return;
        }
        let b = &mut self.shell.settings_state.billing;
        let revision = b.revisions.entry(source.into()).or_default();
        *revision += 1;
        let revision = *revision;
        let source = source.to_owned();
        let team = b.team.clone();
        b.tests.remove(&source);
        cx.spawn_in(window, async move |view, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(600))
                .await;
            loop {
                let waiting = view
                    .update_in(cx, |this, window, cx| {
                        let b = &this.shell.settings_state.billing;
                        if !this.shell.settings
                            || this.shell.settings_state.section != "apiKeys"
                            || b.team != team
                            || b.revisions.get(&source) != Some(&revision)
                        {
                            return false;
                        }
                        if this.shell.settings_state.pending.is_some() {
                            return true;
                        }
                        if let Some(body) = this.billing_key_body(&source, false, cx) {
                            this.billing_write("/api/teams/byokKeys", body, window, cx);
                        }
                        false
                    })
                    .unwrap_or(false);
                if !waiting {
                    break;
                }
                cx.background_executor()
                    .timer(Duration::from_millis(100))
                    .await;
            }
        })
        .detach();
    }
    fn billing_key_body(&self, source: &str, test: bool, cx: &App) -> Option<Value> {
        let b = &self.shell.settings_state.billing;
        let snapshot = b
            .snapshot
            .as_ref()
            .filter(|snapshot| snapshot.manage_keys())?;
        let key = b.inputs[source].read(cx).content.trim();
        let saved = snapshot.keys["keys"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|row| row["provider"] == source);
        let mut body = json!({"teamId":b.team,"provider":source});
        if !key.is_empty() {
            body["apiKey"] = json!(key);
        }
        if source == "custom" {
            let base = b.inputs["baseUrl"].read(cx).content.trim();
            let model = b.inputs["modelId"].read(cx).content.trim();
            if base.is_empty()
                || model.is_empty()
                || (key.is_empty() && !saved.is_some_and(|row| row["hasSecret"] == true))
                || (test && snapshot.keys["gdprSafeMode"] == true && !b.custom_gdpr)
            {
                return None;
            }
            body["baseUrl"] = json!(base);
            body["modelId"] = json!(model);
            body["gdprCompliant"] = json!(b.custom_gdpr);
        } else if key.is_empty() && (!test || !saved.is_some_and(|row| row["hasSecret"] == true)) {
            return None;
        }
        Some(body)
    }
    fn billing_write(
        &mut self,
        path: &str,
        body: Value,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.shell.settings_state.pending.is_some() {
            return;
        }
        let b = &mut self.shell.settings_state.billing;
        let Some(snapshot) = b.snapshot.as_mut() else {
            return;
        };
        if path.starts_with("/api/stripe/") && !snapshot.owner()
            || !path.starts_with("/api/stripe/") && !snapshot.manage_keys()
        {
            return;
        }
        b.previous = Some(snapshot.keys.clone());
        if path == "/api/teams/byokKeys" {
            for row in snapshot.keys["keys"]
                .as_array_mut()
                .into_iter()
                .flatten()
                .filter(|row| row["provider"] == body["provider"])
            {
                if body["enabled"].is_boolean() {
                    row["enabled"] = body["enabled"].clone();
                }
                if body["clearSecret"] == true {
                    row["hasSecret"] = json!(false);
                    row["maskedKey"] = Value::Null;
                    row["enabled"] = json!(false);
                }
            }
        } else if path == "/api/settings/byok-test" {
            b.tests.insert(
                body["provider"].as_str().unwrap_or("").into(),
                json!({"result":"testing"}),
            );
        }
        self.settings_request("POST", path.into(), Some(body), window, cx);
        if self.shell.settings_state.pending.is_none() {
            self.rollback_billing();
        }
    }
    fn rollback_billing(&mut self) {
        let b = &mut self.shell.settings_state.billing;
        if let (Some(snapshot), Some(previous)) = (&mut b.snapshot, b.previous.take()) {
            snapshot.keys = previous;
        }
    }
    pub(super) fn billing_result(
        &mut self,
        request: &Request,
        result: Result<Value, String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let provider = request
            .body
            .as_ref()
            .and_then(|body| body["provider"].as_str())
            .unwrap_or("")
            .to_owned();
        match result {
            Err(error) => {
                let status = error
                    .strip_prefix("HTTP ")
                    .and_then(|error| error.split(':').next())
                    .and_then(|status| status.parse::<u16>().ok());
                let message = error
                    .split_once(": ")
                    .filter(|_| status.is_some())
                    .map(|(_, message)| message.to_owned())
                    .unwrap_or_else(|| error.clone());
                self.rollback_billing();
                if request.path == "/api/settings/byok-test" {
                    self.shell.settings_state.billing.tests.insert(
                        provider,
                        json!({"result":"error","status":status,"elapsedMs":0,"model":""}),
                    );
                }
                self.shell.settings_state.error = Some(match request.path.as_str() {
                    "/api/stripe/session/checkout" => "Could not open checkout".into(),
                    "/api/stripe/cancelSubscription" => "Could not switch to Free Plan".into(),
                    _ => message,
                });
            }
            Ok(value) if request.path == "/api/settings/byok-test" => {
                self.shell.settings_state.billing.previous = None;
                self.shell
                    .settings_state
                    .billing
                    .tests
                    .insert(provider, value);
            }
            Ok(value) if request.path == "/api/stripe/session/checkout" => {
                self.shell.settings_state.billing.previous = None;
                if let Some(url) = value["url"]
                    .as_str()
                    .filter(|url| url.starts_with("https://") && !url.chars().any(char::is_control))
                {
                    cx.open_url(url);
                } else {
                    self.shell.settings_state.error = Some("Could not open checkout".into());
                }
            }
            Ok(_) => {
                let b = &mut self.shell.settings_state.billing;
                b.previous = None;
                if let Some(body) = request
                    .body
                    .as_ref()
                    .filter(|_| request.path == "/api/teams/byokKeys")
                {
                    if body["clearSecret"] == true {
                        b.tests.remove(&provider);
                    }
                    b.applying = true;
                    let sent = body["apiKey"].as_str();
                    if b.inputs.contains_key(&provider)
                        && (body["clearSecret"] == true
                            || sent.is_some_and(|sent| {
                                b.inputs[&provider].read(cx).content.trim() == sent
                            }))
                    {
                        b.observed.insert(provider.clone(), String::new());
                        b.inputs[&provider]
                            .update(cx, |input, cx| input.set_content(String::new(), cx));
                    }
                    if provider == "custom" && body["clearSecret"] == true {
                        for key in ["baseUrl", "modelId"] {
                            b.observed.insert(key.into(), String::new());
                            b.inputs[key]
                                .update(cx, |input, cx| input.set_content(String::new(), cx));
                        }
                        b.custom_gdpr = false;
                    }
                    b.applying = false;
                }
                if request.path == "/api/stripe/cancelSubscription" {
                    b.confirm_free = false;
                    self.shell.settings_state.message =
                        Some("Successfully switched to Free Plan".into());
                }
                self.load_billing(window, cx);
                if let Some(sidebar) = &self.ai_sidebar {
                    sidebar.update(cx, |sidebar, cx| sidebar.refresh_model_settings(cx));
                }
            }
        }
        cx.notify();
    }
    pub(super) fn billing_content(&self, cx: &mut Context<Self>) -> AnyElement {
        let b = &self.shell.settings_state.billing;
        let mut content = div().flex().flex_col().gap(px(12.));
        if b.loading {
            return content
                .child(if self.shell.settings_state.section == "apiKeys" {
                    "Loading API keys"
                } else {
                    "Loading"
                })
                .into_any_element();
        }
        if b.team.is_none() {
            return content
                .child("Open a team board before managing API keys.")
                .into_any_element();
        }
        let Some(snapshot) = &b.snapshot else {
            return content.into_any_element();
        };
        content = content.child(snapshot.team["title"].as_str().unwrap_or("").to_owned());
        if self.shell.settings_state.section == "plans" {
            return self.plan_controls(content, snapshot, cx);
        }
        if snapshot.plan() == "Free" {
            return content.into_any_element();
        }
        content = content.child("Bring your own key").child("Add a provider key and requests to that provider’s models run on your key instead of Hypertask’s.").child("A key covers all models of that provider, everywhere in Hypertask. No key means models run on Hypertask’s pooled allowance. Toggle a key off to fall back without deleting it.");
        if !snapshot.manage_keys() {
            content = content.child("Only the team owner can change this.");
        }
        let mut rows = providers();
        rows.push(json!({"byokKey":"custom","label":"Custom endpoint"}));
        for (index, provider) in rows
            .into_iter()
            .enumerate()
            .filter(|(_, row)| snapshot.keys["gdprSafeMode"] != true || row["chinaHosted"] != true)
        {
            let source = provider["byokKey"].as_str().unwrap().to_owned();
            let saved = snapshot.keys["keys"]
                .as_array()
                .into_iter()
                .flatten()
                .find(|row| row["provider"] == source)
                .cloned()
                .unwrap_or(json!({}));
            let busy = self.shell.settings_state.pending.is_some();
            let can_manage = snapshot.manage_keys() && !busy;
            let can_test = can_manage
                && b.tests
                    .get(&source)
                    .is_none_or(|test| test["result"] != "testing")
                && self.billing_key_body(&source, true, cx).is_some();
            if index == 0 {
                content = content.child("Gateway");
            } else if index == 1 {
                content = content.child("Providers");
            } else if source == "custom" {
                content = content.child("Advanced");
            }
            let mut row = div().flex().flex_col().gap(px(8.)).py(px(12.)).border_b_1().border_color(color(self.colors.border))
                .child(div().flex().gap(px(12.)).child(provider["label"].as_str().unwrap_or("").to_owned()).when(provider["chinaHosted"] == true, |d|d.child("China-hosted")).when_some(provider["badge"].as_str().map(str::to_owned), |d,badge|d.child(badge)).child(saved["maskedKey"].as_str().unwrap_or("").to_owned())
                    .when(saved["hasSecret"] == true, |d| d.child(div().id(("byok-remove", index)).when(!can_manage,|d|d.opacity(0.45)).child("Remove").when(can_manage, |d|d.cursor_pointer().on_click(cx.listener({ let source = source.clone(); move |this, _, window, cx| {
                        *this.shell.settings_state.billing.revisions.entry(source.clone()).or_default() += 1;
                        this.billing_write("/api/teams/byokKeys", json!({"teamId":this.shell.settings_state.billing.team,"provider":source,"clearSecret":true}), window, cx);
                    }}))))));
            if source != "custom" {
                row = row.child(div().id(("byok-enabled", index)).when(!can_manage,|d|d.opacity(0.45)).child(format!("Use {} API key    {}", provider["label"].as_str().unwrap_or(""), if saved["enabled"] == true { "✓" } else { "" })).when(can_manage, |d|d.cursor_pointer().on_click(cx.listener({ let source=source.clone(); let enabled=saved["enabled"]!=true; move |this,_,window,cx| this.billing_write("/api/teams/byokKeys",json!({"teamId":this.shell.settings_state.billing.team,"provider":source,"enabled":enabled}),window,cx) }))));
                let models = if source == "gateway" {
                    "Entire AI catalog".into()
                } else if source == "openrouter" {
                    "Legacy model routes".into()
                } else {
                    let mut seen = std::collections::HashSet::new();
                    ht_core::ai_models::catalog()
                        .iter()
                        .filter(|model| {
                            model["definition"]["provider"] == provider["key"]
                                && seen.insert(model["family"].as_str().unwrap_or(""))
                        })
                        .filter_map(|model| model["definition"]["label"].as_str())
                        .collect::<Vec<_>>()
                        .join(" · ")
                };
                row = row.child(models);
                if let Some(destination) = provider["requestDestination"].as_str() {
                    row = row.child(format!(
                        "With your key, requests go directly to {destination}."
                    ));
                }
                if let Some(note) = provider["gdprNote"].as_str() {
                    row = row.child(note.to_owned());
                }
                if let Some(url) = provider["keyUrl"].as_str() {
                    let url = url.to_owned();
                    row = row.child(
                        div()
                            .id(("byok-link", index))
                            .cursor_pointer()
                            .child(format!(
                                "Get a key at {}",
                                provider["keyUrlLabel"].as_str().unwrap_or("")
                            ))
                            .on_click(move |_, _, cx| cx.open_url(&url)),
                    );
                }
            } else {
                row = row.child("Any OpenAI-compatible API, including Cloudflare AI Gateway, Groq, Together, or a self-hosted endpoint. Its model appears in the picker under Custom.")
                    .child(div().id("custom-gdpr").cursor_pointer().child(format!("{} I confirm this endpoint operates under an EU/US data agreement. Required while GDPR safe mode is on.",if b.custom_gdpr{"✓"}else{""})).on_click(cx.listener(|this,_,window,cx| { if this.shell.settings_state.billing.snapshot.as_ref().is_some_and(TeamSettings::manage_keys) { this.shell.settings_state.billing.custom_gdpr=!this.shell.settings_state.billing.custom_gdpr; this.schedule_billing_key("custom",window,cx); cx.notify(); } })))
                    .child(b.inputs["baseUrl"].clone()).child(b.inputs["modelId"].clone());
            }
            row = row
                .child(
                    div()
                        .when(!snapshot.manage_keys(), |d| d.opacity(0.45))
                        .child(b.inputs[&source].clone()),
                )
                .child(
                    div()
                        .id(("byok-test", index))
                        .when(!can_test, |d| d.opacity(0.45))
                        .child(if source == "custom" {
                            "Test endpoint"
                        } else {
                            "Test key"
                        })
                        .when(can_test, |d| {
                            d.cursor_pointer().on_click(cx.listener({
                                let source = source.clone();
                                move |this, _, window, cx| {
                                    if let Some(body) = this.billing_key_body(&source, true, cx) {
                                        this.billing_write(
                                            "/api/settings/byok-test",
                                            body,
                                            window,
                                            cx,
                                        );
                                    }
                                }
                            }))
                        }),
                )
                .child(test_copy(b.tests.get(&source)));
            content = content.child(row.layout_key(
                &self.layout,
                format!("settings.byok.{source}"),
                None,
            ));
        }
        content.into_any_element()
    }
    fn plan_controls(
        &self,
        mut content: gpui::Div,
        snapshot: &TeamSettings,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let b = &self.shell.settings_state.billing;
        let plan = snapshot.plan();
        let busy = self.shell.settings_state.pending.is_some();
        content = content.child(
            div()
                .flex()
                .gap(px(12.))
                .child("Billing interval")
                .children(
                    [("Monthly", false), ("Annually", true)]
                        .into_iter()
                        .enumerate()
                        .map(|(i, (label, yearly))| {
                            div()
                                .id(("billing-interval", i))
                                .cursor_pointer()
                                .when(b.yearly == yearly, |d| d.bg(color(self.colors.active)))
                                .child(label)
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.shell.settings_state.billing.yearly = yearly;
                                    cx.notify();
                                }))
                        }),
                ),
        );
        for (index, (id, name, monthly, annual)) in [
            ("Free", "Free Trial", "$0", "$0"),
            ("BYOK", "BYOK", "$10", "$96"),
            ("Pro", "Pro", "$20", "$192"),
        ]
        .into_iter()
        .enumerate()
        {
            let current = id == plan;
            let exact = current && (id == "Free" || snapshot.billing_interval() == Some(b.yearly));
            let rank = |id: &str| match id {
                "Free" => 0,
                "BYOK" => 1,
                "AI" => 2,
                _ => 3,
            };
            let label = if !snapshot.owner() {
                "Owner only"
            } else if current {
                "Switch"
            } else if rank(id) > rank(&plan) {
                "Upgrade"
            } else {
                "Downgrade"
            };
            content = content.child(
                div()
                    .flex()
                    .gap(px(12.))
                    .child(name)
                    .child(if b.yearly { annual } else { monthly })
                    .child(if id == "Free" {
                        if snapshot.flags.is_null() {
                            "Checking plan access…"
                        } else if snapshot.flags["htpr-7551-start-plan-checkout"] == true {
                            "Free plan"
                        } else {
                            "/ 14 days"
                        }
                    } else if b.yearly {
                        "per user / year"
                    } else {
                        "per user / month"
                    })
                    .when(current, |d| d.child("Current"))
                    .when(exact && id == "BYOK", |d| {
                        d.child(
                            div()
                                .id("plan-api-keys")
                                .cursor_pointer()
                                .child("API keys")
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.select_settings("apiKeys", window, cx)
                                })),
                        )
                    })
                    .when(!exact, |d| {
                        d.child(div().id(("billing-plan", index)).child(label).when(
                            snapshot.owner() && !busy,
                            |d| {
                                d.cursor_pointer().on_click(cx.listener(
                                    move |this, _, window, cx| {
                                        if id == "Free" {
                                            this.shell.settings_state.billing.confirm_free = true;
                                            cx.notify();
                                            return;
                                        }
                                        let Some(snapshot) =
                                            &this.shell.settings_state.billing.snapshot
                                        else {
                                            return;
                                        };
                                        match snapshot
                                            .checkout(id, this.shell.settings_state.billing.yearly)
                                        {
                                            Ok(body) => this.billing_write(
                                                "/api/stripe/session/checkout",
                                                body,
                                                window,
                                                cx,
                                            ),
                                            Err(error) => {
                                                this.shell.settings_state.error =
                                                    Some(error.to_string());
                                                cx.notify();
                                            }
                                        }
                                    },
                                ))
                            },
                        ))
                    }),
            );
        }
        if b.confirm_free {
            content=content.child(div().child("Are you sure you want to downgrade?").child("This will cancel your current plan immediately and switch this team to the Free Plan.")
            .child(div().id("billing-keep").cursor_pointer().child("Keep plan").on_click(cx.listener(|this,_,_,cx|{this.shell.settings_state.billing.confirm_free=false;cx.notify();})))
            .child(div().id("billing-confirm").cursor_pointer().child("Confirm").on_click(cx.listener(|this,_,window,cx|{if let Some(customer)=this.shell.settings_state.billing.snapshot.as_ref().and_then(|snapshot|snapshot.team["stripe_customer_id"].as_str()).map(str::to_owned){this.billing_write("/api/stripe/cancelSubscription",json!({"stripe_customer_id":customer}),window,cx);}}))));
        }
        content.into_any_element()
    }
}
fn test_copy(value: Option<&Value>) -> String {
    let Some(value) = value else {
        return "not tested".into();
    };
    match value["result"].as_str() {
        Some("testing") => "testing…".into(),
        Some("works") => format!(
            "✓ works · {} replied in {:.1}s",
            value["model"].as_str().unwrap_or(""),
            value["elapsedMs"].as_f64().unwrap_or(0.) / 1000.
        ),
        Some("invalid") => format!(
            "✗ invalid key ({})",
            value["status"].as_i64().unwrap_or(401)
        ),
        Some("no_credit") => "✗ no credit".into(),
        _ => format!(
            "✗ test failed{}",
            value["status"]
                .as_i64()
                .map(|status| format!(" ({status})"))
                .unwrap_or_default()
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::task_fields::tests::desktop;
    struct Web {
        status: u16,
        path: String,
        body: Value,
        reply: Value,
    }
    impl ht_core::Transport for Web {
        fn get(
            &mut self,
            path: &str,
        ) -> Result<ht_core::http::Response, ht_core::http::TransportError> {
            assert_eq!(path, "/user/context");
            Ok(ht_core::http::Response {
                status: 200,
                body: json!({"success":true,"user":{"id":42}}),
            })
        }
        fn post(
            &mut self,
            path: &str,
            body: &Value,
            _: &str,
        ) -> Result<ht_core::http::Response, ht_core::http::TransportError> {
            assert_eq!(path, self.path);
            assert_eq!(body, &self.body);
            Ok(ht_core::http::Response {
                status: self.status,
                body: if self.status == 200 {
                    self.reply.clone()
                } else {
                    json!({"message":"Team access denied"})
                },
            })
        }
    }
    fn snapshot() -> TeamSettings {
        TeamSettings {
            user: json!({"id":42,"accountId":"account"}),
            team: json!({"id":"target-team","title":"Selected team","stripe_customer_id":"cus_1","googleAccountId":"account","totalSeats":2,"subscriptionPlan":[{"subscriptionStatus":"active","priceId":"price_1QjJeDIhmcH60VcciHzZ3mTJ"}]}),
            members: json!({"owner":{"id":42},"members":[]}),
            keys: json!({"keys":[{"provider":"openai","enabled":true,"hasSecret":true,"maskedKey":"sk…1234"},{"provider":"custom","enabled":true,"hasSecret":true,"baseUrl":"https://example.test/v1","modelId":"model","gdprCompliant":true}],"gdprSafeMode":true}),
            providers: json!({"providers":[]}),
            flags: json!({}),
        }
    }
    fn prepare(this: &mut Desktop, window: &mut Window, cx: &mut Context<Desktop>) {
        this.shell.settings = true;
        this.shell.settings_state.section = "apiKeys";
        this.shell.settings_state.billing.team = Some("target-team".into());
        this.apply_billing(Ok(snapshot()), window, cx);
    }
    fn settle(
        this: &mut Desktop,
        status: u16,
        path: &str,
        body: Value,
        reply: Value,
        window: &mut Window,
        cx: &mut Context<Desktop>,
    ) {
        this.db
            .drain_once(
                &mut Web {
                    status,
                    path: path.into(),
                    body,
                    reply,
                },
                chrono::Utc::now().timestamp() + 1,
            )
            .unwrap();
        assert!(!this.poll_settings_write(window, cx));
    }
    #[gpui::test]
    fn model_settings_handoff_targets_the_picker_team_not_the_current_board_or_browser(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(crate::input::init);
        let (view, cx) = cx.add_window_view(|window, cx| desktop(window, cx));
        view.update_in(cx, |this, window, cx| {
            this.open_model_settings("target-team".into(), "plans", window, cx);
            assert!(this.shell.settings);
            assert_eq!(this.shell.settings_state.section, "plans");
            assert_eq!(
                this.shell.settings_state.billing.team.as_deref(),
                Some("target-team")
            );
            assert_eq!(
                this.db.kv_get("selectedSettingsTeamId").unwrap().as_deref(),
                Some("target-team")
            );
            this.apply_billing(Ok(snapshot()), window, cx);
            assert!(this.db.mutations().unwrap().is_empty());
            this.open_model_settings("target-team".into(), "apiKeys", window, cx);
            assert_eq!(this.shell.settings_state.section, "apiKeys");
            assert_eq!(this.options.board, 15);
        });
    }
    #[gpui::test]
    fn native_key_enable_remove_custom_and_test_handlers_queue_exact_payloads_and_rollback_denials(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(crate::input::init);
        let (view, cx) = cx.add_window_view(|window, cx| desktop(window, cx));
        for status in [401, 403, 200] {
            for body in [
                json!({"teamId":"target-team","provider":"openai","enabled":false}),
                json!({"teamId":"target-team","provider":"openai","clearSecret":true}),
                json!({"teamId":"target-team","provider":"custom","baseUrl":"https://example.test/v1","modelId":"model","gdprCompliant":true,"apiKey":"fixture-secret"}),
            ] {
                view.update_in(cx, |this, window, cx| {
                    prepare(this, window, cx);
                    let before = this
                        .shell
                        .settings_state
                        .billing
                        .snapshot
                        .as_ref()
                        .unwrap()
                        .keys
                        .clone();
                    this.shell.settings_state.billing.inputs["custom"].update(cx, |input, cx| {
                        input.set_content("fixture-secret".into(), cx)
                    });
                    if body["provider"] == "custom" {
                        assert_eq!(
                            this.billing_key_body("custom", false, cx),
                            Some(body.clone())
                        );
                    }
                    this.billing_write("/api/teams/byokKeys", body.clone(), window, cx);
                    assert!(this.shell.settings_state.pending.is_some());
                    if body["enabled"] == false {
                        assert_eq!(
                            this.shell
                                .settings_state
                                .billing
                                .snapshot
                                .as_ref()
                                .unwrap()
                                .keys["keys"][0]["enabled"],
                            false
                        );
                    }
                    if body["clearSecret"] == true {
                        assert_eq!(
                            this.shell
                                .settings_state
                                .billing
                                .snapshot
                                .as_ref()
                                .unwrap()
                                .keys["keys"][0]["hasSecret"],
                            false
                        );
                    }
                    settle(
                        this,
                        status,
                        "/api/teams/byokKeys",
                        body.clone(),
                        json!({"ok":true}),
                        window,
                        cx,
                    );
                    if status != 200 {
                        assert_eq!(
                            this.shell.settings_state.error.as_deref(),
                            Some("Team access denied")
                        );
                        assert_eq!(
                            this.shell
                                .settings_state
                                .billing
                                .snapshot
                                .as_ref()
                                .unwrap()
                                .keys,
                            before
                        );
                        assert_eq!(
                            this.shell.settings_state.billing.inputs["custom"]
                                .read(cx)
                                .content
                                .as_ref(),
                            "fixture-secret"
                        );
                    } else if body["provider"] == "custom" {
                        assert!(
                            this.shell.settings_state.billing.inputs["custom"]
                                .read(cx)
                                .is_empty()
                        );
                    }
                });
            }
        }
        for status in [401, 403, 200] {
            view.update_in(cx, |this, window, cx| {
                prepare(this, window, cx);
                let body = json!({"teamId":"target-team","provider":"openai"});
                this.billing_write("/api/settings/byok-test", body.clone(), window, cx);
                assert_eq!(
                    test_copy(this.shell.settings_state.billing.tests.get("openai")),
                    "testing…"
                );
                settle(
                    this,
                    status,
                    "/api/settings/byok-test",
                    body,
                    json!({"result":"works","status":200,"elapsedMs":200,"model":"test-model"}),
                    window,
                    cx,
                );
                let text = test_copy(this.shell.settings_state.billing.tests.get("openai"));
                assert_eq!(text.starts_with("✓ works"), status == 200);
                if status != 200 {
                    assert!(text.starts_with("✗ test failed"));
                }
            });
        }
    }
    #[gpui::test]
    fn readonly_key_controls_cannot_enqueue_and_checkout_refusals_keep_confirmation(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(crate::input::init);
        let (view, cx) = cx.add_window_view(|window, cx| desktop(window, cx));
        view.update_in(cx, |this, window, cx| {
            prepare(this, window, cx);
            this.shell
                .settings_state
                .billing
                .snapshot
                .as_mut()
                .unwrap()
                .user = json!({"id":99,"accountId":"other"});
            this.billing_write(
                "/api/teams/byokKeys",
                json!({"teamId":"target-team","provider":"openai","enabled":false}),
                window,
                cx,
            );
            assert!(this.db.mutations().unwrap().is_empty());
        });
        for status in [401, 403, 200] {
            view.update_in(cx, |this, window, cx| {
                prepare(this, window, cx);
                this.shell.settings_state.section = "plans";
                this.shell.settings_state.billing.confirm_free = true;
                let body = json!({"stripe_customer_id":"cus_1"});
                this.billing_write("/api/stripe/cancelSubscription", body.clone(), window, cx);
                settle(
                    this,
                    status,
                    "/api/stripe/cancelSubscription",
                    body,
                    json!({"message":"Success"}),
                    window,
                    cx,
                );
                assert_eq!(
                    this.shell.settings_state.billing.confirm_free,
                    status != 200
                );
                if status != 200 {
                    assert_eq!(
                        this.shell.settings_state.error.as_deref(),
                        Some("Could not switch to Free Plan")
                    );
                }
            });
        }
    }
    #[gpui::test]
    fn key_autosave_observes_real_edits_debounces_latest_value_and_keeps_refused_secret(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(crate::input::init);
        let (view, cx) = cx.add_window_view(|window, cx| desktop(window, cx));
        for status in [401, 403, 200] {
            view.update_in(cx, |this, window, cx| {
                prepare(this, window, cx);
                this.load_billing(window, cx);
                this.shell
                    .settings_state
                    .billing
                    .observed
                    .insert("openai".into(), String::new());
                this.shell.settings_state.billing.inputs["openai"]
                    .update(cx, |input, cx| input.set_content(String::new(), cx));
            });
            cx.run_until_parked();
            view.read_with(cx, |this, _| {
                assert!(
                    !this
                        .db
                        .mutations()
                        .unwrap()
                        .iter()
                        .any(|row| row.status != "failed")
                )
            });
            view.update_in(cx, |this, _, cx| {
                this.shell.settings_state.billing.inputs["openai"].update(cx, |input, cx| {
                    input.set_content("first-fixture".into(), cx)
                })
            });
            cx.run_until_parked();
            cx.executor().advance_clock(Duration::from_millis(500));
            cx.run_until_parked();
            view.read_with(cx, |this, _| {
                assert!(
                    !this
                        .db
                        .mutations()
                        .unwrap()
                        .iter()
                        .any(|row| row.status != "failed")
                )
            });
            view.update_in(cx, |this, _, cx| {
                this.shell.settings_state.billing.inputs["openai"].update(cx, |input, cx| {
                    input.set_content("latest-fixture".into(), cx)
                })
            });
            cx.run_until_parked();
            cx.executor().advance_clock(Duration::from_millis(599));
            cx.run_until_parked();
            view.read_with(cx, |this, _| {
                assert!(
                    !this
                        .db
                        .mutations()
                        .unwrap()
                        .iter()
                        .any(|row| row.status != "failed")
                )
            });
            cx.executor().advance_clock(Duration::from_millis(1));
            cx.run_until_parked();
            view.update_in(cx, |this, window, cx| {
                let row = this
                    .db
                    .mutations()
                    .unwrap()
                    .into_iter()
                    .find(|row| row.status != "failed")
                    .unwrap();
                let body =
                    json!({"teamId":"target-team","provider":"openai","apiKey":"latest-fixture"});
                assert_eq!(row.payload_json["body"], body);
                settle(
                    this,
                    status,
                    "/api/teams/byokKeys",
                    body,
                    json!({"ok":true}),
                    window,
                    cx,
                );
                assert_eq!(
                    this.shell.settings_state.billing.inputs["openai"]
                        .read(cx)
                        .content
                        .as_ref(),
                    if status == 200 { "" } else { "latest-fixture" }
                );
            });
            cx.run_until_parked();
        }
    }
}
