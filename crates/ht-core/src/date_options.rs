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
