use crate::{theme::ThemeName, ui::Screen};
use gpui::{
    AnyElement, App, Bounds, Element, ElementId, GlobalElementId, Hsla, InspectorElementId,
    IntoElement, LayoutId, Pixels, Size, StyleRefinement, Styled, Window,
};
use serde_json::{Value, json};
use std::{cell::RefCell, collections::BTreeMap, path::Path, rc::Rc};

#[derive(Default)]
pub struct LayoutDump {
    elements: BTreeMap<String, Value>,
    active_tab: usize,
}

pub type Capture = Option<Rc<RefCell<LayoutDump>>>;

impl LayoutDump {
    pub fn begin(&mut self, active_tab: usize) {
        self.elements.clear();
        self.active_tab = active_tab;
    }

    fn record(&mut self, key: &str, mut element: Value) {
        if key == format!("inbox.tab.{}", self.active_tab) {
            let mut active = element.clone();
            active["key"] = json!("inbox.tab.active");
            self.elements.insert("inbox.tab.active".into(), active);
        }
        element["key"] = json!(key);
        self.elements.insert(key.into(), element);
    }

    fn json(&self, screen: Screen, theme: ThemeName, size: Size<Pixels>) -> Value {
        let keys = keys(screen);
        json!({
            "screen": match screen { Screen::Inbox => "inbox", Screen::Board => "board", Screen::Task => "task" },
            "theme": match theme { ThemeName::Porcelain => "porcelain", ThemeName::Graphite => "graphite", ThemeName::Amoled => "amoled", ThemeName::Dia => "dia" },
            "width": f32::from(size.width),
            "height": f32::from(size.height),
            "elements": keys.iter().filter_map(|key| self.elements.get(key)).collect::<Vec<_>>(),
            "missing": keys.iter().filter(|key| !self.elements.contains_key(*key)).collect::<Vec<_>>(),
        })
    }

    pub fn write(
        &self,
        path: &Path,
        screen: Screen,
        theme: ThemeName,
        size: Size<Pixels>,
    ) -> anyhow::Result<()> {
        std::fs::write(
            path,
            serde_json::to_vec_pretty(&self.json(screen, theme, size))?,
        )?;
        Ok(())
    }
}

fn keys(screen: Screen) -> Vec<String> {
    let mut keys: Vec<String> = ["rail", "main", "rightpanel", "footer", "footer.text"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    keys.extend((0..13).map(|i| format!("rail.icon.{i}")));
    let (single, repeated): (&[&str], &[(&str, usize)]) = match screen {
        Screen::Inbox => (
            &[
                "inbox.tabs",
                "inbox.tab.active",
                "inbox.section",
                "inbox.row.0.author",
                "inbox.row.0.taskid",
                "inbox.row.0.title",
                "inbox.row.0.kind",
                "inbox.row.0.actions",
            ],
            &[("inbox.tab", 6), ("inbox.row", 5)],
        ),
        Screen::Board => (
            &[
                "board.header",
                "board.title",
                "board.count",
                "board.toolbar",
                "board.column.0.title",
                "board.column.0.count",
                "board.column.0.add",
                "board.card.0.id",
                "board.card.0.title",
                "board.card.0.badge",
                "board.card.0.label",
                "board.card.0.meta",
            ],
            &[("board.column", 4), ("board.card", 5)],
        ),
        Screen::Task => (
            &[
                "task.header",
                "task.title",
                "task.id",
                "task.description",
                "task.comment.0",
                "task.comment.0.author",
                "task.comment.0.body",
                "task.composer",
            ],
            &[("task.meta", 6)],
        ),
    };
    keys.extend(single.iter().map(|s| (*s).to_owned()));
    for (prefix, count) in repeated {
        keys.extend((0..*count).map(|i| format!("{prefix}.{i}")));
    }
    keys
}

fn hex(color: Hsla) -> String {
    let rgb = color.to_rgb();
    format!(
        "#{:02x}{:02x}{:02x}",
        (rgb.r * 255.).round() as u8,
        (rgb.g * 255.).round() as u8,
        (rgb.b * 255.).round() as u8
    )
}

pub trait LayoutKey: Styled + IntoElement + Sized {
    fn layout_key(
        mut self,
        capture: &Capture,
        key: impl Into<String>,
        text: Option<String>,
    ) -> AnyElement {
        let Some(capture) = capture else {
            return self.into_any_element();
        };
        let style = self.style().clone();
        LayoutElement {
            element: self.into_any_element(),
            capture: capture.clone(),
            key: key.into(),
            style,
            text,
        }
        .into_any_element()
    }
}

impl<T: Styled + IntoElement> LayoutKey for T {}

struct LayoutElement {
    element: AnyElement,
    capture: Rc<RefCell<LayoutDump>>,
    key: String,
    style: StyleRefinement,
    text: Option<String>,
}

impl LayoutElement {
    fn json(&self, bounds: Bounds<Pixels>, text: gpui::TextStyle, rem: Pixels) -> Value {
        let font_size = text.font_size.to_pixels(rem);
        let bg = self
            .style
            .background
            .as_ref()
            .and_then(|bg| bg.as_ref())
            .and_then(|bg| bg.color())
            .and_then(|bg| bg.as_solid())
            .filter(|c| !c.is_transparent())
            .map(hex);
        json!({
            "key": self.key,
            "x": f32::from(bounds.origin.x), "y": f32::from(bounds.origin.y),
            "w": f32::from(bounds.size.width), "h": f32::from(bounds.size.height),
            "color": hex(text.color), "bg": bg,
            "fontSize": f32::from(font_size), "fontWeight": text.font_weight.0,
            "lineHeight": f32::from(text.line_height.to_pixels(font_size.into(), rem)),
            "radius": self.style.corner_radii.top_left.map(|r| f32::from(r.to_pixels(rem))),
            "borderColor": self.style.border_color.filter(|c| !c.is_transparent()).map(hex),
            "text": self.text.as_ref().map(|s| s.chars().take(40).collect::<String>()),
        })
    }
}

impl IntoElement for LayoutElement {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl Element for LayoutElement {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        (self.element.request_layout(window, cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.element.prepaint(window, cx);
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.element.paint(window, cx);
        let element = window.with_text_style(Some(self.style.text.clone()), |window| {
            self.json(bounds, window.text_style(), window.rem_size())
        });
        self.capture.borrow_mut().record(&self.key, element);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{px, size};

    #[test]
    fn layout_json_matches_shared_shape() {
        let mut dump = LayoutDump::default();
        let mut style = gpui::div()
            .rounded(px(5.))
            .border_color(gpui::rgb(0x33373e));
        let element = LayoutElement {
            element: gpui::div().into_any_element(),
            capture: Rc::new(RefCell::new(LayoutDump::default())),
            key: "inbox.tab.0".into(),
            style: style.style().clone(),
            text: Some("界".repeat(45)),
        };
        let value = element.json(
            Bounds::new(gpui::point(px(12.5), px(24.)), size(px(80.), px(21.))),
            gpui::TextStyle {
                color: gpui::rgb(0xffffff).into(),
                font_size: px(14.).into(),
                font_weight: gpui::FontWeight::MEDIUM,
                line_height: gpui::relative(1.5),
                ..Default::default()
            },
            px(16.),
        );
        assert_eq!(value["x"], 12.5);
        assert_eq!(value["y"], 24.);
        assert_eq!(value["w"], 80.);
        assert_eq!(value["h"], 21.);
        assert_eq!(value["color"], "#ffffff");
        assert_eq!(value["fontSize"], 14.);
        assert_eq!(value["fontWeight"], 500.);
        assert_eq!(value["lineHeight"], 21.);
        assert_eq!(value["radius"], 5.);
        assert_eq!(value["borderColor"], "#33373e");
        assert_eq!(value["text"].as_str().unwrap().chars().count(), 40);
        dump.record("inbox.tab.0", value);
        let value = dump.json(
            Screen::Inbox,
            ThemeName::Graphite,
            size(px(1440.), px(900.)),
        );
        assert_eq!(value.as_object().unwrap().len(), 6);
        assert_eq!(value["screen"], "inbox");
        assert_eq!(value["theme"], "graphite");
        assert_eq!(value["width"], 1440.);
        assert_eq!(value["height"], 900.);
        let elements = value["elements"].as_array().unwrap();
        assert_eq!(elements.len(), 2);
        assert_eq!(elements[0]["key"], "inbox.tab.active");
        assert_eq!(elements[1]["key"], "inbox.tab.0");
        let fields = [
            "key",
            "x",
            "y",
            "w",
            "h",
            "color",
            "bg",
            "fontSize",
            "fontWeight",
            "lineHeight",
            "radius",
            "borderColor",
            "text",
        ];
        for element in elements {
            assert_eq!(element.as_object().unwrap().len(), fields.len());
            assert!(fields.iter().all(|field| element.get(field).is_some()));
            assert!(element["bg"].is_null());
        }
        for screen in [Screen::Inbox, Screen::Board, Screen::Task] {
            let value = dump.json(screen, ThemeName::Graphite, size(px(1440.), px(900.)));
            let missing = value["missing"].as_array().unwrap();
            let elements = value["elements"].as_array().unwrap();
            for key in keys(screen) {
                assert_eq!(
                    usize::from(missing.contains(&json!(key)))
                        + elements.iter().filter(|e| e["key"] == key).count(),
                    1
                );
            }
        }
        dump.begin(1);
        assert!(dump.elements.is_empty());
    }
}
