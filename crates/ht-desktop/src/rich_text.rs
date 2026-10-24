use crate::{
    editor_document::{Document, Tag, safe_link},
    theme::{Theme, color},
};
use gpui::{
    AnyElement, FontStyle, FontWeight, HighlightStyle, InteractiveText, SharedString, StyledText,
    UnderlineStyle, div, prelude::*, px,
};
use std::ops::Range;
use ego_tree::iter::Edge;
use scraper::{Html, Node};

#[derive(Clone, Default)]
pub struct RichText {
    pub text: String,
    highlights: Vec<(Range<usize>, HighlightStyle)>,
    blocks: Vec<RenderedBlock>,
}
#[derive(Clone)]
struct RenderedBlock {
    text: String,
    highlights: Vec<(Range<usize>, HighlightStyle)>,
    fonts: Vec<(Range<usize>, SharedString)>,
    links: Vec<(Range<usize>, String)>,
    prefix: String,
    depth: usize,
    quote: bool,
    size: f32,
    code: bool,
    background: u32,
}

pub(crate) fn mark_style(marks: &[Tag], theme: Theme) -> HighlightStyle {
    let light = color(theme.page).l > 0.5;
    let mut style = HighlightStyle::default();
    for mark in marks {
        match mark.name.as_str() {
            "strong" | "b" => style.font_weight = Some(FontWeight::BOLD),
            "em" | "i" => style.font_style = Some(FontStyle::Italic),
            "u" | "a" => {
                style.underline = Some(UnderlineStyle {
                    thickness: px(1.),
                    ..Default::default()
                });
                if mark.name == "a" {
                    style.color = Some(color(theme.link));
                }
            }
            "mark" => {
                let custom = mark
                    .attrs
                    .iter()
                    .find(|(k, _)| k == "data-color")
                    .and_then(|(_, v)| v.strip_prefix('#'))
                    .and_then(|hex| u32::from_str_radix(hex, 16).ok());
                style.background_color = Some(color(custom.unwrap_or(0xffe066)));
                style.color = Some(gpui::black());
            }
            "code" => {
                style.color = Some(color(if light { 0xd63384 } else { 0xe8912d }));
                style.background_color = Some(color(if light { 0xf5f5f5 } else { 0x292c30 }));
            }
            _ => (),
        }
    }
    style
}
pub(crate) fn block_size(tag: &str) -> f32 {
    match tag {
        "h1" => 24.,
        "h2" => 18.,
        "h3" => 16.,
        "pre" => 12.,
        _ => 14.,
    }
}

impl RichText {
    pub(crate) fn memory_payload_bytes(&self) -> usize {
        self.text.len() + self.highlights.len() * std::mem::size_of::<(Range<usize>, HighlightStyle)>()
            + self.blocks.iter().map(|b| b.text.len() + b.prefix.len()
                + b.highlights.len() * std::mem::size_of::<(Range<usize>, HighlightStyle)>()
                + b.links.iter().map(|(_, url)| url.len()).sum::<usize>()).sum::<usize>()
    }

    pub fn parse(html: &str, theme: Theme) -> Self {
        let document = Document::parse(html);
        let light = color(theme.page).l > 0.5;
        let mut out = Self::default();
        for display in document.display() {
            let b = display.block;
            let text = b.text();
            let mut block = RenderedBlock {
                text: text.clone(),
                highlights: vec![],
                fonts: vec![],
                links: vec![],
                prefix: display.prefix,
                depth: display.depth,
                quote: display.quote,
                size: block_size(&b.tag.name),
                code: b.tag.name == "pre",
                background: if light { 0xf5f5f5 } else { 0x292c30 },
            };
            let mut at = 0;
            for span in &b.spans {
                let range = at..at + span.text.len();
                let mut style = mark_style(&span.marks, theme);
                if matches!(b.tag.name.as_str(), "h1" | "h2" | "h3") {
                    style.font_weight = Some(if b.tag.name == "h3" {
                        FontWeight::MEDIUM
                    } else {
                        FontWeight::SEMIBOLD
                    });
                }
                if block.code {
                    style.color = Some(color(if light { 0xd63384 } else { 0xe8912d }));
                }
                if span.marks.iter().any(|m| m.name == "code") || block.code {
                    block.fonts.push((range.clone(), "monospace".into()));
                }
                for link in span.marks.iter().filter(|m| m.name == "a") {
                    if let Some((_, url)) = link.attrs.iter().find(|(k, _)| k == "href") {
                        if safe_link(url) {
                            block.links.push((range.clone(), url.clone()));
                        }
                    }
                }
                block.highlights.push((range, style));
                at += span.text.len();
            }
            if display.quote {
                out.text.push_str("│ ");
            }
            if !block.prefix.is_empty() {
                out.text.push_str(&block.prefix);
                out.text.push(' ');
            }
            let start = out.text.len();
            out.text.push_str(&text);
            out.text.push('\n');
            out.highlights.extend(
                block
                    .highlights
                    .iter()
                    .map(|(r, s)| (r.start + start..r.end + start, *s)),
            );
            out.blocks.push(block);
        }
        out
    }
    pub fn element(&self) -> AnyElement {
        div()
            .flex()
            .flex_col()
            .children(self.blocks.iter().enumerate().map(|(i, b)| {
                let links = b.links.clone();
                let text = StyledText::new(if b.text.is_empty() {
                    " ".to_owned()
                } else {
                    b.text.clone()
                })
                .with_highlights(b.highlights.clone())
                .with_font_family_overrides(b.fonts.clone());
                let interactive = InteractiveText::new(("rich-text", i), text).on_click(
                    links.iter().map(|(r, _)| r.clone()).collect(),
                    move |index, _, cx| {
                        if let Some((_, url)) = links.get(index) {
                            let url = if url.starts_with('/') {
                                format!("{}{url}", ht_core::auth::SITE_URL)
                            } else {
                                url.clone()
                            };
                            cx.open_url(&url);
                        }
                    },
                );
                div()
                    .flex()
                    .gap(px(8.))
                    .text_size(px(b.size))
                    .line_height(px(b.size * 1.5))
                    .when(i > 0, |d| d.mt(px(6.3)))
                    .when(b.depth > 0, |d| d.ml(px(16. * b.depth as f32)))
                    .when(b.quote, |d| {
                        d.border_l(px(3.))
                            .border_color(gpui::rgba(0xffffff80))
                            .pl(px(16.))
                    })
                    .when(b.code, |d| {
                        d.p(px(16.)).bg(color(b.background)).rounded(px(4.))
                    })
                    .when(!b.prefix.is_empty(), |d| d.child(b.prefix.clone()))
                    .child(div().min_w_0().flex_1().child(interactive))
            }))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserves_entities_unicode_blocks_and_inline_styles() {
        let r = RichText::parse(
            "<p>Hello <strong>é🙂</strong> &amp; <a href='/'>link</a></p><ul><li><p>one</p></li></ul><script>hidden</script>",
            crate::theme::THEMES[0],
        );
        assert!(r.text.contains("Hello é🙂 & link\n"));
        assert!(r.text.contains("• one"));
        assert!(!r.text.contains("hidden"));
        assert!(
            r.highlights
                .iter()
                .any(|(range, s)| &r.text[range.clone()] == "é🙂"
                    && s.font_weight == Some(FontWeight::BOLD))
        );
        assert!(r.highlights.iter().any(|(_, s)| s.underline.is_some()));
    }
    #[test]
    fn rendered_web_headings_highlights_code_links_and_list_markers_are_not_plain_text() {
        let r = RichText::parse(
            "<h1>Heading</h1><h2>Second</h2><h3>Third</h3><p><mark data-color=\"#74c0fc\">blue</mark> <code>x</code> <a href=\"https://example.com\">link</a></p><ol start=\"3\"><li><p>one</p></li><li><p>two</p></li></ol><ul data-type=\"taskList\"><li data-type=\"taskItem\" data-checked=\"true\"><label><input type=\"checkbox\"></label><div><p>done</p></div></li></ul><pre><code>code\nlines</code></pre>",
            crate::theme::THEMES[1],
        );
        assert_eq!(
            r.blocks.iter().take(3).map(|b| b.size).collect::<Vec<_>>(),
            [24., 18., 16.]
        );
        assert!(
            r.highlights
                .iter()
                .any(|(_, s)| s.background_color == Some(color(0x74c0fc)))
        );
        assert!(
            r.blocks[3]
                .fonts
                .iter()
                .any(|(_, f)| f.as_ref() == "monospace")
        );
        assert_eq!(r.blocks[3].links[0].1, "https://example.com");
        assert_eq!(r.blocks[4].prefix, "3.");
        assert_eq!(r.blocks[5].prefix, "4.");
        assert_eq!(r.blocks[6].prefix, "☑");
        assert!(r.blocks[7].code);
        assert!(r.blocks[7].text.contains("code\nlines"));
    }
    #[test]
    fn code_colors_follow_both_web_light_themes_including_nonwhite_dia() {
        for (index, theme) in crate::theme::THEMES.into_iter().enumerate() {
            let light = matches!(index, 0 | 3);
            let rich = RichText::parse(
                "<p><code>inline</code></p><pre><code>block</code></pre>",
                theme,
            );
            let expected = color(if light { 0xd63384 } else { 0xe8912d });
            assert_eq!(rich.blocks[0].highlights[0].1.color, Some(expected));
            assert_eq!(rich.blocks[1].highlights[0].1.color, Some(expected));
            assert_eq!(
                rich.blocks[1].background,
                if light { 0xf5f5f5 } else { 0x292c30 }
            );
        }
    }
}

#[cfg(test)]
mod main_render_tests {
    use super::*;
    #[test]
    fn preserves_entities_unicode_blocks_and_inline_styles() {
        let r = RichText::parse(
            "<p>Hello <strong>é🙂</strong> &amp; <a href='/'>link</a></p><ul><li>one</li></ul><script>hidden</script>",
            crate::theme::THEMES[0],
        );
        assert!(r.text.contains("Hello é🙂 & link\n"));
        assert!(r.text.contains("• one"));
        assert!(!r.text.contains("hidden"));
        assert!(
            r.highlights
                .iter()
                .any(|(range, s)| &r.text[range.clone()] == "é🙂"
                    && s.font_weight == Some(FontWeight::BOLD))
        );
        assert!(r.highlights.iter().any(|(_, s)| s.underline.is_some()));
    }
}

/// Apply the editor's text delta without replacing unrelated marks, links or media.
pub fn edited_html(html: &str, previous: &str, next: &str) -> String {
    if previous == next {
        return html.to_owned();
    }
    let mut start = 0;
    for (a, b) in previous.chars().zip(next.chars()) {
        if a != b {
            break;
        }
        start += a.len_utf8();
    }
    let mut old_end = previous.len();
    let mut new_end = next.len();
    for (a, b) in previous[start..]
        .chars()
        .rev()
        .zip(next[start..].chars().rev())
    {
        if a != b {
            break;
        }
        old_end -= a.len_utf8();
        new_end -= b.len_utf8();
    }
    let inserted = &next[start..new_end];
    let doc = Html::parse_fragment(html);
    let mut output = String::new();
    let mut offset = 0;
    let mut did_insert = false;
    let escape = |text: &str| {
        text.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
    };
    for edge in doc.tree.root().traverse() {
        match edge {
            Edge::Open(node) => match node.value() {
                Node::Element(el) => {
                    if el.name() == "html" {
                        continue;
                    }
                    output.push('<');
                    output.push_str(el.name());
                    for (key, value) in el.attrs() {
                        output.push(' ');
                        output.push_str(key);
                        output.push_str("=\"");
                        output.push_str(&escape(value));
                        output.push('"');
                    }
                    output.push('>');
                }
                Node::Text(text) => {
                    let normalized: &str = if text.trim().is_empty() {
                        " "
                    } else {
                        text.as_ref()
                    };
                    let Some(at) = previous[offset..].find(normalized).map(|at| at + offset) else {
                        output.push_str(&escape(text));
                        continue;
                    };
                    let end = at + normalized.len();
                    let from = start.saturating_sub(at).min(normalized.len());
                    let to = old_end.saturating_sub(at).min(normalized.len());
                    if end >= start && at <= old_end {
                        output.push_str(&escape(&normalized[..from]));
                        if !did_insert {
                            output.push_str(&escape(inserted).replace('\n', "<br>"));
                            did_insert = true;
                        }
                        output.push_str(&escape(&normalized[to.max(from)..]));
                    } else {
                        output.push_str(&escape(text));
                    }
                    offset = end;
                }
                _ => (),
            },
            Edge::Close(node) => {
                if let Node::Element(el) = node.value() {
                    if !matches!(
                        el.name(),
                        "html"
                            | "area"
                            | "base"
                            | "br"
                            | "col"
                            | "embed"
                            | "hr"
                            | "img"
                            | "input"
                            | "link"
                            | "meta"
                            | "param"
                            | "source"
                            | "track"
                            | "wbr"
                    ) {
                        output.push_str("</");
                        output.push_str(el.name());
                        output.push('>');
                    }
                }
            }
        }
    }
    if !did_insert {
        output.push_str(&escape(inserted).replace('\n', "<br>"));
    }
    output
}

#[cfg(test)]
mod main_edit_tests {
    use super::*;
    #[test]
    fn comment_text_edit_preserves_existing_marks_links_and_attachments() {
        let html = "<p>Hello <strong>é🙂</strong> <a href=\"https://example.com\">link</a></p><p><img src=\"https://example.com/image.png\"></p>";
        let old = RichText::parse(html, crate::theme::THEMES[0]).text;
        let next = old.replace("Hello", "Goodbye");
        let edited = edited_html(html, &old, &next);
        assert!(edited.contains("<strong>é🙂</strong>"));
        assert!(edited.contains("<a href=\"https://example.com\">link</a>"));
        assert!(edited.contains("<img src=\"https://example.com/image.png\">"));
        assert_eq!(RichText::parse(&edited, crate::theme::THEMES[0]).text, next);
        assert_eq!(edited_html(html, &old, &old), html);
    }
}
