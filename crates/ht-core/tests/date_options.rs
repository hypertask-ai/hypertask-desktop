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

#[test]
fn phrase_date_options_match_web_sugar_ranking_edges_clamping_start_direction_and_deduplication() {
    use std::{io::Write, process::{Command, Stdio}};
    use serde_json::{Value, json};
    let queries=["tomorrow","to","mon","next","week","month","end","friday","next friday","this weekend","next weekend","next week","next month","next year","end of week","end of month","sun","t","m"];
    for (zone,offset) in [("UTC","+00:00"),("Etc/GMT-2","+02:00")] {
        let mut cases=vec![];
        let mut actual=vec![];
        for day in ["2026-01-31","2026-02-01","2026-02-02","2028-02-29"] {
            for time in ["00:00:00","08:42:17","18:42:17"] {
                let now=DateTime::parse_from_rfc3339(&format!("{day}T{time}{offset}")).unwrap();
                for past in [false,true] { for query in queries {
                    cases.push(json!({"query":query,"now":now.to_rfc3339(),"past":past}));
                    actual.push(ht_core::date_options::phrase_options(query,now,past).expect(query));
                }}
            }
        }
        let mut oracle=Command::new("node").arg(format!("{}/tests/date_reference.cjs",env!("CARGO_MANIFEST_DIR")))
            .env("TZ",zone).env("LANG","en_US.UTF-8").stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().expect("hosted CI must provide Node for the real Sugar reference");
        oracle.stdin.take().unwrap().write_all(serde_json::to_string(&cases).unwrap().as_bytes()).unwrap();
        let output=oracle.wait_with_output().unwrap();
        assert!(output.status.success(),"{}",String::from_utf8_lossy(&output.stderr));
        let expected:Vec<Vec<Value>>=serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(expected.len(),actual.len());
        let mut mismatches = vec![];
        for ((case,actual),expected) in cases.iter().zip(actual).zip(expected) {
            let actual:Vec<_>=actual.into_iter().map(|row|(row.display,DateTime::parse_from_rfc3339(&row.date).unwrap().timestamp_millis())).collect();
            let expected:Vec<_>=expected.into_iter().map(|row|(row["display"].as_str().unwrap().to_owned(),DateTime::parse_from_rfc3339(row["date"].as_str().unwrap()).unwrap().timestamp_millis())).collect();
            if actual != expected { mismatches.push(format!("{zone} {case}\nactual {actual:?}\nweb {expected:?}")); }
        }
        assert!(mismatches.is_empty(), "{} mismatches\n{}", mismatches.len(), mismatches.into_iter().take(24).collect::<Vec<_>>().join("\n"));
    }
}

#[test]
fn everyday_time_queries_match_real_web_sugar_options_and_refuse_malformed_input() {
    use std::{io::Write, process::{Command, Stdio}};
    use serde_json::{Value, json};
    let queries = [
        "tomorrow at 3pm", "tomorrow 15:30", "today at 8am", "today at 11:59pm",
        "next friday at 9:15 am", "monday at 14:30", "next week at 4pm",
        "next month at 10am", "yesterday at 6pm", "in 2 hours", "in 30 minutes",
        "in 2 days", "in 1 week", "in 1 month", "in 1 year", "2 hours ago",
        "3 days ago", "1 month ago", "2 weeks from now", "2 days later",
        "2 hours", "1 month", "30 minutes", "3pm", "15:30", "9:15 am",
        "tomorrow at 25:00", "in bananas", "tomorrow at 3pm junk", "3pm junk",
    ];
    for (zone, offset) in [("UTC", "+00:00"), ("Etc/GMT-2", "+02:00")] {
        let mut cases = vec![];
        let mut actual = vec![];
        for day in ["2026-01-31", "2026-02-02", "2028-02-29"] {
            for time in ["00:00:00", "08:42:17", "18:42:17"] {
                let now = DateTime::parse_from_rfc3339(&format!("{day}T{time}{offset}")).unwrap();
                for past in [false, true] { for query in queries {
                    cases.push(json!({"query":query,"now":now.to_rfc3339(),"past":past}));
                    actual.push(ht_core::date_options::relative_options(query, now, past)
                        .or_else(|| ht_core::date_options::phrase_options(query, now, past)).unwrap_or_default());
                }}
            }
        }
        let mut oracle = Command::new("node").arg(format!("{}/tests/date_reference.cjs", env!("CARGO_MANIFEST_DIR")))
            .env("TZ", zone).env("LANG", "en_US.UTF-8").stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped())
            .spawn().expect("hosted CI must provide Node for the real Sugar reference");
        oracle.stdin.take().unwrap().write_all(serde_json::to_string(&cases).unwrap().as_bytes()).unwrap();
        let output = oracle.wait_with_output().unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        let expected: Vec<Vec<Value>> = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(expected.len(), actual.len());
        let mut mismatches = vec![];
        for ((case, actual), expected) in cases.iter().zip(actual).zip(expected) {
            let actual: Vec<_> = actual.into_iter().map(|row| (row.display, DateTime::parse_from_rfc3339(&row.date).unwrap().timestamp_millis())).collect();
            let expected: Vec<_> = expected.into_iter().map(|row| (row["display"].as_str().unwrap().to_owned(), DateTime::parse_from_rfc3339(row["date"].as_str().unwrap()).unwrap().timestamp_millis())).collect();
            if actual != expected { mismatches.push(format!("{zone} {case}\nactual {actual:?}\nweb {expected:?}")); }
        }
        assert!(mismatches.is_empty(), "{} mismatches\n{}", mismatches.len(), mismatches.into_iter().take(40).collect::<Vec<_>>().join("\n"));
    }
}
