use std::collections::BTreeMap;

use crate::db::repositories::StoredEvent;
use crate::report::display::event_full_text;

#[derive(Debug, Clone, Copy)]
pub(in crate::report::insights) struct TextOptions {
    pub(in crate::report::insights) limit: usize,
    pub(in crate::report::insights) max_chars: usize,
}

impl TextOptions {
    pub(in crate::report::insights) const FULL: Self = Self {
        limit: 30,
        max_chars: 220,
    };

    pub(in crate::report::insights) const COMPACT: Self = Self {
        limit: 3,
        max_chars: 96,
    };
}

pub(super) fn text_items(
    events: &[StoredEvent],
    predicate: impl Fn(&StoredEvent) -> bool,
    options: TextOptions,
) -> Vec<String> {
    let mut groups = BTreeMap::<String, TextItem>::new();
    for event in events.iter().filter(|event| predicate(event)) {
        let Some(label) = event_full_text(event).filter(|label| !is_low_value(label)) else {
            continue;
        };
        let label = truncate(&label, options.max_chars);
        groups
            .entry(normalize_key(&label))
            .and_modify(|item| item.count += 1)
            .or_insert_with(|| TextItem::new(label, event.source_agent_id.clone()));
    }
    sorted(groups.into_values(), options.limit, |item| item.render())
}

pub(super) fn count_items<'a>(
    labels: impl Iterator<Item = &'a str>,
    limit: usize,
    unit: &str,
) -> Vec<String> {
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
        "Completed:",
        "Finished:",
        "Done:",
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

fn truncate(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let truncated = chars.by_ref().take(max_chars).collect::<String>();
    if chars.next().is_some() {
        format!("{truncated}...")
    } else {
        truncated
    }
}
