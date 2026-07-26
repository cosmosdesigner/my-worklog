mod items;

use std::collections::BTreeSet;

use crate::db::repositories::StoredEvent;
use crate::report::display::event_metrics;
use crate::report::insights::ReportPeriod;

pub(super) use self::items::TextOptions;
use self::items::{count_items, text_items};

pub(super) fn text_report(
    title: &str,
    period: ReportPeriod,
    events: &[StoredEvent],
    predicate: impl Fn(&StoredEvent) -> bool,
    options: TextOptions,
) -> String {
    report(
        title,
        period,
        events,
        text_items(events, predicate, options),
    )
}

pub(super) fn file_report(period: ReportPeriod, events: &[StoredEvent], limit: usize) -> String {
    report("Files", period, events, file_items(events, limit))
}

pub(super) fn command_report(period: ReportPeriod, events: &[StoredEvent], limit: usize) -> String {
    report("Commands", period, events, command_items(events, limit))
}

pub(super) fn agent_report(period: ReportPeriod, events: &[StoredEvent], limit: usize) -> String {
    report("Agents", period, events, agent_items(events, limit))
}

pub(super) fn done_report(period: ReportPeriod, events: &[StoredEvent]) -> String {
    report(
        "Done",
        period,
        events,
        done_items(events, TextOptions::FULL),
    )
}

pub(super) fn done_compact(period: ReportPeriod, events: &[StoredEvent]) -> String {
    report(
        "Done",
        period,
        events,
        done_items(events, TextOptions::COMPACT),
    )
}

pub(super) fn status(period: ReportPeriod, events: &[StoredEvent], options: TextOptions) -> String {
    status_with(
        period,
        events,
        options,
        super::is_blocker,
        super::is_decision,
        super::is_open_loop,
    )
}

pub(super) fn status_compact(period: ReportPeriod, events: &[StoredEvent]) -> String {
    status_with(
        period,
        events,
        TextOptions::COMPACT,
        super::is_compact_blocker,
        super::is_compact_decision,
        super::is_compact_open_loop,
    )
}

fn status_with(
    period: ReportPeriod,
    events: &[StoredEvent],
    options: TextOptions,
    is_blocker: fn(&StoredEvent) -> bool,
    is_decision: fn(&StoredEvent) -> bool,
    is_open_loop: fn(&StoredEvent) -> bool,
) -> String {
    let metrics = event_metrics(events);
    let duration = metrics
        .duration_label()
        .unwrap_or_else(|| "Not available".to_string());
    let tokens = metrics
        .token_label()
        .unwrap_or_else(|| "Not available".to_string());
    let sessions = events
        .iter()
        .map(|event| event.session_id.as_str())
        .collect::<BTreeSet<_>>()
        .len();
    let mut output = format!(
        "# Status - {}\n\n## At a glance\n- Events: {}\n- Sessions: {sessions}\n- Total time: {duration}\n- Tokens: {tokens}\n",
        period.label(),
        events.len()
    );
    section(
        &mut output,
        "Blockers",
        text_items(events, is_blocker, options),
    );
    section(
        &mut output,
        "Decisions",
        text_items(events, is_decision, options),
    );
    section(
        &mut output,
        "Open loops",
        text_items(events, is_open_loop, options),
    );
    section(&mut output, "Completed work", done_items(events, options));
    section(&mut output, "Files", file_items(events, 5));
    section(&mut output, "Commands", command_items(events, 5));
    section(&mut output, "Agents", agent_items(events, 5));
    output
}

fn report(title: &str, period: ReportPeriod, events: &[StoredEvent], items: Vec<String>) -> String {
    let mut output = header(title, period, events);
    results(&mut output, title, items);
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

fn section(output: &mut String, title: &str, items: Vec<String>) {
    output.push_str(&format!("\n## {title}\n"));
    results(output, title, items);
}

fn results(output: &mut String, title: &str, items: Vec<String>) {
    if items.is_empty() {
        output.push_str(&format!(
            "No {} found for this period.\n",
            title.to_lowercase()
        ));
        return;
    }
    for item in items {
        output.push_str(&format!("- {item}\n"));
    }
}

fn done_items(events: &[StoredEvent], options: TextOptions) -> Vec<String> {
    text_items(events, super::is_done, options)
}

fn file_items(events: &[StoredEvent], limit: usize) -> Vec<String> {
    count_items(
        events.iter().filter_map(|event| event.file_path.as_deref()),
        limit,
        "events",
    )
}

fn command_items(events: &[StoredEvent], limit: usize) -> Vec<String> {
    count_items(
        events.iter().filter_map(|event| event.command.as_deref()),
        limit,
        "runs",
    )
}

fn agent_items(events: &[StoredEvent], limit: usize) -> Vec<String> {
    count_items(
        events.iter().map(|event| event.source_agent_id.as_str()),
        limit,
        "events",
    )
}
