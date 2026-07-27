use std::fs;

use chrono::{Datelike, Days, Local, NaiveDate, TimeZone};
use my_worklog_core::WorklogDb;
use my_worklog_core::db::repositories::{events_between, search_events};
use my_worklog_core::ingest::spool::import_spool;
use my_worklog_core::privacy::redact::Redactor;
use my_worklog_core::report::insights::{self, ReportPeriod};
use my_worklog_core::report::{daily, weekly};
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
fn repository_events_include_project_metadata_and_backfill_session_context() {
    let dir = tempdir().expect("tempdir");
    let spool = dir.path().join("spool/opencode");
    fs::create_dir_all(&spool).expect("create spool");
    fs::write(
        spool.join("events.jsonl"),
        r#"{"source_agent":"opencode","source_session_id":"s-project","source_event_id":"first","type":"user_prompt","timestamp":"2026-07-24T09:00:00Z","content":"Plan repository metadata"}
{"source_agent":"opencode","source_session_id":"s-project","source_event_id":"second","type":"assistant_message","timestamp":"2026-07-24T09:01:00Z","cwd":"/workspace/parent/api","project_root":"/workspace/parent/api","content":"Implemented project metadata"}
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
    let results = search_events(db.connection(), "Implemented project metadata").expect("search");
    let session_context: (Option<String>, Option<String>) = db
        .connection()
        .query_row(
            "SELECT project.root_path, work_session.cwd
             FROM work_session LEFT JOIN project ON project.id = work_session.project_id
             WHERE work_session.source_session_id = 's-project'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("session context");

    assert_eq!(events.len(), 2);
    assert_eq!(
        events[0].project_root.as_deref(),
        Some("/workspace/parent/api")
    );
    assert_eq!(events[0].project_name.as_deref(), Some("api"));
    assert_eq!(
        events[1].project_root.as_deref(),
        Some("/workspace/parent/api")
    );
    assert_eq!(results[0].project_name.as_deref(), Some("api"));
    assert_eq!(session_context.0.as_deref(), Some("/workspace/parent/api"));
    assert_eq!(session_context.1.as_deref(), Some("/workspace/parent/api"));
}

#[test]
fn day_and_week_reports_group_worked_projects_without_sibling_discovery() {
    let dir = tempdir().expect("tempdir");
    let spool = dir.path().join("spool/opencode");
    fs::create_dir_all(&spool).expect("create spool");
    fs::write(
        spool.join("events.jsonl"),
        r#"{"source_agent":"opencode","source_session_id":"api","source_event_id":"api-1","type":"user_prompt","role":"user","timestamp":"2026-07-24T09:00:00Z","project_root":"/workspace/company/api","cwd":"/workspace/company/api","content":"Plan API route"}
{"source_agent":"opencode","source_session_id":"web","source_event_id":"web-1","type":"assistant_message","role":"assistant","timestamp":"2026-07-24T09:01:00Z","project_root":"/workspace/company/web","cwd":"/workspace/company/web","content":"Implemented web UI"}
{"source_agent":"opencode","source_session_id":"ops","source_event_id":"ops-1","type":"command","timestamp":"2026-07-24T09:02:00Z","project_root":"/workspace/company/ops","cwd":"/workspace/company/ops","command":"terraform plan","content":"terraform plan"}
"#,
    )
    .expect("write fixture");
    let db = WorklogDb::open(&dir.path().join("worklog.sqlite")).expect("open db");
    let redactor = Redactor::new(None).expect("redactor");
    import_spool(db.connection(), &dir.path().join("spool"), &redactor).expect("import");
    let date = chrono::NaiveDate::from_ymd_opt(2026, 7, 24).expect("valid date");
    let end = chrono::Utc
        .with_ymd_and_hms(2026, 7, 24, 10, 0, 0)
        .single()
        .expect("valid end")
        .with_timezone(&chrono::Local);

    let day = daily::render_day(db.connection(), date).expect("render day");
    let week = weekly::render_week_until(db.connection(), end).expect("render week");

    for report in [day, week] {
        assert!(report.contains("## Projects"));
        assert!(report.contains("- api: 1 events"));
        assert!(report.contains("- web: 1 events"));
        assert!(report.contains("- ops: 1 events"));
        assert!(report.contains("## Main work"));
        assert!(report.contains("### api"));
        assert!(report.contains("User: Plan API route [opencode]"));
        assert!(report.contains("### web"));
        assert!(report.contains("Assistant: Implemented web UI [opencode]"));
        assert!(report.contains("### ops"));
        assert!(report.contains("Command: terraform plan [opencode]"));
        assert!(!report.contains("### mobile"));
    }
}

#[test]
fn single_project_day_report_keeps_flat_main_work_shape() {
    let dir = tempdir().expect("tempdir");
    let spool = dir.path().join("spool/opencode");
    fs::create_dir_all(&spool).expect("create spool");
    fs::write(
        spool.join("events.jsonl"),
        r#"{"source_agent":"opencode","source_session_id":"api","source_event_id":"api-1","type":"user_prompt","role":"user","timestamp":"2026-07-24T09:00:00Z","project_root":"/workspace/company/api","cwd":"/workspace/company/api","content":"Plan API route"}
"#,
    )
    .expect("write fixture");
    let db = WorklogDb::open(&dir.path().join("worklog.sqlite")).expect("open db");
    let redactor = Redactor::new(None).expect("redactor");
    import_spool(db.connection(), &dir.path().join("spool"), &redactor).expect("import");
    let date = chrono::NaiveDate::from_ymd_opt(2026, 7, 24).expect("valid date");

    let report = daily::render_day(db.connection(), date).expect("render day");

    assert!(report.contains("## Main work"));
    assert!(report.contains("- User: Plan API route [opencode]"));
    assert!(!report.contains("## Projects"));
    assert!(!report.contains("### api"));
}

#[test]
fn insight_reports_group_results_by_worked_project_when_multiple_projects_exist() {
    let dir = tempdir().expect("tempdir");
    let spool = dir.path().join("spool/opencode");
    fs::create_dir_all(&spool).expect("create spool");
    let yesterday = Local::now().date_naive() - Days::new(1);
    fs::write(
        spool.join("events.jsonl"),
        format!(
            "{{\"source_agent\":\"opencode\",\"source_session_id\":\"api\",\"source_event_id\":\"api-decision\",\"type\":\"assistant_message\",\"role\":\"assistant\",\"timestamp\":\"{}\",\"project_root\":\"/workspace/company/api\",\"cwd\":\"/workspace/company/api\",\"content\":\"Decision: use axum for API\"}}\n{{\"source_agent\":\"opencode\",\"source_session_id\":\"api\",\"source_event_id\":\"api-done\",\"type\":\"assistant_message\",\"role\":\"assistant\",\"timestamp\":\"{}\",\"project_root\":\"/workspace/company/api\",\"cwd\":\"/workspace/company/api\",\"content\":\"Completed: added API route\"}}\n{{\"source_agent\":\"opencode\",\"source_session_id\":\"api\",\"source_event_id\":\"api-file\",\"type\":\"file_edit\",\"timestamp\":\"{}\",\"project_root\":\"/workspace/company/api\",\"cwd\":\"/workspace/company/api\",\"file_path\":\"api/src/main.rs\",\"content\":\"Edited API file\"}}\n{{\"source_agent\":\"opencode\",\"source_session_id\":\"web\",\"source_event_id\":\"web-loop\",\"type\":\"assistant_message\",\"role\":\"assistant\",\"timestamp\":\"{}\",\"project_root\":\"/workspace/company/web\",\"cwd\":\"/workspace/company/web\",\"content\":\"TODO: follow up on web polish\"}}\n{{\"source_agent\":\"opencode\",\"source_session_id\":\"web\",\"source_event_id\":\"web-blocker\",\"type\":\"assistant_message\",\"role\":\"assistant\",\"timestamp\":\"{}\",\"project_root\":\"/workspace/company/web\",\"cwd\":\"/workspace/company/web\",\"content\":\"Blocker: missing design token\"}}\n{{\"source_agent\":\"opencode\",\"source_session_id\":\"web\",\"source_event_id\":\"web-command\",\"type\":\"command\",\"timestamp\":\"{}\",\"project_root\":\"/workspace/company/web\",\"cwd\":\"/workspace/company/web\",\"command\":\"bun test\",\"content\":\"bun test\"}}\n",
            timestamp(yesterday, 9, 0),
            timestamp(yesterday, 9, 1),
            timestamp(yesterday, 9, 2),
            timestamp(yesterday, 9, 3),
            timestamp(yesterday, 9, 4),
            timestamp(yesterday, 9, 5)
        ),
    )
    .expect("write fixture");
    let db = WorklogDb::open(&dir.path().join("worklog.sqlite")).expect("open db");
    let redactor = Redactor::new(None).expect("redactor");
    import_spool(db.connection(), &dir.path().join("spool"), &redactor).expect("import");

    let status = insights::status(db.connection(), ReportPeriod::Yesterday).expect("status");
    let done = insights::done(db.connection(), ReportPeriod::Yesterday).expect("done");
    let decisions =
        insights::decisions(db.connection(), ReportPeriod::Yesterday).expect("decisions");
    let open_loops = insights::open_loops(db.connection(), ReportPeriod::Yesterday).expect("loops");
    let blockers = insights::blockers(db.connection(), ReportPeriod::Yesterday).expect("blockers");
    let files = insights::files(db.connection(), ReportPeriod::Yesterday).expect("files");
    let commands = insights::commands(db.connection(), ReportPeriod::Yesterday).expect("commands");

    assert!(status.contains("## Projects"));
    assert!(status.contains("- api: 3 events"));
    assert!(status.contains("- web: 3 events"));
    assert!(status.contains("### api"));
    assert!(status.contains("Decision: use axum for API [opencode]"));
    assert!(status.contains("Completed: added API route [opencode]"));
    assert!(status.contains("- api/src/main.rs: 1 events"));
    assert!(status.contains("### web"));
    assert!(status.contains("TODO: follow up on web polish [opencode]"));
    assert!(status.contains("Blocker: missing design token [opencode]"));
    assert!(status.contains("- bun test: 1 runs"));
    assert!(!status.contains("### mobile"));
    assert!(done.contains("### api"));
    assert!(done.contains("Completed: added API route [opencode]"));
    assert!(decisions.contains("### api"));
    assert!(decisions.contains("Decision: use axum for API [opencode]"));
    assert!(open_loops.contains("### web"));
    assert!(open_loops.contains("TODO: follow up on web polish [opencode]"));
    assert!(blockers.contains("### web"));
    assert!(blockers.contains("Blocker: missing design token [opencode]"));
    assert!(files.contains("### api"));
    assert!(files.contains("- api/src/main.rs: 1 events"));
    assert!(commands.contains("### web"));
    assert!(commands.contains("- bun test: 1 runs"));
}
