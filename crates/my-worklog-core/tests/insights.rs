use std::fs;

use chrono::{Datelike, Days, Local, TimeZone};
use my_worklog_core::WorklogDb;
use my_worklog_core::ingest::spool::import_spool;
use my_worklog_core::privacy::redact::Redactor;
use my_worklog_core::report::insights::{self, ReportPeriod};
use tempfile::tempdir;

#[test]
fn insight_reports_filter_local_work_events_and_keep_metrics() {
    let dir = tempdir().expect("tempdir");
    let spool = dir.path().join("spool/opencode");
    fs::create_dir_all(&spool).expect("create spool");
    let yesterday = Local::now().date_naive() - Days::new(1);
    let timestamp = |hour: u32, minute: u32| {
        Local
            .with_ymd_and_hms(
                yesterday.year(),
                yesterday.month(),
                yesterday.day(),
                hour,
                minute,
                0,
            )
            .single()
            .expect("valid local fixture time")
            .with_timezone(&chrono::Utc)
            .to_rfc3339()
    };
    fs::write(
        spool.join("events.jsonl"),
        format!(
            "{{\"source_agent\":\"opencode\",\"source_session_id\":\"s1\",\"source_event_id\":\"decision\",\"type\":\"assistant_message\",\"role\":\"assistant\",\"timestamp\":\"{}\",\"content\":\"Decision: use local SQLite for reports\",\"duration_ms\":5000,\"usage\":{{\"input_tokens\":100,\"output_tokens\":25,\"total_tokens\":125}}}}\n{{\"source_agent\":\"opencode\",\"source_session_id\":\"s1\",\"source_event_id\":\"loop\",\"type\":\"user_prompt\",\"role\":\"user\",\"timestamp\":\"{}\",\"content\":\"Next follow up is adding open loops\"}}\n{{\"source_agent\":\"opencode\",\"source_session_id\":\"s1\",\"source_event_id\":\"blocker\",\"type\":\"assistant_message\",\"role\":\"assistant\",\"timestamp\":\"{}\",\"content\":\"Blocked by missing adapter data\"}}\n{{\"source_agent\":\"opencode\",\"source_session_id\":\"s1\",\"source_event_id\":\"file\",\"type\":\"file_edit\",\"timestamp\":\"{}\",\"file_path\":\"src/report.rs\",\"content\":\"Edited report file\"}}\n{{\"source_agent\":\"opencode\",\"source_session_id\":\"s1\",\"source_event_id\":\"cmd\",\"type\":\"command\",\"timestamp\":\"{}\",\"command\":\"cargo test\",\"content\":\"cargo test\"}}\n",
            timestamp(9, 0),
            timestamp(9, 1),
            timestamp(9, 2),
            timestamp(9, 3),
            timestamp(9, 4)
        ),
    )
    .expect("write fixture");
    let db = WorklogDb::open(&dir.path().join("worklog.sqlite")).expect("open db");
    let redactor = Redactor::new(None).expect("redactor");
    import_spool(db.connection(), &dir.path().join("spool"), &redactor).expect("import");

    let decisions =
        insights::decisions(db.connection(), ReportPeriod::Yesterday).expect("render decisions");
    let open_loops =
        insights::open_loops(db.connection(), ReportPeriod::Yesterday).expect("render open loops");
    let blockers =
        insights::blockers(db.connection(), ReportPeriod::Yesterday).expect("render blockers");
    let files = insights::files(db.connection(), ReportPeriod::Yesterday).expect("render files");
    let commands =
        insights::commands(db.connection(), ReportPeriod::Yesterday).expect("render commands");
    let agents = insights::agents(db.connection(), ReportPeriod::Yesterday).expect("render agents");

    assert!(decisions.contains("# Decisions - yesterday"));
    assert!(decisions.contains("Decision: use local SQLite for reports"));
    assert!(decisions.contains("- Total time: 0m 05s"));
    assert!(decisions.contains("- Tokens: 125 total (100 input, 25 output)"));
    assert!(open_loops.contains("Next follow up is adding open loops"));
    assert!(blockers.contains("Blocked by missing adapter data"));
    assert!(files.contains("- src/report.rs: 1 events"));
    assert!(commands.contains("- cargo test: 1 runs"));
    assert!(agents.contains("- opencode: 5 events"));
}

#[test]
fn insight_reports_group_duplicates_and_status_suppresses_noise() {
    let dir = tempdir().expect("tempdir");
    let spool = dir.path().join("spool/opencode");
    fs::create_dir_all(&spool).expect("create spool");
    let yesterday = Local::now().date_naive() - Days::new(1);
    let timestamp = |hour: u32, minute: u32| {
        Local
            .with_ymd_and_hms(
                yesterday.year(),
                yesterday.month(),
                yesterday.day(),
                hour,
                minute,
                0,
            )
            .single()
            .expect("valid local fixture time")
            .with_timezone(&chrono::Utc)
            .to_rfc3339()
    };
    fs::write(
        spool.join("events.jsonl"),
        format!(
            "{{\"source_agent\":\"opencode\",\"source_session_id\":\"s2\",\"source_event_id\":\"decision-1\",\"type\":\"assistant_message\",\"role\":\"assistant\",\"timestamp\":\"{}\",\"content\":\"Decision: use SQLite for local reports\",\"duration_ms\":5000,\"usage\":{{\"input_tokens\":100,\"output_tokens\":25,\"total_tokens\":125}}}}\n{{\"source_agent\":\"opencode\",\"source_session_id\":\"s2\",\"source_event_id\":\"decision-2\",\"type\":\"assistant_message\",\"role\":\"assistant\",\"timestamp\":\"{}\",\"content\":\"Decided: use SQLite for local reports\"}}\n{{\"source_agent\":\"opencode\",\"source_session_id\":\"s2\",\"source_event_id\":\"loop\",\"type\":\"assistant_message\",\"role\":\"assistant\",\"timestamp\":\"{}\",\"content\":\"TODO: follow up on OpenCode tool install\"}}\n{{\"source_agent\":\"opencode\",\"source_session_id\":\"s2\",\"source_event_id\":\"blocker\",\"type\":\"assistant_message\",\"role\":\"assistant\",\"timestamp\":\"{}\",\"content\":\"Blocker: adapter install test missing\"}}\n{{\"source_agent\":\"opencode\",\"source_session_id\":\"s2\",\"source_event_id\":\"file-1\",\"type\":\"file_edit\",\"timestamp\":\"{}\",\"file_path\":\"src/report.rs\",\"content\":\"Edited report file\"}}\n{{\"source_agent\":\"opencode\",\"source_session_id\":\"s2\",\"source_event_id\":\"file-2\",\"type\":\"file_edit\",\"timestamp\":\"{}\",\"file_path\":\"src/report.rs\",\"content\":\"Edited report file again\"}}\n{{\"source_agent\":\"opencode\",\"source_session_id\":\"s2\",\"source_event_id\":\"cmd-1\",\"type\":\"command\",\"timestamp\":\"{}\",\"command\":\"cargo test\",\"content\":\"cargo test\"}}\n{{\"source_agent\":\"opencode\",\"source_session_id\":\"s2\",\"source_event_id\":\"cmd-2\",\"type\":\"command\",\"timestamp\":\"{}\",\"command\":\"cargo test\",\"content\":\"cargo test\"}}\n{{\"source_agent\":\"opencode\",\"source_session_id\":\"s2\",\"source_event_id\":\"noise\",\"type\":\"assistant_message\",\"role\":\"assistant\",\"timestamp\":\"{}\",\"content\":\"tool result\"}}\n",
            timestamp(9, 0),
            timestamp(9, 1),
            timestamp(9, 2),
            timestamp(9, 3),
            timestamp(9, 4),
            timestamp(9, 5),
            timestamp(9, 6),
            timestamp(9, 7),
            timestamp(9, 8)
        ),
    )
    .expect("write fixture");
    let db = WorklogDb::open(&dir.path().join("worklog.sqlite")).expect("open db");
    let redactor = Redactor::new(None).expect("redactor");
    import_spool(db.connection(), &dir.path().join("spool"), &redactor).expect("import");

    let decisions =
        insights::decisions(db.connection(), ReportPeriod::Yesterday).expect("render decisions");
    let files = insights::files(db.connection(), ReportPeriod::Yesterday).expect("render files");
    let commands =
        insights::commands(db.connection(), ReportPeriod::Yesterday).expect("render commands");
    let status = insights::status(db.connection(), ReportPeriod::Yesterday).expect("render status");

    assert!(decisions.contains("Decision: use SQLite for local reports (2 events) [opencode]"));
    assert!(!decisions.contains("Decided: use SQLite for local reports\n"));
    assert!(!decisions.contains("tool result"));
    assert!(files.contains("- src/report.rs: 2 events"));
    assert!(commands.contains("- cargo test: 2 runs"));
    assert!(status.contains("# Status - yesterday"));
    assert!(status.contains("## At a glance"));
    assert!(status.contains("## Blockers"));
    assert!(status.contains("Blocker: adapter install test missing [opencode]"));
    assert!(status.contains("## Decisions"));
    assert!(status.contains("Decision: use SQLite for local reports (2 events) [opencode]"));
    assert!(status.contains("## Open loops"));
    assert!(status.contains("TODO: follow up on OpenCode tool install [opencode]"));
    assert!(status.contains("- src/report.rs: 2 events"));
    assert!(status.contains("- cargo test: 2 runs"));
    assert!(status.contains("- opencode: 9 events"));
    assert!(!status.contains("tool result"));
}

#[test]
fn done_report_groups_completed_work_and_status_includes_it() {
    let dir = tempdir().expect("tempdir");
    let spool = dir.path().join("spool/opencode");
    fs::create_dir_all(&spool).expect("create spool");
    let yesterday = Local::now().date_naive() - Days::new(1);
    let timestamp = |hour: u32, minute: u32| {
        Local
            .with_ymd_and_hms(
                yesterday.year(),
                yesterday.month(),
                yesterday.day(),
                hour,
                minute,
                0,
            )
            .single()
            .expect("valid local fixture time")
            .with_timezone(&chrono::Utc)
            .to_rfc3339()
    };
    fs::write(
        spool.join("events.jsonl"),
        format!(
            "{{\"source_agent\":\"opencode\",\"source_session_id\":\"s3\",\"source_event_id\":\"done-1\",\"type\":\"assistant_message\",\"role\":\"assistant\",\"timestamp\":\"{}\",\"content\":\"Completed: added local status dashboard\"}}\n{{\"source_agent\":\"opencode\",\"source_session_id\":\"s3\",\"source_event_id\":\"done-2\",\"type\":\"assistant_message\",\"role\":\"assistant\",\"timestamp\":\"{}\",\"content\":\"Finished: added local status dashboard\"}}\n{{\"source_agent\":\"opencode\",\"source_session_id\":\"s3\",\"source_event_id\":\"fix\",\"type\":\"assistant_message\",\"role\":\"assistant\",\"timestamp\":\"{}\",\"content\":\"Fixed OpenCode helper generation\"}}\n{{\"source_agent\":\"opencode\",\"source_session_id\":\"s3\",\"source_event_id\":\"loop\",\"type\":\"assistant_message\",\"role\":\"assistant\",\"timestamp\":\"{}\",\"content\":\"TODO: write docs next\"}}\n",
            timestamp(10, 0),
            timestamp(10, 1),
            timestamp(10, 2),
            timestamp(10, 3)
        ),
    )
    .expect("write fixture");
    let db = WorklogDb::open(&dir.path().join("worklog.sqlite")).expect("open db");
    let redactor = Redactor::new(None).expect("redactor");
    import_spool(db.connection(), &dir.path().join("spool"), &redactor).expect("import");

    let done = insights::done(db.connection(), ReportPeriod::Yesterday).expect("render done");
    let status = insights::status(db.connection(), ReportPeriod::Yesterday).expect("render status");

    assert!(done.contains("# Done - yesterday"));
    assert!(done.contains("Completed: added local status dashboard (2 events) [opencode]"));
    assert!(done.contains("Fixed OpenCode helper generation [opencode]"));
    assert!(!done.contains("TODO: write docs next"));
    assert!(status.contains("## Completed work"));
    assert!(status.contains("Completed: added local status dashboard (2 events) [opencode]"));
}

#[test]
fn compact_done_and_status_skip_meta_dumps_and_truncate_items() {
    let dir = tempdir().expect("tempdir");
    let spool = dir.path().join("spool/opencode");
    fs::create_dir_all(&spool).expect("create spool");
    let yesterday = Local::now().date_naive() - Days::new(1);
    let timestamp = |hour: u32, minute: u32| {
        Local
            .with_ymd_and_hms(
                yesterday.year(),
                yesterday.month(),
                yesterday.day(),
                hour,
                minute,
                0,
            )
            .single()
            .expect("valid local fixture time")
            .with_timezone(&chrono::Utc)
            .to_rfc3339()
    };
    fs::write(
        spool.join("events.jsonl"),
        format!(
            "{{\"source_agent\":\"opencode\",\"source_session_id\":\"s4\",\"source_event_id\":\"meta\",\"type\":\"assistant_message\",\"role\":\"assistant\",\"timestamp\":\"{}\",\"content\":\"Implementation Plan: completed architecture summary with Cargo.toml crates/my-worklog-core/src/report/markdown.rs tests fixtures and a very long planning dump that should not be classified as completed work\"}}\n{{\"source_agent\":\"opencode\",\"source_session_id\":\"s4\",\"source_event_id\":\"plan_tag\",\"type\":\"assistant_message\",\"role\":\"assistant\",\"timestamp\":\"{}\",\"content\":\"<plan> # OpenCode Transcript Import Plan ## Context User Request Summary: plan a read-only import and next implementation steps\"}}\n{{\"source_agent\":\"opencode\",\"source_session_id\":\"s4\",\"source_event_id\":\"analysis_tag\",\"type\":\"assistant_message\",\"role\":\"assistant\",\"timestamp\":\"{}\",\"content\":\"<analysis> Literal Request: inspect the Rust workspace read-only, focusing on phase work and deciding what to implement next\"}}\n{{\"source_agent\":\"opencode\",\"source_session_id\":\"s4\",\"source_event_id\":\"advice\",\"type\":\"assistant_message\",\"role\":\"assistant\",\"timestamp\":\"{}\",\"content\":\"Recommended shape: use clap derive Parser Subcommand Args for typed commands and keep report generation pure and time-bounded, with fixed timestamps in tests and complete validation gates\"}}\n{{\"source_agent\":\"opencode\",\"source_session_id\":\"s4\",\"source_event_id\":\"reminder\",\"type\":\"user_prompt\",\"role\":\"user\",\"timestamp\":\"{}\",\"content\":\"<system-reminder> Completed background task. Continue if you have next steps or stop if blocked.\"}}\n{{\"source_agent\":\"opencode\",\"source_session_id\":\"s4\",\"source_event_id\":\"done\",\"type\":\"assistant_message\",\"role\":\"assistant\",\"timestamp\":\"{}\",\"content\":\"Completed: implemented compact status output with top sections, short bullets, OpenCode helper defaults, README updates, regression tests, and validation gates for the local worklog CLI\"}}\n{{\"source_agent\":\"opencode\",\"source_session_id\":\"s4\",\"source_event_id\":\"decision\",\"type\":\"assistant_message\",\"role\":\"assistant\",\"timestamp\":\"{}\",\"content\":\"Decision: keep compact insights deterministic and local-only\"}}\n",
            timestamp(1, 0),
            timestamp(1, 1),
            timestamp(1, 2),
            timestamp(1, 3),
            timestamp(1, 4),
            timestamp(1, 5),
            timestamp(1, 6)
        ),
    )
    .expect("write fixture");
    let db = WorklogDb::open(&dir.path().join("worklog.sqlite")).expect("open db");
    let redactor = Redactor::new(None).expect("redactor");
    import_spool(db.connection(), &dir.path().join("spool"), &redactor).expect("import");

    let done = insights::done_compact(db.connection(), ReportPeriod::Yesterday).expect("done");
    let status =
        insights::status_compact(db.connection(), ReportPeriod::Yesterday).expect("status");

    assert!(done.contains("# Done - yesterday"));
    assert!(done.contains("Completed: implemented compact status output"));
    assert!(done.contains("... [opencode]"));
    assert!(!done.contains("Implementation Plan"));
    assert!(!done.contains("Cargo.toml"));
    assert!(!done.contains("<analysis>"));
    assert!(!done.contains("<plan>"));
    assert!(!done.contains("Recommended shape"));
    assert!(status.contains("# Status - yesterday"));
    assert!(status.contains("## Completed work"));
    assert!(status.contains("## Decisions"));
    assert!(!status.contains("Implementation Plan"));
    assert!(!status.contains("<analysis>"));
    assert!(!status.contains("<plan>"));
    assert!(!status.contains("system-reminder"));
    assert!(!status.contains("Recommended shape"));
    assert!(!status.contains("regression tests, and validation gates"));
}
