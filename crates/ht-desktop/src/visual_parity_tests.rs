use super::*;
use crate::ui::shell::tests::{draw, measured};
use gpui::VisualTestContext;
use serde_json::{Value, json};

fn reference(screen: &str, window: &mut Window, cx: &mut Context<Desktop>) -> Desktop {
    let mut db = Db::memory().unwrap();
    let fixture: Value = serde_json::from_str(include_str!("../../../gates/fixtures/visual3/data.json")).unwrap();
    db.apply_response(&fixture).unwrap();
    let options = Options::parse([
        "--offline", "--screen", screen, "--board", "15", "--task", "103", "--theme", "graphite",
    ].map(String::from)).unwrap();
    let mut view = Desktop::new(db, options, window, cx);
    view.layout = Some(Rc::new(RefCell::new(LayoutDump::default())));
    view.ready_scheduled = true;
    window.focus(&view.focus, cx);
    view
}

#[gpui::test]
fn board_web_grid_uses_column_count_and_cards_follow_content(cx: &mut gpui::TestAppContext) {
    cx.update(crate::input::init);
    for (count, width) in [(3, 458.8125), (4, 340.109375), (6, 280.)] {
        let view = cx.open_window(gpui::size(px(1920.), px(1009.)), |window, cx| {
            let mut view = reference("board", window, cx);
            for i in 3..count {
                view.db.apply_response(&json!({"success":true,"sections":[{
                    "id":i+1,"projectId":15,"section_title":format!("Column {i}"),
                    "ranking":format!("{}", i+1),"visibility":true,"deleted":false,
                }]})).unwrap();
            }
            view.reload();
            view
        });
        let mut cx = VisualTestContext::from_window(view.into(), cx);
        draw(view, &mut cx);
        for i in 0..count {
            let column = measured(view, &mut cx, &format!("board.column.{i}"));
            assert!((column["w"].as_f64().unwrap() - width).abs() <= 0.5, "{column}");
            assert!((column["x"].as_f64().unwrap() - (69.78125 + i as f64 * (width + 16.))).abs() <= 0.5, "{column}");
            assert_eq!(column["y"], 72.);
            assert_eq!(column["h"], 865.);
        }
        if count == 3 {
            let card = measured(view, &mut cx, "board.card.0");
            let second = measured(view, &mut cx, "board.card.1");
            let title = measured(view, &mut cx, "board.card.0.title");
            assert!((card["w"].as_f64().unwrap() - 440.8125).abs() <= 0.5);
            assert!((card["h"].as_f64().unwrap() - 58.).abs() <= 1.);
            assert!((second["h"].as_f64().unwrap() - 63.5).abs() <= 1.);
            assert!((title["w"].as_f64().unwrap() - 211.).abs() <= 1.);
            assert!((title["h"].as_f64().unwrap() - 19.5).abs() <= 0.5);
            assert_eq!(title["x"].as_f64().unwrap() - card["x"].as_f64().unwrap(), 12.);
            assert_eq!(second["y"].as_f64().unwrap() - card["y"].as_f64().unwrap() - card["h"].as_f64().unwrap(), 16.);
        }
        view.read_with(&mut cx, |this, _| {
            assert_eq!(this.columns.len(), count);
            assert_eq!(this.columns.iter().map(|c| c.cards.len()).sum::<usize>(), 2);
            assert!(this.db.mutations().unwrap().is_empty());
        }).unwrap();
    }
}

#[gpui::test]
fn inbox_web_tabs_and_intrinsic_row_widths_follow_reference_content(cx: &mut gpui::TestAppContext) {
    cx.update(crate::input::init);
    let view = cx.open_window(gpui::size(px(1920.), px(1009.)), |window, cx| reference("inbox", window, cx));
    let mut cx = VisualTestContext::from_window(view.into(), cx);
    for (i, x, width) in [(0, 118., 96.), (1, 223., 72.), (2, 304., 82.), (3, 395., 124.), (4, 528., 69.)] {
        let tab = measured(view, &mut cx, &format!("inbox.tab.{i}"));
        assert!((tab["x"].as_f64().unwrap() - x).abs() <= 1., "{tab}");
        assert!((tab["w"].as_f64().unwrap() - width).abs() <= 1., "{tab}");
        assert_eq!(tab["y"], 24.);
        assert_eq!(tab["h"], 32.);
    }
    for (key, x, width) in [("author", 118., 112.), ("title", 395.390625, 283.), ("kind", 686.390625, 653.609375)] {
        let element = measured(view, &mut cx, &format!("inbox.row.0.{key}"));
        assert!((element["x"].as_f64().unwrap() - x).abs() <= 1., "{element}");
        assert!((element["w"].as_f64().unwrap() - width).abs() <= 1., "{element}");
        assert_eq!(element["fontSize"], 13.);
        assert_eq!(element["lineHeight"], 19.5);
    }
    view.read_with(&mut cx, |this, _| {
        assert_eq!(this.splits.len(), 5);
        assert_eq!(this.filtered.len(), 1);
        assert_eq!(this.inbox.len(), 2);
        let elements = this.layout.as_ref().unwrap().borrow().interaction_elements();
        assert!(!elements.iter().any(|e| e["key"] == "inbox.tab.5" || e["key"] == "inbox.row.1"));
        assert!(this.db.mutations().unwrap().is_empty());
    }).unwrap();
}
