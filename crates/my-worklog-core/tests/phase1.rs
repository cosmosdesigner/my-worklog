use std::fs;

use chrono::TimeZone;
use my_worklog_core::WorklogDb;
use my_worklog_core::db::repositories::{events_between, search_events};
use my_worklog_core::ingest::spool::import_spool;
use my_worklog_core::privacy::redact::Redactor;
use my_worklog_core::report::{daily, weekly};
use my_worklog_core::search::fts::search_markdown;
use tempfile::tempdir;

#[test]
fn migration_creates_source_agents_when_database_opens() {
    let dir = tempdir().expect("tempdir");
    let db = WorklogDb::open(&dir.path().join("worklog.sqlite")).expect("open db");

    let count: i64 = db
        .connection()
        .query_row("SELECT count(*) FROM source_agent", [], |row| row.get(0))
        .expect("count agents");

    assert_eq!(count, 3);
}

#[test]
fn import_spool_is_idempotent_when_run_twice() {
    let dir = tempdir().expect("tempdir");
    let spool = dir.path().join("spool/opencode");
    fs::create_dir_all(&spool).expect("create spool");
    fs::write(
        spool.join("events.jsonl"),
        r#"{"source_agent":"opencode","source_session_id":"s1","source_event_id":"e1","type":"user_prompt","timestamp":"2026-07-24T09:00:00Z","title":"Database migration","content":"Authorization: Bearer secret"}
"#,
    )
    .expect("write fixture");
    let db = WorklogDb::open(&dir.path().join("worklog.sqlite")).expect("open db");
    let redactor = Redactor::new(None).expect("redactor");

    let first =
        import_spool(db.connection(), &dir.path().join("spool"), &redactor).expect("first import");
    let second =
        import_spool(db.connection(), &dir.path().join("spool"), &redactor).expect("second import");

    assert_eq!(first.imported, 1);
    assert_eq!(second.duplicates, 1);
}

#[test]
fn search_events_matches_redacted_content() {
    let dir = tempdir().expect("tempdir");
    let spool = dir.path().join("spool/codex");
    fs::create_dir_all(&spool).expect("create spool");
    fs::write(
        spool.join("events.jsonl"),
        r#"{"source_agent":"codex","source_session_id":"s1","source_event_id":"e1","type":"command","timestamp":"2026-07-24T09:00:00Z","title":"Run cargo test","content":"database migration failed","command":"cargo test"}
"#,
    )
    .expect("write fixture");
    let db = WorklogDb::open(&dir.path().join("worklog.sqlite")).expect("open db");
    let redactor = Redactor::new(None).expect("redactor");
    import_spool(db.connection(), &dir.path().join("spool"), &redactor).expect("import");

    let results = search_events(db.connection(), "migration").expect("search");

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].source_agent_id, "codex");
}

#[test]
fn daily_report_mentions_no_events_for_empty_day() {
    let dir = tempdir().expect("tempdir");
    let db = WorklogDb::open(&dir.path().join("worklog.sqlite")).expect("open db");

    let report = daily::render_day(
        db.connection(),
        chrono::NaiveDate::from_ymd_opt(2026, 7, 24).expect("valid date"),
    )
    .expect("render day");

    assert!(report.contains("No captured work events"));
}

#[test]
fn date_query_returns_events_inside_window() {
    let dir = tempdir().expect("tempdir");
    let spool = dir.path().join("spool/claude");
    fs::create_dir_all(&spool).expect("create spool");
    fs::write(
        spool.join("events.jsonl"),
        r#"{"source_agent":"claude","source_session_id":"s1","source_event_id":"e1","type":"decision","timestamp":"2026-07-24T09:00:00Z","title":"Use local SQLite","content":"We will keep reports local-first."}
"#,
    )
    .expect("write fixture");
    let db = WorklogDb::open(&dir.path().join("worklog.sqlite")).expect("open db");
    let redactor = Redactor::new(None).expect("redactor");
    import_spool(db.connection(), &dir.path().join("spool"), &redactor).expect("import");

    let start = chrono::DateTime::parse_from_rfc3339("2026-07-24T00:00:00Z")
        .expect("parse start")
        .with_timezone(&chrono::Utc);
    let end = chrono::DateTime::parse_from_rfc3339("2026-07-25T00:00:00Z")
        .expect("parse end")
        .with_timezone(&chrono::Utc);
    let events = events_between(db.connection(), start, end).expect("events");

    assert_eq!(events.len(), 1);
}

#[test]
fn daily_report_hides_raw_opencode_json_when_transcripts_exist() {
    let dir = tempdir().expect("tempdir");
    let spool = dir.path().join("spool/opencode");
    fs::create_dir_all(&spool).expect("create spool");
    fs::write(
        spool.join("events.jsonl"),
        r#"{"source_agent":"opencode","source_session_id":"s1","source_event_id":"raw1","type":"event","timestamp":"2026-07-24T09:00:00Z","content":"{\"event\":{\"type\":\"message.updated\",\"properties\":{\"sessionID\":\"s1\"}}}"}
{"source_agent":"opencode","source_session_id":"s1","source_event_id":"u1","type":"user_prompt","role":"user","timestamp":"2026-07-24T09:01:00Z","content":"Plan the readable report"}
{"source_agent":"opencode","source_session_id":"s1","source_event_id":"a1","type":"assistant_message","role":"assistant","timestamp":"2026-07-24T09:02:00Z","content":"Implemented readable report rendering"}
"#,
    )
    .expect("write fixture");
    let db = WorklogDb::open(&dir.path().join("worklog.sqlite")).expect("open db");
    let redactor = Redactor::new(None).expect("redactor");
    import_spool(db.connection(), &dir.path().join("spool"), &redactor).expect("import");

    let report = daily::render_day(
        db.connection(),
        chrono::NaiveDate::from_ymd_opt(2026, 7, 24).expect("valid date"),
    )
    .expect("render day");

    assert!(report.contains("User: Plan the readable report"));
    assert!(report.contains("Assistant: Implemented readable report rendering"));
    assert!(!report.contains("{\"event\""));
    assert!(!report.contains("message.updated"));
    assert!(!report.contains("sessionID"));
}

#[test]
fn search_markdown_hides_raw_opencode_json_matches() {
    let dir = tempdir().expect("tempdir");
    let spool = dir.path().join("spool/opencode");
    fs::create_dir_all(&spool).expect("create spool");
    fs::write(
        spool.join("events.jsonl"),
        r#"{"source_agent":"opencode","source_session_id":"s1","source_event_id":"raw1","type":"event","timestamp":"2026-07-24T09:00:00Z","content":"{\"event\":{\"type\":\"message.updated\",\"properties\":{\"sessionID\":\"s1\"}}}"}
"#,
    )
    .expect("write fixture");
    let db = WorklogDb::open(&dir.path().join("worklog.sqlite")).expect("open db");
    let redactor = Redactor::new(None).expect("redactor");
    import_spool(db.connection(), &dir.path().join("spool"), &redactor).expect("import");

    let output = search_markdown(db.connection(), "sessionID").expect("search");

    assert!(output.contains("No human-readable matching work events found."));
    assert!(!output.contains("{\"event\""));
    assert!(!output.contains("message.updated"));
}

#[test]
fn full_day_report_includes_items_beyond_terminal_cap() {
    let dir = tempdir().expect("tempdir");
    let spool = dir.path().join("spool/opencode");
    fs::create_dir_all(&spool).expect("create spool");
    let mut records = String::new();
    for index in 1..=25 {
        records.push_str(&format!(
            "{{\"source_agent\":\"opencode\",\"source_session_id\":\"s1\",\"source_event_id\":\"u{index}\",\"type\":\"user_prompt\",\"role\":\"user\",\"timestamp\":\"2026-07-24T09:{index:02}:00Z\",\"content\":\"Readable work item {index}\"}}\n"
        ));
    }
    fs::write(spool.join("events.jsonl"), records).expect("write fixture");
    let db = WorklogDb::open(&dir.path().join("worklog.sqlite")).expect("open db");
    let redactor = Redactor::new(None).expect("redactor");
    import_spool(db.connection(), &dir.path().join("spool"), &redactor).expect("import");
    let date = chrono::NaiveDate::from_ymd_opt(2026, 7, 24).expect("valid date");

    let terminal_report = daily::render_day(db.connection(), date).expect("render day");
    let full_report = daily::render_full_day(db.connection(), date).expect("render full day");

    assert!(!terminal_report.contains("Readable work item 25"));
    assert!(full_report.contains("Readable work item 25"));
}

#[test]
fn active_day_report_stops_at_call_time() {
    let dir = tempdir().expect("tempdir");
    let spool = dir.path().join("spool/opencode");
    fs::create_dir_all(&spool).expect("create spool");
    fs::write(
        spool.join("events.jsonl"),
        r#"{"source_agent":"opencode","source_session_id":"s1","source_event_id":"past","type":"user_prompt","role":"user","timestamp":"2026-07-24T09:00:00Z","content":"Past work item"}
{"source_agent":"opencode","source_session_id":"s1","source_event_id":"future","type":"user_prompt","role":"user","timestamp":"2026-07-24T10:30:00Z","content":"Future work item"}
"#,
    )
    .expect("write fixture");
    let db = WorklogDb::open(&dir.path().join("worklog.sqlite")).expect("open db");
    let redactor = Redactor::new(None).expect("redactor");
    import_spool(db.connection(), &dir.path().join("spool"), &redactor).expect("import");
    let end = chrono::Utc
        .with_ymd_and_hms(2026, 7, 24, 10, 0, 0)
        .single()
        .expect("valid end")
        .with_timezone(&chrono::Local);

    let report =
        daily::render_day_until(db.connection(), end.date_naive(), end).expect("render active day");

    assert!(report.contains("Past work item"));
    assert!(!report.contains("Future work item"));
}

#[test]
fn active_yesterday_reports_stop_at_previous_day_call_time() {
    let dir = tempdir().expect("tempdir");
    let spool = dir.path().join("spool/opencode");
    fs::create_dir_all(&spool).expect("create spool");
    fs::write(
        spool.join("events.jsonl"),
        r#"{"source_agent":"opencode","source_session_id":"s1","source_event_id":"past","type":"user_prompt","role":"user","timestamp":"2026-07-24T09:00:00Z","content":"Yesterday included item"}
{"source_agent":"opencode","source_session_id":"s1","source_event_id":"late","type":"user_prompt","role":"user","timestamp":"2026-07-24T10:30:00Z","content":"Yesterday later item"}
{"source_agent":"opencode","source_session_id":"s1","source_event_id":"today","type":"user_prompt","role":"user","timestamp":"2026-07-25T09:00:00Z","content":"Today item"}
"#,
    )
    .expect("write fixture");
    let db = WorklogDb::open(&dir.path().join("worklog.sqlite")).expect("open db");
    let redactor = Redactor::new(None).expect("redactor");
    import_spool(db.connection(), &dir.path().join("spool"), &redactor).expect("import");
    let now = chrono::Utc
        .with_ymd_and_hms(2026, 7, 25, 10, 0, 0)
        .single()
        .expect("valid now")
        .with_timezone(&chrono::Local);

    let terminal_report =
        daily::render_yesterday_until(db.connection(), now).expect("render active yesterday");
    let full_report =
        daily::render_full_yesterday_until(db.connection(), now).expect("render full yesterday");
    let share_context =
        daily::render_share_yesterday_until(db.connection(), now).expect("render share yesterday");

    for report in [terminal_report, full_report, share_context] {
        assert!(report.contains("Yesterday included item"));
        assert!(!report.contains("Yesterday later item"));
        assert!(!report.contains("Today item"));
    }
}

#[test]
fn active_week_report_stops_at_call_time() {
    let dir = tempdir().expect("tempdir");
    let spool = dir.path().join("spool/opencode");
    fs::create_dir_all(&spool).expect("create spool");
    fs::write(
        spool.join("events.jsonl"),
        r#"{"source_agent":"opencode","source_session_id":"s1","source_event_id":"past","type":"user_prompt","role":"user","timestamp":"2026-07-20T09:00:00Z","content":"Week-to-date work item"}
{"source_agent":"opencode","source_session_id":"s1","source_event_id":"future","type":"user_prompt","role":"user","timestamp":"2026-07-22T09:00:00Z","content":"Future week item"}
"#,
    )
    .expect("write fixture");
    let db = WorklogDb::open(&dir.path().join("worklog.sqlite")).expect("open db");
    let redactor = Redactor::new(None).expect("redactor");
    import_spool(db.connection(), &dir.path().join("spool"), &redactor).expect("import");
    let end = chrono::Utc
        .with_ymd_and_hms(2026, 7, 21, 10, 0, 0)
        .single()
        .expect("valid end")
        .with_timezone(&chrono::Local);

    let report = weekly::render_week_until(db.connection(), end).expect("render active week");

    assert!(report.contains("Week-to-date work item"));
    assert!(!report.contains("Future week item"));
}
