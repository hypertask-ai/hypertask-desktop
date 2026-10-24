use super::*;
use ht_core::{Http, Transport};
use serde_json::Value;
use std::{collections::BTreeMap, time::Instant};

pub(super) struct Skills {
    token: Option<String>,
    project: Option<i64>,
    rows: Vec<Value>,
    pending: Option<Receiver<Result<Vec<Value>, String>>>,
    fetched: Option<Instant>,
    choice: usize,
    query: String,
    dismissed: Option<String>,
}
impl Skills {
    pub fn new(token: Option<String>) -> Self {
        Self {
            token,
            project: None,
            rows: vec![],
            pending: None,
            fetched: None,
            choice: 0,
            query: String::new(),
            dismissed: None,
        }
    }
}
fn skills_from_response(value: &Value) -> Vec<Value> {
    let mut by_slug: BTreeMap<String, Value> = BTreeMap::new();
    for skill in value["skills"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|skill| skill["enabled"] == true)
    {
        let Some(slug) = skill["slug"].as_str().filter(|slug| !slug.is_empty()) else {
            continue;
        };
        if !by_slug.contains_key(slug) || !skill["projectId"].is_null() {
            by_slug.insert(slug.into(), skill.clone());
        }
    }
    let mut rows: Vec<_> = by_slug.into_values().collect();
    rows.sort_by_key(|skill| skill["name"].as_str().unwrap_or("").to_lowercase());
    rows
}
impl AiSidebar {
    fn poll_skills(&mut self, cx: &mut Context<Self>) {
        let project = self
            .scope
            .as_ref()
            .and_then(|scope| scope.context.project_id);
        if self.skills.project != project {
            self.skills.project = project;
            self.skills.rows.clear();
            self.skills.pending = None;
            self.skills.fetched = None;
            self.skills.dismissed = None;
        }
        let token = self.input.read(cx).slash_token();
        let query = token
            .as_ref()
            .map_or(String::new(), |(_, query)| query.to_lowercase());
        if self.skills.query != query {
            self.skills.choice = 0;
            self.skills.query = query;
            cx.notify();
        }
        if token.is_none() {
            self.skills.dismissed = None;
            return;
        }
        if let Some(result) = self
            .skills
            .pending
            .as_ref()
            .and_then(|pending| pending.try_recv().ok())
        {
            self.skills.pending = None;
            self.skills.fetched = Some(Instant::now());
            if let Ok(rows) = result {
                self.skills.rows = rows;
            }
            cx.notify();
        }
        if self.skills.pending.is_none()
            && self
                .skills
                .fetched
                .is_none_or(|at| at.elapsed() >= Duration::from_secs(60))
        {
            let Some(token) = self.skills.token.clone() else {
                return;
            };
            let (send, receive) = mpsc::channel();
            self.skills.pending = Some(receive);
            thread::spawn(move || {
                let result = (|| {
                    let mut http = Http::new(token).map_err(|error| error.to_string())?;
                    let path = project.map_or_else(
                        || "/skills".into(),
                        |project| format!("/skills?project_id={project}"),
                    );
                    let response = http.get(&path).map_err(|error| error.to_string())?;
                    if !(200..300).contains(&response.status) || response.body["success"] != true {
                        return Err("Skills unavailable".into());
                    }
                    Ok(skills_from_response(&response.body))
                })();
                let _ = send.send(result);
            });
        }
    }
    fn skill_choices(&self, cx: &App) -> Vec<Value> {
        if self.input.read(cx).slash_token().is_none()
            || self.skills.dismissed.as_deref() == Some(self.input.read(cx).content.as_ref())
        {
            return vec![];
        }
        self.skills
            .rows
            .iter()
            .filter(|skill| {
                format!(
                    "/{} {}",
                    skill["slug"].as_str().unwrap_or(""),
                    skill["name"].as_str().unwrap_or("")
                )
                .to_lowercase()
                .contains(&self.skills.query)
            })
            .cloned()
            .collect()
    }
    fn choose_skill(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(skill) = self.skill_choices(cx).get(index).cloned() else {
            return;
        };
        let Some((range, _)) = self.input.read(cx).slash_token() else {
            return;
        };
        let prefix = self.input.read(cx).content[..range.start]
            .chars()
            .next_back()
            .filter(|c| !c.is_whitespace() && *c != '(')
            .map_or("", |_| " ");
        let text = format!("{prefix}/{} ", skill["slug"].as_str().unwrap());
        self.input.update(cx, |input, cx| {
            input.replace_token(range, &text, window, cx)
        });
        self.skills.choice = 0;
        cx.notify();
    }
    fn skill_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if !self.input.focus_handle(cx).is_focused(window) {
            return;
        }
        self.poll_skills(cx);
        if self.input.read(cx).slash_token().is_none()
            || self.skills.dismissed.as_deref() == Some(self.input.read(cx).content.as_ref())
        {
            return;
        }
        let count = self.skill_choices(cx).len();
        match event.keystroke.key.as_str() {
            "escape" => self.skills.dismissed = Some(self.input.read(cx).content.to_string()),
            "up" => {
                self.skills.choice = if count == 0 {
                    0
                } else {
                    (self.skills.choice + count - 1) % count
                }
            }
            "down" => {
                self.skills.choice = if count == 0 {
                    0
                } else {
                    (self.skills.choice + 1) % count
                }
            }
            "enter" if !event.keystroke.modifiers.shift => {
                self.choose_skill(self.skills.choice, window, cx)
            }
            _ => return,
        }
        cx.notify();
        cx.stop_propagation();
    }
    fn skill_menu(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let choices = self.skill_choices(cx);
        let t = self.colors;
        div()
            .id("ai-skill-menu")
            .max_h(px(240.))
            .overflow_y_scroll()
            .children(choices.into_iter().enumerate().map(|(i, skill)| {
                div()
                    .id(("ai-skill", i))
                    .p(px(8.))
                    .text_size(px(13.))
                    .cursor_pointer()
                    .when(i == self.skills.choice, |d| d.bg(color(t.active)))
                    .child(format!("/{}", skill["slug"].as_str().unwrap()))
                    .child(
                        div().text_size(px(11.)).text_color(color(t.muted)).child(
                            skill["description"]
                                .as_str()
                                .filter(|s| !s.is_empty())
                                .or(skill["name"].as_str())
                                .unwrap_or("")
                                .to_owned(),
                        ),
                    )
                    .on_click(cx.listener(move |this, _, window, cx| {
                        cx.stop_propagation();
                        this.choose_skill(i, window, cx);
                    }))
            }))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::ThemeName;
    use serde_json::json;
    #[test]
    fn bearer_skill_catalog_deduplicates_enabled_board_override_and_never_invents_rows() {
        let rows = skills_from_response(&json!({"success":true,"skills":[
            {"slug":"review","name":"Personal","enabled":true,"projectId":null},
            {"slug":"review","name":"Board","enabled":true,"projectId":15},
            {"slug":"disabled","name":"Disabled","enabled":false},
            {"slug":"","name":"Invalid","enabled":true}
        ]}));
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["name"], "Board");
        assert!(skills_from_response(&json!({"skills":[]})).is_empty());
    }
    #[gpui::test]
    fn ai_slash_skill_unicode_filter_navigation_escape_and_insert_never_send(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(crate::input::init);
        let (view,cx)=cx.add_window_view(|window,cx|{
            let focus=cx.focus_handle();let mut sidebar=AiSidebar::new(ThemeName::Porcelain.colors(),None,focus,window,cx);
            sidebar.skills.rows=skills_from_response(&json!({"skills":[{"slug":"review","name":"Review","enabled":true},{"slug":"write","name":"Write","enabled":true}]}));
            sidebar.skills.fetched=Some(Instant::now());sidebar.focus_composer(window,cx);sidebar
        });
        cx.simulate_input("é🙂/re");
        cx.run_until_parked();
        cx.simulate_keystrokes("down enter");
        view.read_with(cx, |this, cx| {
            assert_eq!(this.input.read(cx).content.as_ref(), "é🙂 /review ");
            assert!(this.chats[0].messages.is_empty());
        });
        cx.simulate_input("/wr");
        cx.run_until_parked();
        cx.simulate_keystrokes("escape");
        view.read_with(cx, |this, cx| {
            assert!(this.skill_choices(cx).is_empty());
            assert!(this.input.read(cx).content.ends_with("/wr"));
            assert!(this.chats[0].messages.is_empty());
        });
    }
}
