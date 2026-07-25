use rusqlite::Connection;

use crate::error::WorklogResult;

pub fn cleanup_opencode_noise(conn: &Connection) -> WorklogResult<usize> {
    install_noise_function(conn)?;
    if has_fts(conn)? {
        conn.execute(
            "DELETE FROM search_index
             WHERE entity_id IN (
               SELECT id FROM work_event
               WHERE source_agent_id = 'opencode' AND type = 'event' AND is_opencode_noise(normalized_content)
             )",
            [],
        )?;
    }
    let event_deleted = conn.execute(
        "DELETE FROM work_event
         WHERE source_agent_id = 'opencode' AND type = 'event' AND is_opencode_noise(normalized_content)",
        [],
    )?;
    conn.execute(
        "DELETE FROM work_session
         WHERE source_agent_id = 'opencode'
           AND NOT EXISTS (SELECT 1 FROM work_event WHERE work_event.session_id = work_session.id)",
        [],
    )?;
    Ok(event_deleted)
}

pub fn install_noise_function(conn: &Connection) -> WorklogResult<()> {
    conn.create_scalar_function(
        "is_opencode_noise",
        1,
        rusqlite::functions::FunctionFlags::SQLITE_DETERMINISTIC,
        |ctx| {
            let value = ctx.get::<String>(0)?;
            Ok(is_opencode_noise_payload(&value))
        },
    )?;
    Ok(())
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

pub fn is_opencode_noise_payload(value: &str) -> bool {
    let Ok(parsed) = serde_json::from_str::<serde_json::Value>(value) else {
        return false;
    };
    let Some(event_type) = parsed
        .get("event")
        .and_then(|event| event.get("type"))
        .and_then(serde_json::Value::as_str)
    else {
        return false;
    };
    !matches!(
        event_type,
        "session.created"
            | "session.updated"
            | "session.idle"
            | "session.compacted"
            | "session.error"
            | "message.updated"
            | "file.edited"
            | "todo.updated"
    )
}

use rusqlite::OptionalExtension;
