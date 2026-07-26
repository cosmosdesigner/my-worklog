use std::collections::{BTreeMap, BTreeSet};

use crate::db::repositories::StoredEvent;
use crate::report::display::{event_full_text, event_metrics};
use crate::report::insights::ReportPeriod;

pub(super) fn text_report(
    title: &str,
    period: ReportPeriod,
    events: &[StoredEvent],
    predicate: impl Fn(&StoredEvent) -> bool,
    limit: usize,
) -> String {
    report(title, period, events, text_items(events, predicate, limit))
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

pub(super) fn status(period: ReportPeriod, events: &[StoredEvent]) -> String {
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
        text_items(events, super::is_blocker, 5),
    );
    section(
        &mut output,
        "Decisions",
        text_items(events, super::is_decision, 5),
    );
    section(
        &mut output,
        "Open loops",
        text_items(events, super::is_open_loop, 5),
    );
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

fn text_items(
    events: &[StoredEvent],
    predicate: impl Fn(&StoredEvent) -> bool,
    limit: usize,
) -> Vec<String> {
    let mut groups = BTreeMap::<String, TextItem>::new();
    for event in events.iter().filter(|event| predicate(event)) {
        let Some(label) = event_full_text(event).filter(|label| !is_low_value(label)) else {
            continue;
        };
        groups
            .entry(normalize_key(&label))
            .and_modify(|item| item.count += 1)
            .or_insert_with(|| TextItem::new(label, event.source_agent_id.clone()));
    }
    sorted(groups.into_values(), limit, |item| item.render())
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

fn count_items<'a>(labels: impl Iterator<Item = &'a str>, limit: usize, unit: &str) -> Vec<String> {
    let mut counts = BTreeMap::<String, CountItem>::new();
    for label in labels {
        let label = label.trim();
        counts
            .entry(label.to_string())
            .and_modify(|item| item.count += 1)
            .or_insert_with(|| CountItem::new(label));
    }
    sorted(counts.into_values(), limit, |item| item.render(unit))
}

fn sorted<T: ItemCount>(
    items: impl Iterator<Item = T>,
    limit: usize,
    render: impl Fn(&T) -> String,
) -> Vec<String> {
    let mut items = items.collect::<Vec<_>>();
    items.sort_by(|left, right| {
        right
            .count()
            .cmp(&left.count())
            .then_with(|| render(left).cmp(&render(right)))
    });
    items
        .into_iter()
        .take(limit)
        .map(|item| render(&item))
        .collect()
}

trait ItemCount {
    fn count(&self) -> usize;
}

struct TextItem {
    label: String,
    source_agent_id: String,
    count: usize,
}

impl TextItem {
    fn new(label: String, source_agent_id: String) -> Self {
        Self {
            label,
            source_agent_id,
            count: 1,
        }
    }

    fn render(&self) -> String {
        if self.count == 1 {
            format!("{} [{}]", self.label, self.source_agent_id)
        } else {
            format!(
                "{} ({} events) [{}]",
                self.label, self.count, self.source_agent_id
            )
        }
    }
}

impl ItemCount for TextItem {
    fn count(&self) -> usize {
        self.count
    }
}

struct CountItem {
    label: String,
    count: usize,
}

impl CountItem {
    fn new(label: &str) -> Self {
        Self {
            label: label.to_string(),
            count: 1,
        }
    }

    fn render(&self, unit: &str) -> String {
        format!("{}: {} {unit}", self.label, self.count)
    }
}

impl ItemCount for CountItem {
    fn count(&self) -> usize {
        self.count
    }
}

fn normalize_key(label: &str) -> String {
    let mut value = label.split_whitespace().collect::<Vec<_>>().join(" ");
    for prefix in [
        "Assistant:",
        "User:",
        "Todo:",
        "Decision:",
        "Decided:",
        "TODO:",
        "Next:",
        "Blocked:",
        "Blocker:",
    ] {
        value = strip_prefix_ci(&value, prefix).to_string();
    }
    value.to_lowercase()
}

fn strip_prefix_ci<'a>(value: &'a str, prefix: &str) -> &'a str {
    value
        .get(..prefix.len())
        .filter(|head| head.eq_ignore_ascii_case(prefix))
        .map_or(value, |_| value[prefix.len()..].trim_start())
}

fn is_low_value(label: &str) -> bool {
    matches!(
        normalize_key(label).as_str(),
        "session updated" | "message updated" | "tool result" | "command finished"
    )
}
