use std::fs;

use my_worklog_core::WorklogDb;
use my_worklog_core::ingest::spool::import_spool;
use my_worklog_core::privacy::redact::Redactor;
use my_worklog_core::report::daily;
use tempfile::tempdir;

#[test]
fn report_shows_metrics_when_events_store_duration_and_tokens() {
    let dir = tempdir().expect("tempdir");
    let spool = dir.path().join("spool/opencode");
    fs::create_dir_all(&spool).expect("create spool");
    fs::write(
        spool.join("events.jsonl"),
        r#"{"source_agent":"opencode","source_session_id":"s1","source_event_id":"u1","type":"assistant_message","role":"assistant","timestamp":"2026-07-24T09:00:00Z","content":"Implemented report metrics","duration_ms":123000,"usage":{"input_tokens":900,"output_tokens":334,"total_tokens":1234}}
"#,
    )
    .expect("write fixture");
    let db = WorklogDb::open(&dir.path().join("worklog.sqlite")).expect("open db");
    let redactor = Redactor::new(None).expect("redactor");
    import_spool(db.connection(), &dir.path().join("spool"), &redactor).expect("import");
    let date = chrono::NaiveDate::from_ymd_opt(2026, 7, 24).expect("valid date");

    let report = daily::render_day(db.connection(), date).expect("render day");

    assert!(report.contains("## Metrics"));
    assert!(report.contains("- Events: 1"));
    assert!(report.contains("- Sessions: 1"));
    assert!(report.contains("- User prompts: 0"));
    assert!(report.contains("- Assistant messages: 1"));
    assert!(report.contains("- Commands: 0"));
    assert!(report.contains("- File events: 0"));
    assert!(report.contains("- Total time: 2m 03s"));
    assert!(report.contains("- Tokens: 1,234 total (900 input, 334 output)"));
    assert!(!report.contains("\"usage\""));
}

#[test]
fn report_metrics_include_unavailable_time_and_tokens() {
    let dir = tempdir().expect("tempdir");
    let spool = dir.path().join("spool/opencode");
    fs::create_dir_all(&spool).expect("create spool");
    fs::write(
        spool.join("events.jsonl"),
        r#"{"source_agent":"opencode","source_session_id":"s1","source_event_id":"u1","type":"user_prompt","role":"user","timestamp":"2026-07-24T09:00:00Z","content":"Plan metrics output"}
"#,
    )
    .expect("write fixture");
    let db = WorklogDb::open(&dir.path().join("worklog.sqlite")).expect("open db");
    let redactor = Redactor::new(None).expect("redactor");
    import_spool(db.connection(), &dir.path().join("spool"), &redactor).expect("import");
    let date = chrono::NaiveDate::from_ymd_opt(2026, 7, 24).expect("valid date");

    let report = daily::render_day(db.connection(), date).expect("render day");

    assert!(report.contains("## Metrics"));
    assert!(report.contains("- Events: 1"));
    assert!(report.contains("- Sessions: 1"));
    assert!(report.contains("- User prompts: 1"));
    assert!(report.contains("- Assistant messages: 0"));
    assert!(report.contains("- Commands: 0"));
    assert!(report.contains("- File events: 0"));
    assert!(report.contains("- Total time: Not available"));
    assert!(report.contains("- Tokens: Not available"));
}

#[test]
fn share_context_includes_complete_session_messages() {
    let dir = tempdir().expect("tempdir");
    let spool = dir.path().join("spool/opencode");
    fs::create_dir_all(&spool).expect("create spool");
    let long_message = "Detailed implementation note ".repeat(16);
    let mut records = String::new();
    for index in 1..=25 {
        records.push_str(&format!(
            "{{\"source_agent\":\"opencode\",\"source_session_id\":\"s1\",\"source_event_id\":\"u{index}\",\"type\":\"user_prompt\",\"role\":\"user\",\"timestamp\":\"2026-07-24T09:{index:02}:00Z\",\"content\":\"Readable work item {index}\"}}\n"
        ));
    }
    records.push_str(&format!(
        "{{\"source_agent\":\"opencode\",\"source_session_id\":\"s1\",\"source_event_id\":\"long\",\"type\":\"assistant_message\",\"role\":\"assistant\",\"timestamp\":\"2026-07-24T09:30:00Z\",\"content\":{}}}\n",
        serde_json::to_string(&long_message).expect("serialize message")
    ));
    fs::write(spool.join("events.jsonl"), records).expect("write fixture");
    let db = WorklogDb::open(&dir.path().join("worklog.sqlite")).expect("open db");
    let redactor = Redactor::new(None).expect("redactor");
    import_spool(db.connection(), &dir.path().join("spool"), &redactor).expect("import");
    let date = chrono::NaiveDate::from_ymd_opt(2026, 7, 24).expect("valid date");

    let terminal_report = daily::render_day(db.connection(), date).expect("render day");
    let share_context = daily::render_share_day(db.connection(), date).expect("render share day");

    assert!(!terminal_report.contains("Readable work item 25"));
    assert!(share_context.contains("Readable work item 25"));
    assert!(share_context.contains(&long_message));
    assert!(!share_context.contains("\"source_agent\""));
}
