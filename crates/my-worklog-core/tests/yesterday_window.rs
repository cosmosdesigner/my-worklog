use std::fs;

use chrono::{Datelike, Days, Local, NaiveDate, TimeZone};
use my_worklog_core::WorklogDb;
use my_worklog_core::ingest::spool::import_spool;
use my_worklog_core::privacy::redact::Redactor;
use my_worklog_core::report::daily;
use my_worklog_core::report::insights::{self, ReportPeriod};
use tempfile::tempdir;

fn timestamp(date: NaiveDate, hour: u32, minute: u32) -> String {
    Local
        .with_ymd_and_hms(date.year(), date.month(), date.day(), hour, minute, 0)
        .single()
        .expect("valid local fixture time")
        .with_timezone(&chrono::Utc)
        .to_rfc3339()
}

#[test]
fn yesterday_reports_include_full_previous_calendar_day() {
    let dir = tempdir().expect("tempdir");
    let spool = dir.path().join("spool/opencode");
    fs::create_dir_all(&spool).expect("create spool");
    fs::write(
        spool.join("events.jsonl"),
        r#"{"source_agent":"opencode","source_session_id":"s1","source_event_id":"early","type":"user_prompt","role":"user","timestamp":"2026-07-24T09:00:00Z","content":"Yesterday early item"}
{"source_agent":"opencode","source_session_id":"s1","source_event_id":"late","type":"user_prompt","role":"user","timestamp":"2026-07-24T22:30:00Z","content":"Yesterday late item"}
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
        daily::render_yesterday_until(db.connection(), now).expect("render yesterday");
    let full_report =
        daily::render_full_yesterday_until(db.connection(), now).expect("render full yesterday");
    let share_context =
        daily::render_share_yesterday_until(db.connection(), now).expect("render share yesterday");

    for report in [terminal_report, full_report, share_context] {
        assert!(report.contains("Yesterday early item"));
        assert!(report.contains("Yesterday late item"));
        assert!(!report.contains("Today item"));
    }
}

#[test]
fn yesterday_insights_include_full_previous_calendar_day() {
    let dir = tempdir().expect("tempdir");
    let spool = dir.path().join("spool/opencode");
    fs::create_dir_all(&spool).expect("create spool");
    let yesterday = Local::now().date_naive() - Days::new(1);
    let today = Local::now().date_naive();
    fs::write(
        spool.join("events.jsonl"),
        format!(
            "{{\"source_agent\":\"opencode\",\"source_session_id\":\"s1\",\"source_event_id\":\"early\",\"type\":\"assistant_message\",\"role\":\"assistant\",\"timestamp\":\"{}\",\"content\":\"Decision: include early yesterday item\"}}\n{{\"source_agent\":\"opencode\",\"source_session_id\":\"s1\",\"source_event_id\":\"late\",\"type\":\"assistant_message\",\"role\":\"assistant\",\"timestamp\":\"{}\",\"content\":\"Decision: include late yesterday item\"}}\n{{\"source_agent\":\"opencode\",\"source_session_id\":\"s1\",\"source_event_id\":\"today\",\"type\":\"assistant_message\",\"role\":\"assistant\",\"timestamp\":\"{}\",\"content\":\"Decision: exclude today item\"}}\n",
            timestamp(yesterday, 0, 30),
            timestamp(yesterday, 23, 30),
            timestamp(today, 0, 30)
        ),
    )
    .expect("write fixture");
    let db = WorklogDb::open(&dir.path().join("worklog.sqlite")).expect("open db");
    let redactor = Redactor::new(None).expect("redactor");
    import_spool(db.connection(), &dir.path().join("spool"), &redactor).expect("import");

    let output = insights::decisions(db.connection(), ReportPeriod::Yesterday).expect("decisions");

    assert!(output.contains("Decision: include early yesterday item"));
    assert!(output.contains("Decision: include late yesterday item"));
    assert!(!output.contains("Decision: exclude today item"));
}
