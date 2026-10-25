#![cfg_attr(windows, windows_subsystem = "windows")]

mod ai_sidebar;
mod behavior;
mod command_center;
mod footer;
mod fonts;
mod input;
mod editor_document;
mod emoji;
mod layout;
mod memory;
mod rich_text;
mod shortcuts;
mod signin;
mod store;
mod theme;
mod ui;
#[cfg(windows)]
mod updater;

use anyhow::{Context as _, Result, bail};
use gpui::{
    App, AssetSource, Bounds, SharedString, TitlebarOptions, WindowBounds, WindowDecorations,
    WindowOptions, point, prelude::*, px, size,
};
use std::{borrow::Cow, path::PathBuf};
use theme::ThemeName;
use ui::{Desktop, Screen};

#[derive(Clone)]
pub struct Options {
    pub db: PathBuf,
    pub offline: bool,
    pub background: bool,
    pub ai_sidebar: bool,
    pub screen: Screen,
    pub settings: bool,
    pub board: i64,
    pub board_given: bool,
    pub task: i64,
    pub theme: ThemeName,
    pub exit_after_ready: bool,
    pub dump_layout: Option<PathBuf>,
    pub window_size: (u32, u32),
    pub print_memory: bool,
    pub bench_input: usize,
}

impl Options {
    fn parse(args: impl IntoIterator<Item = String>) -> Result<Self> {
        let mut result = Self {
            db: dirs::data_dir()
                .context("OS app-data directory is unavailable")?
                .join("Hypertask/local.db"),
            offline: false,
            background: false,
            ai_sidebar: false,
            screen: Screen::Inbox,
            settings: false,
            board: 15,
            board_given: false,
            task: 64897,
            theme: ThemeName::Porcelain,
            exit_after_ready: false,
            dump_layout: None,
            window_size: (1440, 900),
            print_memory: false,
            bench_input: 0,
        };
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--offline" => result.offline = true,
                "--background" => result.background = true,
                "--ai-sidebar" => result.ai_sidebar = true,
                "--hold-after-ready" => (),
                "--exit-after-ready" => result.exit_after_ready = true,
                "--dump-layout" => {
                    result.dump_layout =
                        Some(args.next().context("--dump-layout requires a path")?.into())
                }
                "--window-size" => {
                    let value = args.next().context("--window-size requires WIDTHxHEIGHT")?;
                    let (width, height) = value.split_once('x').context("--window-size requires WIDTHxHEIGHT")?;
                    result.window_size = (width.parse()?, height.parse()?);
                    anyhow::ensure!(result.window_size.0 > 0 && result.window_size.1 > 0, "--window-size must be positive");
                }
                "--print-memory" => result.print_memory = true,
                "--db" => result.db = args.next().context("--db requires a path")?.into(),
                "--screen" => {
                    let screen = args.next().context("--screen requires inbox, board, task or settings")?;
                    result.settings = screen == "settings";
                    result.screen = if result.settings { Screen::Inbox } else { Screen::parse(&screen)? };
                }
                "--theme" => {
                    result.theme =
                        ThemeName::parse(&args.next().context("--theme requires a name")?)?
                }
                "--board" => {
                    result.board_given = true;
                    result.board = args
                        .next()
                        .context("--board requires a project id")?
                        .parse()?
                }
                "--task" => {
                    result.task = args.next().context("--task requires a task id")?.parse()?
                }
                "--bench-input" => {
                    result.bench_input =
                        args.next().context("--bench-input requires N")?.parse()?;
                    anyhow::ensure!(result.bench_input > 0, "--bench-input must be positive");
                }
                _ => bail!("unknown option: {arg}"),
            }
        }
        anyhow::ensure!(
            !(result.exit_after_ready && result.bench_input > 0),
            "choose --exit-after-ready or --bench-input"
        );
        anyhow::ensure!(
            !result.background || (result.offline && result.dump_layout.is_some()),
            "--background requires --offline and --dump-layout"
        );
        Ok(result)
    }
}

struct Assets;
impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if path == "htc-logo" { return Ok(Some(Cow::Borrowed(include_bytes!("../assets/icons/htc-logo.png").as_slice()))); }
        macro_rules! icons {
            ($($name:literal),*) => { match path {
                $($name => Some(Cow::Borrowed(include_bytes!(concat!("../assets/icons/", $name, ".svg")).as_slice())),)*
                _ => None,
            }};
        }
        Ok(icons!(
            "inbox",
            "columns-2",
            "rows-3",
            "calendar",
            "sparkles",
            "search",
            "bot",
            "user-plus",
            "rocket",
            "message-circle",
            "settings",
            "panel-left",
            "chevron-right",
            "chevron-left",
            "chevron-up",
            "chevron-down",
            "arrow-left",
            "arrow-up-down",
            "filter",
            "ellipsis",
            "plus",
            "archive",
            "archive-x",
            "clock",
            "circle-check",
            "layers",
            "paperclip",
            "mic",
            "send",
            "x",
            "house",
            "message-square",
            "user-round",
            "tag",
            "arrow-right",
            "corner-left-up",
            "ai-sparkles",
            "send-archive",
            "square-pen",
            "minimize-2",
            "minus"
        ))
    }
    fn list(&self, _: &str) -> Result<Vec<SharedString>> {
        Ok(vec![])
    }
}

fn run() -> Result<()> {
    let options = Options::parse(std::env::args().skip(1))?;
    if let Some(parent) = options.db.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    #[cfg(windows)]
    if !options.offline && updater::start(&options.db) {
        return Ok(());
    }
    let db = ht_core::Db::open(&options.db)?;
    gpui_platform::application()
        .with_assets(Assets)
        .run(move |cx: &mut App| {
            fonts::load(options.theme, cx);
            input::init(cx);
            if cfg!(target_os = "linux") {
                cx.set_text_rendering_mode(gpui::TextRenderingMode::Grayscale);
            }
            let origin_x = if cfg!(target_os = "linux")
                && std::env::var_os("DISPLAY").is_some()
                && std::env::var("WAYLAND_DISPLAY")
                    .unwrap_or_default()
                    .is_empty()
            {
                -2.
            } else {
                0.
            };
            let background = options.background;
            let window = cx
                .open_window(
                    WindowOptions {
                        window_bounds: Some({
                            let bounds =
                                Bounds::new(point(px(origin_x), px(0.)), size(px(options.window_size.0 as f32), px(options.window_size.1 as f32)));
                            if cfg!(windows) && !background {
                                WindowBounds::Maximized(bounds)
                            } else {
                                WindowBounds::Windowed(bounds)
                            }
                        }),
                        focus: !background,
                        show: !background,
                        titlebar: Some(TitlebarOptions {
                            title: Some("Hypertask".into()),
                            appears_transparent: false,
                            ..Default::default()
                        }),
                        window_decorations: Some(WindowDecorations::Server),
                        inactive_frame_interval: None,
                        app_id: Some("ai.hypertask.desktop".into()),
                        ..Default::default()
                    },
                    |window, cx| cx.new(|cx| Desktop::new(db, options, window, cx)),
                )
                .expect("open native window");
            window
                .update(cx, |view, window, cx| window.focus(&view.focus, cx))
                .expect("focus window");
            cx.on_window_closed(|cx, _| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();
            if !background {
                cx.activate(true);
                #[cfg(all(windows, not(feature = "untrimmed-startup")))]
                memory::trim_after_startup(cx);
            }
        });
    Ok(())
}

#[cfg(windows)]
fn follow_windows_display_scaling() {
    use windows::Win32::UI::HiDpi::{
        DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
    };
    unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) }.ok();
}

fn main() {
    #[cfg(windows)]
    follow_windows_display_scaling();
    if let Err(error) = run() {
        eprintln!("Hypertask: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn static_inter_weights_preserve_the_full_character_map() {
        fn cmap(font: &[u8]) -> &[u8] {
            let count = u16::from_be_bytes(font[4..6].try_into().unwrap()) as usize;
            for table in font[12..12 + count * 16].chunks_exact(16) {
                if &table[..4] == b"cmap" {
                    let offset = u32::from_be_bytes(table[8..12].try_into().unwrap()) as usize;
                    let length = u32::from_be_bytes(table[12..16].try_into().unwrap()) as usize;
                    return &font[offset..offset + length];
                }
            }
            panic!("font has no character map");
        }
        let original = cmap(include_bytes!("../assets/fonts/Inter.ttf"));
        for face in [
            include_bytes!("../assets/fonts/Inter-500.ttf").as_slice(),
            include_bytes!("../assets/fonts/Inter-600.ttf").as_slice(),
            include_bytes!("../assets/fonts/Inter-700.ttf").as_slice(),
        ] {
            assert_eq!(cmap(face), original, "retain the full Inter character map");
        }
    }

    #[test]
    fn embedded_plex_faces_retain_web_weights_and_unhinted_tables() {
        for (weight, face) in [
            (400, include_bytes!("../assets/fonts/IBMPlexSans-400.ttf").as_slice()),
            (500, include_bytes!("../assets/fonts/IBMPlexSans-500.ttf").as_slice()),
            (600, include_bytes!("../assets/fonts/IBMPlexSans-600.ttf").as_slice()),
            (700, include_bytes!("../assets/fonts/IBMPlexSans-700.ttf").as_slice()),
        ] {
            let count = u16::from_be_bytes(face[4..6].try_into().unwrap()) as usize;
            let tables = face[12..12 + count * 16].chunks_exact(16).collect::<Vec<_>>();
            let os2 = tables.iter().find(|table| &table[..4] == b"OS/2").unwrap();
            let offset = u32::from_be_bytes(os2[8..12].try_into().unwrap()) as usize;
            assert_eq!(u16::from_be_bytes(face[offset + 4..offset + 6].try_into().unwrap()), weight);
            let name = tables.iter().find(|table| &table[..4] == b"name").unwrap();
            let offset = u32::from_be_bytes(name[8..12].try_into().unwrap()) as usize;
            let names = &face[offset..];
            let count = u16::from_be_bytes(names[2..4].try_into().unwrap()) as usize;
            let base = u16::from_be_bytes(names[4..6].try_into().unwrap()) as usize;
            let family_id = if weight == 500 || weight == 600 { 16 } else { 1 };
            let family = names[6..6 + count * 12].chunks_exact(12).find(|entry|
                u16::from_be_bytes(entry[..2].try_into().unwrap()) == 3 &&
                u16::from_be_bytes(entry[6..8].try_into().unwrap()) == family_id
            ).expect("retain the typographic family for intermediate weights");
            let length = u16::from_be_bytes(family[8..10].try_into().unwrap()) as usize;
            let start = base + u16::from_be_bytes(family[10..12].try_into().unwrap()) as usize;
            let family = names[start..start + length].chunks_exact(2)
                .map(|pair|u16::from_be_bytes(pair.try_into().unwrap())).collect::<Vec<_>>();
            assert_eq!(String::from_utf16(&family).unwrap(), "IBM Plex Sans");
            for tag in [b"fpgm", b"prep", b"cvt "] {
                assert!(!tables.iter().any(|table| &table[..4] == tag), "web font has no hint programs");
            }
        }
    }

    #[test]
    fn cli_validates_all_contract_flags() {
        let opts = Options::parse(
            [
                "--db",
                "/tmp/test.db",
                "--offline",
                "--screen",
                "board",
                "--board",
                "15",
                "--task",
                "58006",
                "--theme",
                "dia",
                "--dump-layout",
                "/tmp/layout.json",
                "--hold-after-ready",
                "--print-memory",
                "--bench-input",
                "300",
            ]
            .map(str::to_owned),
        )
        .unwrap();
        assert_eq!(opts.dump_layout, Some(PathBuf::from("/tmp/layout.json")));
        assert!(Options::parse([]).unwrap().dump_layout.is_none());
        let exit = Options::parse(
            ["--dump-layout", "layout.json", "--exit-after-ready"].map(str::to_owned),
        )
        .unwrap();
        assert!(exit.exit_after_ready);
        assert_eq!(exit.dump_layout, Some(PathBuf::from("layout.json")));
        assert!(opts.offline);
        assert!(!Options::parse([]).unwrap().ai_sidebar);
        assert!(
            Options::parse(["--ai-sidebar".to_owned()])
                .unwrap()
                .ai_sidebar
        );
        assert_eq!(opts.window_size, (1440, 900));
        assert_eq!(Options::parse(["--window-size", "1920x1009"].map(str::to_owned)).unwrap().window_size, (1920, 1009));
        assert!(opts.print_memory);
        assert!(!Options::parse([]).unwrap().print_memory);
        assert_eq!(opts.bench_input, 300);
        assert_eq!(opts.theme, ThemeName::Dia);
        for args in [
            vec!["--screen", "nope"],
            vec!["--db"],
            vec!["--dump-layout"],
            vec!["--theme", "blue"],
            vec!["--bench-input", "0"],
            vec!["--unknown"],
            vec!["--window-size"],
            vec!["--window-size", "1920"],
            vec!["--window-size", "0x1009"],
            vec!["--window-size", "1920x-1"],
            vec!["--exit-after-ready", "--bench-input", "3"],
        ] {
            assert!(Options::parse(args.into_iter().map(str::to_owned)).is_err());
        }
    }
}
