use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::model::source::SourceAgent;
use crate::privacy::redact::Redactor;

#[derive(Debug, Deserialize)]
pub struct SpoolRecord {
    pub source_agent: Option<String>,
    pub agent: Option<String>,
    pub source_session_id: Option<String>,
    pub session_id: Option<String>,
    pub source_event_id: Option<String>,
    pub event_id: Option<String>,
    #[serde(rename = "type")]
    pub event_type: Option<String>,
    pub role: Option<String>,
    pub timestamp: Option<DateTime<Utc>>,
    pub cwd: Option<String>,
    pub project_root: Option<String>,
    pub title: Option<String>,
    pub content: Option<String>,
    pub tool_name: Option<String>,
    pub command: Option<String>,
    pub file_path: Option<String>,
    pub status: Option<String>,
    pub duration_ms: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct NormalizedSpoolEvent {
    pub event_id: String,
    pub session_id: String,
    pub source_agent: SourceAgent,
    pub source_session_id: String,
    pub source_event_id: Option<String>,
    pub event_type: String,
    pub role: Option<String>,
    pub timestamp: Option<DateTime<Utc>>,
    pub cwd: Option<String>,
    pub project_root: Option<String>,
    pub title: Option<String>,
    pub content: Option<String>,
    pub normalized_content: Option<String>,
    pub tool_name: Option<String>,
    pub command: Option<String>,
    pub file_path: Option<String>,
    pub status: Option<String>,
    pub duration_ms: Option<i64>,
    pub raw_json: Option<String>,
    pub raw_ref: Option<String>,
}

impl SpoolRecord {
    pub fn normalize(
        self,
        inferred_agent: SourceAgent,
        redactor: &Redactor,
        raw: Value,
        raw_ref: String,
    ) -> NormalizedSpoolEvent {
        let source_agent = self
            .source_agent
            .or(self.agent)
            .and_then(|agent| agent.parse::<SourceAgent>().ok())
            .unwrap_or(inferred_agent);
        let source_session_id = self
            .source_session_id
            .or(self.session_id)
            .unwrap_or_else(|| stable_id(&raw_ref));
        let source_event_id = self.source_event_id.or(self.event_id);
        let redacted_content = self.content.map(|content| redactor.redact(&content));
        let redacted_title = self.title.map(|title| redactor.redact(&title));
        let event_type = self.event_type.unwrap_or_else(|| "unknown".to_owned());
        let event_id = stable_id(&format!(
            "{}:{}:{}:{}:{}:{}",
            source_agent.id(),
            source_session_id,
            source_event_id.as_deref().unwrap_or_default(),
            self.timestamp
                .map(|time| time.to_rfc3339())
                .unwrap_or_default(),
            event_type,
            redacted_content.as_deref().unwrap_or_default()
        ));
        NormalizedSpoolEvent {
            session_id: stable_id(&format!("{}:{source_session_id}", source_agent.id())),
            event_id,
            source_agent,
            source_session_id,
            source_event_id,
            event_type,
            role: self.role,
            timestamp: self.timestamp,
            cwd: self.cwd,
            project_root: self.project_root,
            title: redacted_title,
            normalized_content: redacted_content.clone(),
            content: redacted_content,
            tool_name: self.tool_name,
            command: self.command.map(|command| redactor.redact(&command)),
            file_path: self.file_path,
            status: self.status,
            duration_ms: self.duration_ms,
            raw_json: Some(redactor.redact(&raw.to_string())),
            raw_ref: Some(raw_ref),
        }
    }
}

pub fn stable_id(input: &str) -> String {
    let digest = Sha256::digest(input.as_bytes());
    format!("{:x}", digest)
}
