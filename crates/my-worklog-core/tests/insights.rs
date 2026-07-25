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
    assert!(files.contains("Edited: src/report.rs"));
    assert!(commands.contains("Command: cargo test"));
    assert!(agents.contains("- opencode: 5 events"));
}
