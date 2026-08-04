use crate::db::repositories::StoredEvent;
use crate::manual::ManualEntry;
use crate::report::display::{event_full_text, event_kind_counts, event_label, event_metrics};
use crate::report::project;

pub fn render_report(title: &str, events: &[StoredEvent]) -> String {
    render_report_with_manual(title, events, &[])
}

pub fn render_report_with_manual(
    title: &str,
    events: &[StoredEvent],
    manual: &[ManualEntry],
) -> String {
    render_report_with_limit(title, events, manual, Some(20))
}

pub fn render_full_report(title: &str, events: &[StoredEvent]) -> String {
    render_full_report_with_manual(title, events, &[])
}

pub fn render_full_report_with_manual(
    title: &str,
    events: &[StoredEvent],
    manual: &[ManualEntry],
) -> String {
    render_report_with_limit(title, events, manual, None)
}

pub fn render_share_context(title: &str, events: &[StoredEvent]) -> String {
    render_share_context_with_manual(title, events, &[])
}

pub fn render_share_context_with_manual(
    title: &str,
    events: &[StoredEvent],
    manual: &[ManualEntry],
) -> String {
    let mut output = render_summary(title, events, manual);
    output.push_str("## Session context\n");
    if events.is_empty() {
        output.push_str("No captured work events for this period.\n");
    }
    let mut current_session = "";
    let mut shown = 0usize;
    for event in events {
        let Some(text) = event_full_text(event) else {
            continue;
        };
        if event.session_id != current_session {
            current_session = &event.session_id;
            output.push_str(&format!("\n### Session {}\n", event.session_id));
        }
        output.push_str(&format!("- {} [{}]\n", text, event.source_agent_id));
        shown += 1;
    }
    if shown == 0 {
        output.push_str("No human-readable work events for this period. Use an explicit raw/export command to inspect stored provider events.\n");
    }
    render_manual_section(&mut output, manual);
    output
}

fn render_report_with_limit(
    title: &str,
    events: &[StoredEvent],
    manual: &[ManualEntry],
    limit: Option<usize>,
) -> String {
    let mut output = render_summary(title, events, manual);
    if project::has_multiple_projects(events) {
        output.push_str("## Projects\n");
        for line in project::count_lines(events) {
            output.push_str(&format!("- {line}\n"));
        }
        output.push('\n');
    }
    output.push_str("## Main work\n");
    if events.is_empty() {
        output.push_str("No captured work events for this period.\n");
    } else if project::has_multiple_projects(events) {
        render_grouped_work(&mut output, events, limit);
    } else {
        render_flat_work(&mut output, events, limit);
    }
    render_manual_section(&mut output, manual);
    output
}

fn render_flat_work(output: &mut String, events: &[StoredEvent], limit: Option<usize>) {
    let mut shown = 0usize;
    for event in events {
        let Some(label) = event_label(event) else {
            continue;
        };
        output.push_str(&format!("- {} [{}]\n", label, event.source_agent_id));
        shown += 1;
        if limit.is_some_and(|limit| shown == limit) {
            break;
        }
    }
    if shown == 0 {
        output.push_str("No human-readable work events for this period. Use an explicit raw/export command to inspect stored provider events.\n");
    }
}

fn render_grouped_work(output: &mut String, events: &[StoredEvent], limit: Option<usize>) {
    let mut shown = 0usize;
    for group in project::groups(events) {
        let mut lines = Vec::new();
        for event in group.events {
            let Some(label) = event_label(event) else {
                continue;
            };
            lines.push(format!("- {} [{}]\n", label, event.source_agent_id));
            shown += 1;
            if limit.is_some_and(|limit| shown == limit) {
                break;
            }
        }
        if !lines.is_empty() {
            output.push_str(&format!("\n### {}\n", group.label));
            for line in lines {
                output.push_str(&line);
            }
        }
        if limit.is_some_and(|limit| shown == limit) {
            break;
        }
    }
    if shown == 0 {
        output.push_str("No human-readable work events for this period. Use an explicit raw/export command to inspect stored provider events.\n");
    }
}

fn render_summary(title: &str, events: &[StoredEvent], manual: &[ManualEntry]) -> String {
    let mut output = format!("# {title}\n\n");
    output.push_str("## At a glance\n");
    output.push_str(&format!("- Events: {}\n", events.len()));
    output.push_str(&format!(
        "- Sessions: {}\n\n",
        count_unique_sessions(events)
    ));
    let counts = event_kind_counts(events);
    let session_count = count_unique_sessions(events);
    output.push_str("## Metrics\n");
    output.push_str(&format!("- Events: {}\n", events.len()));
    output.push_str(&format!("- Sessions: {session_count}\n"));
    output.push_str(&format!("- User prompts: {}\n", counts.user_prompts));
    output.push_str(&format!(
        "- Assistant messages: {}\n",
        counts.assistant_messages
    ));
    output.push_str(&format!("- Commands: {}\n", counts.commands));
    output.push_str(&format!("- File events: {}\n", counts.file_events));
    let metrics = event_metrics(events);
    let duration = metrics
        .duration_label()
        .unwrap_or_else(|| "Not available".to_string());
    let tokens = metrics
        .token_label()
        .unwrap_or_else(|| "Not available".to_string());
    output.push_str(&format!("- Captured agent-session time: {duration}\n"));
    output.push_str(&format!("- Tokens: {tokens}\n"));
    let manual_ms: i64 = manual
        .iter()
        .map(|entry| (entry.end - entry.start).num_milliseconds())
        .sum();
    output.push_str(&format!(
        "- Manual time: {}\n",
        format_manual_duration(manual_ms)
    ));
    output.push_str(&format!(
        "- Total accounted time: {}\n",
        format_manual_duration(metrics.duration_ms + manual_ms)
    ));
    if manual.is_empty() {
        output.push_str("Coverage: captured coding-agent events only; meetings, manual coding, review, planning, browser work, and other uncaptured activity are excluded.\n\n");
    } else {
        output.push_str(
            "Coverage: imported coding-agent events and user-entered manual activity.\n\n",
        );
    }
    output
}

fn render_manual_section(output: &mut String, manual: &[ManualEntry]) {
    output.push_str("## Manual activity\n");
    if manual.is_empty() {
        output.push_str("No manual entries for this period.\n");
        return;
    }
    for entry in manual {
        let minutes = (entry.end - entry.start).num_minutes();
        output.push_str(&format!(
            "- {} [{}] {} ({}m)\n",
            entry.description, entry.category, entry.project, minutes
        ));
        if let Some(tags) = &entry.tags {
            output.push_str(&format!("  Tags: {tags}\n"));
        }
    }
    let mut overlaps = Vec::new();
    for (index, entry) in manual.iter().enumerate() {
        if manual[index + 1..]
            .iter()
            .any(|other| other.start < entry.end && other.end > entry.start)
        {
            overlaps.push(entry.id.as_str());
        }
    }
    if !overlaps.is_empty() {
        output.push_str(
            "Warning: overlapping manual entries detected; durations were not changed.\n",
        );
    }
}

fn format_manual_duration(milliseconds: i64) -> String {
    let minutes = milliseconds.max(0) / 60_000;
    format!("{}h {:02}m", minutes / 60, minutes % 60)
}

fn count_unique_sessions(events: &[StoredEvent]) -> usize {
    let mut sessions = events
        .iter()
        .map(|event| event.session_id.as_str())
        .collect::<Vec<_>>();
    sessions.sort_unstable();
    sessions.dedup();
    sessions.len()
}
