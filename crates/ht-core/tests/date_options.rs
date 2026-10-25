use chrono::{DateTime, Datelike, Timelike};
use ht_core::date_options::relative_options;

#[test]
fn numeric_date_options_match_web_order_prefixes_start_date_direction_and_js_month_rollover() {
    let now = DateTime::parse_from_rfc3339("2026-01-31T15:42:17+02:00").unwrap();
    let rows = relative_options("1", now, false).unwrap();
    assert_eq!(
        rows.iter()
            .map(|row| row.display.as_str())
            .collect::<Vec<_>>(),
        ["1 hour", "1 day", "1 week", "1 month"]
    );
    assert_eq!(rows[0].date, "2026-01-31T16:42:17+02:00");
    assert_eq!(rows[1].date, "2026-02-01T15:42:17+02:00");
    assert_eq!(rows[2].date, "2026-02-07T15:42:17+02:00");
    assert_eq!(rows[3].date, "2026-03-03T15:42:17+02:00");
    let rows = relative_options("10 M", now, false).unwrap();
    assert_eq!(
        rows.iter()
            .map(|row| row.display.as_str())
            .collect::<Vec<_>>(),
        ["10 minutes", "10 months"]
    );
    assert_eq!(rows[0].date, "2026-01-31T15:52:17+02:00");
    assert_eq!(
        relative_options("1 day", now, true).unwrap()[0].date,
        "2026-01-30T15:42:17+02:00"
    );
    assert_eq!(
        relative_options("10 minutes", now, true).unwrap()[0].date,
        "2026-01-31T15:32:17+02:00"
    );
    assert_eq!(
        relative_options("2", now, false)
            .unwrap()
            .iter()
            .map(|row| row.display.as_str())
            .collect::<Vec<_>>(),
        ["2 hours", "2 days", "2 weeks", "2 months"]
    );
    for query in [
        "01 day",
        "+1 day",
        "-0 day",
        "tomorrow",
        "2030-12-31T14:30:00+02:00",
        "",
    ] {
        assert!(relative_options(query, now, false).is_none(), "{query}");
    }
    for query in ["2 nonsense", "2 days later", "5 minutes"] {
        assert!(
            relative_options(query, now, false).unwrap().is_empty(),
            "{query}"
        );
    }
}
#[test]
fn today_time_options_cover_every_hour_no_past_times_and_midnight_boundary() {
    for hour in 0..24 {
        let now =
            DateTime::parse_from_rfc3339(&format!("2026-10-31T{hour:02}:42:17+02:00")).unwrap();
        let rows = relative_options(" ToDaY ", now, false).unwrap();
        assert!(!rows.is_empty());
        assert_eq!(rows, relative_options("today", now, true).unwrap());
        for row in &rows {
            let date = DateTime::parse_from_rfc3339(&row.date).unwrap();
            assert!(date > now);
            assert_eq!(date.minute(), 0);
            assert_eq!(date.second(), 0);
        }
        let last = DateTime::parse_from_rfc3339(&rows.last().unwrap().date).unwrap();
        if hour >= 21 {
            assert_eq!(last.day(), 1);
            assert_eq!(last.hour(), 0);
        }
        if hour == 17 {
            assert_eq!(rows.len(), 3);
            assert_eq!(last.hour(), 20);
        }
        if hour == 23 {
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].display, "today at 12 am");
        }
        if hour == 9 {
            assert_eq!(
                rows.iter()
                    .map(|row| row.display.as_str())
                    .collect::<Vec<_>>(),
                [
                    "today at 10 am",
                    "today at 11 am",
                    "today at 12 pm",
                    "later at 8 pm"
                ]
            );
        }
    }
}
