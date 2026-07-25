use crate::db::repositories::StoredEvent;
use serde_json::Value;

const MAX_LABEL_CHARS: usize = 180;

pub fn event_label(event: &StoredEvent) -> Option<String> {
    match event.event_type.as_str() {
        "user_prompt" => clean_text(event.content.as_deref())
            .or_else(|| clean_text(event.title.as_deref()))
            .map(|text| format!("User: {text}")),
        "assistant_message" => clean_text(event.content.as_deref())
            .or_else(|| clean_text(event.title.as_deref()))
            .map(|text| format!("Assistant: {text}")),
        "command" => clean_text(event.command.as_deref())
            .or_else(|| clean_text(event.content.as_deref()))
            .map(|text| format!("Command: {text}")),
        "file_edit" | "file.edited" => event
            .file_path
            .as_ref()
            .map(|path| format!("Edited: {path}")),
        "todo" | "todo.updated" => clean_text(event.title.as_deref())
            .or_else(|| clean_text(event.content.as_deref()))
            .map(|text| format!("Todo: {text}")),
        _ => clean_text(event.title.as_deref()).or_else(|| clean_text(event.content.as_deref())),
    }
}

pub fn event_full_text(event: &StoredEvent) -> Option<String> {
    match event.event_type.as_str() {
        "user_prompt" => clean_full_text(event.content.as_deref())
            .or_else(|| clean_full_text(event.title.as_deref()))
            .map(|text| format!("User: {text}")),
        "assistant_message" => clean_full_text(event.content.as_deref())
            .or_else(|| clean_full_text(event.title.as_deref()))
            .map(|text| format!("Assistant: {text}")),
        "command" => clean_full_text(event.command.as_deref())
            .or_else(|| clean_full_text(event.content.as_deref()))
            .map(|text| format!("Command: {text}")),
        "file_edit" | "file.edited" => event
            .file_path
            .as_ref()
            .map(|path| format!("Edited: {path}")),
        "todo" | "todo.updated" => clean_full_text(event.title.as_deref())
            .or_else(|| clean_full_text(event.content.as_deref()))
            .map(|text| format!("Todo: {text}")),
        _ => clean_full_text(event.title.as_deref())
            .or_else(|| clean_full_text(event.content.as_deref())),
    }
}

pub fn event_metrics(events: &[StoredEvent]) -> EventMetrics {
    let mut metrics = EventMetrics::default();
    for event in events {
        if let Some(duration_ms) = event.duration_ms.filter(|value| *value > 0) {
            metrics.duration_ms += duration_ms;
        }
        if let Some(raw_json) = event.raw_json.as_deref() {
            metrics.add_tokens(raw_json);
        }
    }
    metrics
}

pub fn event_kind_counts(events: &[StoredEvent]) -> EventKindCounts {
    let mut counts = EventKindCounts::default();
    for event in events {
        match event.event_type.as_str() {
            "user_prompt" => counts.user_prompts += 1,
            "assistant_message" => counts.assistant_messages += 1,
            "command" => counts.commands += 1,
            value if value.contains("file") => counts.file_events += 1,
            _ => {}
        }
    }
    counts
}

#[derive(Debug, Default, Clone, Copy)]
pub struct EventKindCounts {
    pub user_prompts: usize,
    pub assistant_messages: usize,
    pub commands: usize,
    pub file_events: usize,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct EventMetrics {
    pub duration_ms: i64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
}

impl EventMetrics {
    pub fn duration_label(self) -> Option<String> {
        if self.duration_ms <= 0 {
            return None;
        }
        Some(format_duration(self.duration_ms))
    }

    pub fn token_label(self) -> Option<String> {
        let total = if self.total_tokens > 0 {
            self.total_tokens
        } else {
            self.input_tokens + self.output_tokens
        };
        if total == 0 {
            return None;
        }
        Some(format!(
            "{} total ({} input, {} output)",
            format_count(total),
            format_count(self.input_tokens),
            format_count(self.output_tokens)
        ))
    }

    fn add_tokens(&mut self, raw_json: &str) {
        let Ok(value) = serde_json::from_str::<Value>(raw_json) else {
            return;
        };
        let usage = value.get("usage").unwrap_or(&value);
        self.input_tokens += token_value(usage, "input_tokens")
            .or_else(|| token_value(usage, "prompt_tokens"))
            .unwrap_or(0);
        self.output_tokens += token_value(usage, "output_tokens")
            .or_else(|| token_value(usage, "completion_tokens"))
            .unwrap_or(0);
        self.total_tokens += token_value(usage, "total_tokens").unwrap_or(0);
    }
}

fn clean_text(value: Option<&str>) -> Option<String> {
    clean_full_text(value).map(|text| truncate(&text))
}

fn clean_full_text(value: Option<&str>) -> Option<String> {
    let text = value?.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.is_empty() || looks_like_json(&text) {
        return None;
    }
    Some(text)
}

fn looks_like_json(value: &str) -> bool {
    let trimmed = value.trim_start();
    (trimmed.starts_with('{') && trimmed.contains('}'))
        || (trimmed.starts_with('[') && trimmed.contains(']'))
}

fn truncate(value: &str) -> String {
    let mut chars = value.chars();
    let truncated = chars.by_ref().take(MAX_LABEL_CHARS).collect::<String>();
    if chars.next().is_some() {
        format!("{truncated}...")
    } else {
        truncated
    }
}

fn token_value(value: &Value, key: &str) -> Option<u64> {
    value.get(key)?.as_u64()
}

fn format_duration(duration_ms: i64) -> String {
    let total_seconds = duration_ms / 1000;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    if hours > 0 {
        format!("{hours}h {minutes:02}m {seconds:02}s")
    } else {
        format!("{minutes}m {seconds:02}s")
    }
}

fn format_count(value: u64) -> String {
    let text = value.to_string();
    let mut output = String::with_capacity(text.len() + text.len() / 3);
    for (index, character) in text.chars().rev().enumerate() {
        if index > 0 && index % 3 == 0 {
            output.push(',');
        }
        output.push(character);
    }
    output.chars().rev().collect()
}
