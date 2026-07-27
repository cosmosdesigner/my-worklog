mod items;

use std::collections::BTreeSet;

use crate::db::repositories::StoredEvent;
use crate::report::display::event_metrics;
use crate::report::insights::ReportPeriod;
use crate::report::project;

pub(super) use self::items::TextOptions;
use self::items::{count_items, text_items};

pub(super) fn text_report(
    title: &str,
    period: ReportPeriod,
    events: &[StoredEvent],
    predicate: impl Fn(&StoredEvent) -> bool,
    options: TextOptions,
) -> String {
    report_with(title, period, events, |events| {
        text_items(events, &predicate, options)
    })
}

pub(super) fn file_report(period: ReportPeriod, events: &[StoredEvent], limit: usize) -> String {
    report_with("Files", period, events, |events| file_items(events, limit))
}

pub(super) fn command_report(period: ReportPeriod, events: &[StoredEvent], limit: usize) -> String {
    report_with("Commands", period, events, |events| {
        command_items(events, limit)
    })
}

pub(super) fn agent_report(period: ReportPeriod, events: &[StoredEvent], limit: usize) -> String {
    report_with("Agents", period, events, |events| {
        agent_items(events, limit)
    })
}

pub(super) fn done_report(period: ReportPeriod, events: &[StoredEvent]) -> String {
    report_with("Done", period, events, |events| {
        done_items(events, TextOptions::FULL)
    })
}

pub(super) fn done_compact(period: ReportPeriod, events: &[StoredEvent]) -> String {
    report_with("Done", period, events, |events| {
        done_items(events, TextOptions::COMPACT)
    })
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
    if project::has_multiple_projects(events) {
        projects(&mut output, events);
    }
    section_with(&mut output, "Blockers", events, |events| {
        text_items(events, is_blocker, options)
    });
    section_with(&mut output, "Decisions", events, |events| {
        text_items(events, is_decision, options)
    });
    section_with(&mut output, "Open loops", events, |events| {
        text_items(events, is_open_loop, options)
    });
    section_with(&mut output, "Completed work", events, |events| {
        done_items(events, options)
    });
    section_with(&mut output, "Files", events, |events| file_items(events, 5));
    section_with(&mut output, "Commands", events, |events| {
        command_items(events, 5)
    });
    section_with(&mut output, "Agents", events, |events| {
        agent_items(events, 5)
    });
    output
}

fn report_with(
    title: &str,
    period: ReportPeriod,
    events: &[StoredEvent],
    items: impl Fn(&[StoredEvent]) -> Vec<String>,
) -> String {
    let mut output = header(title, period, events);
    if project::has_multiple_projects(events) {
        projects(&mut output, events);
        grouped_results(&mut output, title, events, items);
    } else {
        results(&mut output, title, items(events));
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

fn section_with(
    output: &mut String,
    title: &str,
    events: &[StoredEvent],
    items: impl Fn(&[StoredEvent]) -> Vec<String>,
) {
    output.push_str(&format!("\n## {title}\n"));
    if project::has_multiple_projects(events) {
        grouped_results(output, title, events, items);
    } else {
        results(output, title, items(events));
    }
}

fn projects(output: &mut String, events: &[StoredEvent]) {
    output.push_str("\n## Projects\n");
    for line in project::count_lines(events) {
        output.push_str(&format!("- {line}\n"));
    }
}

fn grouped_results(
    output: &mut String,
    title: &str,
    events: &[StoredEvent],
    items: impl Fn(&[StoredEvent]) -> Vec<String>,
) {
    let mut wrote = false;
    for group in project::groups(events) {
        let project_events = group.events.into_iter().cloned().collect::<Vec<_>>();
        let project_items = items(&project_events);
        if project_items.is_empty() {
            continue;
        }
        output.push_str(&format!("\n### {}\n", group.label));
        for item in project_items {
            output.push_str(&format!("- {item}\n"));
        }
        wrote = true;
    }
    if !wrote {
        output.push_str(&format!(
            "No {} found for this period.\n",
            title.to_lowercase()
        ));
    }
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
