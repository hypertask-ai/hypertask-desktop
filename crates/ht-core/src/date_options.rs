use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveDateTime, Offset, TimeZone, Timelike};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DateOption {
    pub display: String,
    pub date: String,
}

fn local<Tz: TimeZone>(now: &DateTime<Tz>, date: NaiveDateTime) -> Option<DateTime<Tz>> {
    let zone = now.timezone();
    if let Some(value) = zone.from_local_datetime(&date).earliest() {
        return Some(value);
    }
    // JS date setters move a nonexistent wall time forward by the DST gap.
    let before = (1..=48).find_map(|hours| {
        zone.from_local_datetime(&date.checked_sub_signed(Duration::hours(hours))?)
            .earliest()
    })?;
    let after = (1..=48).find_map(|hours| {
        zone.from_local_datetime(&date.checked_add_signed(Duration::hours(hours))?)
            .earliest()
    })?;
    let gap = after.offset().fix().local_minus_utc() - before.offset().fix().local_minus_utc();
    if gap <= 0 {
        return None;
    }
    zone.from_local_datetime(&date.checked_add_signed(Duration::seconds(gap as i64))?)
        .earliest()
}
fn relative<Tz: TimeZone>(now: &DateTime<Tz>, amount: i32, unit: &str) -> Option<DateTime<Tz>> {
    let date = now.naive_local();
    let date = match unit.trim_end_matches('s') {
        "minute" => date.checked_add_signed(Duration::minutes(amount as i64))?,
        "hour" => date.checked_add_signed(Duration::hours(amount as i64))?,
        "day" => date.checked_add_signed(Duration::days(amount as i64))?,
        "week" => date.checked_add_signed(Duration::weeks(amount as i64))?,
        "month" => {
            // JS setMonth rolls an overflowing day forward; chrono's month addition clamps it.
            let index = date
                .year()
                .checked_mul(12)?
                .checked_add(date.month0() as i32)?
                .checked_add(amount)?;
            NaiveDate::from_ymd_opt(index.div_euclid(12), index.rem_euclid(12) as u32 + 1, 1)?
                .checked_add_signed(Duration::days(date.day0() as i64))?
                .and_time(date.time())
        }
        _ => return None,
    };
    local(now, date)
}
fn at<Tz: TimeZone>(now: &DateTime<Tz>, date: NaiveDate, hour: u32) -> Option<DateTime<Tz>> {
    local(now, date.and_hms_opt(hour, 0, 0)?)
}

/// Web dateParse.ts getTodayOptions/getOptions; other queries stay with the existing date parser.
pub fn relative_options<Tz: TimeZone>(
    input: &str,
    now: DateTime<Tz>,
    past: bool,
) -> Option<Vec<DateOption>> {
    let query = input.trim().to_lowercase();
    let mut rows = Vec::new();
    let mut push = |display: String, date: DateTime<Tz>| {
        rows.push(DateOption {
            display,
            date: date.to_rfc3339(),
        })
    };
    if query == "today" {
        let hour = now.hour();
        let mut last = hour;
        for index in 1..=3 {
            let next = hour + index;
            let (date, next) = if next == 24 {
                (now.date_naive().succ_opt(), 0)
            } else {
                (Some(now.date_naive()), next)
            };
            let Some(date) = date.and_then(|date| at(&now, date, next)) else {
                continue;
            };
            push(
                format!(
                    "today at {} {}",
                    if next % 12 == 0 { 12 } else { next % 12 },
                    if next < 12 { "am" } else { "pm" }
                ),
                date,
            );
            last = next;
            if next == 0 {
                break;
            }
        }
        if last > 0 && last < 20 {
            if let Some(date) = at(&now, now.date_naive(), 20) {
                push("later at 8 pm".into(), date);
            }
        } else if last > 20 {
            if let Some(date) = now
                .date_naive()
                .succ_opt()
                .and_then(|date| at(&now, date, 0))
            {
                push("later at 12 am".into(), date);
            }
        }
        return Some(rows);
    }
    let words: Vec<_> = input.split_whitespace().collect();
    let amount = words.first()?.parse::<i32>().ok()?;
    if words[0] != amount.to_string() {
        return None;
    }
    if words.len() > 2 {
        return Some(rows);
    }
    let mut units = if amount == 1 {
        vec!["hour", "day", "week", "month"]
    } else {
        vec!["hours", "days", "weeks", "months"]
    };
    if amount >= 10 {
        units.insert(0, "minutes");
    }
    for unit in units {
        if words.len() == 1 || unit.starts_with(&words[1].to_lowercase()) {
            if let Some(date) = amount
                .checked_mul(if past { -1 } else { 1 })
                .and_then(|amount| relative(&now, amount, unit))
            {
                push(format!("{amount} {unit}"), date);
            }
        }
    }
    Some(rows)
}

const PHRASES: &[&str] = &[
    "today", "tomorrow", "this weekend", "next week", "next weekend", "next month", "next year",
    "end of week", "end of month", "monday", "tuesday", "wednesday", "thursday", "friday", "saturday", "sunday",
    "next monday", "next tuesday", "next wednesday", "next thursday", "next friday", "next saturday", "next sunday",
];

fn phrase_date<Tz: TimeZone>(phrase: &str, now: &DateTime<Tz>, past: bool) -> Option<DateTime<Tz>> {
    let today = now.date_naive();
    let weekday = today.weekday().num_days_from_sunday() as i64;
    let date = match phrase {
        "today" => at(now, today, 9)?,
        "tomorrow" => at(now, today.checked_add_signed(Duration::days(1))?, 9)?,
        "next week" => local(now, now.naive_local().checked_add_signed(Duration::weeks(1))?)?,
        "next month" => local(now, today.checked_add_months(chrono::Months::new(1))?.and_time(now.time()))?,
        "next year" => {
            let date = NaiveDate::from_ymd_opt(today.year().checked_add(1)?, today.month(), 1)?.checked_add_signed(Duration::days(today.day0() as i64))?;
            local(now, date.and_time(now.time()))?
        }
        "this weekend" | "next weekend" => at(now, today.checked_add_signed(Duration::days(6 - weekday + if phrase == "next weekend" {7} else {0}))?, 9)?,
        "end of week" | "end of month" => {
            let date = if phrase == "end of week" { today.checked_add_signed(Duration::days(6 - weekday))? }
                else { today.with_day(1)?.checked_add_months(chrono::Months::new(1))?.pred_opt()? };
            local(now, date.and_hms_milli_opt(23, 59, 59, 999)?)?
        }
        _ => {
            let next = phrase.starts_with("next ");
            let day = phrase.strip_prefix("next ").unwrap_or(phrase);
            let target = ["sunday", "monday", "tuesday", "wednesday", "thursday", "friday", "saturday"].iter().position(|candidate| *candidate == day)? as i64;
            let mut date = today.checked_add_signed(Duration::days(target - weekday + if next {7} else {0}))?;
            if !next {
                let midnight = at(now, date, 0)?;
                if if past {midnight >= *now} else {midnight <= *now} {
                    date = date.checked_add_signed(Duration::days(if past {-7} else {7}))?;
                }
            }
            at(now, date, 9)?
        }
    };
    if date.hour() == 0 && date.minute() == 0 && date.second() == 0 { at(now, date.date_naive(), 9) } else { Some(date) }
}

/// The web's ranked phrase table and exact-phrase Sugar result. Other grammar stays with the caller.
pub fn phrase_options<Tz: TimeZone>(input: &str, now: DateTime<Tz>, past: bool) -> Option<Vec<DateOption>> {
    let query = input.trim().to_lowercase();
    if query.is_empty() || query == "today" || query.starts_with(|character: char| character.is_ascii_digit()) { return None; }
    let mut phrases: Vec<_> = PHRASES.iter().filter_map(|phrase| phrase.find(&query).map(|index| (*phrase, index != 0, phrase.len()))).collect();
    if phrases.is_empty() { return None; }
    phrases.sort_by_key(|(_, contains, length)| (*contains, *length));
    let mut rows = vec![];
    for (phrase, _, _) in phrases.into_iter().take(6) {
        if let Some(date) = phrase_date(phrase, &now, past) {
            if !past && date < now { continue; }
            let display = phrase.split(' ').map(|word| { let mut chars=word.chars(); format!("{}{}", chars.next().unwrap().to_ascii_uppercase(), chars.as_str()) }).collect::<Vec<_>>().join(" ");
            rows.push(DateOption { display, date:date.to_rfc3339() });
        }
    }
    if let Some(date) = phrase_date(&query, &now, past).and_then(|date| at(&now, date.date_naive(), 9)) {
        let difference = date.signed_duration_since(&now).num_milliseconds();
        if if past {difference < 60_000} else {difference > 60_000} {
            rows.push(DateOption { display:date.naive_local().format("%-m/%-d/%Y, %-I:%M:%S %p").to_string(), date:date.to_rfc3339() });
        }
    }
    let mut seen = std::collections::BTreeSet::new();
    rows.retain(|row| DateTime::parse_from_rfc3339(&row.date).is_ok_and(|date| seen.insert(date.timestamp().div_euclid(60))));
    Some(rows)
}
