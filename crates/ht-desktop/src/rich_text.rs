use crate::theme::{Theme, color};
use ego_tree::iter::Edge;
use gpui::{FontStyle, FontWeight, HighlightStyle, StyledText, UnderlineStyle, px};
use scraper::{Html, Node};
use std::ops::Range;

#[derive(Clone, Default)]
pub struct RichText {
    pub text: String,
    highlights: Vec<(Range<usize>, HighlightStyle)>,
}

impl RichText {
    pub fn parse(html: &str, theme: Theme) -> Self {
        let doc = Html::parse_fragment(html);
        let mut out = Self::default();
        let mut styles = vec![HighlightStyle::default()];
        let mut skip = 0;
        for edge in doc.tree.root().traverse() {
            match edge {
                Edge::Open(node) => match node.value() {
                    Node::Element(el) => {
                        let name = el.name();
                        if matches!(name, "script" | "style") {
                            skip += 1;
                        }
                        if matches!(name, "p" | "h1" | "h2" | "h3" | "div" | "blockquote" | "li")
                            && !out.text.is_empty()
                            && !out.text.ends_with('\n')
                            && !out.text.ends_with("• ")
                        {
                            out.text.push('\n');
                        }
                        if name == "li" {
                            out.text.push_str("• ");
                        }
                        if name == "blockquote" {
                            out.text.push_str("│ ");
                        }
                        if name == "br" {
                            out.text.push('\n');
                        }
                        let mut style = *styles.last().unwrap();
                        if matches!(name, "strong" | "b" | "h1" | "h2" | "h3") {
                            style.font_weight = Some(FontWeight::BOLD);
                        }
                        if matches!(name, "em" | "i") {
                            style.font_style = Some(FontStyle::Italic);
                        }
                        if matches!(name, "a" | "u") {
                            style.underline = Some(UnderlineStyle {
                                thickness: px(1.),
                                ..Default::default()
                            });
                        }
                        if name == "a" {
                            style.color = Some(color(theme.link));
                        }
                        styles.push(style);
                    }
                    Node::Text(text) if skip == 0 => {
                        if text.trim().is_empty()
                            && out.text.chars().last().is_none_or(char::is_whitespace)
                        {
                            continue;
                        }
                        let text = if text.trim().is_empty() {
                            " ".into()
                        } else {
                            text.to_string()
                        };
                        let start = out.text.len();
                        out.text.push_str(&text);
                        out.highlights
                            .push((start..out.text.len(), *styles.last().unwrap()));
                    }
                    _ => (),
                },
                Edge::Close(node) => {
                    if let Node::Element(el) = node.value() {
                        if matches!(el.name(), "script" | "style") {
                            skip -= 1;
                        }
                        styles.pop();
                        if matches!(
                            el.name(),
                            "p" | "h1" | "h2" | "h3" | "div" | "li" | "blockquote"
                        ) && !out.text.ends_with('\n')
                        {
                            out.text.push('\n');
                        }
                    }
                }
            }
        }
        out
    }

    pub fn element(&self) -> StyledText {
        StyledText::new(self.text.clone()).with_highlights(self.highlights.clone())
    }
}

#[cfg(test)]
mod tests {
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
mod edit_tests {
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
