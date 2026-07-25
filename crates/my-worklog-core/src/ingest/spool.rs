use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use rusqlite::Connection;
use serde_json::Value;
use walkdir::WalkDir;

use crate::db::cleanup::is_opencode_noise_payload;
use crate::db::repositories::{ImportOutcome, insert_event};
use crate::error::{WorklogError, WorklogResult};
use crate::ingest::normalize::SpoolRecord;
use crate::model::source::SourceAgent;
use crate::privacy::redact::Redactor;

pub fn import_spool(
    conn: &Connection,
    spool_dir: &Path,
    redactor: &Redactor,
) -> WorklogResult<ImportOutcome> {
    let mut outcome = ImportOutcome {
        imported: 0,
        duplicates: 0,
        malformed: 0,
        skipped_noise: 0,
    };
    for entry in WalkDir::new(spool_dir).into_iter().filter_map(Result::ok) {
        if entry.file_type().is_file() && entry.path().extension().is_some_and(|ext| ext == "jsonl")
        {
            import_file(conn, entry.path(), redactor, &mut outcome)?;
        }
    }
    Ok(outcome)
}

fn import_file(
    conn: &Connection,
    path: &Path,
    redactor: &Redactor,
    outcome: &mut ImportOutcome,
) -> WorklogResult<()> {
    let agent = infer_agent(path).unwrap_or(SourceAgent::OpenCode);
    let file = File::open(path).map_err(|source| WorklogError::Io {
        path: path.display().to_string(),
        source,
    })?;
    let mut reader = BufReader::new(file);
    let mut line = String::new();
    let mut line_number = 0usize;
    loop {
        line.clear();
        let bytes = reader
            .read_line(&mut line)
            .map_err(|source| WorklogError::Io {
                path: path.display().to_string(),
                source,
            })?;
        if bytes == 0 {
            break;
        }
        line_number += 1;
        if line.trim().is_empty() {
            continue;
        }
        match parse_line(&line, agent, redactor, path, line_number) {
            Ok(event) => {
                if is_opencode_noise_event(&event) {
                    outcome.skipped_noise += 1;
                    continue;
                }
                if insert_event(conn, &event)? {
                    outcome.imported += 1;
                } else {
                    outcome.duplicates += 1;
                }
            }
            Err(error) => {
                outcome.malformed += 1;
                tracing::warn!(%error, file = %path.display(), line = line_number, "skipped malformed spool record");
            }
        }
    }
    Ok(())
}

fn is_opencode_noise_event(event: &crate::ingest::normalize::NormalizedSpoolEvent) -> bool {
    event.source_agent == SourceAgent::OpenCode
        && event.event_type == "event"
        && event
            .normalized_content
            .as_deref()
            .is_some_and(is_opencode_noise_payload)
}

fn parse_line(
    line: &str,
    agent: SourceAgent,
    redactor: &Redactor,
    path: &Path,
    line_number: usize,
) -> WorklogResult<crate::ingest::normalize::NormalizedSpoolEvent> {
    let raw: Value = serde_json::from_str(line)?;
    let record: SpoolRecord = serde_json::from_value(raw.clone())?;
    Ok(record.normalize(
        agent,
        redactor,
        raw,
        format!("{}:{line_number}", path.display()),
    ))
}

fn infer_agent(path: &Path) -> Option<SourceAgent> {
    path.components().rev().find_map(|component| {
        let name = component.as_os_str().to_str()?;
        name.parse::<SourceAgent>().ok()
    })
}
