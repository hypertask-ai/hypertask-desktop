//! The editable document uses UTF-8 offsets, without display-only list/quote markers.
use scraper::{ElementRef, Html, Node};
use std::ops::Range;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tag {
    pub name: String,
    pub attrs: Vec<(String, String)>,
}
impl Tag {
    fn new(name: &str) -> Self {
        Self {
            name: name.into(),
            attrs: vec![],
        }
    }
    fn attr(mut self, name: &str, value: &str) -> Self {
        self.attrs.push((name.into(), value.into()));
        self.attrs.sort();
        self
    }
    fn from(el: ElementRef<'_>) -> Self {
        Self {
            name: el.value().name().into(),
            attrs: el
                .value()
                .attrs()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        }
    }
    fn open(&self) -> String {
        let mut out = format!("<{}", self.name);
        for (k, v) in &self.attrs {
            out.push_str(&format!(" {k}=\"{}\"", escape(v)));
        }
        out.push('>');
        out
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    pub text: String,
    pub marks: Vec<Tag>,
    atom: Option<String>,
}
impl Span {
    fn text(text: &str, marks: &[Tag]) -> Self {
        Self {
            text: text.into(),
            marks: marks.to_vec(),
            atom: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    pub tag: Tag,
    pub spans: Vec<Span>,
    code: Option<Tag>,
}
impl Block {
    pub fn text(&self) -> String {
        self.spans.iter().map(|s| s.text.as_str()).collect()
    }
    fn len(&self) -> usize {
        self.spans.iter().map(|s| s.text.len()).sum()
    }
    fn slice(&self, range: Range<usize>) -> Vec<Span> {
        let mut at = 0;
        let mut out = vec![];
        for span in &self.spans {
            let end = at + span.text.len();
            let start = range.start.max(at);
            let stop = range.end.min(end);
            if start < stop {
                let mut s = span.clone();
                s.text = span.text[start - at..stop - at].into();
                out.push(s);
            }
            at = end;
        }
        out
    }
    fn compact(&mut self) {
        let mut spans: Vec<Span> = vec![];
        for mut span in self.spans.drain(..).filter(|s| !s.text.is_empty()) {
            span.marks.sort_by_key(|m| match m.name.as_str() {
                "a" => 0,
                "code" => 1,
                "strong" => 2,
                "em" => 3,
                "s" => 4,
                "mark" => 5,
                "u" => 6,
                _ => 7,
            });
            if let Some(last) = spans.last_mut() {
                if last.marks == span.marks && last.atom.is_none() && span.atom.is_none() {
                    last.text.push_str(&span.text);
                    continue;
                }
            }
            spans.push(span);
        }
        self.spans = spans;
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Part {
    Block(Block),
    Container(Tag, Vec<Part>),
    Atom(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Document {
    parts: Vec<Part>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Bold,
    Italic,
    Underline,
    Highlight,
    Code,
    Quote,
    BulletList,
    OrderedList,
    TaskList,
    Paragraph,
    Heading(u8),
    CodeBlock,
}
impl Format {
    pub fn mark(self) -> Option<&'static str> {
        match self {
            Self::Bold => Some("strong"),
            Self::Italic => Some("em"),
            Self::Underline => Some("u"),
            Self::Highlight => Some("mark"),
            Self::Code => Some("code"),
            _ => None,
        }
    }
}

pub fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
fn is_block(name: &str) -> bool {
    matches!(name, "p" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "pre")
}
fn inline(el: ElementRef<'_>, marks: &[Tag], out: &mut Vec<Span>) {
    for child in el.children() {
        match child.value() {
            Node::Text(text) => out.push(Span::text(text, marks)),
            Node::Element(_) => {
                let child = ElementRef::wrap(child).unwrap();
                let name = child.value().name();
                match name {
                    "br" => out.push(Span::text("\n", marks)),
                    "script" | "style" => (),
                    "img" | "iframe" | "video" | "input" => out.push(Span {
                        text: "\u{fffc}".into(),
                        marks: vec![],
                        atom: Some(child.html()),
                    }),
                    "code" if el.value().name() == "pre" => inline(child, marks, out),
                    _ => {
                        let mut tag = Tag::from(child);
                        tag.name = match name {
                            "b" => "strong",
                            "i" => "em",
                            _ => name,
                        }
                        .into();
                        let mut nested = marks.to_vec();
                        nested.push(tag);
                        inline(child, &nested, out);
                    }
                }
            }
            _ => (),
        }
    }
}
fn parse_parts(el: ElementRef<'_>) -> Vec<Part> {
    fn flush(parts: &mut Vec<Part>, pending: &mut Vec<Span>) {
        if !pending.is_empty() {
            let mut block = Block {
                tag: Tag::new("p"),
                code: None,
                spans: std::mem::take(pending),
            };
            block.compact();
            parts.push(Part::Block(block));
        }
    }
    let mut parts = vec![];
    let mut pending = vec![];
    for child in el.children() {
        match child.value() {
            Node::Text(text) if !text.trim().is_empty() || !pending.is_empty() => {
                pending.push(Span::text(text, &[]))
            }
            Node::Element(_) => {
                let child = ElementRef::wrap(child).unwrap();
                let name = child.value().name();
                if matches!(name, "script" | "style" | "label") {
                    continue;
                }
                if matches!(
                    name,
                    "strong" | "b" | "em" | "i" | "u" | "s" | "mark" | "code" | "a" | "span" | "br"
                ) {
                    if name == "br" {
                        pending.push(Span::text("\n", &[]));
                    } else {
                        let mut tag = Tag::from(child);
                        tag.name = match name {
                            "b" => "strong",
                            "i" => "em",
                            _ => name,
                        }
                        .into();
                        inline(child, &[tag], &mut pending);
                    }
                    continue;
                }
                flush(&mut parts, &mut pending);
                if is_block(name) {
                    let mut spans = vec![];
                    inline(child, &[], &mut spans);
                    let code = if name == "pre" {
                        child
                            .children()
                            .filter_map(ElementRef::wrap)
                            .find(|e| e.value().name() == "code")
                            .map(Tag::from)
                    } else {
                        None
                    };
                    let mut block = Block {
                        tag: Tag::from(child),
                        code,
                        spans,
                    };
                    if name == "pre" {
                        for span in &mut block.spans {
                            span.marks.clear();
                        }
                    }
                    block.compact();
                    parts.push(Part::Block(block));
                } else if matches!(name, "blockquote" | "ul" | "ol" | "li")
                    || (name == "div" && child.value().attr("class") != Some("video-wrapper"))
                {
                    let task = name == "li" && child.value().attr("data-type") == Some("taskItem");
                    let mut children = if task {
                        child
                            .children()
                            .filter_map(ElementRef::wrap)
                            .filter(|e| e.value().name() == "div")
                            .flat_map(parse_parts)
                            .collect()
                    } else {
                        parse_parts(child)
                    };
                    if children.is_empty() {
                        children.push(Part::Block(Block {
                            tag: Tag::new("p"),
                            code: None,
                            spans: vec![],
                        }));
                    }
                    parts.push(Part::Container(Tag::from(child), children));
                } else {
                    parts.push(Part::Atom(child.html()));
                }
            }
            _ => (),
        }
    }
    flush(&mut parts, &mut pending);
    parts
}
fn visit<'a>(parts: &'a [Part], out: &mut Vec<&'a Block>) {
    for part in parts {
        match part {
            Part::Block(b) => out.push(b),
            Part::Container(_, p) => visit(p, out),
            Part::Atom(_) => (),
        }
    }
}
fn each_mut(parts: &mut [Part], index: &mut usize, f: &mut impl FnMut(usize, &mut Block)) {
    for part in parts {
        match part {
            Part::Block(b) => {
                f(*index, b);
                *index += 1;
            }
            Part::Container(_, p) => each_mut(p, index, f),
            Part::Atom(_) => (),
        }
    }
}
fn html_parts(parts: &[Part], out: &mut String) {
    for part in parts {
        match part {
            Part::Atom(html) => out.push_str(html),
            Part::Block(b) => {
                out.push_str(&b.tag.open());
                if b.tag.name == "pre" {
                    out.push_str(
                        &b.code
                            .clone()
                            .unwrap_or_else(|| {
                                Tag::new("code").attr("class", "language-javascript")
                            })
                            .open(),
                    );
                }
                let mut active: Vec<Tag> = vec![];
                for span in &b.spans {
                    let common = active
                        .iter()
                        .zip(&span.marks)
                        .take_while(|(a, b)| a == b)
                        .count();
                    for mark in active[common..].iter().rev() {
                        out.push_str(&format!("</{}>", mark.name));
                    }
                    for mark in &span.marks[common..] {
                        out.push_str(&mark.open());
                    }
                    active = span.marks.clone();
                    if let Some(atom) = &span.atom {
                        out.push_str(atom);
                    } else {
                        let text = escape(&span.text);
                        out.push_str(&if b.tag.name == "pre" {
                            text
                        } else {
                            text.replace('\n', "<br>")
                        });
                    }
                }
                for mark in active.iter().rev() {
                    out.push_str(&format!("</{}>", mark.name));
                }
                if b.tag.name == "pre" {
                    out.push_str("</code>");
                }
                out.push_str(&format!("</{}>", b.tag.name));
            }
            Part::Container(tag, children) => {
                out.push_str(&tag.open());
                let task = tag.name == "li"
                    && tag
                        .attrs
                        .iter()
                        .any(|(k, v)| k == "data-type" && v == "taskItem");
                if task {
                    let checked = tag
                        .attrs
                        .iter()
                        .any(|(k, v)| k == "data-checked" && v == "true");
                    out.push_str(if checked { "<label><input type=\"checkbox\" checked=\"checked\"><span></span></label><div>" } else { "<label><input type=\"checkbox\"><span></span></label><div>" });
                }
                html_parts(children, out);
                if task {
                    out.push_str("</div>");
                }
                out.push_str(&format!("</{}>", tag.name));
            }
        }
    }
}
#[derive(Clone)]
pub struct DisplayBlock<'a> {
    pub block: &'a Block,
    pub prefix: String,
    pub depth: usize,
    pub quote: bool,
    pub task: bool,
}
fn display_blocks<'a>(
    parts: &'a [Part],
    depth: usize,
    quote: bool,
    prefix: &str,
    task: bool,
    out: &mut Vec<DisplayBlock<'a>>,
) {
    let mut prefix = prefix.to_owned();
    for part in parts {
        match part {
            Part::Block(block) => {
                out.push(DisplayBlock {
                    block,
                    prefix: std::mem::take(&mut prefix),
                    depth,
                    quote,
                    task,
                });
            }
            Part::Container(tag, children) if matches!(tag.name.as_str(), "ul" | "ol") => {
                let mut number = tag
                    .attrs
                    .iter()
                    .find(|(k, _)| k == "start")
                    .and_then(|(_, v)| v.parse::<usize>().ok())
                    .unwrap_or(1);
                for item in children {
                    let mut marker = if tag.name == "ol" {
                        format!("{number}.")
                    } else {
                        "•".into()
                    };
                    let mut is_task = false;
                    if let Part::Container(li, _) = item {
                        is_task = li
                            .attrs
                            .iter()
                            .any(|(k, v)| k == "data-type" && v == "taskItem");
                        if is_task {
                            marker = if li
                                .attrs
                                .iter()
                                .any(|(k, v)| k == "data-checked" && v == "true")
                            {
                                "☑"
                            } else {
                                "☐"
                            }
                            .into();
                        }
                    }
                    display_blocks(
                        std::slice::from_ref(item),
                        depth + 1,
                        quote,
                        &marker,
                        is_task,
                        out,
                    );
                    number += 1;
                }
            }
            Part::Container(tag, children) => display_blocks(
                children,
                depth,
                quote || tag.name == "blockquote",
                &prefix,
                task,
                out,
            ),
            Part::Atom(_) => (),
        }
    }
}

impl Default for Document {
    fn default() -> Self {
        Self::plain("")
    }
}
impl Document {
    pub fn plain(text: &str) -> Self {
        Self {
            parts: vec![Part::Block(Block {
                tag: Tag::new("p"),
                code: None,
                spans: if text.is_empty() {
                    vec![]
                } else {
                    vec![Span::text(text, &[])]
                },
            })],
        }
    }
    pub fn parse(html: &str) -> Self {
        let doc = Html::parse_fragment(html);
        let mut parts = parse_parts(doc.root_element());
        if parts.is_empty() {
            return Self::default();
        }
        let mut blocks = vec![];
        visit(&parts, &mut blocks);
        if blocks.is_empty() {
            parts.push(Part::Block(Block {
                tag: Tag::new("p"),
                code: None,
                spans: vec![],
            }));
        }
        Self { parts }
    }
    pub fn html(&self) -> String {
        let mut out = String::new();
        html_parts(&self.parts, &mut out);
        out
    }
    pub fn blocks(&self) -> Vec<&Block> {
        let mut out = vec![];
        visit(&self.parts, &mut out);
        out
    }
    pub fn display(&self) -> Vec<DisplayBlock<'_>> {
        let mut out = vec![];
        display_blocks(&self.parts, 0, false, "", false, &mut out);
        out
    }
    pub fn text(&self) -> String {
        self.blocks()
            .iter()
            .map(|b| b.text())
            .collect::<Vec<_>>()
            .join("\n")
    }
    fn location(&self, offset: usize) -> (usize, usize) {
        let blocks = self.blocks();
        let mut at = 0;
        for (i, b) in blocks.iter().enumerate() {
            if offset <= at + b.len() {
                return (i, offset - at);
            }
            at += b.len() + 1;
        }
        let i = blocks.len() - 1;
        (i, blocks[i].len())
    }
    pub fn block_at(&self, offset: usize) -> &Block {
        self.blocks()[self.location(offset).0]
    }
    pub fn marks_at(&self, offset: usize) -> Vec<Tag> {
        let (i, offset) = self.location(offset);
        let blocks = self.blocks();
        let mut at = 0;
        for span in &blocks[i].spans {
            at += span.text.len();
            if offset <= at {
                let mut marks = span.marks.clone();
                if offset == at || offset == 0 {
                    marks.retain(|m| {
                        m.name != "a"
                            && !(m.name == "span"
                                && m.attrs
                                    .iter()
                                    .any(|(k, v)| k == "data-type" && v == "emoji"))
                    });
                }
                return marks;
            }
        }
        vec![]
    }
    pub fn replace(&mut self, range: Range<usize>, text: &str, marks: &[Tag]) {
        let (first, start) = self.location(range.start);
        let (last, end) = self.location(range.end);
        let blocks = self.blocks();
        let mut spans = blocks[first].slice(0..start);
        spans.push(Span::text(text, marks));
        spans.extend(blocks[last].slice(end..blocks[last].len()));
        let mut next = Block {
            tag: blocks[first].tag.clone(),
            code: blocks[first].code.clone(),
            spans,
        };
        next.compact();
        let mut index = 0;
        each_mut(&mut self.parts, &mut index, &mut |i, b| {
            if i == first {
                *b = next.clone();
            }
        });
        if first != last {
            let mut index = 0;
            remove_blocks(&mut self.parts, &mut index, first + 1..last + 1);
        }
    }
    pub fn set_mark(&mut self, range: Range<usize>, tag: Tag, enabled: bool) {
        let mut at = 0;
        each_mut(&mut self.parts, &mut 0, &mut |_, b| {
            let len = b.len();
            if b.tag.name == "pre" {
                at += len + 1;
                return;
            }
            let start = range.start.saturating_sub(at).min(len);
            let end = range.end.saturating_sub(at).min(len);
            if start < end {
                let mut spans = b.slice(0..start);
                for mut span in b.slice(start..end) {
                    if enabled && tag.name != "code" && span.marks.iter().any(|m| m.name == "code")
                    {
                        spans.push(span);
                        continue;
                    }
                    span.marks.retain(|m| {
                        m.name != tag.name
                            && (tag.name != "code" || !enabled)
                            && (m.name != "code" || !enabled)
                    });
                    if enabled && span.atom.is_none() {
                        span.marks.push(tag.clone());
                    }
                    spans.push(span);
                }
                spans.extend(b.slice(end..len));
                b.spans = spans;
                b.compact();
            }
            at += len + 1;
        });
    }
    pub fn has_mark(&self, range: Range<usize>, name: &str) -> bool {
        if range.is_empty() {
            return self.marks_at(range.start).iter().any(|m| m.name == name);
        }
        let mut at = 0;
        let mut found = false;
        for b in self.blocks() {
            for s in &b.spans {
                let end = at + s.text.len();
                if at < range.end && end > range.start {
                    if !s.marks.iter().any(|m| m.name == name) {
                        return false;
                    }
                    found = true;
                }
                at = end;
            }
            at += 1;
        }
        found
    }
    pub fn format(&mut self, range: Range<usize>, format: Format) {
        if let Some(name) = format.mark() {
            let enabled = !self.has_mark(range.clone(), name);
            self.set_mark(range, Tag::new(name), enabled);
            return;
        }
        let (first, _) = self.location(range.start);
        let (mut last, end) = self.location(range.end);
        if end == 0 && last > first && !range.is_empty() {
            last -= 1;
        }
        match format {
            Format::Quote | Format::BulletList | Format::OrderedList | Format::TaskList => {
                if format == Format::Quote {
                    let mut index = 0;
                    let mut unwrapped = false;
                    unwrap_selected(
                        &mut self.parts,
                        &mut index,
                        first..last + 1,
                        "blockquote",
                        false,
                        &mut unwrapped,
                    );
                    if !unwrapped {
                        wrap_selected(&mut self.parts, &mut 0, first..last + 1, format);
                    }
                } else if !toggle_list(&mut self.parts, &mut 0, first..last + 1, format) {
                    wrap_selected(&mut self.parts, &mut 0, first..last + 1, format);
                }
            }
            _ => {
                let name = match format {
                    Format::Heading(1) => "h1",
                    Format::Heading(2) => "h2",
                    Format::Heading(3) => "h3",
                    Format::Heading(4) => "h4",
                    Format::Heading(5) => "h5",
                    Format::Heading(6) => "h6",
                    Format::CodeBlock => "pre",
                    _ => "p",
                };
                let same = self.blocks()[first].tag.name == name && format != Format::Paragraph;
                each_mut(&mut self.parts, &mut 0, &mut |i, b| {
                    if i >= first && i <= last {
                        b.tag = Tag::new(if same { "p" } else { name });
                        b.code = if b.tag.name == "pre" {
                            Some(Tag::new("code").attr("class", "language-javascript"))
                        } else {
                            None
                        };
                        if name == "pre" {
                            for s in &mut b.spans {
                                s.marks.clear();
                            }
                        }
                    }
                });
            }
        }
    }
    pub fn append_code_block(&mut self) {
        let block = Block {
            tag: Tag::new("pre"),
            code: Some(Tag::new("code").attr("class", "language-javascript")),
            spans: vec![],
        };
        if matches!(self.parts.last(),Some(Part::Block(b)) if b.tag.name=="p"&&b.len()==0) {
            *self.parts.last_mut().unwrap() = Part::Block(block);
        } else {
            self.parts.push(Part::Block(block));
        }
    }
    pub fn code_block(&mut self, range: Range<usize>) -> Range<usize> {
        let text = self.text()[range.clone()].to_owned();
        let (last, end) = self.location(range.end);
        let (first, start) = self.location(range.start);
        let (prefix, after) = split_parts(&self.parts, &mut 0, last, end);
        let (mut before, _) = split_parts(&prefix, &mut 0, first, start);
        let before_count = count(&before);
        let caret = if before_count == 0 {
            0
        } else {
            Document {
                parts: before.clone(),
            }
            .text()
            .len()
                + 1
        };
        before.push(Part::Block(Block {
            tag: Tag::new("pre"),
            code: Some(Tag::new("code").attr("class", "language-javascript")),
            spans: if text.is_empty() {
                vec![]
            } else {
                vec![Span::text(&text, &[])]
            },
        }));
        before.extend(after);
        self.parts = before;
        caret..caret + text.len()
    }
    pub fn split(&mut self, range: Range<usize>) -> usize {
        let (index, local) = self.location(range.start);
        if let Some(after) = exit_reply_quote(&mut self.parts, &mut 0, index) {
            return self.blocks().iter().take(after).map(|b| b.len() + 1).sum();
        }
        let b = self.blocks()[index];
        if b.tag.name == "pre" {
            if range.is_empty() && local == b.len() && b.text().ends_with("\n\n") {
                self.replace(range.start - 2..range.start, "", &[]);
                let b = self.blocks()[index];
                let left = b.clone();
                let right = Block {
                    tag: Tag::new("p"),
                    code: None,
                    spans: vec![],
                };
                split_block(&mut self.parts, &mut 0, index, &left, &right);
                return range.start - 1;
            }
            self.replace(range.clone(), "\n", &[]);
            return range.start + 1;
        }
        if range.is_empty() && b.len() == 0 && lift_empty(&mut self.parts, &mut 0, index) {
            return range.start;
        }
        self.replace(range.clone(), "", &[]);
        let (index, offset) = self.location(range.start);
        let b = self.blocks()[index];
        let mut left = b.clone();
        left.spans = b.slice(0..offset);
        let mut right = b.clone();
        right.spans = b.slice(offset..b.len());
        if offset == b.len() && b.tag.name.starts_with('h') {
            right.tag = Tag::new("p");
        }
        split_block(&mut self.parts, &mut 0, index, &left, &right);
        range.start + 1
    }
    pub fn link_at(&self, offset: usize) -> Option<String> {
        let mut at = 0;
        for b in self.blocks() {
            for s in &b.spans {
                let end = at + s.text.len();
                if offset >= at && offset < end {
                    return s
                        .marks
                        .iter()
                        .find(|m| m.name == "a")
                        .and_then(|m| {
                            m.attrs
                                .iter()
                                .find(|(k, _)| k == "href")
                                .map(|(_, v)| v.clone())
                        })
                        .filter(|v| safe_link(v));
                }
                at = end;
            }
            at += 1;
        }
        None
    }
    pub fn link_range(&self, range: Range<usize>) -> Range<usize> {
        let Some(url) = self.link_at(range.start) else {
            return range;
        };
        if !range.is_empty() && self.link_at(range.end - 1).as_ref() != Some(&url) {
            return range;
        }
        let mut at = range.start;
        while at > 0 {
            let previous = self.text()[..at]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
            if self.link_at(previous).as_ref() != Some(&url) {
                break;
            }
            at = previous;
        }
        let mut end = range.end;
        let text = self.text();
        while end < text.len() && self.link_at(end).as_ref() == Some(&url) {
            end += text[end..].chars().next().unwrap().len_utf8();
        }
        at..end
    }
    pub fn link(&mut self, range: Range<usize>, href: &str) -> bool {
        if !safe_link(href) {
            return false;
        }
        let tag = Tag::new("a")
            .attr("target", "_blank")
            .attr("rel", "noopener noreferrer nofollow")
            .attr("href", href);
        self.set_mark(range, tag, true);
        true
    }
    pub fn unlink(&mut self, range: Range<usize>) {
        self.set_mark(range, Tag::new("a"), false);
    }
    pub fn is_empty(&self) -> bool {
        fn empty(parts: &[Part]) -> bool {
            parts.iter().all(|p| match p {
                Part::Block(b) => b.text().trim().is_empty(),
                Part::Container(_, p) => empty(p),
                Part::Atom(_) => false,
            })
        }
        empty(&self.parts)
    }
    pub fn insert_html(&mut self, range: Range<usize>, html: &str) -> usize {
        self.replace(range.clone(), "", &[]);
        let (index, local) = self.location(range.start);
        let (mut before, after) = split_parts(&self.parts, &mut 0, index, local);
        before.extend(
            Self::parse(html)
                .parts
                .into_iter()
                .filter(|p| !matches!(p,Part::Block(b) if b.len()==0)),
        );
        let end = Document {
            parts: before.clone(),
        }
        .text()
        .len();
        before.push(Part::Block(Block {
            tag: Tag::new("p"),
            code: None,
            spans: vec![],
        }));
        before.extend(after);
        self.parts = before;
        if count(&self.parts) > 1 { end + 1 } else { 0 }
    }
    pub fn append_html(&mut self, html: &str) {
        if self.text().is_empty() && self.parts.len() == 1 {
            self.parts.clear();
        }
        self.parts.extend(Self::parse(html).parts);
        self.parts.push(Part::Block(Block {
            tag: Tag::new("p"),
            code: None,
            spans: vec![],
        }));
    }
    pub fn toggle_task(&mut self, block: usize) {
        fn toggle(parts: &mut [Part], at: &mut usize, target: usize) -> bool {
            for p in parts {
                match p {
                    Part::Block(_) => *at += 1,
                    Part::Container(tag, children) => {
                        let start = *at;
                        let hit = toggle(children, at, target);
                        if hit {
                            return true;
                        }
                        if target >= start && target < *at && tag.name == "li" {
                            if let Some((_, v)) =
                                tag.attrs.iter_mut().find(|(k, _)| k == "data-checked")
                            {
                                *v = if v == "true" { "false" } else { "true" }.into();
                                return true;
                            }
                        }
                    }
                    _ => (),
                }
            }
            false
        }
        toggle(&mut self.parts, &mut 0, block);
    }
}
fn exit_reply_quote(parts: &mut Vec<Part>, index: &mut usize, target: usize) -> Option<usize> {
    let mut found = None;
    let mut out = vec![];
    for mut p in parts.drain(..) {
        if found == Some(*index)
            && matches!(&p,Part::Block(b) if b.len()==0)
            && matches!(out.last(),Some(Part::Block(b)) if b.len()==0)
        {
            *index += 1;
            continue;
        }
        match &mut p {
            Part::Block(_) => *index += 1,
            Part::Container(tag, children) => {
                let end = *index + count(children);
                if tag.name == "blockquote"
                    && tag
                        .attrs
                        .iter()
                        .any(|(k, v)| k == "data-quote-reply" && v == "true")
                    && target >= *index
                    && target < end
                {
                    *index = end;
                    found = Some(end);
                    out.push(p);
                    out.push(Part::Block(Block {
                        tag: Tag::new("p"),
                        code: None,
                        spans: vec![],
                    }));
                    continue;
                }
                found = exit_reply_quote(children, index, target).or(found);
            }
            _ => (),
        }
        out.push(p);
    }
    *parts = out;
    found
}
fn lift_empty(parts: &mut Vec<Part>, index: &mut usize, target: usize) -> bool {
    let mut out = vec![];
    let mut changed = false;
    for mut p in parts.drain(..) {
        match &mut p {
            Part::Block(_) => *index += 1,
            Part::Container(tag, children) => {
                let start = *index;
                let end = start + count(children);
                let wrapper = matches!(tag.name.as_str(), "ul" | "ol" | "blockquote");
                let mut at = start;
                let child = if wrapper {
                    children.iter().position(|p| {
                        let n = count(std::slice::from_ref(p));
                        let here = at;
                        at += n;
                        here == target
                            && n == 1
                            && match p {
                                Part::Block(b) => b.len() == 0,
                                Part::Container(li, p) => {
                                    li.name == "li"
                                        && matches!(p.as_slice(),[Part::Block(b)] if b.len()==0)
                                }
                                _ => false,
                            }
                    })
                } else {
                    None
                };
                if let Some(child) = child {
                    let after = children.split_off(child + 1);
                    children.pop();
                    if !children.is_empty() {
                        out.push(Part::Container(tag.clone(), std::mem::take(children)));
                    }
                    out.push(Part::Block(Block {
                        tag: Tag::new("p"),
                        code: None,
                        spans: vec![],
                    }));
                    if !after.is_empty() {
                        out.push(Part::Container(tag.clone(), after));
                    }
                    *index = end;
                    changed = true;
                    continue;
                }
                changed = lift_empty(children, index, target) || changed;
            }
            _ => (),
        }
        out.push(p);
    }
    *parts = out;
    changed
}
fn split_parts(
    parts: &[Part],
    index: &mut usize,
    target: usize,
    offset: usize,
) -> (Vec<Part>, Vec<Part>) {
    let mut before = vec![];
    let mut after = vec![];
    for p in parts {
        match p {
            Part::Block(b) => {
                if *index < target {
                    before.push(p.clone());
                } else if *index > target {
                    after.push(p.clone());
                } else {
                    if offset > 0 {
                        let mut left = b.clone();
                        left.spans = b.slice(0..offset);
                        before.push(Part::Block(left));
                    }
                    if offset < b.len() {
                        let mut right = b.clone();
                        right.spans = b.slice(offset..b.len());
                        after.push(Part::Block(right));
                    }
                }
                *index += 1;
            }
            Part::Container(tag, children) => {
                let (left, right) = split_parts(children, index, target, offset);
                if !left.is_empty() {
                    before.push(Part::Container(tag.clone(), left));
                }
                if !right.is_empty() {
                    after.push(Part::Container(tag.clone(), right));
                }
            }
            Part::Atom(_) => {
                if *index <= target {
                    before.push(p.clone());
                } else {
                    after.push(p.clone());
                }
            }
        }
    }
    (before, after)
}
fn remove_blocks(parts: &mut Vec<Part>, index: &mut usize, range: Range<usize>) {
    parts.retain_mut(|p| match p {
        Part::Block(_) => {
            let keep = !range.contains(index);
            *index += 1;
            keep
        }
        Part::Container(_, p) => {
            remove_blocks(p, index, range.clone());
            !p.is_empty()
        }
        _ => true,
    });
}
fn count(parts: &[Part]) -> usize {
    let mut b = vec![];
    visit(parts, &mut b);
    b.len()
}
fn list_tag(format: Format) -> Tag {
    match format {
        Format::OrderedList => Tag::new("ol").attr("class", "numbered-list"),
        Format::TaskList => Tag::new("ul")
            .attr("class", "taskList")
            .attr("data-type", "taskList"),
        _ => Tag::new("ul"),
    }
}
fn item_tag(format: Format) -> Tag {
    if format == Format::TaskList {
        Tag::new("li")
            .attr("data-type", "taskItem")
            .attr("data-checked", "false")
    } else {
        Tag::new("li")
    }
}
fn toggle_list(
    parts: &mut Vec<Part>,
    index: &mut usize,
    range: Range<usize>,
    format: Format,
) -> bool {
    let mut out = vec![];
    let mut changed = false;
    for mut part in parts.drain(..) {
        match &mut part {
            Part::Block(_) => *index += 1,
            Part::Container(tag, children) => {
                let start = *index;
                let end = start + count(children);
                if !changed
                    && matches!(tag.name.as_str(), "ul" | "ol")
                    && start <= range.start
                    && end >= range.end
                {
                    let task = tag
                        .attrs
                        .iter()
                        .any(|(k, v)| k == "data-type" && v == "taskList");
                    let same = if format == Format::TaskList {
                        task
                    } else if format == Format::OrderedList {
                        tag.name == "ol"
                    } else {
                        tag.name == "ul" && !task
                    };
                    if same {
                        let mut before = vec![];
                        let mut after = vec![];
                        let mut lifted = vec![];
                        let mut at = start;
                        for item in children.drain(..) {
                            let end = at + count(std::slice::from_ref(&item));
                            if end <= range.start {
                                before.push(item);
                            } else if at >= range.end {
                                after.push(item);
                            } else {
                                match item {
                                    Part::Container(li, p) if li.name == "li" => lifted.extend(p),
                                    other => lifted.push(other),
                                }
                            }
                            at = end;
                        }
                        if !before.is_empty() {
                            out.push(Part::Container(tag.clone(), before));
                        }
                        out.extend(lifted);
                        if !after.is_empty() {
                            out.push(Part::Container(tag.clone(), after));
                        }
                        *index = end;
                        changed = true;
                        continue;
                    } else {
                        *tag = list_tag(format);
                        for child in children.iter_mut() {
                            if let Part::Container(li, _) = child {
                                if li.name == "li" {
                                    *li = item_tag(format);
                                }
                            }
                        }
                        *index = end;
                        changed = true;
                    }
                } else {
                    changed = toggle_list(children, index, range.clone(), format) || changed;
                }
            }
            _ => (),
        }
        out.push(part);
    }
    *parts = out;
    changed
}
fn unwrap_selected(
    parts: &mut Vec<Part>,
    index: &mut usize,
    range: Range<usize>,
    name: &str,
    task: bool,
    changed: &mut bool,
) {
    let mut out = vec![];
    for mut part in parts.drain(..) {
        match &mut part {
            Part::Block(_) => *index += 1,
            Part::Container(tag, children) => {
                let end = *index + count(children);
                let is_task = tag
                    .attrs
                    .iter()
                    .any(|(k, v)| k == "data-type" && v == "taskList");
                if tag.name == name && is_task == task && *index <= range.start && end >= range.end
                {
                    let start = *index;
                    *index = end;
                    *changed = true;
                    let mut before = vec![];
                    let mut after = vec![];
                    let mut lifted = vec![];
                    let mut at = start;
                    for p in children.drain(..) {
                        let stop = at + count(std::slice::from_ref(&p));
                        if stop <= range.start {
                            before.push(p);
                        } else if at >= range.end {
                            after.push(p);
                        } else {
                            lifted.push(p);
                        }
                        at = stop;
                    }
                    if !before.is_empty() {
                        out.push(Part::Container(tag.clone(), before));
                    }
                    out.extend(lifted);
                    if !after.is_empty() {
                        out.push(Part::Container(tag.clone(), after));
                    }
                    continue;
                }
                unwrap_selected(children, index, range.clone(), name, task, changed);
            }
            _ => (),
        }
        out.push(part);
    }
    *parts = out;
}
fn wrap_selected(parts: &mut Vec<Part>, index: &mut usize, range: Range<usize>, format: Format) {
    let mut out = vec![];
    let mut selected = vec![];
    fn flush(out: &mut Vec<Part>, selected: &mut Vec<Part>, format: Format) {
        if selected.is_empty() {
            return;
        }
        let mut children = std::mem::take(selected);
        let tag = if format == Format::Quote {
            Tag::new("blockquote")
        } else {
            list_tag(format)
        };
        if format != Format::Quote {
            children = children
                .into_iter()
                .map(|p| {
                    let li = item_tag(format);
                    Part::Container(li, vec![p])
                })
                .collect();
        }
        out.push(Part::Container(tag, children));
    }
    for mut part in parts.drain(..) {
        match &mut part {
            Part::Block(_) => {
                let selected_here = range.contains(index);
                *index += 1;
                if selected_here {
                    selected.push(part);
                    continue;
                }
            }
            Part::Container(_, p) => wrap_selected(p, index, range.clone(), format),
            _ => (),
        }
        flush(&mut out, &mut selected, format);
        out.push(part);
    }
    flush(&mut out, &mut selected, format);
    *parts = out;
}
fn split_block(parts: &mut Vec<Part>, at: &mut usize, target: usize, left: &Block, right: &Block) {
    let mut out = vec![];
    for mut part in parts.drain(..) {
        match &mut part {
            Part::Block(_) => {
                let hit = *at == target;
                *at += 1;
                if hit {
                    out.push(Part::Block(left.clone()));
                    out.push(Part::Block(right.clone()));
                    continue;
                }
            }
            Part::Container(tag, p) => {
                let start = *at;
                split_block(p, at, target, left, right);
                if tag.name == "li" && target >= start && target < *at {
                    // Enter in a list creates a sibling item, not a second paragraph in the item.
                    let mut seen = 0;
                    let split = p
                        .iter()
                        .position(|p| {
                            let hit = seen == target - start + 1;
                            seen += count(std::slice::from_ref(p));
                            hit
                        })
                        .unwrap_or(p.len());
                    if split < p.len() {
                        let next = p.split_off(split);
                        out.push(Part::Container(tag.clone(), std::mem::take(p)));
                        let mut next_tag = tag.clone();
                        if let Some((_, v)) =
                            next_tag.attrs.iter_mut().find(|(k, _)| k == "data-checked")
                        {
                            *v = "false".into();
                        }
                        out.push(Part::Container(next_tag, next));
                        continue;
                    }
                }
            }
            _ => (),
        }
        out.push(part);
    }
    *parts = out;
}
pub fn safe_link(href: &str) -> bool {
    let lower = href.trim().to_ascii_lowercase();
    !lower.chars().any(char::is_control)
        && (lower.starts_with("https://")
            || lower.starts_with("http://")
            || lower.starts_with("mailto:")
            || lower.starts_with("tel:")
            || lower.starts_with('/')
            || lower.starts_with('#'))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn web_marks_selection_unicode_and_html_round_trip() {
        for (format, tag) in [
            (Format::Bold, "strong"),
            (Format::Italic, "em"),
            (Format::Underline, "u"),
            (Format::Highlight, "mark"),
            (Format::Code, "code"),
        ] {
            let mut doc = Document::plain("a é🙂 z");
            doc.format(2..8, format);
            assert_eq!(doc.html(), format!("<p>a <{tag}>é🙂</{tag}> z</p>"));
            assert_eq!(Document::parse(&doc.html()), doc);
            doc.format(2..8, format);
            assert_eq!(doc.html(), "<p>a é🙂 z</p>");
        }
    }
    #[test]
    fn web_blocks_lists_and_heading_round_trip() {
        for level in 1..=6 {
            let mut d = Document::plain("Heading");
            d.format(0..7, Format::Heading(level));
            assert_eq!(d.html(), format!("<h{level}>Heading</h{level}>"));
            assert_eq!(Document::parse(&d.html()), d);
            d.format(0..7, Format::Paragraph);
            assert_eq!(d.html(), "<p>Heading</p>");
        }
        for (f, html) in [
            (Format::Quote, "<blockquote><p>one</p></blockquote>"),
            (Format::BulletList, "<ul><li><p>one</p></li></ul>"),
            (
                Format::OrderedList,
                "<ol class=\"numbered-list\"><li><p>one</p></li></ol>",
            ),
        ] {
            let mut d = Document::plain("one");
            d.format(0..3, f);
            assert_eq!(d.html(), html);
            assert_eq!(Document::parse(html).html(), html);
            d.format(0..3, f);
            assert_eq!(d.html(), "<p>one</p>");
        }
        let mut d = Document::plain("one");
        d.format(0..3, Format::TaskList);
        let html = d.html();
        assert!(html.contains("data-type=\"taskList\""));
        assert!(html.contains("data-checked=\"false\""));
        assert_eq!(Document::parse(&html).text(), "one");
        d.toggle_task(0);
        assert!(d.html().contains("data-checked=\"true\""));
        let mut d = Document::plain("let x = 1;\nnext");
        d.format(0..0, Format::CodeBlock);
        assert_eq!(
            d.html(),
            "<pre><code class=\"language-javascript\">let x = 1;\nnext</code></pre>"
        );
        assert_eq!(Document::parse(&d.html()).html(), d.html());
    }
    #[test]
    fn replacing_across_blocks_keeps_unrelated_links_marks_media_and_quote_attributes() {
        let mut d = Document::parse(
            "<blockquote data-quote-reply=\"true\"><p>Hello <strong>é🙂</strong></p></blockquote><p><a href=\"https://example.com\">link</a><img src=\"https://example.com/a.png\"></p>",
        );
        d.replace(0..5, "Goodbye", &[]);
        assert!(d.html().contains("<strong>é🙂</strong>"));
        assert!(d.html().contains("data-quote-reply=\"true\""));
        assert!(d.html().contains("<img src="));
        assert!(d.html().contains("href=\"https://example.com\""));
        let mut d = Document::parse("<p>one</p><p>two</p>");
        d.replace(2..5, "X", &[]);
        assert_eq!(d.html(), "<p>onXwo</p>");
        assert_eq!(d.text(), "onXwo");
    }
    #[test]
    fn links_validate_schemes_and_preserve_web_attributes() {
        let mut d = Document::plain("link");
        assert!(!d.link(0..4, "javascript:alert(1)"));
        assert!(d.link(0..4, "https://example.com?a=1&b=2"));
        assert!(d.html().contains("target=\"_blank\""));
        assert!(d.html().contains("rel=\"noopener noreferrer nofollow\""));
        assert_eq!(Document::parse(&d.html()).html(), d.html());
        d.unlink(0..4);
        assert_eq!(d.html(), "<p>link</p>");
    }
    #[test]
    fn enter_splits_paragraph_and_list_items_while_hard_break_stays_inline() {
        let mut d = Document::plain("onetwo");
        d.split(3..3);
        assert_eq!(d.html(), "<p>one</p><p>two</p>");
        let mut d = Document::parse("<ul><li><p>onetwo</p></li></ul>");
        d.split(3..3);
        assert_eq!(d.html(), "<ul><li><p>one</p></li><li><p>two</p></li></ul>");
        let mut d = Document::plain("one");
        d.replace(3..3, "\ntwo", &[]);
        assert_eq!(d.html(), "<p>one<br>two</p>");
    }
    #[test]
    fn imported_inline_content_and_task_items_are_lossless_not_extra_paragraphs() {
        let d = Document::parse("<ul><li>one <strong>two</strong> three</li></ul>");
        assert_eq!(
            d.html(),
            "<ul><li><p>one <strong>two</strong> three</p></li></ul>"
        );
        assert_eq!(d.text(), "one two three");
        let mut d = Document::plain("task");
        d.format(0..4, Format::TaskList);
        let html = d.html();
        assert_eq!(Document::parse(&html).html(), html);
        assert_eq!(Document::parse(&html), d);
        let html = "<pre><code class=\"language-rust\">fn main() {}</code></pre>";
        assert_eq!(Document::parse(html).html(), html);
    }

    #[test]
    fn list_conversion_and_partial_lifting_do_not_nest_or_destroy_other_items() {
        let mut d = Document::parse("<ul><li><p>one</p></li><li><p>two</p></li></ul>");
        d.format(0..0, Format::OrderedList);
        assert_eq!(
            d.html(),
            "<ol class=\"numbered-list\"><li><p>one</p></li><li><p>two</p></li></ol>"
        );
        d.format(4..7, Format::OrderedList);
        assert_eq!(
            d.html(),
            "<ol class=\"numbered-list\"><li><p>one</p></li></ol><p>two</p>"
        );
        let mut d = Document::parse("<blockquote><p>one</p><p>two</p><p>three</p></blockquote>");
        d.format(4..7, Format::Quote);
        assert_eq!(
            d.html(),
            "<blockquote><p>one</p></blockquote><p>two</p><blockquote><p>three</p></blockquote>"
        );
    }
    #[test]
    fn code_override_replaces_only_selected_text_and_coalesces_multiple_blocks() {
        let mut d = Document::parse("<p>hello world!</p>");
        d.code_block(6..11);
        assert_eq!(
            d.html(),
            "<p>hello </p><pre><code class=\"language-javascript\">world</code></pre><p>!</p>"
        );
        let mut d = Document::parse("<p>one</p><p>two</p>");
        d.code_block(0..7);
        assert_eq!(
            d.html(),
            "<pre><code class=\"language-javascript\">one\ntwo</code></pre>"
        );
        let mut d = Document::parse("<ul><li><p>one</p></li><li><p>two</p></li></ul>");
        d.code_block(0..7);
        assert_eq!(
            d.html(),
            "<pre><code class=\"language-javascript\">one\ntwo</code></pre>"
        );
    }
    #[test]
    fn enter_exits_empty_lists_and_quotes_and_preserves_code_newlines() {
        let mut d = Document::parse("<ul><li><p>aa</p></li></ul>");
        d.split(1..1);
        assert_eq!(d.html(), "<ul><li><p>a</p></li><li><p>a</p></li></ul>");
        let mut d = Document::parse("<ul><li><p>one</p></li><li><p></p></li></ul>");
        assert_eq!(d.split(4..4), 4);
        assert_eq!(d.html(), "<ul><li><p>one</p></li></ul><p></p>");
        let mut d = Document::parse("<blockquote><p>quote</p><p></p></blockquote>");
        d.split(6..6);
        assert_eq!(d.html(), "<blockquote><p>quote</p></blockquote><p></p>");
        let mut d = Document::parse("<pre><code>one</code></pre>");
        assert_eq!(d.split(3..3), 4);
        assert_eq!(d.text(), "one\n");
        d.split(4..4);
        assert_eq!(d.text(), "one\n\n");
        assert_eq!(d.split(5..5), 4);
        assert_eq!(d.html(), "<pre><code>one</code></pre><p></p>");
        let mut d =
            Document::parse("<blockquote data-quote-reply=\"true\"><p>quote</p></blockquote>");
        assert_eq!(d.split(2..2), 6);
        assert_eq!(
            d.html(),
            "<blockquote data-quote-reply=\"true\"><p>quote</p></blockquote><p></p>"
        );
    }
    #[test]
    fn unicode_edit_ranges_keep_the_document_text_invariant_and_html_round_trip() {
        for html in [
            "<p>é🙂 alpha</p><p>漢字 beta</p>",
            "<ul><li><p><strong>one</strong></p></li><li><p>two</p></li></ul>",
            "<blockquote><p>quote<br>line</p></blockquote>",
        ] {
            let original = Document::parse(html);
            let text = original.text();
            let bounds = text
                .char_indices()
                .map(|(i, _)| i)
                .chain(std::iter::once(text.len()))
                .collect::<Vec<_>>();
            for &start in &bounds {
                for &end in bounds.iter().filter(|&&end| end >= start) {
                    let mut d = original.clone();
                    d.replace(start..end, "Ω🙂", &[]);
                    let expected = format!("{}Ω🙂{}", &text[..start], &text[end..]);
                    assert_eq!(d.text(), expected, "{html}: {start}..{end}");
                    assert_eq!(Document::parse(&d.html()).text(), expected);
                }
            }
        }
    }
    #[test]
    fn code_blocks_do_not_accept_inline_marks_or_autolinks() {
        let mut d = Document::parse(
            "<pre><code class=\"language-javascript\">https://example.com</code></pre>",
        );
        let before = d.html();
        let len = d.text().len();
        for format in [
            Format::Bold,
            Format::Italic,
            Format::Underline,
            Format::Highlight,
            Format::Code,
        ] {
            d.format(0..len, format);
            assert_eq!(d.html(), before);
        }
        d.link(0..len, "https://example.com");
        assert_eq!(d.html(), before);
    }
}
