use chrono::{Datelike, Days, Local, LocalResult, TimeZone, Utc};
use rusqlite::Connection;

use crate::db::repositories::events_between;
use crate::error::WorklogResult;
use crate::manual::list_between;
use crate::report::markdown::{
    render_full_report_with_manual, render_report_with_manual, render_share_context_with_manual,
};

pub fn week(conn: &Connection) -> WorklogResult<String> {
    render_week_at(conn, Local::now(), WeekOutput::Terminal)
}

pub fn week_full(conn: &Connection) -> WorklogResult<String> {
    render_week_at(conn, Local::now(), WeekOutput::Full)
}

pub fn week_share_context(conn: &Connection) -> WorklogResult<String> {
    render_week_at(conn, Local::now(), WeekOutput::Share)
}

pub fn render_week_until(conn: &Connection, end: chrono::DateTime<Local>) -> WorklogResult<String> {
    render_week_at(conn, end, WeekOutput::Terminal)
}

pub fn render_full_week_until(
    conn: &Connection,
    end: chrono::DateTime<Local>,
) -> WorklogResult<String> {
    render_week_at(conn, end, WeekOutput::Full)
}

enum WeekOutput {
    Terminal,
    Full,
    Share,
}

fn render_week_at(
    conn: &Connection,
    end: chrono::DateTime<Local>,
    output: WeekOutput,
) -> WorklogResult<String> {
    let today = end.date_naive();
    let days_from_monday = u64::from(today.weekday().num_days_from_monday());
    let monday = today - Days::new(days_from_monday);
    let midnight = match monday.and_hms_opt(0, 0, 0) {
        Some(value) => value,
        None => unreachable!("midnight is valid for every NaiveDate"),
    };
    let start = match Local.from_local_datetime(&midnight) {
        LocalResult::Single(value) => value,
        LocalResult::Ambiguous(earliest, _) => earliest,
        LocalResult::None => Utc.from_utc_datetime(&midnight).with_timezone(&Local),
    };
    let start = start.with_timezone(&Utc);
    let end = end.with_timezone(&Utc);
    let events = events_between(conn, start, end)?;
    let manual = list_between(conn, start, end)?;
    let title = format!(
        "Weekly Worklog - {}-W{:02}",
        today.year(),
        today.iso_week().week()
    );
    match output {
        WeekOutput::Terminal => Ok(render_report_with_manual(&title, &events, &manual)),
        WeekOutput::Full => Ok(render_full_report_with_manual(&title, &events, &manual)),
        WeekOutput::Share => Ok(render_share_context_with_manual(&title, &events, &manual)),
    }
}
