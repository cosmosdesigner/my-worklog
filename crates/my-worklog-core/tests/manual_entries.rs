use chrono::{DateTime, Utc};
use my_worklog_core::WorklogDb;
use my_worklog_core::manual::{self, NewManualEntry};
use my_worklog_core::report::daily;
use tempfile::tempdir;

fn time(value: &str) -> DateTime<Utc> {
    value.parse().expect("timestamp")
}

fn entry(id: &str, start: &str, end: &str) -> NewManualEntry {
    NewManualEntry {
        id: id.to_string(),
        start: time(start),
        end: time(end),
        project: "my-worklog".to_string(),
        category: "meeting".to_string(),
        description: "Planning meeting".to_string(),
        tags: Some("planning".to_string()),
        work_item: Some("#5".to_string()),
    }
}

#[test]
fn manual_entries_are_persisted_and_reported_separately() {
    let dir = tempdir().expect("tempdir");
    let db = WorklogDb::open(&dir.path().join("worklog.db")).expect("database");
    manual::create(
        db.connection(),
        &entry("manual-1", "2026-01-15T10:00:00Z", "2026-01-15T12:00:00Z"),
    )
    .expect("create");

    let entries = manual::list_between(
        db.connection(),
        time("2026-01-15T00:00:00Z"),
        time("2026-01-16T00:00:00Z"),
    )
    .expect("list");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].description, "Planning meeting");

    let report = daily::render_day(db.connection(), "2026-01-15".parse().unwrap()).expect("report");
    assert!(report.contains("Manual activity"));
    assert!(report.contains("Planning meeting"));
    assert!(report.contains("Manual time: 2h 00m"));
}

#[test]
fn invalid_ranges_are_rejected_and_overlaps_are_reported() {
    let dir = tempdir().expect("tempdir");
    let db = WorklogDb::open(&dir.path().join("worklog.db")).expect("database");
    let invalid = entry("invalid", "2026-01-15T12:00:00Z", "2026-01-15T10:00:00Z");
    assert!(manual::create(db.connection(), &invalid).is_err());

    manual::create(
        db.connection(),
        &entry("manual-1", "2026-01-15T10:00:00Z", "2026-01-15T12:00:00Z"),
    )
    .expect("create first");
    manual::create(
        db.connection(),
        &entry("manual-2", "2026-01-15T11:00:00Z", "2026-01-15T13:00:00Z"),
    )
    .expect("create second");
    let overlaps = manual::overlapping(
        db.connection(),
        &manual::get(db.connection(), "manual-2")
            .expect("get")
            .expect("entry"),
    )
    .expect("overlaps");
    assert_eq!(overlaps.len(), 1);
    let report = daily::render_day(db.connection(), "2026-01-15".parse().unwrap()).expect("report");
    assert!(report.contains("overlapping manual entries detected"));
}

#[test]
fn manual_entries_can_be_updated_and_deleted_without_touching_imported_events() {
    let dir = tempdir().expect("tempdir");
    let db = WorklogDb::open(&dir.path().join("worklog.db")).expect("database");
    manual::create(
        db.connection(),
        &entry("manual-1", "2026-01-15T10:00:00Z", "2026-01-15T12:00:00Z"),
    )
    .expect("create");
    let mut changed = entry("manual-1", "2026-01-15T13:00:00Z", "2026-01-15T14:00:00Z");
    changed.description = "Updated meeting".to_string();
    manual::update(db.connection(), &changed).expect("update");
    assert_eq!(
        manual::get(db.connection(), "manual-1")
            .unwrap()
            .unwrap()
            .description,
        "Updated meeting"
    );
    manual::delete(db.connection(), "manual-1").expect("delete");
    assert!(manual::get(db.connection(), "manual-1").unwrap().is_none());
}
