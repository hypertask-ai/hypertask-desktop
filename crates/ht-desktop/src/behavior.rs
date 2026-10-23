//! Read-only interaction evidence for the isolated, offline behavior runner.
use crate::{layout::LayoutDump, theme::ThemeName, ui::Screen};
use gpui::Window;
use serde_json::Value;
use std::{cell::RefCell, path::PathBuf, rc::Rc};

pub fn snapshot(
    window: &Window,
    path: PathBuf,
    capture: Rc<RefCell<LayoutDump>>,
    screen: Screen,
    theme: ThemeName,
    state: Value,
) {
    window.on_next_frame(move |window, _| {
        let capture = capture.borrow();
        let result = capture.write(&path, screen, theme, window.viewport_size()).and_then(|()| {
            let value = serde_json::json!({
                "state": state,
                "elements": capture.interaction_elements(),
            });
            std::fs::write(path.with_extension("behavior.json"), serde_json::to_vec_pretty(&value)?)?;
            Ok(())
        });
        if let Err(error) = result {
            eprintln!("Hypertask: interaction snapshot failed: {error:#}");
        }
    });
}
