use crate::{Db, Result, Transport};
use anyhow::{Context, ensure};
use chrono::{Duration, NaiveDate, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeEntryChange {
    Create {
        date: String,
        minutes: u16,
        offset: i32,
        note: String,
    },
    Update {
        entry: i64,
        date: Option<String>,
        minutes: u16,
        offset: i32,
        note: String,
    },
    Delete {
        entry: i64,
    },
}
impl TimeEntryChange {
    pub fn request(&self, task: i64) -> Result<(&'static str, Value)> {
        ensure!(task > 0, "Task is unavailable");
        Ok(match self {
            Self::Create {
                date,
                minutes,
                offset,
                note,
            } => {
                manual_times(date, *minutes, *offset)?;
                (
                    "POST",
                    json!({"taskId":task,"date":date,"minutes":minutes,"timezoneOffsetMinutes":offset,"note":note}),
                )
            }
            Self::Update {
                entry,
                date,
                minutes,
                offset,
                note,
            } => {
                ensure!(*entry > 0, "Time entry not found or access denied");
                ensure!(
                    (1..=1440).contains(minutes),
                    "Enter a duration from 1 minute to 24 hours"
                );
                let mut body = json!({"entryId":entry,"minutes":minutes,"note":note});
                if let Some(date) = date {
                    manual_times(date, *minutes, *offset)?;
                    body["date"] = json!(date);
                    body["timezoneOffsetMinutes"] = json!(offset);
                }
                ("PATCH", body)
            }
            Self::Delete { entry } => {
                ensure!(*entry > 0, "Time entry not found or access denied");
                ("DELETE", json!({"entryId":entry}))
            }
        })
    }
}
fn manual_times(date: &str, minutes: u16, offset: i32) -> Result<(String, String)> {
    ensure!(
        (1..=1440).contains(&minutes),
        "Enter a duration from 1 minute to 24 hours"
    );
    ensure!((-840..=840).contains(&offset), "Invalid timezone offset");
    ensure!(
        date.len() == 10
            && date.bytes().enumerate().all(|(i, c)| if i == 4 || i == 7 {
                c == b'-'
            } else {
                c.is_ascii_digit()
            }),
        "Pick a date"
    );
    let day = NaiveDate::parse_from_str(date, "%Y-%m-%d").context("Pick a date")?;
    ensure!(day.format("%Y-%m-%d").to_string() == date, "Pick a date");
    let start = Utc.from_utc_datetime(&day.and_hms_opt(12, 0, 0).unwrap())
        + Duration::minutes(i64::from(offset));
    Ok((
        start.to_rfc3339(),
        (start + Duration::minutes(i64::from(minutes))).to_rfc3339(),
    ))
}
fn normalized_note(note: &str) -> Value {
    let text: String = note.trim().chars().take(500).collect();
    if text.is_empty() {
        Value::Null
    } else {
        json!(text)
    }
}
fn valid_entry(row: &Value) -> bool {
    row["id"].as_i64().is_some_and(|id| id > 0)
        && row["userId"].as_i64().is_some_and(|id| id > 0)
        && row["userName"].is_string()
        && row["seconds"].as_i64().is_some_and(|n| n >= 0)
        && row["startedAt"]
            .as_str()
            .is_some_and(|s| chrono::DateTime::parse_from_rfc3339(s).is_ok())
        && ["endedAt", "pausedAt"].iter().all(|field| {
            row.get(*field).is_some_and(|v| {
                v.is_null()
                    || v.as_str()
                        .is_some_and(|s| chrono::DateTime::parse_from_rfc3339(s).is_ok())
            })
        })
}
pub fn load(transport: &mut impl Transport, task: i64) -> Result<Value> {
    let response = transport.get(&format!("/api/time/entries?taskId={task}"))?;
    ensure!(
        response.status == 200 && response.body["success"] == true,
        "{}",
        response.body["error"]
            .as_str()
            .unwrap_or("Time tracking request failed")
    );
    ensure!(
        response.body["canManage"].is_boolean()
            && response.body["entries"]
                .as_array()
                .is_some_and(|rows| rows.iter().all(valid_entry)),
        "Invalid time entries response"
    );
    Ok(response.body)
}
fn key(user: i64, task: i64) -> String {
    format!("time-entries:{user}:{task}")
}
impl Db {
    pub fn time_entries(&self, task: i64) -> Result<Option<Value>> {
        let user = self.current_user_id().context("Unauthorized")?;
        self.kv_get(&key(user, task))?
            .map(|s| serde_json::from_str(&s).map_err(Into::into))
            .transpose()
    }
    pub fn cache_time_entries(&self, user: i64, task: i64, value: &Value) -> Result<()> {
        ensure!(self.current_user_id() == Some(user), "Unauthorized");
        self.kv_set(&key(user, task), &value.to_string())
    }
    pub(crate) fn prepare_time_entries(
        &self,
        task: i64,
        change: &TimeEntryChange,
    ) -> Result<Value> {
        let before = self
            .time_entries(task)?
            .context("Time entries are still loading")?;
        let user = self.current_user_id().context("Unauthorized")?;
        ensure!(self.task(task)?.is_some(), "Task is unavailable");
        if let TimeEntryChange::Update { entry, .. } | TimeEntryChange::Delete { entry } = change {
            let row = before["entries"]
                .as_array()
                .into_iter()
                .flatten()
                .find(|row| row["id"] == *entry)
                .context("Time entry not found or access denied")?;
            ensure!(
                row["userId"] == user || before["canManage"] == true,
                "Time entry not found or access denied"
            );
            if matches!(change, TimeEntryChange::Update { .. }) {
                ensure!(
                    row["endedAt"].is_string(),
                    "Stop the timer before editing this entry"
                );
            }
        } else {
            ensure!(
                self.task_time(task)?
                    .is_some_and(|summary| summary["enabled"] == true),
                "Time tracking is disabled for this board"
            );
        }
        Ok(before)
    }
    pub(crate) fn optimistic_time_entries(
        &self,
        payload: &Value,
        task: i64,
        change: &TimeEntryChange,
        mutation: i64,
        now: i64,
    ) -> Result<()> {
        let user = payload["user_id"].as_i64().context("Unauthorized")?;
        let mut value = payload["before"].clone();
        let rows = value["entries"]
            .as_array_mut()
            .context("Invalid time entries")?;
        match change {
            TimeEntryChange::Create {
                date,
                minutes,
                offset,
                note,
            } => {
                let (start, end) = manual_times(date, *minutes, *offset)?;
                rows.insert(0, json!({"id":-mutation,"taskId":task,"userId":user,"userName":"","note":normalized_note(note),"startedAt":start,"endedAt":end,"pausedAt":null,"createdAt":chrono::DateTime::from_timestamp(now,0).context("Invalid date")?.to_rfc3339(),"seconds":i64::from(*minutes)*60,"pending":true}));
            }
            TimeEntryChange::Update {
                entry,
                date,
                minutes,
                offset,
                note,
            } => {
                let row = rows
                    .iter_mut()
                    .find(|row| row["id"] == *entry)
                    .context("Time entry not found or access denied")?;
                let (start, end) = if let Some(date) = date {
                    manual_times(date, *minutes, *offset)?
                } else {
                    let start = chrono::DateTime::parse_from_rfc3339(
                        row["startedAt"].as_str().context("Invalid date")?,
                    )?;
                    (
                        start.to_rfc3339(),
                        (start + Duration::minutes(i64::from(*minutes))).to_rfc3339(),
                    )
                };
                row["startedAt"] = json!(start);
                row["endedAt"] = json!(end);
                row["note"] = normalized_note(note);
                row["seconds"] = json!(i64::from(*minutes) * 60);
                row["pending"] = json!(true);
            }
            TimeEntryChange::Delete { entry } => rows.retain(|row| row["id"] != *entry),
        }
        self.cache_time_entries(user, task, &value)
    }
    pub(crate) fn validate_time_entry_ack(
        &self,
        payload: &Value,
        task: i64,
        change: &TimeEntryChange,
        response: &Value,
    ) -> Result<()> {
        ensure!(
            response["success"] == true,
            "Time entry was not acknowledged"
        );
        let (id, minutes, note) = match change {
            TimeEntryChange::Delete { .. } => return Ok(()),
            TimeEntryChange::Create { minutes, note, .. } => (None, minutes, note),
            TimeEntryChange::Update {
                entry,
                minutes,
                note,
                ..
            } => (Some(*entry), minutes, note),
        };
        let value = self.time_entries(task)?.context("Missing time entries")?;
        let row = value["entries"]
            .as_array()
            .context("Missing time entries")?
            .iter()
            .find(|row| {
                row["pending"] == true
                    && if let Some(id) = id {
                        row["id"] == id
                    } else {
                        row["id"].as_i64().is_some_and(|id| id < 0)
                    }
            })
            .context("Missing pending time entry")?;
        let ack = &response["entry"];
        if id.is_some() {
            ensure!(
                valid_entry(ack) && ack["seconds"] == i64::from(*minutes) * 60,
                "Wrong time entry acknowledgement"
            );
        } else {
            ensure!(
                !value["entries"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|existing| existing["id"] == ack["id"]),
                "Wrong time entry acknowledgement"
            );
        }
        ensure!(
            ack["id"].as_i64().is_some_and(|id| id > 0)
                && id.is_none_or(|id| ack["id"] == id)
                && ack["userId"] == row["userId"]
                && ack["note"] == normalized_note(note)
                && (id.is_some() || ack["taskId"] == task && ack["userId"] == payload["user_id"]),
            "Wrong time entry acknowledgement"
        );
        for field in ["startedAt", "endedAt"] {
            let actual = chrono::DateTime::parse_from_rfc3339(
                ack[field].as_str().context("Missing time entry date")?,
            )?;
            let expected = chrono::DateTime::parse_from_rfc3339(
                row[field].as_str().context("Missing pending date")?,
            )?;
            ensure!(actual == expected, "Wrong time entry acknowledgement");
        }
        ensure!(
            row["seconds"] == i64::from(*minutes) * 60,
            "Wrong time entry duration"
        );
        Ok(())
    }
    pub(crate) fn resolve_time_entries(
        &self,
        payload: &Value,
        task: i64,
        change: &TimeEntryChange,
        response: &Value,
    ) -> Result<()> {
        self.validate_time_entry_ack(payload, task, change, response)?;
        let user = payload["user_id"].as_i64().context("Unauthorized")?;
        let mut value = self.time_entries(task)?.context("Missing time entries")?;
        let rows = value["entries"]
            .as_array_mut()
            .context("Invalid time entries")?;
        match change {
            TimeEntryChange::Delete { .. } => (),
            TimeEntryChange::Create { minutes, note, .. }
            | TimeEntryChange::Update { minutes, note, .. } => {
                let ack = &response["entry"];
                let start = chrono::DateTime::parse_from_rfc3339(
                    ack["startedAt"]
                        .as_str()
                        .context("Missing time entry start")?,
                )?;
                let end = chrono::DateTime::parse_from_rfc3339(
                    ack["endedAt"].as_str().context("Missing time entry end")?,
                )?;
                ensure!(
                    ack["id"].as_i64().is_some_and(|id| id > 0)
                        && ack["note"] == normalized_note(note)
                        && (end - start).num_seconds() == i64::from(*minutes) * 60,
                    "Wrong time entry acknowledgement"
                );
                let row = if let TimeEntryChange::Update { entry, .. } = change {
                    ensure!(ack["id"] == *entry, "Wrong time entry acknowledgement");
                    rows.iter_mut().find(|row| row["id"] == *entry)
                } else {
                    ensure!(
                        ack["taskId"] == task && ack["userId"] == user,
                        "Wrong time entry acknowledgement"
                    );
                    rows.iter_mut().find(|row| {
                        row["pending"] == true && row["id"].as_i64().is_some_and(|id| id < 0)
                    })
                }
                .context("Missing pending time entry")?;
                ensure!(
                    ack["userId"] == row["userId"]
                        && ack["startedAt"]
                            .as_str()
                            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                            == row["startedAt"]
                                .as_str()
                                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok()),
                    "Wrong time entry acknowledgement"
                );
                for field in [
                    "id",
                    "note",
                    "startedAt",
                    "endedAt",
                    "userName",
                    "createdAt",
                    "pausedAt",
                    "seconds",
                ] {
                    if let Some(v) = ack.get(field) {
                        row[field] = v.clone();
                    }
                }
                row.as_object_mut().unwrap().remove("pending");
            }
        }
        self.cache_time_entries(user, task, &value)?;
        self.request_refetch("task", task)
    }
    pub(crate) fn rollback_time_entries(&self, payload: &Value, task: i64) -> Result<()> {
        self.cache_time_entries(
            payload["user_id"].as_i64().context("Unauthorized")?,
            task,
            &payload["before"],
        )
    }
}
