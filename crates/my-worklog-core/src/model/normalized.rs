use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::source::SourceAgent;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkSessionStatus {
    Active,
    Idle,
    Completed,
    Failed,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedSession {
    pub id: String,
    pub source_agent: SourceAgent,
    pub source_session_id: String,
    pub project_root: Option<PathBuf>,
    pub cwd: Option<PathBuf>,
    pub title: Option<String>,
    pub started_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub git_branch: Option<String>,
    pub git_commit_start: Option<String>,
    pub git_commit_end: Option<String>,
    pub status: WorkSessionStatus,
    pub raw_ref: Option<String>,
    pub events: Vec<NormalizedEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NormalizedEventType {
    SessionStart,
    SessionEnd,
    SessionIdle,
    UserPrompt,
    AssistantMessage,
    ToolCall,
    ToolResult,
    Command,
    FileRead,
    FileEdit,
    FileCreate,
    FileDelete,
    TestRun,
    BuildRun,
    LintRun,
    Error,
    Todo,
    Decision,
    Blocker,
    Summary,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedEvent {
    pub id: String,
    pub source_event_id: Option<String>,
    pub event_type: NormalizedEventType,
    pub role: Option<String>,
    pub timestamp: Option<DateTime<Utc>>,
    pub cwd: Option<PathBuf>,
    pub title: Option<String>,
    pub content: Option<String>,
    pub normalized_content: Option<String>,
    pub tool_name: Option<String>,
    pub command: Option<String>,
    pub file_path: Option<PathBuf>,
    pub status: Option<String>,
    pub duration_ms: Option<i64>,
    pub raw: Option<serde_json::Value>,
    pub redacted: bool,
}
