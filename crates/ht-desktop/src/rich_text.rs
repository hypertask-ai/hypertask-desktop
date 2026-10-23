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
