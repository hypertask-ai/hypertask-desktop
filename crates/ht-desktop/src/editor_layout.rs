use super::*;
use crate::editor_document::Document;

pub(super) struct EditorRow {
    pub range: Range<usize>,
    pub font_size: Pixels,
    pub height: Pixels,
    pub top: Pixels,
    pub indent: Pixels,
    pub prefix: String,
    pub quote: bool,
    pub code: bool,
    pub task: Option<usize>,
}

pub(super) fn editor_rows(
    text: &str,
    document: Option<&Document>,
    width: Pixels,
    font: gpui::Font,
    font_size: Pixels,
    line_height: Pixels,
    multiline: bool,
    window: &Window,
) -> Vec<EditorRow> {
    let Some(document) = document else {
        return line_ranges(text, width, multiline, font, font_size, window)
            .into_iter()
            .enumerate()
            .map(|(i, range)| EditorRow {
                range,
                font_size,
                height: line_height,
                top: line_height * i as f32,
                indent: px(0.),
                prefix: String::new(),
                quote: false,
                code: false,
                task: None,
            })
            .collect();
    };
    let mut rows = vec![];
    let mut offset = 0;
    let mut top = px(0.);
    for (index, display) in document.display().into_iter().enumerate() {
        let text = display.block.text();
        let code = display.block.tag.name == "pre";
        let font_size = px(crate::rich_text::block_size(&display.block.tag.name));
        let height = font_size * 1.5;
        let indent = px(display.depth as f32 * 20.
            + if display.quote { 16. } else { 0. }
            + if code { 16. } else { 0. });
        let mut font = font.clone();
        if code {
            font.family = "monospace".into();
        }
        if index > 0 {
            top += px(6.3);
        }
        if code {
            top += px(16.);
        }
        for (row, range) in line_ranges(
            &text,
            (width - indent - px(if code { 16. } else { 0. })).max(px(1.)),
            true,
            font,
            font_size,
            window,
        )
        .into_iter()
        .enumerate()
        {
            rows.push(EditorRow {
                range: offset + range.start..offset + range.end,
                font_size,
                height,
                top,
                indent,
                prefix: if row == 0 {
                    display.prefix.clone()
                } else {
                    String::new()
                },
                quote: display.quote,
                code,
                task: if row == 0 && display.task {
                    Some(index)
                } else {
                    None
                },
            });
            top += height;
        }
        if code {
            top += px(16.);
        }
        offset += text.len() + 1;
    }
    rows
}
