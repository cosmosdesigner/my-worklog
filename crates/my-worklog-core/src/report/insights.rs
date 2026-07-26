mod grouped;

use chrono::{Datelike, Days, Local, LocalResult, NaiveDate, TimeZone, Utc};
use rusqlite::Connection;

use crate::db::repositories::{StoredEvent, events_between};
use crate::error::WorklogResult;

#[derive(Debug, Clone, Copy)]
pub enum ReportPeriod {
    Today,
    Yesterday,
    Week,
}

pub fn decisions(conn: &Connection, period: ReportPeriod) -> WorklogResult<String> {
    let events = period_events(conn, period)?;
    Ok(grouped::text_report(
        "Decisions",
        period,
        &events,
        is_decision,
        grouped::TextOptions::FULL,
    ))
}

pub fn open_loops(conn: &Connection, period: ReportPeriod) -> WorklogResult<String> {
    let events = period_events(conn, period)?;
    Ok(grouped::text_report(
        "Open loops",
        period,
        &events,
        is_open_loop,
        grouped::TextOptions::FULL,
    ))
}

pub fn blockers(conn: &Connection, period: ReportPeriod) -> WorklogResult<String> {
    let events = period_events(conn, period)?;
    Ok(grouped::text_report(
        "Blockers",
        period,
        &events,
        is_blocker,
        grouped::TextOptions::FULL,
    ))
}

pub fn done(conn: &Connection, period: ReportPeriod) -> WorklogResult<String> {
    let events = period_events(conn, period)?;
    Ok(grouped::done_report(period, &events))
}

pub fn done_compact(conn: &Connection, period: ReportPeriod) -> WorklogResult<String> {
    let events = period_events(conn, period)?;
    Ok(grouped::done_compact(period, &events))
}

pub fn files(conn: &Connection, period: ReportPeriod) -> WorklogResult<String> {
    let events = period_events(conn, period)?;
    Ok(grouped::file_report(period, &events, 30))
}

pub fn commands(conn: &Connection, period: ReportPeriod) -> WorklogResult<String> {
    let events = period_events(conn, period)?;
    Ok(grouped::command_report(period, &events, 30))
}

pub fn agents(conn: &Connection, period: ReportPeriod) -> WorklogResult<String> {
    let events = period_events(conn, period)?;
    Ok(grouped::agent_report(period, &events, 30))
}

pub fn status(conn: &Connection, period: ReportPeriod) -> WorklogResult<String> {
    let events = period_events(conn, period)?;
    Ok(grouped::status(period, &events, grouped::TextOptions::FULL))
}

pub fn status_compact(conn: &Connection, period: ReportPeriod) -> WorklogResult<String> {
    let events = period_events(conn, period)?;
    Ok(grouped::status_compact(period, &events))
}

fn period_events(conn: &Connection, period: ReportPeriod) -> WorklogResult<Vec<StoredEvent>> {
    let now = Local::now();
    let (start, end) = match period {
        ReportPeriod::Today => (day_start(now.date_naive()), now),
        ReportPeriod::Yesterday => {
            let date = now.date_naive() - Days::new(1);
            (day_start(date), local_datetime(date, now.time()))
        }
        ReportPeriod::Week => {
            let days_from_monday = u64::from(now.date_naive().weekday().num_days_from_monday());
            (
                day_start(now.date_naive() - Days::new(days_from_monday)),
                now,
            )
        }
    };
    events_between(conn, start.with_timezone(&Utc), end.with_timezone(&Utc))
}

fn day_start(date: NaiveDate) -> chrono::DateTime<Local> {
    local_datetime(date, chrono::NaiveTime::MIN)
}

fn local_datetime(date: NaiveDate, time: chrono::NaiveTime) -> chrono::DateTime<Local> {
    match Local.from_local_datetime(&date.and_time(time)) {
        LocalResult::Single(value) => value,
        LocalResult::Ambiguous(earliest, _) => earliest,
        LocalResult::None => Utc
            .from_utc_datetime(&date.and_time(time))
            .with_timezone(&Local),
    }
}

fn is_decision(event: &StoredEvent) -> bool {
    !is_meta_dump(event)
        && contains_any(
            event,
            &["decision", "decided", "choose", "chose", "use ", "we will"],
        )
}

fn is_open_loop(event: &StoredEvent) -> bool {
    !is_meta_dump(event)
        && contains_any(
            event,
            &["todo", "open loop", "follow up", "next", "remaining"],
        )
}

fn is_blocker(event: &StoredEvent) -> bool {
    !is_meta_dump(event)
        && contains_any(
            event,
            &["blocked", "blocker", "cannot", "can't", "failed", "error"],
        )
}

fn is_compact_decision(event: &StoredEvent) -> bool {
    !is_meta_dump(event)
        && text_starts_with_any(event, &["decision:", "decided:", "we decided", "i decided"])
}

fn is_compact_open_loop(event: &StoredEvent) -> bool {
    !is_meta_dump(event)
        && text_starts_with_any(
            event,
            &[
                "todo:",
                "open loop:",
                "follow up:",
                "follow-up:",
                "next:",
                "next step:",
                "remaining:",
            ],
        )
}

fn is_compact_blocker(event: &StoredEvent) -> bool {
    !is_meta_dump(event)
        && text_starts_with_any(
            event,
            &[
                "blocker:",
                "blocked:",
                "blocked by",
                "blocked on",
                "cannot proceed",
                "can't proceed",
            ],
        )
}

fn is_done(event: &StoredEvent) -> bool {
    event.event_type == "assistant_message"
        && contains_any(
            event,
            &[
                "completed",
                "finished",
                "done",
                "implemented",
                "added",
                "fixed",
                "resolved",
                "shipped",
            ],
        )
        && !is_open_loop(event)
        && !is_meta_dump(event)
}

fn is_meta_dump(event: &StoredEvent) -> bool {
    let haystack = format!(
        "{} {}",
        event.title.as_deref().unwrap_or_default(),
        event.content.as_deref().unwrap_or_default()
    )
    .to_lowercase();
    [
        "implementation plan",
        "architecture summary",
        "## objective",
        "## work state",
        "## next move",
        "relevant files",
        "recommended shape",
        "source-backed guidance",
        "keep report generation pure",
        "<analysis>",
        "<plan>",
        "<results>",
        "<files>",
        "<system-reminder>",
        "background task completed",
        "continue if you have next steps",
    ]
    .iter()
    .any(|needle| haystack.contains(needle))
}

fn contains_any(event: &StoredEvent, needles: &[&str]) -> bool {
    let haystack = format!(
        "{} {} {}",
        event.title.as_deref().unwrap_or_default(),
        event.content.as_deref().unwrap_or_default(),
        event.command.as_deref().unwrap_or_default()
    )
    .to_lowercase();
    needles.iter().any(|needle| haystack.contains(needle))
}

fn text_starts_with_any(event: &StoredEvent, prefixes: &[&str]) -> bool {
    event
        .title
        .as_deref()
        .into_iter()
        .chain(event.content.as_deref())
        .map(|text| text.trim_start().to_lowercase())
        .any(|text| prefixes.iter().any(|prefix| text.starts_with(prefix)))
}

impl ReportPeriod {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Today => "today",
            Self::Yesterday => "yesterday",
            Self::Week => "this week",
        }
    }
}
