use chrono::{DateTime, Utc};
use my_worklog_core::ingest::normalize::{NormalizedSpoolEvent, stable_id};
use my_worklog_core::model::source::SourceAgent;
use my_worklog_core::privacy::redact::Redactor;
use serde_json::{Value, json};

#[derive(Debug, Clone)]
pub struct OpenCodeMessage {
    pub session_id: String,
    pub message_id: String,
    pub role: OpenCodeRole,
    pub timestamp: Option<DateTime<Utc>>,
    pub title: Option<String>,
    pub cwd: Option<String>,
    pub content: String,
    pub duration_ms: Option<i64>,
    pub raw_json: Option<String>,
    pub raw_ref: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenCodeRole {
    User,
    Assistant,
}

impl OpenCodeRole {
    const fn event_type(self) -> &'static str {
        match self {
            Self::User => "user_prompt",
            Self::Assistant => "assistant_message",
        }
    }

    const fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Assistant => "assistant",
        }
    }
}

pub fn messages_from_export(value: &Value, raw_ref: &str) -> Vec<OpenCodeMessage> {
    let info = value.get("info");
    let fallback_session_id = info
        .and_then(|info| string_at(info, &["id", "sessionID", "session_id"]))
        .unwrap_or_else(|| stable_id(raw_ref));
    let title = info.and_then(|info| string_at(info, &["title"]));
    let cwd = info.and_then(|info| string_at(info, &["directory", "cwd", "project_root"]));

    value
        .get("messages")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .enumerate()
        .filter_map(|(index, message)| {
            message_from_value(
                message,
                &fallback_session_id,
                title.clone(),
                cwd.clone(),
                &format!("{raw_ref}:messages[{index}]"),
            )
        })
        .collect()
}

pub fn message_from_db_rows(
    message_data: &str,
    part_data: &[String],
    fallback_session_id: &str,
    fallback_message_id: &str,
    raw_ref: &str,
) -> Option<OpenCodeMessage> {
    let data = serde_json::from_str::<Value>(message_data).ok()?;
    let text = text_from_part_strings(part_data);
    if text.trim().is_empty() {
        return None;
    }
    let role = parse_role(string_at(&data, &["role"]).as_deref()?)?;
    Some(OpenCodeMessage {
        session_id: string_at(&data, &["sessionID", "session_id"])
            .unwrap_or_else(|| fallback_session_id.to_owned()),
        message_id: string_at(&data, &["id"]).unwrap_or_else(|| fallback_message_id.to_owned()),
        role,
        timestamp: timestamp_from_value(&data),
        title: None,
        cwd: None,
        content: text,
        duration_ms: duration_ms_from_value(&data),
        raw_json: metrics_json_from_value(&data),
        raw_ref: raw_ref.to_owned(),
    })
}

pub fn to_normalized_event(message: &OpenCodeMessage, redactor: &Redactor) -> NormalizedSpoolEvent {
    let content = redactor.redact(&message.content);
    let title = message.title.as_ref().map(|title| redactor.redact(title));
    let source_session_id = message.session_id.clone();
    let source_event_id = Some(message.message_id.clone());
    let event_type = message.role.event_type().to_owned();
    let event_id = stable_id(&format!(
        "opencode:{}:{}:{}:{}:{}",
        source_session_id,
        message.message_id,
        message
            .timestamp
            .map(|timestamp| timestamp.to_rfc3339())
            .unwrap_or_default(),
        event_type,
        content
    ));
    NormalizedSpoolEvent {
        event_id,
        session_id: stable_id(&format!("opencode:{source_session_id}")),
        source_agent: SourceAgent::OpenCode,
        source_session_id,
        source_event_id,
        event_type,
        role: Some(message.role.as_str().to_owned()),
        timestamp: message.timestamp,
        cwd: message.cwd.clone(),
        project_root: message.cwd.clone(),
        title,
        content: Some(content.clone()),
        normalized_content: Some(content),
        tool_name: None,
        command: None,
        file_path: None,
        status: None,
        duration_ms: message.duration_ms,
        raw_json: message.raw_json.clone(),
        raw_ref: Some(message.raw_ref.clone()),
    }
}

fn message_from_value(
    value: &Value,
    fallback_session_id: &str,
    title: Option<String>,
    cwd: Option<String>,
    raw_ref: &str,
) -> Option<OpenCodeMessage> {
    let info = value.get("info").unwrap_or(value);
    let role = parse_role(string_at(info, &["role"]).as_deref()?)?;
    let content = string_at(value, &["content", "text"])
        .or_else(|| value.get("parts").and_then(text_from_parts))
        .unwrap_or_default();
    if content.trim().is_empty() {
        return None;
    }
    Some(OpenCodeMessage {
        session_id: string_at(info, &["sessionID", "session_id"])
            .unwrap_or_else(|| fallback_session_id.to_owned()),
        message_id: string_at(info, &["id"]).unwrap_or_else(|| stable_id(raw_ref)),
        role,
        timestamp: timestamp_from_value(info),
        title,
        cwd,
        content,
        duration_ms: duration_ms_from_value(info),
        raw_json: metrics_json_from_value(info),
        raw_ref: raw_ref.to_owned(),
    })
}

fn parse_role(value: &str) -> Option<OpenCodeRole> {
    match value {
        "user" => Some(OpenCodeRole::User),
        "assistant" => Some(OpenCodeRole::Assistant),
        _ => None,
    }
}

fn string_at(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(Value::as_str))
        .map(str::to_owned)
}

fn timestamp_from_value(value: &Value) -> Option<DateTime<Utc>> {
    timestamp_at(value, &["created", "created_at", "timestamp"]).or_else(|| {
        value
            .get("time")
            .and_then(|time| timestamp_at(time, &["created", "completed"]))
    })
}

fn duration_ms_from_value(value: &Value) -> Option<i64> {
    let time = value.get("time")?;
    let created = timestamp_at(time, &["created"])?;
    let completed = timestamp_at(time, &["completed"])?;
    Some((completed - created).num_milliseconds()).filter(|duration| *duration > 0)
}

fn metrics_json_from_value(value: &Value) -> Option<String> {
    let tokens = value.get("tokens")?;
    let input_tokens = token_at(tokens, &["input", "input_tokens", "prompt_tokens"])?;
    let output_tokens = token_at(tokens, &["output", "output_tokens", "completion_tokens"])?;
    Some(
        json!({
            "usage": {
                "input_tokens": input_tokens,
                "output_tokens": output_tokens,
                "total_tokens": input_tokens + output_tokens
            },
            "cost": value.get("cost").and_then(Value::as_f64)
        })
        .to_string(),
    )
}

fn token_at(value: &Value, keys: &[&str]) -> Option<u64> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(Value::as_u64))
}

fn timestamp_at(value: &Value, keys: &[&str]) -> Option<DateTime<Utc>> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(timestamp_leaf))
}

fn timestamp_leaf(value: &Value) -> Option<DateTime<Utc>> {
    match value {
        Value::String(timestamp) => DateTime::parse_from_rfc3339(timestamp)
            .ok()
            .map(|timestamp| timestamp.with_timezone(&Utc)),
        Value::Number(number) => number.as_i64().and_then(timestamp_from_unix),
        _ => None,
    }
}

fn timestamp_from_unix(value: i64) -> Option<DateTime<Utc>> {
    if value.unsigned_abs() >= 1_000_000_000_000 {
        DateTime::from_timestamp_millis(value)
    } else {
        DateTime::from_timestamp(value, 0)
    }
}

fn text_from_parts(value: &Value) -> Option<String> {
    let parts = value.as_array()?;
    let text = parts
        .iter()
        .filter_map(text_from_part)
        .collect::<Vec<_>>()
        .join("\n\n");
    if text.trim().is_empty() {
        None
    } else {
        Some(text)
    }
}

fn text_from_part_strings(parts: &[String]) -> String {
    parts
        .iter()
        .filter_map(|part| serde_json::from_str::<Value>(part).ok())
        .filter_map(|part| text_from_part(&part))
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn text_from_part(value: &Value) -> Option<String> {
    match value.get("type").and_then(Value::as_str) {
        Some("text") => value.get("text").and_then(Value::as_str).map(str::to_owned),
        _ => None,
    }
}
