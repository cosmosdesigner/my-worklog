use std::collections::HashMap;
use std::path::Path;

use my_worklog_core::WorklogResult;
use my_worklog_core::db::repositories::{ImportOutcome, insert_event};
use my_worklog_core::privacy::redact::Redactor;
use rusqlite::{Connection, OpenFlags, OptionalExtension, params};
use serde_json::Value;

use crate::normalize::{message_from_db_rows, to_normalized_event};

pub fn import_opencode_db(
    worklog_conn: &Connection,
    db_path: &Path,
    redactor: &Redactor,
) -> WorklogResult<ImportOutcome> {
    let source = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let mut outcome = ImportOutcome {
        imported: 0,
        duplicates: 0,
        malformed: 0,
        skipped_noise: 0,
    };
    if has_table(&source, "message")? && has_table(&source, "part")? {
        import_modern_tables(worklog_conn, &source, db_path, redactor, &mut outcome)?;
    }
    Ok(outcome)
}

fn import_modern_tables(
    worklog_conn: &Connection,
    source: &Connection,
    db_path: &Path,
    redactor: &Redactor,
    outcome: &mut ImportOutcome,
) -> WorklogResult<()> {
    let message_session_column = column_name(source, "message", &["sessionID", "session_id"])?;
    let part_message_column = column_name(source, "part", &["messageID", "message_id"])?;
    let session_directories = session_directories(source)?;
    let mut stmt = source.prepare(&format!(
        "SELECT id, {message_session_column}, data FROM message ORDER BY rowid ASC"
    ))?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;
    for row in rows {
        let (message_id, session_id, message_data) = row?;
        let parts = part_data(source, &part_message_column, &message_id)?;
        let raw_ref = format!("{}:message:{message_id}", db_path.display());
        let cwd = session_directories.get(&session_id).cloned();
        if let Some(message) = message_from_db_rows(
            &message_data,
            &parts,
            &session_id,
            &message_id,
            cwd,
            &raw_ref,
        ) {
            let event = to_normalized_event(&message, redactor);
            if insert_event(worklog_conn, &event)? {
                outcome.imported += 1;
            } else {
                outcome.duplicates += 1;
            }
        }
    }
    Ok(())
}

fn session_directories(source: &Connection) -> WorklogResult<HashMap<String, String>> {
    if !has_table(source, "session")? || !has_columns(source, "session", &["id", "data"])? {
        return Ok(HashMap::new());
    }
    let mut stmt = source.prepare("SELECT id, data FROM session")?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    let mut directories = HashMap::new();
    for row in rows {
        let (session_id, data) = row?;
        if let Some(directory) = session_directory(&data) {
            directories.insert(session_id, directory);
        }
    }
    Ok(directories)
}

fn session_directory(data: &str) -> Option<String> {
    let value = serde_json::from_str::<Value>(data).ok()?;
    string_at(&value, &["directory", "cwd", "project_root"]).or_else(|| {
        value
            .get("info")
            .and_then(|info| string_at(info, &["directory", "cwd", "project_root"]))
    })
}

fn string_at(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(Value::as_str))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn part_data(
    source: &Connection,
    message_column: &str,
    message_id: &str,
) -> WorklogResult<Vec<String>> {
    let mut stmt = source.prepare(&format!(
        "SELECT data FROM part WHERE {message_column} = ?1 ORDER BY rowid ASC"
    ))?;
    let rows = stmt.query_map(params![message_id], |row| row.get::<_, String>(0))?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

fn column_name(conn: &Connection, table: &str, candidates: &[&str]) -> WorklogResult<String> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;
    let columns = rows.collect::<Result<Vec<_>, _>>()?;
    let found = candidates
        .iter()
        .find(|candidate| columns.iter().any(|column| column == *candidate))
        .copied()
        .unwrap_or(candidates[0]);
    Ok(found.to_owned())
}

fn has_columns(conn: &Connection, table: &str, required: &[&str]) -> WorklogResult<bool> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;
    let columns = rows.collect::<Result<Vec<_>, _>>()?;
    Ok(required
        .iter()
        .all(|required_column| columns.iter().any(|column| column == required_column)))
}

fn has_table(conn: &Connection, table: &str) -> WorklogResult<bool> {
    let found: Option<String> = conn
        .query_row(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name = ?1",
            params![table],
            |row| row.get(0),
        )
        .optional()?;
    Ok(found.is_some())
}
