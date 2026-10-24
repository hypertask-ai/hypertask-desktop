use super::*;
use crate::editor_document::{Document, Format, Tag};
use gpui::{AnyElement, FontWeight, KeyDownEvent, ScrollHandle};

#[derive(Clone)]
pub(super) struct EditorState {
    pub document: Document,
    marks: Option<Vec<Tag>>,
    slash: Option<usize>,
    choice: usize,
    scroll: ScrollHandle,
    dialog: Option<Dialog>,
    pub comment: bool,
}
#[derive(Clone)]
struct Dialog {
    kind: DialogKind,
    input: Entity<TextInput>,
    selection: Range<usize>,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum DialogKind {
    Link,
    Video,
    Emoji,
}
#[derive(Clone, Copy)]
enum Command {
    Format(Format),
    Emoji,
    Video,
}
const COMMANDS: &[(&str, &str, Command)] = &[
    ("Embed Video", "Add Youtube Video Embed", Command::Video),
    ("Emoji", "Insert an emoji", Command::Emoji),
    (
        "Heading 1",
        "Level 1 Heading",
        Command::Format(Format::Heading(1)),
    ),
    (
        "Heading 2",
        "Level 2 Heading",
        Command::Format(Format::Heading(2)),
    ),
    (
        "Highlight",
        "Toggle Highlight text",
        Command::Format(Format::Highlight),
    ),
    (
        "Bullet List",
        "Create a bullet list",
        Command::Format(Format::BulletList),
    ),
    (
        "Task List",
        "Toggle a task list",
        Command::Format(Format::TaskList),
    ),
    (
        "Numbered List",
        "Create a numbered list",
        Command::Format(Format::OrderedList),
    ),
    (
        "Text",
        "Just start typing with plain text.",
        Command::Format(Format::Paragraph),
    ),
    ("Bold", "Bold style text", Command::Format(Format::Bold)),
    (
        "Italic",
        "Italic style text",
        Command::Format(Format::Italic),
    ),
    (
        "Underline",
        "Underline style text",
        Command::Format(Format::Underline),
    ),
    (
        "BlockQuote",
        "Add BlockQuote",
        Command::Format(Format::Quote),
    ),
    ("Code", "Add inline code", Command::Format(Format::Code)),
    (
        "Code Block",
        "Add code block",
        Command::Format(Format::CodeBlock),
    ),
];
impl EditorState {
    pub fn clear_marks(&mut self) {
        self.marks = None;
    }
    pub fn new(text: &str) -> Self {
        Self {
            document: Document::plain(text),
            marks: None,
            slash: None,
            choice: 0,
            scroll: ScrollHandle::new(),
            dialog: None,
            comment: false,
        }
    }
}
impl TextInput {
    pub fn comment_editor(mut self) -> Self {
        self = self.multiline_editor();
        self.editor.as_mut().unwrap().comment = true;
        self
    }
    pub fn html(&self) -> String {
        self.editor.as_ref().map_or_else(
            || crate::editor_document::Document::plain(&self.content).html(),
            |e| e.document.html(),
        )
    }
    pub fn set_html(&mut self, html: &str, cx: &mut Context<Self>) {
        let comment = self.editor.as_ref().is_some_and(|e| e.comment);
        let document = Document::parse(html);
        self.set_content(document.text(), cx);
        let editor = self.editor.as_mut().expect("rich editor");
        editor.document = document;
        editor.comment = comment;
    }
    pub fn append_html(&mut self, html: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.remember_edit();
        if let Some(e) = &mut self.editor {
            e.document.append_html(html);
            self.content = e.document.text().into();
        }
        self.move_to(self.content.len(), window, cx);
        window.notify_text_input_changed(&self.focus_handle);
    }
    pub(super) fn replace_document(&mut self, range: Range<usize>, text: &str) {
        if let Some(e) = &mut self.editor {
            let marks = e
                .marks
                .clone()
                .unwrap_or_else(|| e.document.marks_at(range.start));
            e.document.replace(range, text, &marks);
        }
    }
    pub(super) fn update_slash(&mut self, start: usize, text: &str) {
        let Some(e) = &mut self.editor else {
            return;
        };
        e.choice = 0;
        if let Some(slash) = text.rfind('/') {
            e.slash = Some(start + slash);
        }
        if let Some(slash) = e.slash {
            let end = self.selected_range.end;
            if slash >= end
                || end > self.content.len()
                || !self.content[slash..].starts_with('/')
                || self.content[slash + 1..end]
                    .chars()
                    .any(char::is_whitespace)
            {
                e.slash = None;
            }
        }
    }
    fn slash_items(&self) -> Vec<usize> {
        let Some(e) = &self.editor else {
            return vec![];
        };
        let Some(start) = e.slash else {
            return vec![];
        };
        let end = self.selected_range.end;
        if !self.selected_range.is_empty() || start >= end || end > self.content.len() {
            return vec![];
        }
        let query = self.content[start + 1..end].to_lowercase();
        COMMANDS
            .iter()
            .enumerate()
            .filter_map(|(i, (title, _, command))| {
                if matches!(command, Command::Emoji) && !e.comment {
                    return None;
                }
                (title.to_lowercase().contains(&query)
                    || (matches!(command, Command::Emoji) && "emojis".contains(&query)))
                .then_some(i)
            })
            .collect()
    }
    fn apply_format(
        &mut self,
        format: Format,
        set: bool,
        all_if_empty: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.editor.is_none() {
            return;
        }
        self.remember_edit();
        let mut range = self.selected_range.clone();
        if all_if_empty && range.is_empty() {
            range = 0..self.content.len();
            self.selected_range = range.clone();
        }
        let e = self.editor.as_mut().unwrap();
        if let Some(name) = format.mark() {
            if range.is_empty() {
                let mut marks = e
                    .marks
                    .clone()
                    .unwrap_or_else(|| e.document.marks_at(range.start));
                let enabled = set || !marks.iter().any(|m| m.name == name);
                marks.retain(|m| m.name != name && (name != "code" || !enabled));
                if enabled {
                    marks.push(Tag {
                        name: name.into(),
                        attrs: vec![],
                    });
                }
                e.marks = Some(marks);
            } else {
                if set {
                    e.document.set_mark(
                        range,
                        Tag {
                            name: name.into(),
                            attrs: vec![],
                        },
                        true,
                    );
                } else {
                    e.document.format(range, format);
                }
                e.marks = None;
            }
        } else {
            e.document.format(range, format);
            e.marks = None;
        }
        self.content = e.document.text().into();
        self.last_layout = None;
        window.notify_text_input_changed(&self.focus_handle);
        cx.notify();
    }
    fn select_slash(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(start) = self.editor.as_mut().and_then(|e| e.slash.take()) else {
            return;
        };
        self.selected_range = start..self.selected_range.end;
        self.replace_text_in_range(None, "", window, cx);
        match COMMANDS[index].2 {
            Command::Format(Format::Code) => {
                self.move_to(self.content.len(), window, cx);
                self.apply_format(Format::Code, true, false, window, cx);
                self.replace_text_in_range(None, " ", window, cx);
            }
            Command::Format(Format::CodeBlock) => {
                self.move_to(self.content.len(), window, cx);
                self.editor_enter(window, cx);
                self.apply_format(Format::CodeBlock, false, false, window, cx);
            }
            Command::Format(format) => self.apply_format(
                format,
                matches!(format, Format::Bold | Format::Italic | Format::Underline),
                false,
                window,
                cx,
            ),
            Command::Video => self.open_editor_dialog(DialogKind::Video, window, cx),
            Command::Emoji => self.open_editor_dialog(DialogKind::Emoji, window, cx),
        }
    }
    pub(super) fn editor_enter(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.editor.is_none() {
            return false;
        }
        if !self.slash_items().is_empty() {
            let items = self.slash_items();
            let choice = self.editor.as_ref().unwrap().choice.min(items.len() - 1);
            self.select_slash(items[choice], window, cx);
            return true;
        }
        self.remember_edit();
        let e = self.editor.as_mut().unwrap();
        e.document.split(self.selected_range.clone());
        self.content = e.document.text().into();
        let at = (self.selected_range.start + 1).min(self.content.len());
        self.selected_range = at..at;
        e.marks = None;
        e.slash = None;
        window.notify_text_input_changed(&self.focus_handle);
        cx.notify();
        true
    }
    fn open_editor_dialog(
        &mut self,
        kind: DialogKind,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let placeholder = match kind {
            DialogKind::Link => "Search for a task or enter an external URL",
            DialogKind::Video => "Enter YouTube URL",
            DialogKind::Emoji => "Search",
        };
        let input = cx.new(|cx| TextInput::new(String::new(), placeholder, self.muted, cx));
        cx.observe(&input, |_, _, cx| cx.notify()).detach();
        self.editor.as_mut().unwrap().dialog = Some(Dialog {
            kind,
            input: input.clone(),
            selection: self.selected_range.clone(),
        });
        window.focus(&input.focus_handle(cx), cx);
        cx.notify();
    }
    fn close_editor_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.editor.as_mut().unwrap().dialog = None;
        window.focus(&self.focus_handle, cx);
        cx.notify();
    }
    fn finish_editor_dialog(
        &mut self,
        emoji: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(dialog) = self.editor.as_ref().and_then(|e| e.dialog.clone()) else {
            return;
        };
        let value = dialog.input.read(cx).content.trim().to_owned();
        self.selected_range = dialog.selection.clone();
        match dialog.kind {
            DialogKind::Link => {
                let href = if value.starts_with("http://")
                    || value.starts_with("https://")
                    || value.starts_with('/')
                {
                    value
                } else if value.contains('.') && !value.contains(' ') {
                    format!("http://{value}")
                } else {
                    return;
                };
                self.remember_edit();
                let e = self.editor.as_mut().unwrap();
                if !e.document.link(dialog.selection.clone(), &href) {
                    return;
                }
                e.document.set_mark(
                    dialog.selection,
                    Tag {
                        name: "mark".into(),
                        attrs: vec![],
                    },
                    false,
                );
            }
            DialogKind::Video => {
                let Some(html) = youtube_html(&value) else {
                    return;
                };
                self.append_html(&html, window, cx);
            }
            DialogKind::Emoji => {
                let Some(emoji) = emoji else {
                    return;
                };
                self.replace_text_in_range(None, &emoji, window, cx);
            }
        }
        self.close_editor_dialog(window, cx);
    }
    pub(super) fn editor_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.editor.is_none() {
            return;
        }
        let key = &event.keystroke;
        let ctrl = key.modifiers.control || key.modifiers.platform;
        if let Some(dialog) = self.editor.as_ref().and_then(|e| e.dialog.clone()) {
            match key.key.as_str() {
                "escape" => self.close_editor_dialog(window, cx),
                "enter" => {
                    let emoji = if dialog.kind == DialogKind::Emoji {
                        crate::emoji::emojis(dialog.input.read(cx).content.as_ref())
                            .first()
                            .and_then(|e| e["emoji"].as_str())
                            .map(str::to_owned)
                    } else {
                        None
                    };
                    self.finish_editor_dialog(emoji, window, cx);
                }
                _ => return,
            }
            cx.stop_propagation();
            return;
        }
        if !self.slash_items().is_empty() && !ctrl && !key.modifiers.alt {
            let items = self.slash_items();
            let e = self.editor.as_mut().unwrap();
            match key.key.as_str() {
                "escape" => e.slash = None,
                "up" => {
                    e.choice = (e.choice + items.len() - 1) % items.len();
                    e.scroll.scroll_to_item(e.choice);
                }
                "down" => {
                    e.choice = (e.choice + 1) % items.len();
                    e.scroll.scroll_to_item(e.choice);
                }
                "enter" => {
                    let index = items[e.choice.min(items.len() - 1)];
                    self.select_slash(index, window, cx);
                }
                _ => return,
            }
            cx.stop_propagation();
            cx.notify();
            return;
        }
        if !ctrl {
            return;
        }
        let shift = key.modifiers.shift;
        let alt = key.modifiers.alt;
        let physical = match key.key.as_str() {
            "!" => "1",
            "@" => "2",
            "*" => "8",
            k => k,
        };
        let shift = shift || matches!(key.key.as_str(), "!" | "@" | "*");
        let format = match (physical, shift, alt) {
            ("b", false, false) => Some(Format::Bold),
            ("i", false, false) => Some(Format::Italic),
            ("u", false, false) => Some(Format::Underline),
            ("b", true, false) => Some(Format::Quote),
            ("h", true, false) => Some(Format::Highlight),
            ("e", false, false) | ("c", true, false) => Some(Format::Code),
            ("8", true, false) => Some(Format::BulletList),
            ("7", true, false) => Some(Format::OrderedList),
            ("9", true, false) => Some(Format::TaskList),
            ("1", true, false) => Some(Format::Heading(1)),
            ("2", true, false) => Some(Format::Heading(2)),
            ("0", false, true) => Some(Format::Paragraph),
            ("1", false, true) => Some(Format::Heading(1)),
            ("2", false, true) => Some(Format::Heading(2)),
            ("3", false, true) => Some(Format::Heading(3)),
            ("4", false, true) => Some(Format::Heading(4)),
            ("5", false, true) => Some(Format::Heading(5)),
            ("6", false, true) => Some(Format::Heading(6)),
            ("c", true, true) => Some(Format::CodeBlock),
            _ => None,
        };
        if let Some(format) = format {
            let set = physical == "c" && shift;
            self.apply_format(format, set, set, window, cx);
            cx.stop_propagation();
        } else if physical == "y" && !shift && !alt {
            self.open_editor_dialog(DialogKind::Video, window, cx);
            cx.stop_propagation();
        }
    }
    pub(super) fn editor_menu(&self, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let Some(e) = &self.editor else {
            return vec![];
        };
        let mut out = vec![];
        if let Some(dialog) = &e.dialog {
            let title = match dialog.kind {
                DialogKind::Link => "Link to a task or external URL",
                DialogKind::Video => "Enter YouTube URL",
                DialogKind::Emoji => "Emoji",
            };
            let mut body = div()
                .id("editor-dialog")
                .p(px(12.))
                .bg(rgba(0x333b47ff))
                .text_color(gpui::white())
                .flex()
                .flex_col()
                .gap(px(8.))
                .child(title)
                .child(dialog.input.clone());
            if dialog.kind == DialogKind::Emoji {
                let emojis = crate::emoji::emojis(dialog.input.read(cx).content.as_ref());
                body = body.child(
                    div()
                        .id("editor-emojis")
                        .max_h(px(240.))
                        .overflow_y_scroll()
                        .flex()
                        .flex_wrap()
                        .children(emojis.iter().enumerate().map(|(i, e)| {
                            let emoji = e["emoji"].as_str().unwrap().to_owned();
                            let chosen = emoji.clone();
                            div()
                                .id(("insert-emoji", i))
                                .p(px(6.))
                                .cursor_pointer()
                                .child(emoji)
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(move |this, _, window, cx| {
                                        cx.stop_propagation();
                                        this.finish_editor_dialog(Some(chosen.clone()), window, cx);
                                    }),
                                )
                        })),
                );
            } else {
                body = body.child(
                    div()
                        .id("editor-dialog-confirm")
                        .cursor_pointer()
                        .child("OK")
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, window, cx| {
                                cx.stop_propagation();
                                this.finish_editor_dialog(None, window, cx);
                            }),
                        ),
                );
            }
            return vec![body.into_any_element()];
        }
        let items = self.slash_items();
        if !items.is_empty() {
            let list = div()
                .id("editor-slash-menu")
                .max_h(px(288.))
                .overflow_y_scroll()
                .track_scroll(&e.scroll)
                .p(px(8.))
                .bg(rgba(0x333b47ff))
                .text_color(gpui::white())
                .flex()
                .flex_col()
                .children(items.iter().enumerate().map(|(choice, &index)| {
                    let (title, description, _) = COMMANDS[index];
                    div()
                        .id(("slash-choice", index))
                        .p(px(6.))
                        .cursor_pointer()
                        .when(choice == e.choice, |d| d.bg(rgba(0x4f5766ff)))
                        .child(title)
                        .child(div().text_size(px(12.)).child(description))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |this, _, window, cx| {
                                cx.stop_propagation();
                                this.select_slash(index, window, cx);
                            }),
                        )
                }))
                .child(
                    div()
                        .text_size(px(12.))
                        .child("↑ ↓ to navigate    Enter to select"),
                );
            out.push(list.into_any_element());
        } else if !self.selected_range.is_empty() {
            let buttons = [
                ("B", Format::Bold),
                ("I", Format::Italic),
                ("U", Format::Underline),
                ("h1", Format::Heading(1)),
                ("h2", Format::Heading(2)),
                ("Normal", Format::Paragraph),
                ("Code", Format::Code),
                ("Code Block", Format::CodeBlock),
                ("Bullet List", Format::BulletList),
                ("Numbered List", Format::OrderedList),
                ("Highlight", Format::Highlight),
            ];
            let toolbar = div()
                .id("editor-bubble-menu")
                .flex()
                .flex_wrap()
                .gap(px(8.))
                .children(buttons.into_iter().enumerate().map(|(i, (label, format))| {
                    div()
                        .id(("format", i))
                        .cursor_pointer()
                        .child(label)
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |this, _, window, cx| {
                                cx.stop_propagation();
                                this.apply_format(
                                    format,
                                    matches!(format, Format::Code),
                                    matches!(format, Format::Code | Format::CodeBlock),
                                    window,
                                    cx,
                                );
                            }),
                        )
                }))
                .child(
                    div()
                        .id("editor-link")
                        .cursor_pointer()
                        .child("Link")
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, window, cx| {
                                cx.stop_propagation();
                                this.open_editor_dialog(DialogKind::Link, window, cx);
                            }),
                        ),
                )
                .child(
                    div()
                        .id("editor-unlink")
                        .cursor_pointer()
                        .child("Unlink")
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, _, cx| {
                                cx.stop_propagation();
                                this.remember_edit();
                                let range = this.selected_range.clone();
                                this.editor.as_mut().unwrap().document.unlink(range);
                                cx.notify();
                            }),
                        ),
                );
            out.push(toolbar.into_any_element());
        }
        out
    }
    pub(super) fn editor_runs(&self, range: Range<usize>, default: &TextRun) -> Vec<TextRun> {
        let Some(e) = &self.editor else {
            return vec![TextRun {
                len: range.len(),
                ..default.clone()
            }];
        };
        let mut runs = vec![];
        let mut at = 0;
        for b in e.document.blocks() {
            for s in &b.spans {
                let end = at + s.text.len();
                let start = range.start.max(at);
                let stop = range.end.min(end);
                if start < stop {
                    let mut run = default.clone();
                    run.len = stop - start;
                    if b.tag.name.starts_with('h') {
                        run.font.weight = FontWeight::SEMIBOLD;
                    }
                    let theme = crate::theme::THEMES[if default.color.l > 0.5 { 1 } else { 0 }];
                    let style = crate::rich_text::mark_style(&s.marks, theme);
                    if let Some(weight) = style.font_weight {
                        run.font.weight = weight;
                    }
                    if let Some(style) = style.font_style {
                        run.font.style = style;
                    }
                    if let Some(color) = style.color {
                        run.color = color;
                    }
                    run.background_color = style.background_color;
                    run.underline = style.underline;
                    if s.marks.iter().any(|m| m.name == "code") || b.tag.name == "pre" {
                        run.font.family = "monospace".into();
                    }
                    if let Some(marked) = &self.marked_range {
                        if marked.start < stop && marked.end > start {
                            run.underline = Some(UnderlineStyle {
                                thickness: px(1.),
                                ..Default::default()
                            });
                        }
                    }
                    runs.push(run);
                }
                at = end;
            }
            at += 1;
        }
        if runs.is_empty() {
            runs.push(TextRun {
                len: range.len(),
                ..default.clone()
            });
        }
        runs
    }
}

fn youtube_html(url: &str) -> Option<String> {
    let url = url.trim();
    let bare = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);
    let bare = bare
        .strip_prefix("www.")
        .or_else(|| bare.strip_prefix("m."))
        .or_else(|| bare.strip_prefix("music."))
        .unwrap_or(bare);
    let (host, path) = bare.split_once('/')?;
    if !matches!(host, "youtube.com" | "youtu.be" | "youtube-nocookie.com") {
        return None;
    }
    let embed = if path.starts_with("embed/") {
        format!("https://{host}/{path}")
    } else if host == "youtu.be" {
        format!("https://www.youtube.com/embed/{path}")
    } else {
        let id = if let Some(path) = path.strip_prefix("shorts/") {
            path.split(['?', '&']).next()?
        } else {
            path.split('?')
                .nth(1)?
                .split('&')
                .find_map(|p| p.strip_prefix("v="))?
        };
        if id.is_empty()
            || !id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return None;
        }
        format!("https://www.youtube.com/embed/{id}?controls=0")
    };
    Some(format!(
        "<div class=\"video-wrapper\"><iframe width=\"640\" height=\"480\" allowfullscreen=\"true\" src=\"{}\" start=\"0\"></iframe></div>",
        crate::editor_document::escape(&embed)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[gpui::test]
    fn rich_editor_real_keys_marks_caret_history_and_slash(cx: &mut gpui::TestAppContext) {
        cx.update(init);
        let (view, cx) = cx.add_window_view(|_, cx| {
            TextInput::new(String::new(), "", gpui::black(), cx).comment_editor()
        });
        view.update_in(cx, |input, window, cx| {
            window.focus(&input.focus_handle, cx)
        });
        cx.simulate_keystrokes("ctrl-b");
        cx.simulate_input("é🙂");
        view.read_with(cx, |input, _| {
            assert_eq!(input.html(), "<p><strong>é🙂</strong></p>")
        });
        cx.simulate_keystrokes("ctrl-b");
        cx.simulate_input(" plain");
        cx.simulate_keystrokes("ctrl-a ctrl-i");
        view.read_with(cx, |input, _| assert!(input.html().contains("<em>")));
        cx.simulate_keystrokes("ctrl-z");
        view.read_with(cx, |input, _| assert!(!input.html().contains("<em>")));
        cx.simulate_keystrokes("ctrl-shift-z");
        view.read_with(cx, |input, _| assert!(input.html().contains("<em>")));
        view.update_in(cx, |input, window, cx| input.reset(window, cx));
        cx.simulate_input("/Italic");
        cx.simulate_keystrokes("enter");
        cx.simulate_input("new");
        view.read_with(cx, |input, _| {
            assert_eq!(input.html(), "<p><em>new</em></p>")
        });
    }
    #[gpui::test]
    fn every_format_keyboard_and_slash_choice_is_a_real_editor_action(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(init);
        let (view, cx) = cx.add_window_view(|_, cx| {
            TextInput::new(String::new(), "", gpui::black(), cx).comment_editor()
        });
        for (key, tag) in [
            ("ctrl-b", "strong"),
            ("ctrl-i", "em"),
            ("ctrl-u", "u"),
            ("ctrl-shift-h", "mark"),
            ("ctrl-e", "code"),
            ("ctrl-shift-c", "code"),
            ("ctrl-shift-b", "blockquote"),
            ("ctrl-shift-8", "ul"),
            ("ctrl-alt-1", "h1"),
            ("ctrl-alt-2", "h2"),
            ("ctrl-alt-3", "h3"),
            ("ctrl-alt-4", "h4"),
            ("ctrl-alt-5", "h5"),
            ("ctrl-alt-6", "h6"),
            ("ctrl-shift-1", "h1"),
            ("ctrl-shift-2", "h2"),
            ("ctrl-alt-shift-c", "pre"),
        ] {
            view.update_in(cx, |input, window, cx| {
                input.reset(window, cx);
                window.focus(&input.focus_handle, cx);
            });
            cx.simulate_input("sample");
            cx.simulate_keystrokes("ctrl-a");
            cx.simulate_keystrokes(key);
            view.read_with(cx, |input, _| {
                assert!(
                    input.html().contains(&format!("<{tag}")),
                    "{key}: {}",
                    input.html()
                )
            });
        }
        for (index, (_, _, command)) in COMMANDS.iter().enumerate() {
            if let Command::Format(format) = command {
                view.update_in(cx, |input, window, cx| {
                    input.reset(window, cx);
                    window.focus(&input.focus_handle, cx);
                    input.replace_text_in_range(None, "/", window, cx);
                    input.select_slash(index, window, cx);
                });
                cx.simulate_input("typed");
                view.read_with(cx, |input, _| {
                    assert!(!input.content.contains('/'));
                    assert_eq!(
                        Document::parse(&input.html()).text(),
                        input.content.as_ref()
                    );
                    if let Some(mark) = format.mark() {
                        assert!(
                            input.html().contains(&format!("<{mark}")),
                            "{}",
                            input.html()
                        );
                    }
                });
            }
        }
        cx.simulate_keystrokes("ctrl-alt-0");
        view.read_with(cx, |input, _| assert!(input.html().contains("<p>")));
    }
    #[gpui::test]
    fn slash_filter_navigation_dismissal_modes_and_emoji_insert(cx: &mut gpui::TestAppContext) {
        cx.update(init);
        let (view, cx) = cx.add_window_view(|_, cx| {
            TextInput::new(String::new(), "", gpui::black(), cx).comment_editor()
        });
        view.update_in(cx, |input, window, cx| {
            window.focus(&input.focus_handle, cx)
        });
        cx.simulate_input("/heading");
        cx.simulate_keystrokes("down enter");
        cx.simulate_input("Title");
        view.read_with(cx, |input, _| assert_eq!(input.html(), "<h2>Title</h2>"));
        view.update_in(cx, |input, window, cx| input.reset(window, cx));
        cx.simulate_input("/Bold");
        cx.simulate_keystrokes("escape");
        view.read_with(cx, |input, _| assert_eq!(input.content.as_ref(), "/Bold"));
        view.update_in(cx, |input, window, cx| {
            input.reset(window, cx);
            input.editor.as_mut().unwrap().comment = true;
        });
        cx.simulate_input("/emoji");
        cx.simulate_keystrokes("enter");
        cx.simulate_input("rocket");
        cx.simulate_keystrokes("enter");
        view.read_with(cx, |input, _| assert_eq!(input.html(), "<p>🚀</p>"));
    }
    #[test]
    fn youtube_embed_is_local_web_html_not_a_fabricated_bearer_endpoint() {
        assert!(
            youtube_html("https://www.youtube.com/watch?v=abcd")
                .unwrap()
                .contains("https://www.youtube.com/embed/abcd?controls=0")
        );
        assert!(
            youtube_html("https://youtu.be/abcd")
                .unwrap()
                .contains("video-wrapper")
        );
        assert!(youtube_html("https://youtube.com.evil.example/watch?v=abcd").is_none());
        assert!(youtube_html("javascript:alert(1)").is_none());
    }
}
