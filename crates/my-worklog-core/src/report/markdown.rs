use crate::db::repositories::StoredEvent;
use crate::report::display::{event_full_text, event_kind_counts, event_label, event_metrics};

pub fn render_report(title: &str, events: &[StoredEvent]) -> String {
    render_report_with_limit(title, events, Some(20))
}

pub fn render_full_report(title: &str, events: &[StoredEvent]) -> String {
    render_report_with_limit(title, events, None)
}

pub fn render_share_context(title: &str, events: &[StoredEvent]) -> String {
    let mut output = render_summary(title, events);
    output.push_str("## Session context\n");
    if events.is_empty() {
        output.push_str("No captured work events for this period.\n");
        return output;
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
    output
}

fn render_report_with_limit(title: &str, events: &[StoredEvent], limit: Option<usize>) -> String {
    let mut output = render_summary(title, events);
    output.push_str("## Main work\n");
    if events.is_empty() {
        output.push_str("No captured work events for this period.\n");
        return output;
    }
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
    output
}

fn render_summary(title: &str, events: &[StoredEvent]) -> String {
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
    output.push_str(&format!("- Total time: {duration}\n"));
    output.push_str(&format!("- Tokens: {tokens}\n\n"));
    output
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
