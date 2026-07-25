use chrono::{DateTime, Utc};

use super::source::SourceAgent;

#[derive(Debug, Clone)]
pub struct SessionSummary {
    pub title: String,
    pub short_summary: String,
    pub project: Option<String>,
    pub agent: SourceAgent,
    pub started_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,
    pub main_activities: Vec<String>,
    pub files_touched: Vec<String>,
    pub commands_run: Vec<String>,
    pub errors: Vec<String>,
    pub decisions: Vec<String>,
    pub todos: Vec<String>,
    pub blockers: Vec<String>,
    pub continue_from_here: Vec<String>,
    pub confidence: f32,
}
