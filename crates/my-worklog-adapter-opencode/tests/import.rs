use std::fs;

use my_worklog_adapter_opencode::import_db::import_opencode_db;
use my_worklog_adapter_opencode::import_export::import_opencode_export;
use my_worklog_core::WorklogDb;
use my_worklog_core::db::repositories::{events_between, search_events};
use my_worklog_core::privacy::redact::Redactor;
use my_worklog_core::report::daily;
use rusqlite::Connection;
use tempfile::tempdir;

#[test]
fn import_export_reads_message_content_when_flat_export() {
    let dir = tempdir().expect("tempdir");
    let export = dir.path().join("session.json");
    fs::write(
        &export,
        r#"{
          "info": {"id":"ses_export", "title":"Flat export", "directory":"/tmp/project"},
          "messages": [
            {"id":"msg_user", "role":"user", "created":"2026-07-24T09:00:00Z", "content":"Build the import"},
            {"id":"msg_assistant", "role":"assistant", "created":"2026-07-24T09:01:00Z", "content":"Implemented the import"}
          ]
        }"#,
    )
    .expect("write fixture");
    let db = WorklogDb::open(&dir.path().join("worklog.sqlite")).expect("open db");
    let redactor = Redactor::new(None).expect("redactor");

    let outcome = import_opencode_export(db.connection(), &export, &redactor).expect("import");
    let results = search_events(db.connection(), "Implemented").expect("search");

    assert_eq!(outcome.imported, 2);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].source_agent_id, "opencode");
    assert_eq!(
        results[0].content.as_deref(),
        Some("Implemented the import")
    );
}

#[test]
fn import_export_reads_text_parts_when_modern_export() {
    let dir = tempdir().expect("tempdir");
    let export = dir.path().join("session.json");
    fs::write(
        &export,
        r#"{
          "info": {"id":"ses_parts", "directory":"/tmp/project"},
          "messages": [
            {
              "info": {"id":"msg_user", "sessionID":"ses_parts", "role":"user", "time":{"created":"2026-07-24T10:00:00Z"}},
              "parts": [
                {"type":"text", "text":"What did we ship?"},
                {"type":"tool", "tool":"bash", "state":{}}
              ]
            },
            {
              "info": {"id":"msg_assistant", "sessionID":"ses_parts", "role":"assistant", "time":{"created":"2026-07-24T10:01:00Z"}},
              "parts": [{"type":"text", "text":"We shipped transcript enrichment."}]
            }
          ]
        }"#,
    )
    .expect("write fixture");
    let db = WorklogDb::open(&dir.path().join("worklog.sqlite")).expect("open db");
    let redactor = Redactor::new(None).expect("redactor");

    let outcome = import_opencode_export(db.connection(), &export, &redactor).expect("import");
    let results = search_events(db.connection(), "transcript").expect("search");

    assert_eq!(outcome.imported, 2);
    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0].content.as_deref(),
        Some("We shipped transcript enrichment.")
    );
}

#[test]
fn import_export_redacts_content_and_is_idempotent() {
    let dir = tempdir().expect("tempdir");
    let export = dir.path().join("session.json");
    fs::write(
        &export,
        r#"{
          "info": {"id":"ses_secret"},
          "messages": [
            {"id":"msg_user", "role":"user", "created":"2026-07-24T11:00:00Z", "content":"Authorization: Bearer abc123"}
          ]
        }"#,
    )
    .expect("write fixture");
    let db = WorklogDb::open(&dir.path().join("worklog.sqlite")).expect("open db");
    let redactor = Redactor::new(None).expect("redactor");

    let first = import_opencode_export(db.connection(), &export, &redactor).expect("first import");
    let second =
        import_opencode_export(db.connection(), &export, &redactor).expect("second import");
    let results = search_events(db.connection(), "REDACTED").expect("search");

    assert_eq!(first.imported, 1);
    assert_eq!(second.duplicates, 1);
    assert_eq!(
        results[0].content.as_deref(),
        Some("Authorization: Bearer [REDACTED]")
    );
}

#[test]
fn import_db_reads_message_and_part_tables() {
    let dir = tempdir().expect("tempdir");
    let opencode_db = dir.path().join("opencode.db");
    let source = Connection::open(&opencode_db).expect("open opencode db");
    source
        .execute_batch(
            r#"
            CREATE TABLE message (id TEXT PRIMARY KEY, sessionID TEXT NOT NULL, data TEXT NOT NULL);
            CREATE TABLE part (id TEXT PRIMARY KEY, messageID TEXT NOT NULL, data TEXT NOT NULL);
            INSERT INTO message VALUES ('msg_user', 'ses_db', '{"id":"msg_user","sessionID":"ses_db","role":"user","time":{"created":"2026-07-24T12:00:00Z"}}');
            INSERT INTO part VALUES ('part_user', 'msg_user', '{"type":"text","text":"Read the local OpenCode DB"}');
            INSERT INTO message VALUES ('msg_assistant', 'ses_db', '{"id":"msg_assistant","sessionID":"ses_db","role":"assistant","time":{"created":"2026-07-24T12:01:00Z"}}');
            INSERT INTO part VALUES ('part_assistant', 'msg_assistant', '{"type":"text","text":"Imported from SQLite."}');
            "#,
        )
        .expect("seed opencode db");
    drop(source);
    let worklog = WorklogDb::open(&dir.path().join("worklog.sqlite")).expect("open worklog");
    let redactor = Redactor::new(None).expect("redactor");

    let outcome =
        import_opencode_db(worklog.connection(), &opencode_db, &redactor).expect("import db");
    let results = search_events(worklog.connection(), "SQLite").expect("search");

    assert_eq!(outcome.imported, 2);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].content.as_deref(), Some("Imported from SQLite."));
}

#[test]
fn import_db_preserves_assistant_token_and_timing_metrics() {
    let dir = tempdir().expect("tempdir");
    let opencode_db = dir.path().join("opencode.db");
    let source = Connection::open(&opencode_db).expect("open opencode db");
    source
        .execute_batch(
            r#"
            CREATE TABLE message (id TEXT PRIMARY KEY, session_id TEXT NOT NULL, data TEXT NOT NULL);
            CREATE TABLE part (id TEXT PRIMARY KEY, message_id TEXT NOT NULL, data TEXT NOT NULL);
            INSERT INTO message VALUES ('msg_assistant', 'ses_db', '{"id":"msg_assistant","session_id":"ses_db","role":"assistant","time":{"created":1784908285705}}');
            INSERT INTO part VALUES ('part_assistant', 'msg_assistant', '{"type":"text","text":"Implemented metrics import."}');
            "#,
        )
        .expect("seed opencode db");
    drop(source);
    let worklog = WorklogDb::open(&dir.path().join("worklog.sqlite")).expect("open worklog");
    let redactor = Redactor::new(None).expect("redactor");
    let start = chrono::DateTime::parse_from_rfc3339("2026-07-24T00:00:00Z")
        .expect("parse start")
        .with_timezone(&chrono::Utc);
    let end = chrono::DateTime::parse_from_rfc3339("2026-07-25T00:00:00Z")
        .expect("parse end")
        .with_timezone(&chrono::Utc);

    import_opencode_db(worklog.connection(), &opencode_db, &redactor).expect("initial import");
    let source = Connection::open(&opencode_db).expect("reopen opencode db");
    source
        .execute(
            "UPDATE message SET data = ?1 WHERE id = 'msg_assistant'",
            [r#"{"id":"msg_assistant","session_id":"ses_db","role":"assistant","time":{"created":1784908285705,"completed":1784908290705},"tokens":{"input":1200,"output":345},"cost":0.02}"#],
        )
        .expect("add metrics");
    drop(source);

    let outcome = import_opencode_db(worklog.connection(), &opencode_db, &redactor)
        .expect("metrics backfill import");
    let events = events_between(worklog.connection(), start, end).expect("events");

    assert_eq!(outcome.duplicates, 1);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].duration_ms, Some(5000));
    assert!(events[0].raw_json.as_deref().is_some_and(|raw_json| {
        raw_json.contains(r#""input_tokens":1200"#)
            && raw_json.contains(r#""output_tokens":345"#)
            && raw_json.contains(r#""total_tokens":1545"#)
    }));
    let report = daily::render_day(
        worklog.connection(),
        chrono::NaiveDate::from_ymd_opt(2026, 7, 24).expect("valid date"),
    )
    .expect("render day");
    assert!(report.contains("- Captured agent-session time: 0m 05s"));
    assert_eq!(
        report.matches("Coverage: captured coding-agent events only; meetings, manual coding, review, planning, browser work, and other uncaptured activity are excluded.").count(),
        1
    );
    assert!(report.contains("- Tokens: 1,545 total (1,200 input, 345 output)"));
}

#[test]
fn import_db_reads_snake_case_message_and_part_columns() {
    let dir = tempdir().expect("tempdir");
    let opencode_db = dir.path().join("opencode.db");
    let source = Connection::open(&opencode_db).expect("open opencode db");
    source
        .execute_batch(
            r#"
            CREATE TABLE message (id TEXT PRIMARY KEY, session_id TEXT NOT NULL, data TEXT NOT NULL);
            CREATE TABLE part (id TEXT PRIMARY KEY, message_id TEXT NOT NULL, data TEXT NOT NULL);
            INSERT INTO message VALUES ('msg_user', 'ses_db', '{"id":"msg_user","session_id":"ses_db","role":"user","time":{"created":"2026-07-24T13:00:00Z"}}');
            INSERT INTO part VALUES ('part_user', 'msg_user', '{"type":"text","text":"Import snake case OpenCode DB"}');
            "#,
        )
        .expect("seed opencode db");
    drop(source);
    let worklog = WorklogDb::open(&dir.path().join("worklog.sqlite")).expect("open worklog");
    let redactor = Redactor::new(None).expect("redactor");

    let outcome =
        import_opencode_db(worklog.connection(), &opencode_db, &redactor).expect("import db");
    let results = search_events(worklog.connection(), "snake").expect("search");

    assert_eq!(outcome.imported, 1);
    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0].content.as_deref(),
        Some("Import snake case OpenCode DB")
    );
}

#[test]
fn import_db_parses_unix_millisecond_timestamps() {
    let dir = tempdir().expect("tempdir");
    let opencode_db = dir.path().join("opencode.db");
    let source = Connection::open(&opencode_db).expect("open opencode db");
    source
        .execute_batch(
            r#"
            CREATE TABLE message (id TEXT PRIMARY KEY, session_id TEXT NOT NULL, data TEXT NOT NULL);
            CREATE TABLE part (id TEXT PRIMARY KEY, message_id TEXT NOT NULL, data TEXT NOT NULL);
            INSERT INTO message VALUES ('msg_user', 'ses_db', '{"id":"msg_user","session_id":"ses_db","role":"user","time":{"created":1784908285705}}');
            INSERT INTO part VALUES ('part_user', 'msg_user', '{"type":"text","text":"Numeric timestamp should appear today"}');
            "#,
        )
        .expect("seed opencode db");
    drop(source);
    let worklog = WorklogDb::open(&dir.path().join("worklog.sqlite")).expect("open worklog");
    let redactor = Redactor::new(None).expect("redactor");
    let start = chrono::DateTime::parse_from_rfc3339("2026-07-24T00:00:00Z")
        .expect("parse start")
        .with_timezone(&chrono::Utc);
    let end = chrono::DateTime::parse_from_rfc3339("2026-07-25T00:00:00Z")
        .expect("parse end")
        .with_timezone(&chrono::Utc);

    import_opencode_db(worklog.connection(), &opencode_db, &redactor).expect("import db");
    let events = events_between(worklog.connection(), start, end).expect("events");

    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0].content.as_deref(),
        Some("Numeric timestamp should appear today")
    );
}
