use super::*;
use crate::{
    command_center::{Command, CommandAction},
    shortcuts::Action,
};
use gpui::MouseButton;
use ht_core::{CommentView, CommentWrite};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct CommentActions {
    pub edit_id: Option<i64>,
    pub input: Entity<TextInput>,
    pub delete_id: Option<i64>,
    pub reaction_id: Option<i64>,
    pub query: Entity<TextInput>,
    pub choice: usize,
    pub collapsed: BTreeSet<i64>,
    all_open: bool,
    picker_scroll: ScrollHandle,
}

impl CommentActions {
    pub fn new(cx: &mut Context<Desktop>) -> Self {
        let input = cx.new(|cx| {
            TextInput::new(String::new(), "Comment", color(0x8e9093), cx).multiline_editor()
        });
        let query = cx.new(|cx| TextInput::new(String::new(), "Search", color(0x8e9093), cx));
        cx.observe(&input, |_, _, cx| cx.notify()).detach();
        cx.observe(&query, |this, _, cx| {
            this.comment_actions.choice = 0;
            cx.notify();
        })
        .detach();
        Self {
            edit_id: None,
            input,
            delete_id: None,
            reaction_id: None,
            query,
            choice: 0,
            collapsed: BTreeSet::new(),
            all_open: false,
            picker_scroll: ScrollHandle::new(),
        }
    }
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

pub(super) fn editor_html(text: &str) -> String {
    format!("<p>{}</p>", escape(text).replace('\n', "<br>"))
}

fn activity(comment: &CommentView) -> bool {
    comment.extra.get("activity").is_some_and(|v| !v.is_null())
        || comment.extra.get("type").and_then(Value::as_str) == Some("activity")
}

fn reacted(comment: &CommentView, user: i64, emoji: &str) -> bool {
    comment
        .extra
        .get("reactions")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .any(|r| r["emoji"] == emoji && r["userId"] == user && r["isAgent"] != true)
}

use crate::emoji::emojis;

impl Desktop {
    fn focused_comment(&self) -> Option<&CommentView> {
        self.detail
            .as_ref()?
            .comments
            .get(self.interaction.task_part.checked_sub(1)?)
            .filter(|c| c.id > 0 && !activity(c))
    }

    pub(super) fn reset_comment_actions(&mut self) {
        self.comment_actions.edit_id = None;
        self.comment_actions.delete_id = None;
        self.comment_actions.reaction_id = None;
        self.comment_actions.collapsed.clear();
        self.comment_actions.all_open = false;
    }

    pub(super) fn comment_body_html(&self, cx: &App) -> String {
        self.comment_input.read(cx).html()
    }

    pub(super) fn run_comment_action(
        &mut self,
        action: Action,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.screen != Screen::Task {
            return;
        }
        if action == Action::ExpandComments {
            let Some(detail) = &self.detail else {
                return;
            };
            self.comment_actions.collapsed.clear();
            if self.comment_actions.all_open {
                self.comment_actions.collapsed.extend(
                    detail
                        .comments
                        .iter()
                        .filter(|c| !activity(c))
                        .map(|c| c.id),
                );
                if let Some(last) = detail.comments.iter().rev().find(|c| !activity(c)) {
                    self.comment_actions.collapsed.remove(&last.id);
                }
            }
            self.comment_actions.all_open = !self.comment_actions.all_open;
            cx.notify();
            return;
        }
        let Some(comment) = self.focused_comment().cloned() else {
            return;
        };
        let id = comment.id;
        match action {
            Action::ExpandComment => {
                let last = self
                    .detail
                    .as_ref()
                    .and_then(|d| d.comments.iter().rev().find(|c| !activity(c)))
                    .map(|c| c.id);
                if last != Some(id) && !self.comment_actions.collapsed.remove(&id) {
                    self.comment_actions.collapsed.insert(id);
                }
            }
            Action::ReplyComment => {
                if self.comment_actions.collapsed.remove(&id) {
                    cx.notify();
                    return;
                }
                let name = comment
                    .extra
                    .get("creator")
                    .and_then(|c| c["displayName"].as_str())
                    .unwrap_or("");
                let label = comment
                    .creator_id
                    .filter(|id| *id != 332)
                    .map(|id| format!("name-{id}"))
                    .unwrap_or_default();
                let html = format!(
                    "<span data-type=\"mention\" class=\"mention\" data-id=\"{}\" data-label=\"{}\" uniqueindex=\"\" projectid=\"\" contenteditable=\"false\">{}</span> said <blockquote data-quote-reply=\"true\">{}</blockquote>",
                    escape(name),
                    label,
                    escape(name),
                    comment.text
                );
                self.comment_input
                    .update(cx, |input, cx| input.append_html(&html, window, cx));
                self.edit_comment(window, cx);
            }
            Action::EditComment => {
                if comment.creator_id != self.db.current_user_id() || comment.creator_id.is_none() {
                    return;
                }
                self.comment_actions.collapsed.remove(&id);
                let html = comment.text.clone();
                self.comment_actions
                    .input
                    .update(cx, |input, cx| input.set_html(&html, cx));
                self.comment_actions.edit_id = Some(id);
                self.restore_editor(self.comment_actions.input.clone(),None,cx);
                self.editing = Editing::ExistingComment;
                window.focus(&self.comment_actions.input.focus_handle(cx), cx);
            }
            Action::DeleteComment => {
                if comment.creator_id != self.db.current_user_id() || comment.creator_id.is_none() {
                    return;
                }
                self.comment_actions.delete_id = Some(id);
                window.focus(&self.focus, cx);
            }
            Action::ReactComment => {
                self.comment_actions.reaction_id = Some(id);
                self.comment_actions
                    .query
                    .update(cx, |input, cx| input.set_content(String::new(), cx));
                self.comment_actions.choice = 0;
                window.focus(&self.comment_actions.query.focus_handle(cx), cx);
            }
            Action::LikeComment => self.react_comment(id, "👍", cx),
            _ => (),
        }
        cx.notify();
    }

    fn react_comment(&mut self, id: i64, emoji: &str, cx: &mut Context<Self>) {
        let Some(comment) = self
            .detail
            .as_ref()
            .and_then(|d| d.comments.iter().find(|c| c.id == id))
        else {
            return;
        };
        let Some(user) = self.db.current_user_id() else {
            self.error = Some("Sync your account before reacting".into());
            cx.notify();
            return;
        };
        let active = !reacted(comment, user, emoji);
        self.enqueue(
            Write::CommentAction {
                task_id: self.options.task,
                comment_id: id,
                change: CommentWrite::Reaction {
                    emoji: emoji.into(),
                    active,
                },
            },
            cx,
        );
    }

    pub(super) fn save_existing_comment(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(id) = self.comment_actions.edit_id else {
            return false;
        };
        if self.comment_actions.input.read(cx).is_empty() {
            return false;
        }
        if !self.detail.as_ref().is_some_and(|d| d.comments.iter().any(|c| c.id == id)) {return false;}
        let html = self.comment_actions.input.read(cx).html();
        if !self.enqueue(
            Write::CommentAction {
                task_id: self.options.task,
                comment_id: id,
                change: CommentWrite::Edit { text: html },
            },
            cx,
        ) {
            return false;
        }
        self.finish_editor_job(cx);
        self.comment_actions.edit_id = None;
        self.editing = Editing::None;
        window.focus(&self.focus, cx);
        true
    }

    fn confirm_comment_delete(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(id) = self.comment_actions.delete_id.take() {
            self.enqueue(
                Write::CommentAction {
                    task_id: self.options.task,
                    comment_id: id,
                    change: CommentWrite::Delete,
                },
                cx,
            );
            self.interaction.task_part = self.interaction.task_part.min(self.comments.len() + 1);
        }
        window.focus(&self.focus, cx);
        cx.notify();
    }

    pub(super) fn comment_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let key = &event.keystroke;
        if self.comment_actions.delete_id.is_some() {
            match key.key.as_str() {
                "enter"
                    if !key.modifiers.control && !key.modifiers.platform && !key.modifiers.alt =>
                {
                    self.confirm_comment_delete(window, cx)
                }
                "escape" => {
                    self.comment_actions.delete_id = None;
                    window.focus(&self.focus, cx);
                    cx.notify();
                }
                _ => (),
            }
            cx.stop_propagation();
            return true;
        }
        if self.comment_actions.reaction_id.is_some() {
            let rows = emojis(self.comment_actions.query.read(cx).content.as_ref());
            match key.key.as_str() {
                "escape" => self.comment_actions.reaction_id = None,
                "down" | "right" => {
                    self.comment_actions.choice = (self.comment_actions.choice
                        + if key.key == "down" { 6 } else { 1 })
                    .min(rows.len().saturating_sub(1))
                }
                "up" | "left" => {
                    self.comment_actions.choice = self
                        .comment_actions
                        .choice
                        .saturating_sub(if key.key == "up" { 6 } else { 1 })
                }
                "enter" => {
                    if let Some(emoji) = rows
                        .get(self.comment_actions.choice)
                        .and_then(|e| e["emoji"].as_str())
                    {
                        let id = self.comment_actions.reaction_id.take().unwrap();
                        self.react_comment(id, emoji, cx);
                    }
                }
                _ => return true,
            }
            if self.comment_actions.reaction_id.is_none() {
                window.focus(&self.focus, cx);
            }
            self.comment_actions
                .picker_scroll
                .scroll_to_item(self.comment_actions.choice / 6);
            cx.stop_propagation();
            cx.notify();
            return true;
        }
        false
    }

    pub(super) fn add_comment_commands(&self, rows: &mut Vec<Command>) {
        if self.screen != Screen::Task {
            return;
        }
        let mut commands = vec![];
        if let Some(comment) = self.focused_comment() {
            let own =
                comment.creator_id.is_some() && comment.creator_id == self.db.current_user_id();
            for (key, name, keys, action, enabled) in [
                ("editcomment", "Edit comment", "", Action::EditComment, own),
                (
                    "fastLikeComment",
                    "Like comment",
                    "L",
                    Action::LikeComment,
                    true,
                ),
                (
                    "replyToComment",
                    "Reply to comment",
                    "ENTER",
                    Action::ReplyComment,
                    true,
                ),
                (
                    "reactToComment",
                    "React to comment",
                    "R",
                    Action::ReactComment,
                    true,
                ),
                (
                    "deleteComment",
                    "Delete comment",
                    "CTRL SHIFT 3",
                    Action::DeleteComment,
                    own,
                ),
            ] {
                if enabled {
                    commands.push(Command {
                        key: key.into(),
                        name: name.into(),
                        group: "Task",
                        keys: keys.into(),
                        keywords: "comment",
                        action: CommandAction::Shortcut(action),
                    });
                }
            }
        }
        commands.push(Command {
            key: "expandCollapseAllComments".into(),
            name: if self.comment_actions.all_open {
                "Collapse all comments"
            } else {
                "Expand all comments"
            }
            .into(),
            group: "Task",
            keys: "SHIFT O".into(),
            keywords: "comment",
            action: CommandAction::Shortcut(Action::ExpandComments),
        });
        rows.splice(0..0, commands);
    }

    pub(super) fn comment_row(
        &self,
        i: usize,
        comment: &CommentView,
        body: &RichText,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let t = self.colors;
        let id = comment.id;
        let collapsed = self.comment_actions.collapsed.contains(&id);
        let editing = self.comment_actions.edit_id == Some(id);
        let preview = body
            .text
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .take(150)
            .collect::<String>();
        let mut reactions: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for reaction in comment
            .extra
            .get("reactions")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if let Some(emoji) = reaction["emoji"].as_str() {
                let who = reaction
                    .pointer("/agent/displayName")
                    .or_else(|| reaction.pointer("/user/displayName"))
                    .and_then(Value::as_str)
                    .map(str::to_owned)
                    .unwrap_or_else(|| reaction["userId"].to_string());
                reactions.entry(emoji.into()).or_default().push(who);
            }
        }
        div()
            .id(("comment", i))
            .anchor_scroll(Some(self.comment_anchors[i].clone()))
            .p(px(20.))
            .bg(color(t.comment))
            .border_1()
            .border_color(color(if self.interaction.task_part == i + 1 {
                t.accent
            } else {
                t.border
            }))
            .rounded(px(if self.options.theme == ThemeName::Dia {
                10.
            } else {
                5.
            }))
            .shadow(shadow())
            .flex()
            .flex_col()
            .gap(px(12.))
            .on_click(cx.listener(move |this, _, window, cx| {
                this.interaction.task_part = i + 1;
                window.focus(&this.focus, cx);
                cx.notify();
            }))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .text_color(color(t.muted))
                    .child(div().child(comment_author(&comment.extra)).layout_key(
                        &self.layout,
                        format!("task.comment.{i}.author"),
                        Some(comment_author(&comment.extra)),
                    ))
                    .child(div().flex_1())
                    .when(!activity(comment) && id > 0, |d| {
                        d.child(
                            div()
                                .id(("reply-comment", i))
                                .cursor_pointer()
                                .child(icon("corner-left-up", t.muted, 14.))
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    cx.stop_propagation();
                                    this.interaction.task_part = i + 1;
                                    this.run_comment_action(Action::ReplyComment, window, cx);
                                }))
                                .layout_key(&self.layout, format!("task.comment.{i}.reply"), None),
                        )
                        .child(
                            div()
                                .id(("react-comment", i))
                                .cursor_pointer()
                                .child("☺+")
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    cx.stop_propagation();
                                    this.interaction.task_part = i + 1;
                                    this.run_comment_action(Action::ReactComment, window, cx);
                                })),
                        )
                        .when(
                            !self
                                .db
                                .current_user_id()
                                .is_some_and(|user| reacted(comment, user, "👍")),
                            |d| {
                                d.child(
                                    div()
                                        .id(("like-comment", i))
                                        .cursor_pointer()
                                        .child("👍")
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            cx.stop_propagation();
                                            this.react_comment(id, "👍", cx);
                                        })),
                                )
                            },
                        )
                        .child(
                            div()
                                .id(("comment-options", i))
                                .cursor_pointer()
                                .child("⋯")
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    cx.stop_propagation();
                                    this.interaction.task_part = i + 1;
                                    this.run_shortcut(Action::Commands, window, cx);
                                })),
                        )
                    })
                    .child(short_date(&comment.created_at)),
            )
            .child(if editing {
                div()
                    .id(("edit-comment", i))
                    .on_click(cx.listener(|_, _, _, cx| cx.stop_propagation()))
                    .child(self.comment_actions.input.clone())
                    .child(
                        div()
                            .flex()
                            .gap(px(12.))
                            .child(
                                div()
                                    .id("save-comment-edit")
                                    .cursor_pointer()
                                    .child("Save")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        cx.stop_propagation();
                                        this.save_existing_comment(window, cx);
                                    })),
                            )
                            .child(
                                div()
                                    .id("cancel-comment-edit")
                                    .cursor_pointer()
                                    .child("Cancel")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        cx.stop_propagation();
                                        this.comment_actions.edit_id = None;
                                        this.editing = Editing::None;
                                        window.focus(&this.focus, cx);
                                        cx.notify();
                                    })),
                            ),
                    )
                    .layout_key(&self.layout, format!("task.comment.{i}.editor"), None)
                    .into_any_element()
            } else {
                div()
                    .id(("comment-body", i))
                    .line_height(px(21.))
                    .when(collapsed, |d| {
                        d.cursor_pointer()
                            .on_click(cx.listener(move |this, _, _, cx| {
                                cx.stop_propagation();
                                this.comment_actions.collapsed.remove(&id);
                                cx.notify();
                            }))
                    })
                    .child(if collapsed {
                        div().child(preview).into_any_element()
                    } else {
                        body.element().into_any_element()
                    })
                    .layout_key(
                        &self.layout,
                        format!("task.comment.{i}.body"),
                        Some(body.text.clone()),
                    )
                    .into_any_element()
            })
            .when(!collapsed, |d| {
                d.child(div().flex().gap(px(6.)).children(reactions.into_iter().map(
                    |(emoji, people)| {
                        let e = emoji.clone();
                        div()
                            .id(SharedString::from(format!("reaction-{id}-{emoji}")))
                            .px(px(6.))
                            .rounded(px(4.))
                            .bg(color(t.hover))
                            .cursor_pointer()
                            .child(format!("{emoji} {}", people.len()))
                            .tooltip(move |_, cx| comment_tooltip(people.join(", "), cx))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                cx.stop_propagation();
                                this.react_comment(id, &e, cx);
                            }))
                    },
                )))
            })
            .layout_key(&self.layout, format!("task.comment.{i}"), None)
            .into_any_element()
    }

    pub(super) fn comment_overlay(&self, cx: &mut Context<Self>) -> AnyElement {
        let t = self.colors;
        let content = if self.comment_actions.delete_id.is_some() {
            div()
                .id("delete-comment-confirmation")
                .p(px(24.))
                .flex()
                .flex_col()
                .gap(px(16.))
                .child("Clicking confirm will delete this comment!")
                .child(
                    div()
                        .flex()
                        .gap(px(16.))
                        .child(
                            div()
                                .id("confirm-delete-comment")
                                .cursor_pointer()
                                .child("Delete comment")
                                .on_click(cx.listener(|this, _, window, cx| {
                                    cx.stop_propagation();
                                    this.confirm_comment_delete(window, cx);
                                })),
                        )
                        .child(
                            div()
                                .id("cancel-delete-comment")
                                .cursor_pointer()
                                .child("Cancel")
                                .on_click(cx.listener(|this, _, window, cx| {
                                    cx.stop_propagation();
                                    this.comment_actions.delete_id = None;
                                    window.focus(&this.focus, cx);
                                    cx.notify();
                                })),
                        ),
                )
                .layout_key(&self.layout, "task.comment.delete-confirmation", None)
                .into_any_element()
        } else if let Some(id) = self.comment_actions.reaction_id {
            let emojis = emojis(self.comment_actions.query.read(cx).content.as_ref());
            let mut grid = div()
                .id("emoji-results")
                .h(px(300.))
                .overflow_y_scroll()
                .track_scroll(&self.comment_actions.picker_scroll)
                .flex()
                .flex_col();
            for (row, chunk) in emojis.chunks(6).enumerate() {
                grid = grid.child(div().flex().children(chunk.iter().enumerate().map(
                    |(col, emoji)| {
                        let e = emoji["emoji"].as_str().unwrap().to_owned();
                        let title = emoji["name"].as_str().unwrap().to_owned();
                        div()
                            .id(("emoji", row * 6 + col))
                            .w(px(44.))
                            .h(px(36.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .when(self.comment_actions.choice == row * 6 + col, |d| {
                                d.bg(color(t.hover))
                            })
                            .child(e.clone())
                            .tooltip(move |_, cx| comment_tooltip(title.clone(), cx))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                cx.stop_propagation();
                                this.comment_actions.reaction_id = None;
                                this.react_comment(id, &e, cx);
                                window.focus(&this.focus, cx);
                                cx.notify();
                            }))
                    },
                )));
            }
            div()
                .id("comment-reaction-picker")
                .w(px(300.))
                .p(px(12.))
                .flex()
                .flex_col()
                .gap(px(8.))
                .child(self.comment_actions.query.clone())
                .child(grid)
                .layout_key(&self.layout, "task.comment.reaction-picker", None)
                .into_any_element()
        } else {
            return div().into_any_element();
        };
        div()
            .id("comment-action-backdrop")
            .absolute()
            .size_full()
            .top_0()
            .left_0()
            .bg(rgba(0x00000080))
            .flex()
            .items_center()
            .justify_center()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|_, _, _, cx| cx.stop_propagation()),
            )
            .on_click(cx.listener(|this, _, window, cx| {
                this.comment_actions.delete_id = None;
                this.comment_actions.reaction_id = None;
                window.focus(&this.focus, cx);
                cx.notify();
            }))
            .child(
                div()
                    .id("comment-action-dialog")
                    .bg(color(t.comment))
                    .rounded(px(5.))
                    .shadow(shadow())
                    .on_click(cx.listener(|_, _, _, cx| cx.stop_propagation()))
                    .child(content),
            )
            .into_any_element()
    }
}

struct CommentTooltip(String);
impl Render for CommentTooltip {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .p(px(8.))
            .bg(color(0x202124))
            .text_color(color(0xffffff))
            .rounded(px(4.))
            .text_size(px(12.))
            .child(self.0.clone())
    }
}
fn comment_tooltip(text: String, cx: &mut App) -> gpui::AnyView {
    cx.new(|_| CommentTooltip(text)).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn desktop(window: &mut Window, cx: &mut Context<Desktop>) -> Desktop {
        let mut db = Db::memory().unwrap();
        db.kv_set("current-user-id", "42").unwrap();
        db.apply_task(&json!({"id":1,"projectId":15,"title":"Task","status":"Normal","description":"<p>Description</p>"})).unwrap();
        db.apply_comments(1,&json!({"success":true,"comments":[
            {"id":7,"text":"<p>First <strong>é🙂</strong></p>","creatorId":42,"creator":{"id":42,"displayName":"A & B"},"createdAt":"2026-01-01T00:00:00Z","reactions":[]},
            {"id":8,"text":"<p>Latest</p>","creatorId":43,"creator":{"id":43,"displayName":"Other"},"createdAt":"2026-01-02T00:00:00Z","reactions":[]}
        ]})).unwrap();
        let options = Options::parse(
            [
                "--offline",
                "--screen",
                "task",
                "--task",
                "1",
                "--board",
                "15",
            ]
            .map(String::from),
        )
        .unwrap();
        let view = Desktop::new(db, options, window, cx);
        window.focus(&view.focus, cx);
        view
    }

    #[gpui::test]
    fn comment_reply_quotes_specific_comment_and_queues_html_not_agent_reply_id(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(crate::input::init);
        let (view, cx) = cx.add_window_view(desktop);
        view.update_in(cx, |this, _, _| this.interaction.task_part = 1);
        cx.simulate_keystrokes("enter");
        view.update_in(cx, |this, window, cx| {
            assert!(this.comment_input.focus_handle(cx).is_focused(window));
            assert!(this.comment_input.read(cx).content.contains("First é🙂"));
            this.comment_input.update(cx, |input, cx| {
                input.append("My reply\nSecond line", window, cx)
            });
        });
        cx.simulate_keystrokes("ctrl-shift-enter");
        view.read_with(cx,|this,cx|{
            let rows=this.db.mutations().unwrap(); assert_eq!(rows.len(),1);
            let text=rows[0].payload_json["text"].as_str().unwrap();
            assert!(text.contains("data-label=\"name-42\""));
            assert!(text.contains("A &amp; B"));
            assert!(text.contains("<blockquote data-quote-reply=\"true\"><p>First <strong>é🙂</strong></p></blockquote>"));
            assert!(text.ends_with("<p>My reply<br>Second line</p>"));
            assert!(rows[0].payload_json.get("reply_to_comment_id").is_none());
            assert!(this.comment_input.read(cx).content.is_empty());
        });
    }

    #[gpui::test]
    fn comment_edit_save_cancel_and_ownership_are_real_desktop_actions(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(crate::input::init);
        let (view, cx) = cx.add_window_view(desktop);
        view.update_in(cx, |this, window, cx| {
            this.interaction.task_part = 2;
            this.run_comment_action(Action::EditComment, window, cx);
            assert!(this.comment_actions.edit_id.is_none());
            this.interaction.task_part = 1;
            this.run_comment_action(Action::EditComment, window, cx);
            assert_eq!(this.comment_actions.edit_id, Some(7));
            this.comment_actions
                .input
                .update(cx, |input, cx| input.set_content("Cancelled".into(), cx));
        });
        cx.simulate_keystrokes("escape");
        view.update_in(cx, |this, window, cx| {
            assert!(this.comment_actions.edit_id.is_none());
            assert!(this.db.mutations().unwrap().is_empty());
            this.run_comment_action(Action::EditComment, window, cx);
            this.comment_actions.input.update(cx, |input, cx| {
                input.set_content("Edited\nAgain".into(), cx)
            });
        });
        cx.simulate_keystrokes("ctrl-enter");
        view.read_with(cx, |this, _| {
            assert!(this.comment_actions.edit_id.is_none());
            assert_eq!(
                RichText::parse(&this.detail.as_ref().unwrap().comments[0].text, this.colors)
                    .text
                    .trim_end(),
                "Edited\nAgain"
            );
            let rows = this.db.mutations().unwrap();
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].payload_json["comment_id"], 7);
            assert_eq!(rows[0].payload_json["change"]["action"], "edit");
        });
    }

    #[gpui::test]
    fn comment_delete_requires_confirmation_and_escape_is_not_a_write(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(crate::input::init);
        let (view, cx) = cx.add_window_view(desktop);
        view.update_in(cx, |this, _, _| this.interaction.task_part = 1);
        cx.simulate_keystrokes("ctrl-shift-3");
        view.read_with(cx, |this, _| {
            assert_eq!(this.comment_actions.delete_id, Some(7));
            assert!(this.db.mutations().unwrap().is_empty());
        });
        cx.simulate_keystrokes("escape");
        view.read_with(cx, |this, _| {
            assert!(this.comment_actions.delete_id.is_none());
            assert_eq!(this.detail.as_ref().unwrap().comments.len(), 2);
        });
        cx.simulate_keystrokes("ctrl-shift-3 enter");
        view.read_with(cx, |this, _| {
            assert_eq!(this.detail.as_ref().unwrap().comments.len(), 1);
            assert_eq!(this.detail.as_ref().unwrap().comments[0].id, 8);
            assert_eq!(
                this.db.mutations().unwrap()[0].payload_json["change"]["action"],
                "delete"
            );
        });
    }

    #[gpui::test]
    fn comment_single_all_expansion_keeps_latest_open_and_does_not_fire_in_editor(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(crate::input::init);
        let (view, cx) = cx.add_window_view(desktop);
        view.update_in(cx, |this, _, _| this.interaction.task_part = 1);
        cx.simulate_keystrokes("o");
        view.read_with(cx, |this, _| {
            assert_eq!(this.comment_actions.collapsed, BTreeSet::from([7]))
        });
        cx.simulate_keystrokes("o shift-o shift-o");
        view.read_with(cx, |this, _| {
            assert_eq!(this.comment_actions.collapsed, BTreeSet::from([7]))
        });
        view.update_in(cx, |this, _, _| this.interaction.task_part = 2);
        cx.simulate_keystrokes("o");
        view.read_with(cx, |this, _| {
            assert!(!this.comment_actions.collapsed.contains(&8))
        });
        cx.simulate_keystrokes("ctrl-m");
        cx.simulate_input("o");
        view.read_with(cx, |this, cx| {
            assert_eq!(this.comment_input.read(cx).content.as_ref(), "o");
            assert_eq!(this.comment_actions.collapsed, BTreeSet::from([7]));
        });
    }

    #[gpui::test]
    fn comment_reaction_picker_search_and_fast_like_toggle_the_focused_comment(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(crate::input::init);
        let (view, cx) = cx.add_window_view(desktop);
        view.update_in(cx, |this, _, _| this.interaction.task_part = 1);
        cx.simulate_keystrokes("l l");
        view.read_with(cx, |this, _| {
            let rows = this.db.mutations().unwrap();
            assert_eq!(rows.len(), 2);
            assert_eq!(
                rows[0].payload_json["change"],
                json!({"action":"reaction","emoji":"👍","active":true})
            );
            assert_eq!(
                rows[1].payload_json["change"],
                json!({"action":"reaction","emoji":"👍","active":false})
            );
        });
        cx.simulate_keystrokes("r");
        cx.simulate_input("rocket");
        view.read_with(cx, |this, cx| {
            assert_eq!(this.comment_actions.reaction_id, Some(7));
            assert_eq!(
                this.comment_actions.query.read(cx).content.as_ref(),
                "rocket"
            );
        });
        cx.simulate_keystrokes("enter");
        view.read_with(cx, |this, _| {
            let rows = this.db.mutations().unwrap();
            assert_eq!(rows.len(), 3);
            assert_eq!(rows[2].payload_json["change"]["emoji"], "🚀");
            assert_eq!(rows[2].payload_json["comment_id"], 7);
            assert!(this.comment_actions.reaction_id.is_none());
        });
    }

    #[gpui::test]
    fn comment_and_description_editors_linebreak_undo_redo_do_not_send(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(crate::input::init);
        let (view, cx) = cx.add_window_view(desktop);
        for description in [false, true] {
            view.update_in(cx, |this, window, cx| {
                if description {
                    this.edit_description(window, cx);
                    this.interaction
                        .description
                        .update(cx, |input, cx| input.set_content(String::new(), cx));
                } else {
                    this.edit_comment(window, cx);
                }
            });
            cx.simulate_input("é🙂");
            cx.simulate_keystrokes("shift-enter");
            view.read_with(cx, |this, cx| {
                let input = if description {
                    &this.interaction.description
                } else {
                    &this.comment_input
                };
                assert_eq!(input.read(cx).content.as_ref(), "é🙂\n");
                assert!(this.db.mutations().unwrap().is_empty());
            });
            cx.simulate_keystrokes("ctrl-z");
            view.read_with(cx, |this, cx| {
                let input = if description {
                    &this.interaction.description
                } else {
                    &this.comment_input
                };
                assert_eq!(input.read(cx).content.as_ref(), "é🙂");
            });
            cx.simulate_keystrokes("ctrl-shift-z");
            view.read_with(cx, |this, cx| {
                let input = if description {
                    &this.interaction.description
                } else {
                    &this.comment_input
                };
                assert_eq!(input.read(cx).content.as_ref(), "é🙂\n");
            });
        }
    }

    #[gpui::test]
    fn desktop_composer_grows_with_content_without_inventing_mobile_resize_controls(
        cx: &mut gpui::TestAppContext,
    ) {
        let (view, cx) = cx.add_window_view(desktop);
        view.update_in(cx, |this, _, cx| {
            this.layout = Some(Rc::new(RefCell::new(LayoutDump::default())));
            cx.notify();
        });
        cx.run_until_parked();
        let height = |this: &Desktop| {
            this.layout
                .as_ref()
                .unwrap()
                .borrow()
                .interaction_elements()
                .into_iter()
                .find(|e| e["key"] == "task.composer")
                .expect("composer painted")["h"]
                .as_f64()
                .unwrap()
        };
        let initial = view.read_with(cx, |this, _| height(this));
        view.update_in(cx, |this, window, cx| {
            this.edit_comment(window, cx);
            this.comment_input.update(cx, |input, cx| {
                input.set_content("More text\n".repeat(12), cx)
            });
        });
        cx.run_until_parked();
        view.read_with(cx, |this, cx| {
            assert!(
                height(this) > initial,
                "desktop textarea must grow rather than clip lines"
            );
            let elements = this
                .layout
                .as_ref()
                .unwrap()
                .borrow()
                .interaction_elements();
            assert!(!elements.iter().any(|e| e["key"] == "task.composer.resize"));
            assert!(this.comment_input.read(cx).content.contains("More text"));
        });
    }

    #[test]
    fn web_emoji_catalog_and_plain_html_are_not_a_made_up_reaction_subset() {
        assert_eq!(emojis("").len(), 1870);
        assert!(emojis("rocket").iter().any(|e| e["emoji"] == "🚀"));
        assert!(emojis("not-a-real-emoji-phrase").is_empty());
        assert_eq!(
            editor_html("<script>&\nnext"),
            "<p>&lt;script&gt;&amp;<br>next</p>"
        );
    }
    #[gpui::test]
    fn rich_html_all_three_editor_surfaces_use_real_keys_and_the_durable_queue(cx: &mut gpui::TestAppContext) {
        cx.update(crate::input::init);
        let (view,cx)=cx.add_window_view(desktop);
        for surface in 0..3 {
            view.update_in(cx,|this,window,cx| {
                let input=match surface {
                    0=>{this.edit_comment(window,cx);this.comment_input.clone()},
                    1=>{this.interaction.task_part=1;this.run_comment_action(Action::EditComment,window,cx);this.comment_actions.input.clone()},
                    _=>{this.edit_description(window,cx);this.interaction.description.clone()},
                };
                input.update(cx,|input,cx|input.set_content("é🙂".into(),cx));
            });
            cx.simulate_keystrokes("ctrl-a ctrl-u ctrl-shift-enter");
            view.read_with(cx,|this,_|{
                let rows=this.db.mutations().unwrap();assert_eq!(rows.len(),surface+1);
                let payload=&rows[surface].payload_json;
                let html=match surface {0=>payload["text"].as_str(),1=>payload["change"]["text"].as_str(),_=>payload["description"].as_str()}.expect("queued rich HTML");
                assert_eq!(html,"<p><u>é🙂</u></p>");
            });
        }
        cx.simulate_keystrokes("ctrl-b");
        view.read_with(cx,|this,_|assert!(this.shell.boards,"Ctrl+B switches boards outside editors"));
    }
    #[gpui::test]
    fn discard_is_surface_scoped_and_queues_the_authorized_server_lifecycle(cx: &mut gpui::TestAppContext) {
        cx.update(crate::input::init);let(view,cx)=cx.add_window_view(desktop);
        view.update_in(cx,|this,window,cx|{this.comment_input.update(cx,|input,cx|input.set_content("Keep comment".into(),cx));this.edit_description(window,cx);this.interaction.description.update(cx,|input,cx|input.set_content("Unsaved".into(),cx));});
        cx.simulate_keystrokes("ctrl-shift-,");
        view.update_in(cx,|this,window,cx|{
            assert_eq!(this.interaction.description.read(cx).html(),"<p>Description</p>");
            assert_eq!(this.comment_input.read(cx).content.as_ref(),"Keep comment");
            assert_eq!(this.db.mutations().unwrap()[0].payload_json["draft_type"],"description");this.edit_comment(window,cx);
        });
        cx.simulate_keystrokes("ctrl-shift-,");
        view.read_with(cx,|this,cx|{assert!(this.comment_input.read(cx).content.is_empty());assert_eq!(this.db.mutations().unwrap()[1].payload_json["draft_type"],"comment");});
    }

    struct ImproveServer;
    impl ht_core::Transport for ImproveServer {
        fn get(&mut self,_:&str)->Result<ht_core::http::Response,ht_core::http::TransportError>{panic!("unexpected GET")}
        fn post(&mut self,path:&str,body:&Value,key:&str)->Result<ht_core::http::Response,ht_core::http::TransportError>{
            assert_eq!(path,"/ai/improve");assert_eq!(body["task_id"],1);assert_eq!(body["project_id"],15);assert_eq!(body["text"],"<p>Draft</p>");assert!(!key.is_empty());
            Ok(ht_core::http::Response{status:200,body:json!({"success":true,"command":body["command"],"html":"<h2>Summary</h2>"})})
        }
    }
    #[gpui::test]
    fn all_six_ai_slash_commands_queue_apply_and_undo_without_publishing_on_all_surfaces(cx:&mut gpui::TestAppContext) {
        cx.update(crate::input::init);let(view,cx)=cx.add_window_view(desktop);
        for surface in 0..3 {for query in ["/Fix","/Readability","/Summarize","/Shorter","/English","/German"] {
            view.update_in(cx,|this,window,cx|{
                let input=match surface {0=>{this.edit_comment(window,cx);this.comment_input.clone()},1=>{this.interaction.task_part=1;this.run_comment_action(Action::EditComment,window,cx);this.comment_actions.input.clone()},_=>{this.edit_description(window,cx);this.interaction.description.clone()}};
                input.update(cx,|input,cx|{input.set_content("Draft".into(),cx);input.append(query,window,cx);});
            });
            cx.simulate_keystrokes("enter");
            view.update_in(cx,|this,_,cx|{
                let rows=this.db.mutations().unwrap();assert_eq!(rows.len(),1);assert_eq!(rows[0].kind,"editor_improve");
                assert_eq!(this.db.drain_once(&mut ImproveServer,i64::MAX/2).unwrap(),ht_core::Drain::Applied(rows[0].id));this.poll_editor_results(cx);
                let input=match surface {0=>&this.comment_input,1=>&this.comment_actions.input,_=>&this.interaction.description};assert_eq!(input.read(cx).html(),"<h2>Summary</h2>");assert_eq!(this.db.task(1).unwrap().unwrap().description,"<p>Description</p>");
            });
            cx.simulate_keystrokes("ctrl-z");
            view.read_with(cx,|this,cx|{let input=match surface {0=>&this.comment_input,1=>&this.comment_actions.input,_=>&this.interaction.description};assert_eq!(input.read(cx).html(),"<p>Draft</p>");});
        }}
    }
    #[gpui::test]
    fn late_ai_result_never_overwrites_typing_and_pending_job_reattaches_to_its_original_editor(cx:&mut gpui::TestAppContext) {
        cx.update(crate::input::init);let(view,cx)=cx.add_window_view(desktop);
        view.update_in(cx,|this,window,cx|{this.edit_comment(window,cx);this.comment_input.update(cx,|input,cx|input.append("Draft/Summarize",window,cx));});cx.simulate_keystrokes("enter");
        view.update_in(cx,|this,window,cx|{
            this.comment_input.update(cx,|input,cx|input.append(" changed",window,cx));this.db.drain_once(&mut ImproveServer,i64::MAX/2).unwrap();this.poll_editor_results(cx);assert!(this.comment_input.read(cx).content.ends_with(" changed"));assert!(this.error.as_ref().unwrap().contains("not applied"));
            this.comment_input.update(cx,|input,cx|input.reset(window,cx));this.edit_comment(window,cx);this.comment_input.update(cx,|input,cx|input.append("Draft/Summarize",window,cx));
        });cx.simulate_keystrokes("enter");
        view.update_in(cx,|this,window,cx|{this.comment_input.update(cx,|input,cx|input.reset(window,cx));this.restore_editor(this.comment_input.clone(),None,cx);assert_eq!(this.comment_input.read(cx).html(),"<p>Draft</p>");this.db.drain_once(&mut ImproveServer,i64::MAX/2).unwrap();this.poll_editor_results(cx);assert_eq!(this.comment_input.read(cx).html(),"<h2>Summary</h2>");});
    }

    #[gpui::test]
    fn cached_web_drafts_load_in_their_own_editor_and_discard_keeps_published_content(cx:&mut gpui::TestAppContext) {
        cx.update(crate::input::init);let(view,cx)=cx.add_window_view(desktop);
        view.update_in(cx,|this,window,cx|{
            this.db.kv_set("editor-drafts:42:1",&json!([
                {"id":7,"taskId":1,"draftType":"comment","text":"<p><strong>Draft comment</strong></p>"},
                {"id":8,"taskId":1,"draftType":"description","text":"<h2>Draft description</h2>"}
            ]).to_string()).unwrap();
            this.edit_comment(window,cx);assert_eq!(this.comment_input.read(cx).html(),"<p><strong>Draft comment</strong></p>");this.edit_description(window,cx);assert_eq!(this.interaction.description.read(cx).html(),"<h2>Draft description</h2>");
        });
        cx.simulate_keystrokes("ctrl-shift-,");
        view.update_in(cx,|this,window,cx|{assert_eq!(this.interaction.description.read(cx).html(),"<p>Description</p>");assert_eq!(this.db.editor_drafts(1).unwrap().len(),1);assert_eq!(this.db.task(1).unwrap().unwrap().description,"<p>Description</p>");this.edit_comment(window,cx);});
        cx.simulate_keystrokes("ctrl-shift-,");view.read_with(cx,|this,cx|{assert!(this.comment_input.read(cx).is_empty());assert!(this.db.editor_drafts(1).unwrap().is_empty());assert_eq!(this.db.mutations().unwrap().len(),2);});
    }

}
