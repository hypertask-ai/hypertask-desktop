use crate::theme::ThemeName;
use gpui::{App, Global};
use std::borrow::Cow;

#[derive(Default)]
struct LoadedFonts {
    base: bool,
    dia: bool,
    bytes: usize,
    faces: usize,
}
impl Global for LoadedFonts {}

pub fn load(theme: ThemeName, cx: &mut App) {
    if !cx.has_global::<LoadedFonts>() {
        cx.set_global(LoadedFonts::default());
    }
    let loaded = cx.global::<LoadedFonts>();
    let base = !loaded.base;
    let dia = !loaded.dia && (theme == ThemeName::Dia || !cfg!(feature = "lazy-theme-fonts"));
    let mut fonts: Vec<Cow<'static, [u8]>> = vec![];
    if base {
        fonts.extend([
            Cow::Borrowed(include_bytes!("../assets/fonts/IBMPlexSans-400.ttf").as_slice()),
            Cow::Borrowed(include_bytes!("../assets/fonts/IBMPlexSans-500.ttf").as_slice()),
            Cow::Borrowed(include_bytes!("../assets/fonts/IBMPlexSans-600.ttf").as_slice()),
            Cow::Borrowed(include_bytes!("../assets/fonts/IBMPlexSans-700.ttf").as_slice()),
        ]);
    }
    if dia {
        fonts.extend([
            Cow::Borrowed(include_bytes!("../assets/fonts/Inter.ttf").as_slice()),
            Cow::Borrowed(include_bytes!("../assets/fonts/Inter-500.ttf").as_slice()),
            Cow::Borrowed(include_bytes!("../assets/fonts/Inter-600.ttf").as_slice()),
            Cow::Borrowed(include_bytes!("../assets/fonts/Inter-700.ttf").as_slice()),
        ]);
    }
    if base {
        fonts.push(Cow::Borrowed(include_bytes!("../assets/fonts/LiberationMono-Bold.ttf")));
    }
    if dia {
        fonts.push(Cow::Borrowed(include_bytes!("../assets/fonts/Newsreader-Italic.ttf")));
    }
    if !fonts.is_empty() {
        let bytes: usize = fonts.iter().map(|font| font.len()).sum();
        let faces = fonts.len();
        cx.text_system().add_fonts(fonts).expect("load embedded fonts");
        let loaded = cx.global_mut::<LoadedFonts>();
        loaded.base |= base;
        loaded.dia |= dia;
        loaded.bytes += bytes;
        loaded.faces += faces;
        if let Some(path) = std::env::var_os("GPUI_MEMORY_RECEIPT") {
            let path = std::path::PathBuf::from(path).with_extension("fonts.json");
            let receipt = serde_json::json!({"RegisteredFontBytes": loaded.bytes, "RegisteredFontFaces": loaded.faces});
            if let Err(error) = std::fs::write(path, receipt.to_string()) {
                eprintln!("Hypertask: font memory measurement failed: {error}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[gpui::test]
    fn theme_fonts_load_before_use_and_repeated_switches_do_not_reload(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| {
            load(ThemeName::Graphite, cx);
            assert!(cx.global::<LoadedFonts>().base);
            assert_eq!(cx.global::<LoadedFonts>().dia, !cfg!(feature = "lazy-theme-fonts"));
            load(ThemeName::Porcelain, cx);
            load(ThemeName::Amoled, cx);
            assert_eq!(cx.global::<LoadedFonts>().dia, !cfg!(feature = "lazy-theme-fonts"));
            load(ThemeName::Dia, cx);
            assert!(cx.global::<LoadedFonts>().dia);
            load(ThemeName::Graphite, cx);
            load(ThemeName::Dia, cx);
            assert!(cx.global::<LoadedFonts>().base && cx.global::<LoadedFonts>().dia);
            assert_eq!(cx.global::<LoadedFonts>().faces, 10);
        });
    }
}
