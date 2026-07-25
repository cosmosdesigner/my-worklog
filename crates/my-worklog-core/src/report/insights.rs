use chrono::{Datelike, Days, Local, LocalResult, NaiveDate, TimeZone, Utc};
use rusqlite::Connection;

use crate::db::repositories::{StoredEvent, events_between};
use crate::error::WorklogResult;
use crate::report::display::{event_full_text, event_metrics};

#[derive(Debug, Clone, Copy)]
pub enum ReportPeriod {
    Today,
    Yesterday,
    Week,
}

pub fn decisions(conn: &Connection, period: ReportPeriod) -> WorklogResult<String> {
    let events = period_events(conn, period)?;
    Ok(render_filtered("Decisions", period, &events, |event| {
        contains_any(
            event,
            &["decision", "decided", "choose", "chose", "use ", "we will"],
        )
    }))
}

pub fn open_loops(conn: &Connection, period: ReportPeriod) -> WorklogResult<String> {
    let events = period_events(conn, period)?;
    Ok(render_filtered("Open loops", period, &events, |event| {
        contains_any(
            event,
            &["todo", "open loop", "follow up", "next", "remaining"],
        )
    }))
}

pub fn blockers(conn: &Connection, period: ReportPeriod) -> WorklogResult<String> {
    let events = period_events(conn, period)?;
    Ok(render_filtered("Blockers", period, &events, |event| {
        contains_any(
            event,
            &["blocked", "blocker", "cannot", "can't", "failed", "error"],
        )
    }))
}

pub fn files(conn: &Connection, period: ReportPeriod) -> WorklogResult<String> {
    let events = period_events(conn, period)?;
    Ok(render_filtered("Files", period, &events, |event| {
        event.file_path.is_some() || event.event_type.contains("file")
    }))
}

pub fn commands(conn: &Connection, period: ReportPeriod) -> WorklogResult<String> {
    let events = period_events(conn, period)?;
    Ok(render_filtered("Commands", period, &events, |event| {
        event.command.is_some() || event.event_type == "command"
    }))
}

pub fn agents(conn: &Connection, period: ReportPeriod) -> WorklogResult<String> {
    let events = period_events(conn, period)?;
    let mut agents = events
        .iter()
        .map(|event| event.source_agent_id.as_str())
        .collect::<Vec<_>>();
    agents.sort_unstable();
    let mut output = header("Agents", period, &events);
    if agents.is_empty() {
        output.push_str("No captured work events for this period.\n");
        return Ok(output);
    }
    let mut current = agents[0];
    let mut count = 0usize;
    for agent in agents {
        if agent != current {
            output.push_str(&format!("- {current}: {count} events\n"));
            current = agent;
            count = 0;
        }
        count += 1;
    }
    output.push_str(&format!("- {current}: {count} events\n"));
    Ok(output)
}

fn render_filtered(
    title: &str,
    period: ReportPeriod,
    events: &[StoredEvent],
    predicate: impl Fn(&StoredEvent) -> bool,
) -> String {
    let mut output = header(title, period, events);
    let mut shown = 0usize;
    for event in events.iter().filter(|event| predicate(event)).take(30) {
        if let Some(text) = event_full_text(event) {
            output.push_str(&format!("- {} [{}]\n", text, event.source_agent_id));
            shown += 1;
        }
    }
    if shown == 0 {
        output.push_str(&format!(
            "No {} found for this period.\n",
            title.to_lowercase()
        ));
    }
    output
}

fn header(title: &str, period: ReportPeriod, events: &[StoredEvent]) -> String {
    let metrics = event_metrics(events);
    let duration = metrics
        .duration_label()
        .unwrap_or_else(|| "Not available".to_string());
    let tokens = metrics
        .token_label()
        .unwrap_or_else(|| "Not available".to_string());
    format!(
        "# {title} - {}\n\n## Metrics\n- Events: {}\n- Total time: {duration}\n- Tokens: {tokens}\n\n## Results\n",
        period.label(),
        events.len()
    )
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

impl ReportPeriod {
    fn label(self) -> &'static str {
        match self {
            Self::Today => "today",
            Self::Yesterday => "yesterday",
            Self::Week => "this week",
        }
    }
}
