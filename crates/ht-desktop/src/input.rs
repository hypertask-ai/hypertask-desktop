use std::ops::Range;

#[path = "editor.rs"]
mod editor;
use editor::EditorState;
#[path = "editor_drag.rs"]
mod editor_drag;
#[path = "editor_layout.rs"]
mod editor_layout;
use editor_layout::{EditorRow, editor_rows};

use gpui::{
    App, AvailableSpace, Bounds, ClipboardItem, Context, CursorStyle, ElementId,
    ElementInputHandler, Entity, EntityInputHandler, FocusHandle, Focusable, GlobalElementId,
    IndentAdjustment, KeyBinding, LayoutId, LineFragment, MouseButton, MouseDownEvent,
    MouseMoveEvent, MouseUpEvent, PaintQuad, Pixels, Point, ShapedLine, SharedString, Style,
    TextRun, UTF16Selection, UnderlineStyle, Window, actions, div, fill, point, prelude::*, px,
    relative, rgba, size,
};

use unicode_segmentation::*;

#[derive(Clone)]
pub enum EditorRequest {
    Improve {
        command: ht_core::ImproveCommand,
        html: String,
    },
}
impl gpui::EventEmitter<EditorRequest> for TextInput {}
#[derive(Clone)]
pub struct QuoteSelection { pub html: String }
impl gpui::EventEmitter<QuoteSelection> for TextInput {}
impl TextInput {
    pub fn read_only_comment(mut self, html: &str, cx: &mut Context<Self>) -> Self {
        self=self.multiline_editor();self.set_html(html,cx);self.read_only=true;self
    }
    pub fn has_selection(&self) -> bool { !self.selected_range.is_empty() }
    pub fn slash_token(&self) -> Option<(Range<usize>, String)> {
        if !self.selected_range.is_empty() { return None; }
        let before = &self.content[..self.selected_range.end];
        let start = before.rfind('/')?;
        let query = &before[start+1..];
        if query.chars().any(char::is_whitespace) { return None; }
        Some((start..self.selected_range.end, query.to_owned()))
    }
    pub fn replace_token(&mut self, range: Range<usize>, text: &str, window: &mut Window, cx: &mut Context<Self>) {
        let range = self.range_to_utf16(&range);
        self.replace_text_in_range(Some(range), text, window, cx);
    }
    fn quote_selection(&mut self, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {return;}
        if let Some(editor)=&self.editor {
            cx.emit(QuoteSelection{html:editor.document.selection_html(self.selected_range.clone())});
        }
    }
}


actions!(
    text_input,
    [
        Backspace,
        Delete,
        Left,
        Right,
        Up,
        Down,
        SelectLeft,
        SelectRight,
        SelectAll,
        Home,
        End,
        ShowCharacterPalette,
        Paste,
        PastePlain,
        Cut,
        Copy,
        Newline,
        HardBreak,
        Undo,
        Redo,
    ]
);

#[derive(Clone)]
struct EditSnapshot {
    content: SharedString,
    selection: Range<usize>,
    reversed: bool,
    editor: Option<EditorState>,
}

pub struct TextInput {
    focus_handle: FocusHandle,
    pub content: SharedString,
    placeholder: SharedString,
    selected_range: Range<usize>,
    selection_reversed: bool,
    marked_range: Option<Range<usize>>,
    last_layout: Option<TextLayout>,
    multiline: bool,
    editor_newlines: bool,
    last_bounds: Option<Bounds<Pixels>>,
    is_selecting: bool,
    muted: gpui::Hsla,
    undo: Vec<EditSnapshot>,
    redo: Vec<EditSnapshot>,
    editor: Option<EditorState>,
    read_only: bool,
}

impl TextInput {
    fn snapshot(&self) -> EditSnapshot {
        EditSnapshot {
            content: self.content.clone(),
            selection: self.selected_range.clone(),
            reversed: self.selection_reversed,
            editor: self.editor.as_ref().map(EditorState::history_state),
        }
    }

    fn remember_edit(&mut self) {
        if self.marked_range.is_none() {
            self.undo.push(self.snapshot());
            self.redo.clear();
        }
    }

    fn restore_edit(&mut self, redo: bool, window: &mut Window, cx: &mut Context<Self>) {
        let snapshot = if redo {
            self.redo.pop()
        } else {
            self.undo.pop()
        };
        if self.read_only {return;}
        if let Some(snapshot) = snapshot {
            let current = self.snapshot();
            if redo {
                self.undo.push(current);
            } else {
                self.redo.push(current);
            }
            self.content = snapshot.content;
            self.editor = snapshot.editor;
            self.selected_range = snapshot.selection;
            self.selection_reversed = snapshot.reversed;
            self.marked_range = None;
            self.last_layout = None;
            window.notify_text_input_changed(&self.focus_handle);
            cx.notify();
        }
    }

    fn newline(&mut self, _: &Newline, window: &mut Window, cx: &mut Context<Self>) {
        if self.editor_enter(window, cx) {
            return;
        }
        if self.multiline {
            self.insert_newline(window, cx);
        } else {
            cx.propagate();
        }
    }

    fn left(&mut self, _: &Left, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.move_to(self.previous_boundary(self.cursor_offset()), window, cx);
        } else {
            self.move_to(self.selected_range.start, window, cx)
        }
    }

    fn right(&mut self, _: &Right, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.move_to(self.next_boundary(self.selected_range.end), window, cx);
        } else {
            self.move_to(self.selected_range.end, window, cx)
        }
    }

    fn vertical(&mut self, direction: isize, window: &mut Window, cx: &mut Context<Self>) {
        if self.editor_navigate(direction, cx) {
            return;
        }
        if !self.multiline {
            cx.propagate();
            return;
        }
        if self.content.is_empty() {
            return;
        }
        if let Some(layout) = &self.last_layout {
            if layout.text != self.content {
                return;
            }
            let row = layout.row(self.cursor_offset());
            let target = row
                .saturating_add_signed(direction)
                .min(layout.lines.len() - 1);
            let x = layout.rows[row].indent
                + layout.lines[row].x_for_index(self.cursor_offset() - layout.offsets[row])
                - layout.rows[target].indent;
            let index = layout.offsets[target] + layout.lines[target].closest_index_for_x(x);
            self.move_to(index, window, cx);
        }
    }

    fn select_left(&mut self, _: &SelectLeft, window: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.previous_boundary(self.cursor_offset()), window, cx);
    }

    fn select_right(&mut self, _: &SelectRight, window: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.next_boundary(self.cursor_offset()), window, cx);
    }

    fn select_all(&mut self, _: &SelectAll, window: &mut Window, cx: &mut Context<Self>) {
        self.move_to(0, window, cx);
        self.select_to(self.content.len(), window, cx)
    }

    fn home(&mut self, _: &Home, window: &mut Window, cx: &mut Context<Self>) {
        self.move_to(0, window, cx);
    }

    fn end(&mut self, _: &End, window: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.content.len(), window, cx);
    }

    fn backspace(&mut self, _: &Backspace, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            let prev = self.previous_boundary(self.cursor_offset());
            if self.cursor_offset() == prev {
                window.play_system_bell();
                return;
            }
            self.select_to(prev, window, cx)
        }
        self.replace_text_in_range(None, "", window, cx)
    }

    fn delete(&mut self, _: &Delete, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            let next = self.next_boundary(self.cursor_offset());
            if self.cursor_offset() == next {
                window.play_system_bell();
                return;
            }
            self.select_to(next, window, cx)
        }
        self.replace_text_in_range(None, "", window, cx)
    }

    fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let (Some(layout), Some(bounds)) = (&self.last_layout, self.last_bounds) {
            let row = layout.row_for_y(event.position.y - bounds.top());
            if let Some(block) = layout.rows[row].task.filter(|_|!self.read_only) {
                let x = event.position.x - bounds.left();
                if x >= layout.rows[row].indent - px(24.) && x < layout.rows[row].indent {
                    self.remember_edit();
                    self.editor.as_mut().unwrap().document.toggle_task(block);
                    cx.notify();
                    return;
                }
            }
        }
        self.is_selecting = true;
        if !self.read_only {window.request_virtual_keyboard();}

        if event.modifiers.shift {
            self.select_to(self.index_for_mouse_position(event.position), window, cx);
        } else {
            self.move_to(self.index_for_mouse_position(event.position), window, cx)
        }
    }

    fn on_mouse_up(&mut self, event: &MouseUpEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.finish_block_drag(event, window, cx);
        self.is_selecting = false;
    }

    fn on_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.is_selecting {
            self.select_to(self.index_for_mouse_position(event.position), window, cx);
        }
    }

    fn show_character_palette(
        &mut self,
        _: &ShowCharacterPalette,
        window: &mut Window,
        _: &mut Context<Self>,
    ) {
        window.show_character_palette();
    }

    fn paste(&mut self, _: &Paste, window: &mut Window, cx: &mut Context<Self>) {
        if self.read_only {return;}
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            if self.paste_link(&text, window, cx) {
                return;
            }
            let text = if self.multiline {
                text.replace("\r\n", "\n")
            } else {
                text.replace("\n", " ")
            };
            self.replace_text_in_range(None, &text, window, cx);
        }
    }

    fn paste_plain(&mut self, _: &PastePlain, window: &mut Window, cx: &mut Context<Self>) {
        if self.read_only { return; }
        if let Some(text)=cx.read_from_clipboard().and_then(|item|item.text()) {
            let text=text.replace("\r\n","\n");
            let text=if self.multiline {text}else{text.replace('\n'," ")};
            self.replace_text_in_range(None,&text,window,cx);
        }
    }
    fn copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.content[self.selected_range.clone()].to_string(),
            ));
        }
    }
    fn cut(&mut self, _: &Cut, window: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.content[self.selected_range.clone()].to_string(),
            ));
            self.replace_text_in_range(None, "", window, cx)
        }
    }

    fn move_to(&mut self, offset: usize, window: &Window, cx: &mut Context<Self>) {
        if let Some(e) = &mut self.editor {
            e.clear_marks();
        }
        self.selected_range = offset..offset;
        self.selection_reversed = false;
        window.notify_text_selection_changed(&self.focus_handle);
        cx.notify()
    }

    fn cursor_offset(&self) -> usize {
        if self.selection_reversed {
            self.selected_range.start
        } else {
            self.selected_range.end
        }
    }

    fn index_for_mouse_position(&self, position: Point<Pixels>) -> usize {
        if self.content.is_empty() {
            return 0;
        }

        let (Some(bounds), Some(line)) = (self.last_bounds.as_ref(), self.last_layout.as_ref())
        else {
            return 0;
        };
        if position.y < bounds.top() {
            return 0;
        }
        if position.y > bounds.bottom() {
            return self.content.len();
        }
        let row = line.row_for_y(position.y - bounds.top());
        line.offsets[row]
            + line.lines[row]
                .closest_index_for_x(position.x - bounds.left() - line.rows[row].indent)
    }

    fn select_to(&mut self, offset: usize, window: &Window, cx: &mut Context<Self>) {
        if let Some(e) = &mut self.editor {
            e.clear_marks();
        }
        if self.selection_reversed {
            self.selected_range.start = offset
        } else {
            self.selected_range.end = offset
        };
        if self.selected_range.end < self.selected_range.start {
            self.selection_reversed = !self.selection_reversed;
            self.selected_range = self.selected_range.end..self.selected_range.start;
        }
        window.notify_text_selection_changed(&self.focus_handle);
        cx.notify()
    }

    fn offset_from_utf16(&self, offset: usize) -> usize {
        let mut utf8_offset = 0;
        let mut utf16_count = 0;

        for ch in self.content.chars() {
            if utf16_count >= offset {
                break;
            }
            utf16_count += ch.len_utf16();
            utf8_offset += ch.len_utf8();
        }

        utf8_offset
    }

    fn offset_to_utf16(&self, offset: usize) -> usize {
        let mut utf16_offset = 0;
        let mut utf8_count = 0;

        for ch in self.content.chars() {
            if utf8_count >= offset {
                break;
            }
            utf8_count += ch.len_utf8();
            utf16_offset += ch.len_utf16();
        }

        utf16_offset
    }

    fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.offset_to_utf16(range.start)..self.offset_to_utf16(range.end)
    }

    fn range_from_utf16(&self, range_utf16: &Range<usize>) -> Range<usize> {
        self.offset_from_utf16(range_utf16.start)..self.offset_from_utf16(range_utf16.end)
    }

    fn previous_boundary(&self, offset: usize) -> usize {
        self.content
            .grapheme_indices(true)
            .rev()
            .find_map(|(idx, _)| (idx < offset).then_some(idx))
            .unwrap_or(0)
    }

    fn next_boundary(&self, offset: usize) -> usize {
        self.content
            .grapheme_indices(true)
            .find_map(|(idx, _)| (idx > offset).then_some(idx))
            .unwrap_or(self.content.len())
    }

    pub fn reset(&mut self, window: &Window, cx: &mut Context<Self>) {
        self.undo.clear();
        self.redo.clear();
        self.content = "".into();
        if let Some(e) = &mut self.editor {
            let comment = e.comment;
            *e = EditorState::new("");
            e.comment = comment;
        }
        self.selected_range = 0..0;
        self.selection_reversed = false;
        self.marked_range = None;
        self.last_layout = None;
        self.last_bounds = None;
        self.is_selecting = false;
        window.notify_text_input_changed(&self.focus_handle);
        cx.notify();
    }
}

impl EntityInputHandler for TextInput {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let range = self.range_from_utf16(&range_utf16);
        actual_range.replace(self.range_to_utf16(&range));
        Some(self.content[range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.range_to_utf16(&self.selected_range),
            reversed: self.selection_reversed,
        })
    }

    fn set_selected_text_range(
        &mut self,
        range_utf16: Range<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.selected_range = self.range_from_utf16(&range_utf16);
        self.selection_reversed = false;
        window.notify_text_selection_changed(&self.focus_handle);
        cx.notify();
    }

    fn text_length_utf16(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        Some(self.content.encode_utf16().count())
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        self.marked_range
            .as_ref()
            .map(|range| self.range_to_utf16(range))
    }

    fn unmark_text(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.marked_range = None;
        window.notify_text_input_changed(&self.focus_handle);
        cx.notify();
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.read_only {return;}
        let range = range_utf16
            .as_ref()
            .map(|range_utf16| self.range_from_utf16(range_utf16))
            .or(self.marked_range.clone())
            .unwrap_or(self.selected_range.clone());

        self.remember_edit();
        self.replace_document(range.clone(), new_text);
        self.content =
            (self.content[0..range.start].to_owned() + new_text + &self.content[range.end..])
                .into();
        self.selected_range = range.start + new_text.len()..range.start + new_text.len();
        self.selection_reversed = false;
        self.marked_range.take();
        self.update_slash(range.start, new_text);
        self.autolink(new_text);
        window.notify_text_input_changed(&self.focus_handle);
        cx.notify();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.read_only {return;}
        let range = range_utf16
            .as_ref()
            .map(|range_utf16| self.range_from_utf16(range_utf16))
            .or(self.marked_range.clone())
            .unwrap_or(self.selected_range.clone());

        self.remember_edit();
        self.replace_document(range.clone(), new_text);
        self.content =
            (self.content[0..range.start].to_owned() + new_text + &self.content[range.end..])
                .into();
        if !new_text.is_empty() {
            self.marked_range = Some(range.start..range.start + new_text.len());
        } else {
            self.marked_range = None;
        }
        let insertion_start_utf16 = self.offset_to_utf16(range.start);
        let inserted_length_utf16 = new_text.encode_utf16().count();
        self.selected_range = new_selected_range_utf16
            .map(|selected| {
                self.range_from_utf16(
                    &(insertion_start_utf16 + selected.start.min(inserted_length_utf16)
                        ..insertion_start_utf16 + selected.end.min(inserted_length_utf16)),
                )
            })
            .unwrap_or_else(|| range.start + new_text.len()..range.start + new_text.len());
        self.selection_reversed = false;

        window.notify_text_input_changed(&self.focus_handle);
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let last_layout = self.last_layout.as_ref()?;
        if last_layout.text != self.content {
            return None;
        }
        let range = self.range_from_utf16(&range_utf16);
        let row = last_layout.row(range.start);
        let line = &last_layout.lines[row];
        let offset = last_layout.offsets[row];
        let top = bounds.top() + last_layout.rows[row].top;
        let left = bounds.left() + last_layout.rows[row].indent;
        Some(Bounds::from_corners(
            point(left + line.x_for_index(range.start - offset), top),
            point(
                left + line.x_for_index((range.end - offset).min(line.text.len())),
                top + last_layout.rows[row].height,
            ),
        ))
    }

    fn selection_bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        bounds: Bounds<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<Bounds<Pixels>> {
        self.bounds_for_range(range_utf16, bounds, window, cx)
            .into_iter()
            .collect()
    }

    fn character_index_for_point(
        &mut self,
        point: gpui::Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        let line_point = self.last_bounds?.localize(&point)?;
        let last_layout = self.last_layout.as_ref()?;

        if last_layout.text != self.content {
            return None;
        }
        let row = last_layout.row_for_y(line_point.y);
        let utf8_index = last_layout.offsets[row]
            + last_layout.lines[row]
                .closest_index_for_x(line_point.x - last_layout.rows[row].indent);
        Some(self.offset_to_utf16(utf8_index))
    }
}

fn run_for_editor(style: &gpui::TextStyle, color: gpui::Hsla) -> TextRun {
    TextRun {
        len: 0,
        font: style.font(),
        color,
        background_color: None,
        underline: None,
        strikethrough: None,
    }
}

struct TextLayout {
    text: SharedString,
    lines: Vec<ShapedLine>,
    offsets: Vec<usize>,
    rows: Vec<EditorRow>,
}

impl TextLayout {
    fn row_for_y(&self, y: Pixels) -> usize {
        self.rows
            .partition_point(|row| row.top <= y)
            .saturating_sub(1)
            .min(self.lines.len() - 1)
    }
    fn row(&self, index: usize) -> usize {
        self.offsets
            .partition_point(|offset| *offset <= index)
            .saturating_sub(1)
    }
}

fn line_ranges(
    text: &str,
    width: Pixels,
    multiline: bool,
    font: gpui::Font,
    font_size: Pixels,
    window: &Window,
) -> Vec<Range<usize>> {
    if !multiline {
        return vec![0..text.len()];
    }
    let mut wrapper = window.text_system().line_wrapper(font, font_size);
    let mut ranges = vec![];
    let mut offset = 0;
    for line in text.split('\n') {
        let fragments = [LineFragment::text(line)];
        let mut start = 0;
        for boundary in wrapper.wrap_line(&fragments, width.max(px(1.)), IndentAdjustment::NoIndent)
        {
            ranges.push(offset + start..offset + boundary.ix);
            start = boundary.ix;
        }
        ranges.push(offset + start..offset + line.len());
        offset += line.len() + 1;
    }
    ranges
}

struct TextElement {
    input: Entity<TextInput>,
}

struct PrepaintState {
    layout: TextLayout,
    cursor: Option<PaintQuad>,
    selections: Vec<PaintQuad>,
    decorations: Vec<PaintQuad>,
    prefixes: Vec<(Point<Pixels>, ShapedLine, Pixels)>,
}

impl IntoElement for TextElement {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl Element for TextElement {
    type RequestLayoutState = ();
    type PrepaintState = PrepaintState;

    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        let input = self.input.read(cx);
        let multiline = input.multiline;
        let document = if input.content.is_empty() {
            None
        } else {
            input.editor.as_ref().map(|e| e.document.clone())
        };
        let text = if input.content.is_empty() {
            input.placeholder.clone()
        } else {
            input.content.clone()
        };
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        if !multiline {
            style.size.height = window.line_height().into();
            return (window.request_layout(style, [], cx), ());
        }
        let text_style = window.text_style();
        let font = text_style.font();
        let font_size = text_style.font_size.to_pixels(window.rem_size());
        let line_height = window.line_height();
        let id = window.request_measured_layout(style, move |known, available, window, _cx| {
            let width = known.width.unwrap_or_else(|| match available.width {
                AvailableSpace::Definite(width) => width,
                _ => px(380.),
            });
            let rows = editor_rows(
                &text,
                document.as_ref(),
                width,
                font.clone(),
                font_size,
                line_height,
                true,
                window,
            );
            let height = rows.last().map_or(line_height, |r| {
                r.top + r.height + px(if r.code { 16. } else { 0. })
            });
            size(width, height)
        });
        (id, ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) -> PrepaintState {
        let input = self.input.read(cx);
        let style = window.text_style();
        let (display, text_color) = if input.content.is_empty() {
            (input.placeholder.clone(), input.muted)
        } else {
            (input.content.clone(), style.color)
        };
        let line_height = window.line_height();
        let font_size = style.font_size.to_pixels(window.rem_size());
        let mut layout = TextLayout {
            text: input.content.clone(),
            lines: vec![],
            offsets: vec![],
            rows: vec![],
        };
        let mut selections = vec![];
        let mut cursor = None;
        let mut decorations = vec![];
        let mut prefixes = vec![];
        for row in editor_rows(
            &display,
            if input.content.is_empty() {
                None
            } else {
                input.editor.as_ref().map(|e| &e.document)
            },
            bounds.size.width,
            style.font(),
            font_size,
            line_height,
            input.multiline,
            window,
        ) {
            let offset = row.range.start;
            let text = &display[row.range.clone()];
            let run = TextRun {
                len: text.len(),
                font: style.font(),
                color: text_color,
                background_color: None,
                underline: None,
                strikethrough: None,
            };
            let mut runs = vec![run.clone()];
            if let Some(marked) = &input.marked_range {
                let start = marked.start.saturating_sub(offset).min(text.len());
                let end = marked.end.saturating_sub(offset).min(text.len());
                if start < end {
                    runs = vec![
                        TextRun {
                            len: start,
                            ..run.clone()
                        },
                        TextRun {
                            len: end - start,
                            underline: Some(UnderlineStyle {
                                color: Some(text_color),
                                thickness: px(1.),
                                wavy: false,
                            }),
                            ..run.clone()
                        },
                        TextRun {
                            len: text.len() - end,
                            ..run.clone()
                        },
                    ];
                    runs.retain(|r| r.len > 0);
                }
            }
            if input.editor.is_some() && !input.content.is_empty() {
                runs = input.editor_runs(
                    offset..offset + text.len(),
                    &run_for_editor(&style, text_color),
                );
            }
            let line =
                window
                    .text_system()
                    .shape_line(text.to_owned().into(), row.font_size, &runs, None);
            let top = bounds.top() + row.top;
            let left = bounds.left() + row.indent;
            let line_height = row.height;
            if row.code {
                decorations.push(fill(
                    Bounds::new(
                        point(bounds.left(), top - px(8.)),
                        size(bounds.size.width, line_height + px(16.)),
                    ),
                    rgba(if text_color.l > 0.5 {
                        0x292c30ff
                    } else {
                        0xf5f5f5ff
                    }),
                ));
            }
            if row.quote {
                decorations.push(fill(
                    Bounds::new(point(left - px(16.), top), size(px(3.), line_height)),
                    rgba(0xffffff80),
                ));
            }
            if !row.prefix.is_empty() {
                let prefix = window.text_system().shape_line(
                    row.prefix.clone().into(),
                    row.font_size,
                    &[TextRun {
                        len: row.prefix.len(),
                        ..run.clone()
                    }],
                    None,
                );
                prefixes.push((point(left - px(20.), top), prefix, line_height));
            }
            let start = input
                .selected_range
                .start
                .saturating_sub(offset)
                .min(text.len());
            let end = input
                .selected_range
                .end
                .saturating_sub(offset)
                .min(text.len());
            if start < end {
                selections.push(fill(
                    Bounds::from_corners(
                        point(left + line.x_for_index(start), top),
                        point(left + line.x_for_index(end), top + line_height),
                    ),
                    rgba(0x3311ff30),
                ));
            }
            let index = input.cursor_offset();
            if input.selected_range.is_empty() && index >= offset && index <= offset + text.len() {
                cursor = Some(fill(
                    Bounds::new(
                        point(left + line.x_for_index(index - offset), top),
                        size(px(2.), line_height),
                    ),
                    gpui::blue(),
                ));
            }
            layout.offsets.push(offset);
            layout.lines.push(line);
            layout.rows.push(row);
        }
        PrepaintState {
            layout,
            cursor,
            selections,
            decorations,
            prefixes,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut (),
        prepaint: &mut PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus = self.input.read(cx).focus_handle.clone();
        window.handle_input(
            &focus,
            ElementInputHandler::new(bounds, self.input.clone()),
            cx,
        );
        for decoration in prepaint.decorations.drain(..) {
            window.paint_quad(decoration);
        }
        for (point, line, height) in &prepaint.prefixes {
            line.paint(*point, *height, gpui::TextAlign::Left, None, window, cx)
                .unwrap();
        }
        for selection in prepaint.selections.drain(..) {
            window.paint_quad(selection);
        }
        for (row, line) in prepaint.layout.lines.iter().enumerate() {
            line.paint(
                point(
                    bounds.left() + prepaint.layout.rows[row].indent,
                    bounds.top() + prepaint.layout.rows[row].top,
                ),
                prepaint.layout.rows[row].height,
                gpui::TextAlign::Left,
                None,
                window,
                cx,
            )
            .unwrap();
        }
        if focus.is_focused(window)
            && let Some(cursor) = prepaint.cursor.take()
        {
            window.paint_quad(cursor);
        }
        let layout = std::mem::replace(
            &mut prepaint.layout,
            TextLayout {
                text: "".into(),
                lines: vec![],
                offsets: vec![],
                rows: vec![],
            },
        );
        self.input.update(cx, |input, _| {
            input.last_layout = Some(layout);
            input.last_bounds = Some(bounds);
        });
    }
}

impl Render for TextInput {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let menu = self.editor_menu(_window, cx);
        let handles = self.block_drag_handles(cx);
        div()
            .id("text-input")
            .flex()
            .flex_col()
            .relative()
            .key_context(if self.editor_newlines {
                "TextInput MultilineEditor"
            } else {
                "TextInput"
            })
            .track_focus(&self.focus_handle(cx))
            .cursor(CursorStyle::IBeam)
            .on_key_down(cx.listener(Self::editor_key))
            .on_click(cx.listener(Self::editor_click))
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::delete))
            .on_action(cx.listener(Self::left))
            .on_action(cx.listener(Self::right))
            .on_action(cx.listener(|this, _: &Up, window, cx| this.vertical(-1, window, cx)))
            .on_action(cx.listener(|this, _: &Down, window, cx| this.vertical(1, window, cx)))
            .on_action(cx.listener(Self::select_left))
            .on_action(cx.listener(Self::select_right))
            .on_action(cx.listener(Self::select_all))
            .on_action(cx.listener(Self::home))
            .on_action(cx.listener(Self::end))
            .on_action(cx.listener(Self::show_character_palette))
            .on_action(cx.listener(Self::paste))
            .on_action(cx.listener(Self::paste_plain))
            .on_action(cx.listener(Self::cut))
            .on_action(cx.listener(Self::copy))
            .on_action(cx.listener(Self::newline))
            .on_action(
                cx.listener(|this, _: &HardBreak, window, cx| this.insert_newline(window, cx)),
            )
            .on_action(
                cx.listener(|this, _: &Undo, window, cx| this.restore_edit(false, window, cx)),
            )
            .on_action(
                cx.listener(|this, _: &Redo, window, cx| this.restore_edit(true, window, cx)),
            )
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event, window, cx| {
                    window.focus(&this.focus_handle, cx);
                    this.on_mouse_down(event, window, cx);
                }),
            )
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .w_full()
            .line_height(px(24.))
            .child(TextElement { input: cx.entity() })
            .children(handles)
            .children(
                menu.into_iter()
                    .map(|menu| gpui::deferred(menu).with_priority(1)),
            )
    }
}

impl Focusable for TextInput {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

pub fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("enter", Newline, Some("MultilineEditor")),
        KeyBinding::new("shift-enter", HardBreak, Some("MultilineEditor")),
        KeyBinding::new("ctrl-z", Undo, Some("TextInput")),
        KeyBinding::new("ctrl-shift-z", Redo, Some("TextInput")),
        KeyBinding::new("cmd-z", Undo, Some("TextInput")),
        KeyBinding::new("cmd-shift-z", Redo, Some("TextInput")),
        KeyBinding::new("backspace", Backspace, Some("TextInput")),
        KeyBinding::new("delete", Delete, Some("TextInput")),
        KeyBinding::new("left", Left, Some("TextInput")),
        KeyBinding::new("right", Right, Some("TextInput")),
        KeyBinding::new("up", Up, Some("TextInput")),
        KeyBinding::new("down", Down, Some("TextInput")),
        KeyBinding::new("shift-left", SelectLeft, Some("TextInput")),
        KeyBinding::new("shift-right", SelectRight, Some("TextInput")),
        KeyBinding::new("ctrl-a", SelectAll, Some("TextInput")),
        KeyBinding::new("ctrl-v", Paste, Some("TextInput")),
        KeyBinding::new("ctrl-shift-v", PastePlain, Some("TextInput")),
        KeyBinding::new("cmd-shift-v", PastePlain, Some("TextInput")),
        KeyBinding::new("ctrl-c", Copy, Some("TextInput")),
        KeyBinding::new("ctrl-x", Cut, Some("TextInput")),
        KeyBinding::new("home", Home, Some("TextInput")),
        KeyBinding::new("end", End, Some("TextInput")),
    ]);
}

impl TextInput {
    pub fn new(
        content: String,
        placeholder: &str,
        muted: gpui::Hsla,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            content: content.into(),
            placeholder: placeholder.to_owned().into(),
            selected_range: 0..0,
            selection_reversed: false,
            marked_range: None,
            last_layout: None,
            multiline: false,
            editor_newlines: false,
            last_bounds: None,
            is_selecting: false,
            muted,
            undo: vec![],
            redo: vec![],
            editor: None,
            read_only: false,
        }
    }

    pub fn multiline(mut self) -> Self {
        self.multiline = true;
        self
    }

    pub fn multiline_editor(mut self) -> Self {
        self.multiline = true;
        self.editor_newlines = true;
        self.editor = Some(EditorState::new(self.content.as_ref()));
        self
    }

    pub fn insert_newline(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.replace_text_in_range(None, "\n", window, cx);
    }

    pub fn append(&mut self, text: &str, window: &mut Window, cx: &mut Context<Self>) {
        let end = self.content.len();
        self.move_to(end, window, cx);
        self.replace_text_in_range(None, text, window, cx);
    }

    pub fn configure(&mut self, placeholder: &str, multiline: bool, cx: &mut Context<Self>) {
        self.placeholder = placeholder.to_owned().into();
        self.multiline = multiline;
        self.editor_newlines = multiline;
        cx.notify();
    }

    pub fn set_content(&mut self, text: String, cx: &mut Context<Self>) {
        self.undo.clear();
        self.redo.clear();
        self.selected_range = 0..text.len();
        if let Some(e) = &mut self.editor {
            let comment = e.comment;
            *e = EditorState::new(&text);
            e.comment = comment;
        }
        self.content = text.into();
        self.marked_range = None;
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[gpui::test]
    fn editor_line_break_undo_redo_keys_preserve_unicode_and_selection(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(init);
        let (view, cx) = cx.add_window_view(|_, cx| {
            TextInput::new(String::new(), "", gpui::black(), cx).multiline_editor()
        });
        view.update_in(cx, |input, window, cx| {
            window.focus(&input.focus_handle, cx)
        });
        cx.simulate_input("First é🙂");
        cx.simulate_keystrokes("shift-enter");
        view.update_in(cx, |input, window, cx| {
            input.replace_text_in_range(None, "Second", window, cx)
        });
        view.read_with(cx, |input, _| {
            assert_eq!(input.content.as_ref(), "First é🙂\nSecond")
        });
        cx.simulate_keystrokes("ctrl-z");
        view.read_with(cx, |input, _| {
            assert_eq!(input.content.as_ref(), "First é🙂\n")
        });
        cx.simulate_keystrokes("ctrl-shift-z");
        view.read_with(cx, |input, _| {
            assert_eq!(input.content.as_ref(), "First é🙂\nSecond")
        });
        cx.simulate_keystrokes("ctrl-a");
        view.update_in(cx, |input, window, cx| {
            input.replace_text_in_range(None, "Replacement", window, cx)
        });
        cx.simulate_keystrokes("ctrl-z");
        view.read_with(cx, |input, _| {
            assert_eq!(input.content.as_ref(), "First é🙂\nSecond");
            assert_eq!(input.selected_range, 0..input.content.len());
        });
        cx.simulate_input("New branch");
        cx.simulate_keystrokes("ctrl-shift-z");
        view.read_with(cx, |input, _| {
            assert_eq!(input.content.as_ref(), "New branch")
        });
    }

    #[gpui::test]
    fn editor_ime_composition_is_one_undo_and_loaded_drafts_reset_history(
        cx: &mut gpui::TestAppContext,
    ) {
        let (view, cx) = cx.add_window_view(|_, cx| {
            TextInput::new(String::new(), "", gpui::black(), cx).multiline()
        });
        view.update_in(cx, |input, window, cx| {
            input.replace_and_mark_text_in_range(None, "h", None, window, cx);
            input.replace_and_mark_text_in_range(None, "漢", None, window, cx);
            input.replace_text_in_range(None, "漢字", window, cx);
            input.restore_edit(false, window, cx);
            assert!(input.content.is_empty());
            input.restore_edit(true, window, cx);
            assert_eq!(input.content.as_ref(), "漢字");
            input.set_content("Other task".into(), cx);
            input.restore_edit(false, window, cx);
            assert_eq!(input.content.as_ref(), "Other task");
            input.reset(window, cx);
            input.restore_edit(false, window, cx);
            assert!(input.content.is_empty());
        });
    }

    #[gpui::test]
    fn multiline_composer_wraps_and_preserves_utf8_geometry(cx: &mut gpui::TestAppContext) {
        let text = format!("{}\n\nSecond é🙂", "long text é🙂 ".repeat(100));
        let (view, cx) = cx.add_window_view(|_, cx| {
            TextInput::new(text.clone(), "", gpui::black(), cx).multiline()
        });
        cx.run_until_parked();
        view.update_in(cx, |input, window, cx| {
            let layout = input
                .last_layout
                .as_ref()
                .expect("multiline input was painted");
            assert!(
                layout.lines.len() > text.split('\n').count(),
                "long lines must soft-wrap"
            );
            for (line, &offset) in layout.lines.iter().zip(&layout.offsets) {
                assert!(text.is_char_boundary(offset));
                assert_eq!(&text[offset..offset + line.text.len()], line.text.as_ref());
            }
            let bounds = input.last_bounds.unwrap();
            let start = text.find("Second").unwrap();
            let range = input.range_to_utf16(&(start..text.len()));
            let rectangle = input
                .bounds_for_range(range.clone(), bounds, window, cx)
                .unwrap();
            assert!(rectangle.top() > bounds.top());
            input.set_selected_text_range(range, window, cx);
            input.replace_text_in_range(None, "Replacement 漢", window, cx);
            assert!(input.content.ends_with("\n\nReplacement 漢"));
            input.move_to(0, window, cx);
            input.vertical(1, window, cx);
            assert_eq!(
                input.cursor_offset(),
                0,
                "ignore stale layout after editing"
            );
            input.set_content(String::new(), cx);
            input.vertical(1, window, cx);
            assert_eq!(input.cursor_offset(), 0, "an empty draft has no next row");
        });
    }
}
