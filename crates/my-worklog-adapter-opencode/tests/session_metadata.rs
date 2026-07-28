use chrono::{DateTime, Utc};
use my_worklog_adapter_opencode::import_db::import_opencode_db;
use my_worklog_core::WorklogDb;
use my_worklog_core::db::repositories::events_between;
use my_worklog_core::privacy::redact::Redactor;
use my_worklog_core::report::{daily, project};
use rusqlite::{Connection, params};
use tempfile::tempdir;

const DATE: &str = "2026-07-24";

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

fn create_session_id_only_table(conn: &Connection) {
    conn.execute_batch("CREATE TABLE session (id TEXT PRIMARY KEY)")
        .expect("create id-only session table");
}

fn insert_session(conn: &Connection, id: &str, data: &str) {
    conn.execute(
        "INSERT INTO session (id, data) VALUES (?1, ?2)",
        params![id, data],
    )
    .expect("insert session");
}

fn insert_message(
    conn: &Connection,
    session_id: &str,
    message_id: &str,
    created: &str,
    text: &str,
) {
    let data = format!(
        r#"{{"id":"{message_id}","sessionID":"{session_id}","role":"assistant","time":{{"created":"{created}"}}}}"#
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
fn import_db_recovers_project_metadata_from_matching_session_row() {
    let dir = tempdir().expect("tempdir");
    let opencode_db = dir.path().join("opencode.db");
    let source = Connection::open(&opencode_db).expect("open source");
    create_modern_source(&source);
    create_session_table(&source);
    insert_session(
        &source,
        "ses_api",
        r#"{"directory":"/workspace/company/api"}"#,
    );
    insert_message(
        &source,
        "ses_api",
        "msg_api",
        "2026-07-24T09:00:00Z",
        "Implemented API import metadata",
    );
    drop(source);
    let worklog = WorklogDb::open(&dir.path().join("worklog.sqlite")).expect("open worklog");

    let outcome =
        import_opencode_db(worklog.connection(), &opencode_db, &redactor()).expect("import");
    let (start, end) = window();
    let events = events_between(worklog.connection(), start, end).expect("events");

    assert_eq!(outcome.imported, 1);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].cwd.as_deref(), Some("/workspace/company/api"));
    assert_eq!(
        events[0].project_root.as_deref(),
        Some("/workspace/company/api")
    );
    assert_eq!(events[0].project_name.as_deref(), Some("api"));
}

#[test]
fn import_db_groups_multi_repo_historical_daily_report_by_project() {
    let dir = tempdir().expect("tempdir");
    let opencode_db = dir.path().join("opencode.db");
    let source = Connection::open(&opencode_db).expect("open source");
    create_modern_source(&source);
    create_session_table(&source);
    insert_session(&source, "ses_api", r#"{"cwd":"/workspace/company/api"}"#);
    insert_session(
        &source,
        "ses_web",
        r#"{"project_root":"/workspace/company/web"}"#,
    );
    insert_message(
        &source,
        "ses_api",
        "msg_api",
        "2026-07-24T09:00:00Z",
        "Implemented API importer",
    );
    insert_message(
        &source,
        "ses_web",
        "msg_web",
        "2026-07-24T09:01:00Z",
        "Implemented web importer",
    );
    drop(source);
    let worklog = WorklogDb::open(&dir.path().join("worklog.sqlite")).expect("open worklog");

    import_opencode_db(worklog.connection(), &opencode_db, &redactor()).expect("import");
    let report = daily::render_day(
        worklog.connection(),
        chrono::NaiveDate::parse_from_str(DATE, "%Y-%m-%d").expect("parse date"),
    )
    .expect("render day");

    assert!(report.contains("## Projects"));
    assert!(report.contains("- api: 1 events"));
    assert!(report.contains("- web: 1 events"));
    assert!(report.contains("### api"));
    assert!(report.contains("Assistant: Implemented API importer [opencode]"));
    assert!(report.contains("### web"));
    assert!(report.contains("Assistant: Implemented web importer [opencode]"));
}

#[test]
fn import_db_missing_session_metadata_remains_unknown_project() {
    let dir = tempdir().expect("tempdir");
    let opencode_db = dir.path().join("opencode.db");
    let source = Connection::open(&opencode_db).expect("open source");
    create_modern_source(&source);
    create_session_table(&source);
    insert_session(
        &source,
        "other_session",
        r#"{"directory":"/workspace/other"}"#,
    );
    insert_message(
        &source,
        "ses_missing",
        "msg_missing",
        "2026-07-24T09:00:00Z",
        "Missing session metadata",
    );
    drop(source);
    let worklog = WorklogDb::open(&dir.path().join("worklog.sqlite")).expect("open worklog");

    import_opencode_db(worklog.connection(), &opencode_db, &redactor()).expect("import");
    let (start, end) = window();
    let events = events_between(worklog.connection(), start, end).expect("events");

    assert_eq!(events.len(), 1);
    assert_eq!(events[0].project_root, None);
    assert_eq!(project::label(&events[0]), "Unknown project");
}

#[test]
fn import_db_ignores_session_table_without_data_column() {
    let dir = tempdir().expect("tempdir");
    let opencode_db = dir.path().join("opencode.db");
    let source = Connection::open(&opencode_db).expect("open source");
    create_modern_source(&source);
    create_session_id_only_table(&source);
    insert_message(
        &source,
        "ses_legacy",
        "msg_legacy",
        "2026-07-24T09:00:00Z",
        "Import despite legacy session schema",
    );
    drop(source);
    let worklog = WorklogDb::open(&dir.path().join("worklog.sqlite")).expect("open worklog");

    let outcome =
        import_opencode_db(worklog.connection(), &opencode_db, &redactor()).expect("import");
    let (start, end) = window();
    let events = events_between(worklog.connection(), start, end).expect("events");

    assert_eq!(outcome.imported, 1);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].project_root, None);
    assert_eq!(project::label(&events[0]), "Unknown project");
}
