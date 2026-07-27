use rusqlite::Row;

use crate::db::repositories::StoredEvent;
use crate::error::WorklogResult;

pub(super) const EVENTS_BETWEEN_SQL: &str = r"SELECT work_event.id, work_event.source_agent_id, work_event.session_id, work_event.type,
       work_event.title, work_event.content, work_event.timestamp,
       COALESCE(work_event.cwd, work_session.cwd), project.root_path, project.name,
       work_event.command, work_event.file_path, work_event.duration_ms, work_event.raw_json
FROM work_event
JOIN work_session ON work_session.id = work_event.session_id
LEFT JOIN project ON project.id = work_session.project_id
WHERE work_event.timestamp >= ?1 AND work_event.timestamp < ?2
  AND NOT (work_event.source_agent_id = 'opencode' AND work_event.type = 'event' AND is_opencode_noise(work_event.normalized_content))
ORDER BY work_event.timestamp ASC, work_event.id ASC";

pub(super) const SEARCH_LIKE_SQL: &str = r"SELECT work_event.id, work_event.source_agent_id, work_event.session_id, work_event.type,
       work_event.title, work_event.content, work_event.timestamp,
       COALESCE(work_event.cwd, work_session.cwd), project.root_path, project.name,
       work_event.command, work_event.file_path, work_event.duration_ms, work_event.raw_json
FROM work_event
JOIN work_session ON work_session.id = work_event.session_id
LEFT JOIN project ON project.id = work_session.project_id
WHERE lower(coalesce(work_event.title, '') || ' ' || coalesce(work_event.normalized_content, '') || ' ' || coalesce(work_event.command, '')) LIKE ?1
  AND NOT (work_event.source_agent_id = 'opencode' AND work_event.type = 'event' AND is_opencode_noise(work_event.normalized_content))
ORDER BY work_event.timestamp DESC, work_event.id ASC
LIMIT 50";

pub(super) const EVENT_BY_ID_SQL: &str = r"SELECT work_event.id, work_event.source_agent_id, work_event.session_id, work_event.type,
       work_event.title, work_event.content, work_event.timestamp,
       COALESCE(work_event.cwd, work_session.cwd), project.root_path, project.name,
       work_event.command, work_event.file_path, work_event.duration_ms, work_event.raw_json
FROM work_event
JOIN work_session ON work_session.id = work_event.session_id
LEFT JOIN project ON project.id = work_session.project_id
WHERE work_event.id = ?1";

pub(super) fn collect_events(
    rows: impl Iterator<Item = rusqlite::Result<StoredEvent>>,
) -> WorklogResult<Vec<StoredEvent>> {
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

pub(super) fn row_to_event(row: &Row<'_>) -> rusqlite::Result<StoredEvent> {
    Ok(StoredEvent {
        id: row.get(0)?,
        source_agent_id: row.get(1)?,
        session_id: row.get(2)?,
        event_type: row.get(3)?,
        title: row.get(4)?,
        content: row.get(5)?,
        timestamp: row.get(6)?,
        cwd: row.get(7)?,
        project_root: row.get(8)?,
        project_name: row.get(9)?,
        command: row.get(10)?,
        file_path: row.get(11)?,
        duration_ms: row.get(12)?,
        raw_json: row.get(13)?,
    })
}
