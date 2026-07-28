use chrono::{DateTime, Utc};
use my_worklog_adapter_opencode::import_db::import_opencode_db;
use my_worklog_core::WorklogDb;
use my_worklog_core::db::repositories::events_between;
use my_worklog_core::privacy::redact::Redactor;
use rusqlite::{Connection, params};
use tempfile::tempdir;

fn redactor() -> Redactor {
    Redactor::new(None).expect("redactor")
}

fn window() -> (DateTime<Utc>, DateTime<Utc>) {
    let start = DateTime::parse_from_rfc3339("2026-07-24T00:00:00Z")
        .expect("parse start")
        .with_timezone(&Utc);
    let end = DateTime::parse_from_rfc3339("2026-07-25T00:00:00Z")
        .expect("parse end")
        .with_timezone(&Utc);
    (start, end)
}

fn create_modern_source(conn: &Connection) {
    conn.execute_batch(
        r#"
        CREATE TABLE message (id TEXT PRIMARY KEY, sessionID TEXT NOT NULL, data TEXT NOT NULL);
        CREATE TABLE part (id TEXT PRIMARY KEY, messageID TEXT NOT NULL, data TEXT NOT NULL);
        "#,
    )
    .expect("create source tables");
}

fn create_session_table(conn: &Connection) {
    conn.execute_batch("CREATE TABLE session (id TEXT PRIMARY KEY, data TEXT NOT NULL)")
        .expect("create session table");
}

fn insert_session(conn: &Connection, id: &str, data: &str) {
    conn.execute(
        "INSERT INTO session (id, data) VALUES (?1, ?2)",
        params![id, data],
    )
    .expect("insert session");
}

fn insert_message(conn: &Connection, session_id: &str, message_id: &str, text: &str) {
    let data = format!(
        r#"{{"id":"{message_id}","sessionID":"{session_id}","role":"assistant","time":{{"created":"2026-07-24T09:00:00Z"}}}}"#
    );
    let part = format!(r#"{{"type":"text","text":"{text}"}}"#);
    conn.execute(
        "INSERT INTO message (id, sessionID, data) VALUES (?1, ?2, ?3)",
        params![message_id, session_id, data],
    )
    .expect("insert message");
    conn.execute(
        "INSERT INTO part (id, messageID, data) VALUES (?1, ?2, ?3)",
        params![format!("part_{message_id}"), message_id, part],
    )
    .expect("insert part");
}

#[test]
fn import_db_reimport_backfills_previous_unknown_session_metadata() {
    let dir = tempdir().expect("tempdir");
    let opencode_db = dir.path().join("opencode.db");
    let source = Connection::open(&opencode_db).expect("open source");
    create_modern_source(&source);
    insert_message(
        &source,
        "ses_backfill",
        "msg_backfill",
        "Backfill imported metadata",
    );
    drop(source);
    let worklog = WorklogDb::open(&dir.path().join("worklog.sqlite")).expect("open worklog");

    let first = import_opencode_db(worklog.connection(), &opencode_db, &redactor()).expect("first");
    let source = Connection::open(&opencode_db).expect("reopen source");
    create_session_table(&source);
    insert_session(
        &source,
        "ses_backfill",
        r#"{"directory":"/workspace/company/backfill"}"#,
    );
    drop(source);

    let second =
        import_opencode_db(worklog.connection(), &opencode_db, &redactor()).expect("second");
    let (start, end) = window();
    let events = events_between(worklog.connection(), start, end).expect("events");

    assert_eq!(first.imported, 1);
    assert_eq!(second.imported, 0);
    assert_eq!(second.duplicates, 1);
    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0].cwd.as_deref(),
        Some("/workspace/company/backfill")
    );
    assert_eq!(
        events[0].project_root.as_deref(),
        Some("/workspace/company/backfill")
    );
    assert_eq!(events[0].project_name.as_deref(), Some("backfill"));
}

#[test]
fn import_db_blank_session_directory_does_not_overwrite_existing_metadata() {
    let dir = tempdir().expect("tempdir");
    let opencode_db = dir.path().join("opencode.db");
    let source = Connection::open(&opencode_db).expect("open source");
    create_modern_source(&source);
    create_session_table(&source);
    insert_session(
        &source,
        "ses_blank",
        r#"{"directory":"/workspace/company/good"}"#,
    );
    insert_message(&source, "ses_blank", "msg_blank", "Keep existing metadata");
    drop(source);
    let worklog = WorklogDb::open(&dir.path().join("worklog.sqlite")).expect("open worklog");
    import_opencode_db(worklog.connection(), &opencode_db, &redactor()).expect("first import");
    let source = Connection::open(&opencode_db).expect("reopen source");
    source
        .execute(
            "UPDATE session SET data = ?1 WHERE id = 'ses_blank'",
            [r#"{"directory":"   ","cwd":"\n\t","project_root":""}"#],
        )
        .expect("blank session metadata");
    drop(source);

    let outcome =
        import_opencode_db(worklog.connection(), &opencode_db, &redactor()).expect("second");
    let (start, end) = window();
    let events = events_between(worklog.connection(), start, end).expect("events");

    assert_eq!(outcome.duplicates, 1);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].cwd.as_deref(), Some("/workspace/company/good"));
    assert_eq!(
        events[0].project_root.as_deref(),
        Some("/workspace/company/good")
    );
    assert_eq!(events[0].project_name.as_deref(), Some("good"));
}
