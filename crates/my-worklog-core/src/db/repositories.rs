use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, params};

use crate::db::cleanup::install_noise_function;
use crate::db::event_rows::{
    EVENT_BY_ID_SQL, EVENTS_BETWEEN_SQL, SEARCH_LIKE_SQL, collect_events, row_to_event,
};
use crate::error::WorklogResult;
use crate::ingest::normalize::{NormalizedSpoolEvent, stable_id};

#[derive(Debug, Clone)]
pub struct EventRow {
    pub session_id: String,
    pub source_agent_id: String,
    pub title: Option<String>,
    pub content: Option<String>,
    pub timestamp: Option<String>,
    pub cwd: Option<String>,
    pub project_root: Option<String>,
    pub project_name: Option<String>,
    pub command: Option<String>,
    pub file_path: Option<String>,
    pub duration_ms: Option<i64>,
    pub raw_json: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ImportOutcome {
    pub imported: usize,
    pub duplicates: usize,
    pub malformed: usize,
    pub skipped_noise: usize,
}

#[derive(Debug, Clone)]
pub struct StoredEvent {
    pub id: String,
    pub source_agent_id: String,
    pub session_id: String,
    pub event_type: String,
    pub title: Option<String>,
    pub content: Option<String>,
    pub timestamp: Option<String>,
    pub cwd: Option<String>,
    pub project_root: Option<String>,
    pub project_name: Option<String>,
    pub command: Option<String>,
    pub file_path: Option<String>,
    pub duration_ms: Option<i64>,
    pub raw_json: Option<String>,
}

pub fn insert_event(conn: &Connection, event: &NormalizedSpoolEvent) -> WorklogResult<bool> {
    let now = Utc::now().to_rfc3339();
    let cwd = non_blank(event.cwd.as_deref());
    let project_root = non_blank(event.project_root.as_deref());
    let project_id = project_root.map(stable_id);
    if let (Some(id), Some(root)) = (&project_id, project_root) {
        conn.execute(
            "INSERT OR IGNORE INTO project (id, root_path, name, git_remote, created_at, updated_at)
             VALUES (?1, ?2, ?3, NULL, ?4, ?4)",
            params![id, root, project_name(root), now],
        )?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO work_session
         (id, source_agent_id, source_session_id, project_id, title, started_at, ended_at,
          last_seen_at, cwd, status, summary, raw_ref, imported_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, ?6, ?7, 'unknown', NULL, ?8, ?9, ?9)",
        params![
            event.session_id,
            event.source_agent.id(),
            event.source_session_id,
            project_id,
            event.title,
            optional_time(event.timestamp),
            cwd,
            event.raw_ref,
            now,
        ],
    )?;
    conn.execute(
        "UPDATE work_session
         SET project_id = CASE WHEN project_id IS NULL THEN ?2 ELSE project_id END,
             cwd = CASE WHEN NULLIF(TRIM(cwd), '') IS NULL THEN ?3 ELSE cwd END,
             last_seen_at = COALESCE(?4, last_seen_at),
             updated_at = ?5
          WHERE id = ?1",
        params![
            event.session_id,
            project_id,
            cwd,
            optional_time(event.timestamp),
            now,
        ],
    )?;
    let changed = conn.execute(
        "INSERT OR IGNORE INTO work_event
         (id, session_id, source_agent_id, source_event_id, type, role, timestamp, cwd, title,
          content, normalized_content, tool_name, command, file_path, status, duration_ms, raw_json,
          redacted, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, 1, ?18)",
        params![
            event.event_id,
            event.session_id,
            event.source_agent.id(),
            event.source_event_id,
            event.event_type,
            event.role,
            optional_time(event.timestamp),
            cwd,
            event.title,
            event.content,
            event.normalized_content,
            event.tool_name,
            event.command,
            event.file_path,
            event.status,
            event.duration_ms,
            event.raw_json,
            now,
        ],
    )?;
    if changed == 1 {
        index_event(conn, event)?;
    } else {
        conn.execute(
            "UPDATE work_event
             SET cwd = CASE WHEN NULLIF(TRIM(cwd), '') IS NULL THEN ?2 ELSE cwd END,
                 duration_ms = COALESCE(duration_ms, ?3),
                 raw_json = COALESCE(raw_json, ?4)
              WHERE id = ?1",
            params![event.event_id, cwd, event.duration_ms, event.raw_json],
        )?;
    }
    Ok(changed == 1)
}

pub fn events_between(
    conn: &Connection,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> WorklogResult<Vec<StoredEvent>> {
    install_noise_function(conn)?;
    let mut stmt = conn.prepare(EVENTS_BETWEEN_SQL)?;
    collect_events(stmt.query_map(params![start.to_rfc3339(), end.to_rfc3339()], row_to_event)?)
}

pub fn search_events(conn: &Connection, query: &str) -> WorklogResult<Vec<StoredEvent>> {
    if has_fts(conn)? {
        search_fts(conn, query)
    } else {
        search_like(conn, query)
    }
}

fn search_like(conn: &Connection, query: &str) -> WorklogResult<Vec<StoredEvent>> {
    install_noise_function(conn)?;
    let needle = format!("%{}%", query.to_lowercase());
    let mut stmt = conn.prepare(SEARCH_LIKE_SQL)?;
    collect_events(stmt.query_map(params![needle], row_to_event)?)
}

fn search_fts(conn: &Connection, query: &str) -> WorklogResult<Vec<StoredEvent>> {
    let ids = {
        let mut stmt = conn.prepare(
            "SELECT entity_id FROM search_index WHERE search_index MATCH ?1 ORDER BY rank LIMIT 50",
        )?;
        stmt.query_map(params![query], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?
    };
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let mut events = Vec::with_capacity(ids.len());
    for id in ids {
        if let Some(event) = event_by_id(conn, &id)? {
            events.push(event);
        }
    }
    Ok(events)
}

fn event_by_id(conn: &Connection, id: &str) -> WorklogResult<Option<StoredEvent>> {
    conn.query_row(EVENT_BY_ID_SQL, params![id], row_to_event)
        .optional()
        .map_err(Into::into)
}

fn has_fts(conn: &Connection) -> WorklogResult<bool> {
    let found: Option<String> = conn
        .query_row(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'search_index'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    Ok(found.is_some())
}

fn index_event(conn: &Connection, event: &NormalizedSpoolEvent) -> WorklogResult<()> {
    if !has_fts(conn)? {
        return Ok(());
    }
    conn.execute(
        "INSERT INTO search_index (entity_type, entity_id, title, body, project_path, source_agent, timestamp)
         VALUES ('event', ?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            event.event_id,
            event.title,
            event.normalized_content,
            event.project_root,
            event.source_agent.id(),
            optional_time(event.timestamp),
        ],
    )?;
    Ok(())
}

fn optional_time(value: Option<DateTime<Utc>>) -> Option<String> {
    value.map(|time| time.to_rfc3339())
}

fn non_blank(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn project_name(root: &str) -> Option<String> {
    std::path::Path::new(root)
        .file_name()
        .and_then(|name| name.to_str())
        .map(ToOwned::to_owned)
}
