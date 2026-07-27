use chrono::{DateTime, Days, Local, LocalResult, NaiveDate, TimeZone, Utc};
use rusqlite::Connection;

use crate::db::repositories::events_between;
use crate::error::WorklogResult;
use crate::report::markdown::{render_full_report, render_report, render_share_context};

pub fn today(conn: &Connection) -> WorklogResult<String> {
    let now = Local::now();
    render_day_until(conn, now.date_naive(), now)
}

pub fn today_full(conn: &Connection) -> WorklogResult<String> {
    let now = Local::now();
    render_full_day_until(conn, now.date_naive(), now)
}

pub fn today_share_context(conn: &Connection) -> WorklogResult<String> {
    let now = Local::now();
    render_share_day_until(conn, now.date_naive(), now)
}

pub fn yesterday(conn: &Connection) -> WorklogResult<String> {
    render_yesterday_until(conn, Local::now())
}

pub fn yesterday_full(conn: &Connection) -> WorklogResult<String> {
    render_full_yesterday_until(conn, Local::now())
}

pub fn yesterday_share_context(conn: &Connection) -> WorklogResult<String> {
    render_share_yesterday_until(conn, Local::now())
}

pub fn render_yesterday_until(conn: &Connection, now: DateTime<Local>) -> WorklogResult<String> {
    let (date, end) = yesterday_window_end(now);
    render_day_until(conn, date, end)
}

pub fn render_full_yesterday_until(
    conn: &Connection,
    now: DateTime<Local>,
) -> WorklogResult<String> {
    let (date, end) = yesterday_window_end(now);
    render_full_day_until(conn, date, end)
}

pub fn render_share_yesterday_until(
    conn: &Connection,
    now: DateTime<Local>,
) -> WorklogResult<String> {
    let (date, end) = yesterday_window_end(now);
    render_share_day_until(conn, date, end)
}

pub fn render_day(conn: &Connection, date: NaiveDate) -> WorklogResult<String> {
    let (start, end) = day_window(date);
    let events = events_between(conn, start, end)?;
    Ok(render_report(&format!("Worklog - {date}"), &events))
}

pub fn render_day_until(
    conn: &Connection,
    date: NaiveDate,
    end: DateTime<Local>,
) -> WorklogResult<String> {
    let (start, _) = day_window(date);
    let events = events_between(conn, start, end.with_timezone(&Utc))?;
    Ok(render_report(&format!("Worklog - {date}"), &events))
}

pub fn render_full_day(conn: &Connection, date: NaiveDate) -> WorklogResult<String> {
    let (start, end) = day_window(date);
    let events = events_between(conn, start, end)?;
    Ok(render_full_report(&format!("Worklog - {date}"), &events))
}

pub fn render_share_day(conn: &Connection, date: NaiveDate) -> WorklogResult<String> {
    let (start, end) = day_window(date);
    let events = events_between(conn, start, end)?;
    Ok(render_share_context(&format!("Worklog - {date}"), &events))
}

pub fn render_full_day_until(
    conn: &Connection,
    date: NaiveDate,
    end: DateTime<Local>,
) -> WorklogResult<String> {
    let (start, _) = day_window(date);
    let events = events_between(conn, start, end.with_timezone(&Utc))?;
    Ok(render_full_report(&format!("Worklog - {date}"), &events))
}

pub fn render_share_day_until(
    conn: &Connection,
    date: NaiveDate,
    end: DateTime<Local>,
) -> WorklogResult<String> {
    let (start, _) = day_window(date);
    let events = events_between(conn, start, end.with_timezone(&Utc))?;
    Ok(render_share_context(&format!("Worklog - {date}"), &events))
}

fn day_window(date: NaiveDate) -> (DateTime<Utc>, DateTime<Utc>) {
    let midnight = match date.and_hms_opt(0, 0, 0) {
        Some(value) => value,
        None => unreachable!("midnight is valid for every NaiveDate"),
    };
    let start = match Local.from_local_datetime(&midnight) {
        LocalResult::Single(value) => value,
        LocalResult::Ambiguous(earliest, _) => earliest,
        LocalResult::None => Utc.from_utc_datetime(&midnight).with_timezone(&Local),
    };
    let end = start + Days::new(1);
    (start.with_timezone(&Utc), end.with_timezone(&Utc))
}

fn yesterday_window_end(now: DateTime<Local>) -> (NaiveDate, DateTime<Local>) {
    let date = now.date_naive() - Days::new(1);
    (date, day_start(now.date_naive()))
}

fn day_start(date: NaiveDate) -> DateTime<Local> {
    let midnight = match date.and_hms_opt(0, 0, 0) {
        Some(value) => value,
        None => unreachable!("midnight is valid for every NaiveDate"),
    };
    match Local.from_local_datetime(&midnight) {
        LocalResult::Single(value) => value,
        LocalResult::Ambiguous(earliest, _) => earliest,
        LocalResult::None => Utc.from_utc_datetime(&midnight).with_timezone(&Local),
    }
}
