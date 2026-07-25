use std::fs;
use std::path::Path;

use my_worklog_core::WorklogResult;
use my_worklog_core::db::repositories::{ImportOutcome, insert_event};
use my_worklog_core::error::WorklogError;
use my_worklog_core::privacy::redact::Redactor;
use rusqlite::Connection;
use serde_json::Value;
use walkdir::WalkDir;

use crate::normalize::{messages_from_export, to_normalized_event};

pub fn import_opencode_export(
    conn: &Connection,
    path: &Path,
    redactor: &Redactor,
) -> WorklogResult<ImportOutcome> {
    let mut outcome = ImportOutcome {
        imported: 0,
        duplicates: 0,
        malformed: 0,
        skipped_noise: 0,
    };
    if path.is_dir() {
        for entry in WalkDir::new(path).into_iter().filter_map(Result::ok) {
            if entry.file_type().is_file()
                && entry.path().extension().is_some_and(|ext| ext == "json")
            {
                import_file(conn, entry.path(), redactor, &mut outcome)?;
            }
        }
    } else {
        import_file(conn, path, redactor, &mut outcome)?;
    }
    Ok(outcome)
}

fn import_file(
    conn: &Connection,
    path: &Path,
    redactor: &Redactor,
    outcome: &mut ImportOutcome,
) -> WorklogResult<()> {
    let content = fs::read_to_string(path).map_err(|source| WorklogError::Io {
        path: path.display().to_string(),
        source,
    })?;
    let value = serde_json::from_str::<Value>(&content)?;
    for message in messages_from_export(&value, &path.display().to_string()) {
        let event = to_normalized_event(&message, redactor);
        if insert_event(conn, &event)? {
            outcome.imported += 1;
        } else {
            outcome.duplicates += 1;
        }
    }
    Ok(())
}
