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
    palette: bool,
    job: Option<(i64, i64)>,
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
    Improve(ht_core::ImproveCommand),
}
const COMMANDS: &[(&str, &str, Command)] = &[
    (
        "Fix Spelling & Grammer",
        "Fix Spelling and grammar",
        Command::Improve(ht_core::ImproveCommand::Grammar),
    ),
    (
        "Improve Readability",
        "Let AI improve your sentence readability",
        Command::Improve(ht_core::ImproveCommand::Readability),
    ),
    (
        "Summarize",
        "Summarize all text with AI",
        Command::Improve(ht_core::ImproveCommand::Summarize),
    ),
    (
        "Make Shorter",
        "Make the text shorter with AI",
        Command::Improve(ht_core::ImproveCommand::MakeShorter),
    ),
    (
        "Translate to English",
        "Translate the text to English",
        Command::Improve(ht_core::ImproveCommand::English),
    ),
    (
        "Translate to German",
        "Translate the text to German",
        Command::Improve(ht_core::ImproveCommand::German),
    ),
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
    pub fn history_state(&self) -> Self {
        let mut e = self.clone();
        e.dialog = None;
        e.slash = None;
        e.choice = 0;
        e.palette = false;
        e.job = None;
        e
    }
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
            palette: false,
            job: None,
        }
    }
}
impl TextInput {
    pub fn comment_editor(mut self) -> Self {
        self = self.multiline_editor();
        self.editor.as_mut().unwrap().comment = true;
        self
    }
    pub fn set_editor_job(&mut self, id: i64, task: i64, cx: &mut Context<Self>) {
        if let Some(e) = &mut self.editor {
            e.job = Some((id, task));
            cx.notify();
        }
    }
    pub fn clear_editor_job(&mut self) {
        if let Some(e) = &mut self.editor {
            e.job = None;
        }
    }
    pub fn editor_job(&self) -> Option<(i64, i64)> {
        self.editor.as_ref().and_then(|e| e.job)
    }
    pub fn apply_editor_result(
        &mut self,
        result: ht_core::EditorResult,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        let Some((_, task)) = self.editor_job() else {
            return Ok(());
        };
        self.editor.as_mut().unwrap().job = None;
        if result.task_id != task || result.source != self.html() {
            return Err("Editor changed while AI was running; the result was not applied".into());
        }
        if let Some(error) = result.error {
            return Err(error);
        }
        let html = result
            .html
            .ok_or_else(|| "Missing improved HTML".to_owned())?;
        self.remember_edit();
        let document = Document::parse(&html);
        self.content = document.text().into();
        self.selected_range = self.content.len()..self.content.len();
        self.editor.as_mut().unwrap().document = document;
        self.last_layout = None;
        cx.notify();
        Ok(())
    }
    pub fn is_empty(&self) -> bool {
        self.editor
            .as_ref()
            .map_or_else(|| self.content.trim().is_empty(), |e| e.document.is_empty())
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
        self.selected_range = self.content.len()..self.content.len();
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
                    || (matches!(command, Command::Improve(_)) && "ai".contains(&query))
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
        let mut range = self.selected_range.clone();
        if all_if_empty && range.is_empty() {
            range = 0..self.content.len();
            self.selected_range = range.clone();
        }
        if !range.is_empty() || format.mark().is_none() {
            self.remember_edit();
        }
        let e = self.editor.as_mut().unwrap();
        if let Some(name) = format.mark() {
            if range.is_empty() {
                let mut marks = e
                    .marks
                    .clone()
                    .unwrap_or_else(|| e.document.marks_at(range.start));
                if name != "code" && marks.iter().any(|m| m.name == "code") {
                    return;
                }
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
            if set && format == Format::CodeBlock {
                self.selected_range = e.document.code_block(range);
            } else {
                e.document.format(range, format);
            }
            e.marks = None;
        }
        self.content = e.document.text().into();
        self.last_layout = None;
        window.notify_text_input_changed(&self.focus_handle);
        cx.notify();
    }
    fn select_slash(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let before = self.snapshot();
        let history_len = self.undo.len();
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
                let e = self.editor.as_mut().unwrap();
                e.document.append_code_block();
                e.marks = None;
                self.content = e.document.text().into();
                self.selected_range = self.content.len()..self.content.len();
                cx.notify();
            }
            Command::Format(format) => self.apply_format(
                format,
                matches!(format, Format::Bold | Format::Italic | Format::Underline),
                false,
                window,
                cx,
            ),
            Command::Improve(command) => {
                self.selected_range = 0..self.content.len();
                cx.emit(EditorRequest::Improve {
                    command,
                    html: self.html(),
                });
            }
            Command::Video => self.open_editor_dialog(DialogKind::Video, window, cx),
            Command::Emoji => self.open_editor_dialog(DialogKind::Emoji, window, cx),
        }
        self.undo.truncate(history_len);
        self.undo.push(before);
        self.redo.clear();
    }
    pub(super) fn editor_navigate(&mut self, direction: isize, cx: &mut Context<Self>) -> bool {
        let count = self.slash_items().len();
        if count == 0 {
            return false;
        }
        let e = self.editor.as_mut().unwrap();
        e.choice = (e.choice as isize + direction).rem_euclid(count as isize) as usize;
        e.scroll.scroll_to_item(e.choice);
        cx.notify();
        true
    }
    pub(super) fn editor_enter(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.editor.is_none() {
            return false;
        }
        if let Some(dialog) = self.editor.as_ref().and_then(|e| e.dialog.clone()) {
            let emoji = if dialog.kind == DialogKind::Emoji {
                crate::emoji::emojis(dialog.input.read(cx).content.as_ref())
                    .first()
                    .and_then(|e| e["emoji"].as_str())
                    .map(str::to_owned)
            } else {
                None
            };
            self.finish_editor_dialog(emoji, window, cx);
            return true;
        }

        if !self.slash_items().is_empty() {
            let items = self.slash_items();
            let choice = self.editor.as_ref().unwrap().choice.min(items.len() - 1);
            self.select_slash(items[choice], window, cx);
            return true;
        }
        self.remember_edit();
        let e = self.editor.as_mut().unwrap();
        let at = e.document.split(self.selected_range.clone());
        self.content = e.document.text().into();
        let at = at.min(self.content.len());
        self.selected_range = at..at;
        e.marks = None;
        e.slash = None;
        window.notify_text_input_changed(&self.focus_handle);
        cx.notify();
        true
    }
    fn highlight_color(&mut self, color: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.remember_edit();
        let range = self.selected_range.clone();
        let e = self.editor.as_mut().unwrap();
        let current = e.document.has_mark(range.clone(), "mark")
            && e.document.marks_at(range.start).iter().any(|m| {
                m.name == "mark" && m.attrs.iter().any(|(k, v)| k == "data-color" && v == color)
            });
        e.document.set_mark(
            range,
            Tag {
                name: "mark".into(),
                attrs: vec![
                    ("data-color".into(), color.into()),
                    (
                        "style".into(),
                        format!("background-color: {color}; color: inherit"),
                    ),
                ],
            },
            !current,
        );
        window.notify_text_input_changed(&self.focus_handle);
        cx.notify();
    }
    pub(super) fn paste_link(
        &mut self,
        text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.editor.is_none() || self.selected_range.is_empty() {
            return false;
        }
        let Some(href) = autolink_url(text) else {
            return false;
        };
        self.remember_edit();
        self.editor
            .as_mut()
            .unwrap()
            .document
            .link(self.selected_range.clone(), &href);
        window.notify_text_input_changed(&self.focus_handle);
        cx.notify();
        true
    }
    pub(super) fn autolink(&mut self, inserted: &str) {
        let Some(e) = &mut self.editor else {
            return;
        };
        if !inserted.chars().any(char::is_whitespace) && !inserted.contains("://") {
            return;
        }
        let mut at = 0;
        for word in self.content.split_inclusive(char::is_whitespace) {
            let trimmed = word.trim_end().trim_end_matches([',', '.', '!', ';', ':']);
            if let Some(href) = autolink_url(trimmed) {
                if !e.document.marks_at(at).iter().any(|m| m.name == "code") {
                    e.document.link(at..at + trimmed.len(), &href);
                }
            }
            at += word.len();
        }
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
                let range = e.document.link_range(dialog.selection.clone());
                if !e.document.link(range.clone(), &href) {
                    return;
                }
                e.document.set_mark(
                    range,
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
                self.remember_edit();
                let e = self.editor.as_mut().unwrap();
                let at = e.document.insert_html(self.selected_range.clone(), &html);
                self.content = e.document.text().into();
                self.selected_range = at..at;
                window.notify_text_input_changed(&self.focus_handle);
            }
            DialogKind::Emoji => {
                let Some(emoji) = emoji else {
                    return;
                };
                let start = self.selected_range.start;
                self.replace_text_in_range(None, &emoji, window, cx);
                if let Some(item) = crate::emoji::emojis("")
                    .into_iter()
                    .find(|e| e["emoji"].as_str() == Some(&emoji))
                {
                    let name = item["id"].as_str().unwrap();
                    self.editor.as_mut().unwrap().document.set_mark(
                        start..start + emoji.len(),
                        Tag {
                            name: "span".into(),
                            attrs: vec![
                                ("data-name".into(), name.into()),
                                ("data-type".into(), "emoji".into()),
                            ],
                        },
                        true,
                    );
                }
            }
        }
        self.close_editor_dialog(window, cx);
    }
    pub(super) fn editor_click(
        &mut self,
        event: &gpui::ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.selected_range.is_empty() {
            return;
        }
        let offset = self.index_for_mouse_position(event.position());
        if let Some(url) = self
            .editor
            .as_ref()
            .and_then(|e| e.document.link_at(offset))
        {
            let url = if url.starts_with('/') {
                format!("{}{url}", ht_core::auth::SITE_URL)
            } else {
                url
            };
            cx.open_url(&url);
            cx.stop_propagation();
        }
        let _ = window;
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
    pub(super) fn editor_menu(&self, window: &Window, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let Some(e) = &self.editor else {
            return vec![];
        };
        if !self.focus_handle.is_focused(window)
            && !e
                .dialog
                .as_ref()
                .is_some_and(|d| d.input.focus_handle(cx).is_focused(window))
        {
            return vec![];
        }
        let theme = crate::theme::THEMES[if window.text_style().color.l > 0.5 {
            1
        } else {
            0
        }];
        let top = self.last_layout.as_ref().map_or(px(0.), |l| {
            let row = l.row(self.cursor_offset().min(l.text.len()));
            l.rows[row].top + l.rows[row].height
        });
        let top = if self
            .last_bounds
            .is_some_and(|b| b.top() + top + px(320.) > window.viewport_size().height)
        {
            (top - px(320.)).max(px(0.))
        } else {
            top
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
                .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                    this.editor.as_mut().unwrap().dialog = None;
                    cx.notify();
                }))
                .p(px(12.))
                .bg(crate::theme::color(theme.comment))
                .text_color(crate::theme::color(theme.text))
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
                        .child(if dialog.kind == DialogKind::Link {
                            format!("Link to \"{}\"", dialog.input.read(cx).content)
                        } else {
                            "OK".into()
                        })
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, window, cx| {
                                cx.stop_propagation();
                                this.finish_editor_dialog(None, window, cx);
                            }),
                        ),
                );
            }
            if dialog.kind == DialogKind::Video {
                body = body.child(
                    div()
                        .id("cancel-video")
                        .cursor_pointer()
                        .child("Cancel")
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, window, cx| {
                                cx.stop_propagation();
                                this.close_editor_dialog(window, cx);
                            }),
                        ),
                );
            }
            return vec![
                body.absolute()
                    .left_0()
                    .top(top)
                    .w(px(360.))
                    .shadow_md()
                    .into_any_element(),
            ];
        }
        let items = self.slash_items();
        if !items.is_empty() {
            let list = div()
                .id("editor-slash-menu")
                .max_h(px(288.))
                .overflow_y_scroll()
                .track_scroll(&e.scroll)
                .p(px(8.))
                .bg(crate::theme::color(theme.comment))
                .text_color(crate::theme::color(theme.text))
                .flex()
                .flex_col()
                .children(items.iter().enumerate().map(|(choice, &index)| {
                    let (title, description, _) = COMMANDS[index];
                    div()
                        .id(("slash-choice", index))
                        .debug_selector(|| format!("slash-{title}"))
                        .p(px(6.))
                        .cursor_pointer()
                        .when(choice == e.choice, |d| {
                            d.bg(crate::theme::color(theme.hover))
                        })
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
            out.push(
                list.absolute()
                    .left_0()
                    .top(top)
                    .w(px(360.))
                    .shadow_md()
                    .into_any_element(),
            );
        } else if !self.selected_range.is_empty() {
            if e.palette {
                let palette = div()
                    .id("editor-highlight-colors")
                    .absolute()
                    .left_0()
                    .top(top)
                    .flex()
                    .gap(px(8.))
                    .p(px(8.))
                    .bg(crate::theme::color(theme.comment))
                    .text_color(crate::theme::color(theme.text))
                    .child(
                        div()
                            .id("highlight-back")
                            .cursor_pointer()
                            .child("Back")
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| {
                                    cx.stop_propagation();
                                    this.editor.as_mut().unwrap().palette = false;
                                    cx.notify();
                                }),
                            ),
                    )
                    .children(
                        ["#ffc078", "#8ce99a", "#74c0fc", "#b197fc", "#ff0000"]
                            .into_iter()
                            .enumerate()
                            .map(|(i, color)| {
                                let rgb = u32::from_str_radix(&color[1..], 16).unwrap();
                                div()
                                    .id(("highlight-color", i))
                                    .size(px(24.))
                                    .rounded(px(4.))
                                    .bg(crate::theme::color(rgb))
                                    .cursor_pointer()
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(move |this, _, window, cx| {
                                            cx.stop_propagation();
                                            this.highlight_color(color, window, cx);
                                        }),
                                    )
                            }),
                    );
                return vec![palette.into_any_element()];
            }
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
                        .debug_selector(|| format!("format-{i}"))
                        .cursor_pointer()
                        .child(label)
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |this, _, window, cx| {
                                cx.stop_propagation();
                                if format == Format::Highlight {
                                    this.editor.as_mut().unwrap().palette = true;
                                    cx.notify();
                                    return;
                                }
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
                        .debug_selector(|| "editor-link".into())
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
            out.push(
                toolbar
                    .absolute()
                    .left_0()
                    .top(top)
                    .max_w(px(560.))
                    .p(px(8.))
                    .bg(crate::theme::color(theme.comment))
                    .text_color(crate::theme::color(theme.text))
                    .shadow_md()
                    .into_any_element(),
            );
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
                    if matches!(b.tag.name.as_str(), "h1" | "h2") {
                        run.font.weight = FontWeight::SEMIBOLD;
                    }
                    if b.tag.name == "h3" {
                        run.font.weight = FontWeight::MEDIUM;
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
                        if b.tag.name == "pre" {
                            run.color = crate::theme::color(if theme.page == 0xffffff {
                                0xd63384
                            } else {
                                0xe8912d
                            });
                        }
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

fn autolink_url(text: &str) -> Option<String> {
    if text
        .chars()
        .any(|c| c.is_whitespace() || c.is_control() || matches!(c, '<' | '>' | '"'))
    {
        return None;
    }
    let href = if text.starts_with("http://") || text.starts_with("https://") {
        text.to_owned()
    } else if text.starts_with("www.") {
        format!("http://{text}")
    } else {
        return None;
    };
    let host = href.split("://").nth(1)?.split(['/', '?', '#']).next()?;
    if host.is_empty() || host.starts_with('.') || host.ends_with('.') || host.contains('@') {
        return None;
    }
    Some(href)
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
        "<div class=\"video-wrapper\"><iframe allowfullscreen=\"true\" src=\"{}\" start=\"0\"></iframe></div>",
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
        view.read_with(cx, |input, _| {
            assert_eq!(
                input.html(),
                "<p><span data-name=\"rocket\" data-type=\"emoji\">🚀</span></p>"
            )
        });
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
    #[gpui::test]
    fn links_autolink_paste_selection_palette_and_dialog_are_rich_transactions(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(init);
        let (view, cx) = cx.add_window_view(|_, cx| {
            TextInput::new(String::new(), "", gpui::black(), cx).comment_editor()
        });
        view.update_in(cx, |input, window, cx| {
            window.focus(&input.focus_handle, cx)
        });
        cx.simulate_input("https://example.com ");
        view.read_with(cx, |input, _| {
            assert!(input.html().contains("href=\"https://example.com\""))
        });
        view.update_in(cx, |input, window, cx| {
            input.reset(window, cx);
            input.replace_text_in_range(None, "selected", window, cx);
            input.selected_range = 0..8;
            assert!(input.paste_link("https://example.org", window, cx));
            assert_eq!(input.content.as_ref(), "selected");
            input.highlight_color("#74c0fc", window, cx);
            assert!(input.html().contains("data-color=\"#74c0fc\""));
            input.open_editor_dialog(DialogKind::Link, window, cx);
        });
        cx.simulate_input("example.net");
        cx.simulate_keystrokes("enter");
        view.read_with(cx, |input, _| {
            assert!(input.html().contains("href=\"http://example.net\""));
            assert!(!input.html().contains("<mark"));
            assert_eq!(Document::parse(&input.html()).html(), input.html());
        });
        cx.simulate_keystrokes("ctrl-z");
        view.read_with(cx, |input, _| {
            assert!(input.html().contains("data-color=\"#74c0fc\""))
        });
    }
    #[gpui::test]
    fn rich_editor_geometry_uses_heading_sizes_list_gutters_and_document_offsets(
        cx: &mut gpui::TestAppContext,
    ) {
        let (view, cx) = cx.add_window_view(|_, cx| {
            TextInput::new(String::new(), "", gpui::black(), cx).multiline_editor()
        });
        view.update_in(cx, |input, window, cx| {
            input.set_html(
                "<h1>Title é🙂</h1><ol><li><p>First</p></li><li><p>Second</p></li></ol>",
                cx,
            );
            window.focus(&input.focus_handle, cx);
        });
        cx.run_until_parked();
        view.update_in(cx, |input, window, cx| {
            let l = input.last_layout.as_ref().unwrap();
            assert_eq!(l.rows[0].font_size, px(24.));
            assert_eq!(l.rows[1].prefix, "1.");
            assert_eq!(l.rows[2].prefix, "2.");
            assert!(l.rows[1].indent > px(0.));
            let start = input.content.find("First").unwrap();
            let bounds = input.last_bounds.unwrap();
            let range = input.range_to_utf16(&(start..start + 5));
            let rect = input.bounds_for_range(range, bounds, window, cx).unwrap();
            assert!(rect.left() > bounds.left());
            assert!(rect.top() > bounds.top());
        });
    }
    #[gpui::test]
    fn slash_and_bubble_formatting_work_with_real_pointer_events(cx: &mut gpui::TestAppContext) {
        cx.update(init);
        let (view, cx) = cx.add_window_view(|_, cx| {
            TextInput::new(String::new(), "", gpui::black(), cx).comment_editor()
        });
        view.update_in(cx, |input, window, cx| {
            window.focus(&input.focus_handle, cx)
        });
        cx.simulate_input("/Bold");
        cx.run_until_parked();
        let bounds = cx
            .debug_bounds("slash-Bold")
            .expect("real slash choice was painted");
        cx.simulate_click(bounds.center(), gpui::Modifiers::default());
        cx.simulate_input("bold");
        view.read_with(cx, |input, _| {
            assert_eq!(input.html(), "<p><strong>bold</strong></p>")
        });
        cx.simulate_keystrokes("ctrl-a");
        cx.run_until_parked();
        let bounds = cx
            .debug_bounds("format-1")
            .expect("selection bubble painted");
        cx.simulate_click(bounds.center(), gpui::Modifiers::default());
        view.read_with(cx, |input, _| {
            assert_eq!(input.html(), "<p><strong><em>bold</em></strong></p>")
        });
    }
    #[gpui::test]
    fn unsupported_web_services_are_not_inert_slash_commands_and_video_html_is_real(
        cx: &mut gpui::TestAppContext,
    ) {
        for title in ["File Attachments", "GIFs", "Add Image"] {
            assert!(!COMMANDS.iter().any(|(t, _, _)| *t == title));
        }
        cx.update(init);
        let (view, cx) = cx.add_window_view(|_, cx| {
            TextInput::new(String::new(), "", gpui::black(), cx).multiline_editor()
        });
        view.update_in(cx, |input, window, cx| {
            window.focus(&input.focus_handle, cx);
            input.replace_text_in_range(None, "/emoji", window, cx);
            assert!(
                input.slash_items().is_empty(),
                "emoji is create-comment only, not description/comment-edit"
            );
            input.reset(window, cx);
        });
        cx.simulate_keystrokes("ctrl-y");
        cx.simulate_input("https://youtu.be/abcd");
        cx.simulate_keystrokes("enter");
        view.read_with(cx, |input, _| {
            assert!(
                input
                    .html()
                    .contains("<div class=\"video-wrapper\"><iframe")
            );
            assert!(!input.is_empty(), "an atom-only document can be saved");
            assert_eq!(Document::parse(&input.html()).html(), input.html());
        });
    }
    #[gpui::test]
    fn slash_format_is_one_undo_transaction_and_history_never_reopens_a_dialog(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(init);
        let (view, cx) = cx.add_window_view(|_, cx| {
            TextInput::new(String::new(), "", gpui::black(), cx).comment_editor()
        });
        view.update_in(cx, |input, window, cx| {
            window.focus(&input.focus_handle, cx)
        });
        cx.simulate_input("/Italic");
        cx.simulate_keystrokes("enter ctrl-z");
        view.read_with(cx, |input, _| assert_eq!(input.content.as_ref(), "/Italic"));
        cx.simulate_keystrokes("ctrl-shift-z");
        cx.simulate_input("text");
        view.read_with(cx, |input, _| {
            assert_eq!(input.html(), "<p><em>text</em></p>")
        });
        view.update_in(cx, |input, window, cx| {
            input.selected_range = 0..input.content.len();
            input.open_editor_dialog(DialogKind::Link, window, cx);
        });
        cx.simulate_input("example.com");
        cx.simulate_keystrokes("enter ctrl-z");
        view.read_with(cx, |input, _| {
            assert!(input.editor.as_ref().unwrap().dialog.is_none());
            assert!(!input.html().contains("<a "));
        });
    }
    #[gpui::test]
    fn clicking_a_link_uses_web_open_on_click_without_shell_shortcuts(
        cx: &mut gpui::TestAppContext,
    ) {
        let (view, cx) = cx.add_window_view(|_, cx| {
            TextInput::new(String::new(), "", gpui::black(), cx).comment_editor()
        });
        view.update_in(cx, |input, window, cx| {
            input.set_html("<p><a href=\"https://example.com\">link</a></p>", cx);
            input.move_to(0, window, cx);
        });
        cx.run_until_parked();
        let point = view.read_with(cx, |input, _| {
            let bounds = input.last_bounds.unwrap();
            point(bounds.left() + px(3.), bounds.top() + px(8.))
        });
        cx.simulate_click(point, gpui::Modifiers::default());
        assert_eq!(cx.opened_url().as_deref(), Some("https://example.com"));
    }
}
