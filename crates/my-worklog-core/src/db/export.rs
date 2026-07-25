use rusqlite::Connection;
use serde::Serialize;

use crate::error::WorklogResult;

#[derive(Debug, Clone, Serialize)]
pub struct RawStoredEvent {
    pub id: String,
    pub source_agent_id: String,
    pub session_id: String,
    pub source_event_id: Option<String>,
    pub event_type: String,
    pub role: Option<String>,
    pub timestamp: Option<String>,
    pub cwd: Option<String>,
    pub title: Option<String>,
    pub content: Option<String>,
    pub normalized_content: Option<String>,
    pub tool_name: Option<String>,
    pub command: Option<String>,
    pub file_path: Option<String>,
    pub status: Option<String>,
    pub duration_ms: Option<i64>,
    pub raw_json: Option<String>,
}

pub fn raw_events(conn: &Connection) -> WorklogResult<Vec<RawStoredEvent>> {
    let mut stmt = conn.prepare(
        "SELECT id, source_agent_id, session_id, source_event_id, type, role, timestamp, cwd,
                title, content, normalized_content, tool_name, command, file_path, status,
                duration_ms, raw_json
         FROM work_event
         ORDER BY timestamp ASC, id ASC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(RawStoredEvent {
            id: row.get(0)?,
            source_agent_id: row.get(1)?,
            session_id: row.get(2)?,
            source_event_id: row.get(3)?,
            event_type: row.get(4)?,
            role: row.get(5)?,
            timestamp: row.get(6)?,
            cwd: row.get(7)?,
            title: row.get(8)?,
            content: row.get(9)?,
            normalized_content: row.get(10)?,
            tool_name: row.get(11)?,
            command: row.get(12)?,
            file_path: row.get(13)?,
            status: row.get(14)?,
            duration_ms: row.get(15)?,
            raw_json: row.get(16)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}
