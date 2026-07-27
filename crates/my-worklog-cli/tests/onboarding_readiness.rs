use std::fs;

mod common;

use common::{
    TestEnv, assert_failure, assert_stderr_contains, assert_stdout_contains, assert_success,
};

const TOOL_NAMES: &[&str] = &[
    "worklog_today.ts",
    "worklog_yesterday.ts",
    "worklog_week.ts",
    "worklog_status.ts",
    "worklog_done.ts",
    "worklog_decisions.ts",
    "worklog_open_loops.ts",
    "worklog_blockers.ts",
    "worklog_files.ts",
    "worklog_commands.ts",
    "worklog_agents.ts",
];

#[test]
fn doctor_reports_healthy_opencode_readiness() {
    // Given: initialized my-worklog storage and discoverable OpenCode fixtures.
    let env = TestEnv::new();
    let init = env.run(["init"]);
    assert_success(&init);
    create_opencode_install_fixture(&env);
    create_opencode_db_fixture(&env);

    // When: doctor checks the sandboxed environment.
    let output = env.run(["doctor"]);

    // Then: doctor has an OpenCode readiness section with all green checks.
    assert_success(&output);
    assert_stdout_contains(&output, "OpenCode:");
    assert_stdout_contains(&output, "Plugin: ready");
    assert_stdout_contains(&output, "Helper tools: ready");
    assert_stdout_contains(&output, "Worklog database: ready");
    assert_stdout_contains(&output, "Import source: ready");
}

#[test]
fn doctor_checks_existing_database_without_migrating_or_writing() {
    // Given: an existing readable SQLite file that is not a migrated worklog DB.
    let env = TestEnv::new();
    fs::create_dir_all(&env.home).expect("home dir");
    let database = env.home.join("worklog.sqlite");
    let connection = rusqlite::Connection::open(&database).expect("sqlite db");
    connection
        .execute_batch("PRAGMA user_version = 0;")
        .expect("seed minimal db");
    drop(connection);
    let tables_before = sqlite_table_count(&database);

    // When: doctor checks the existing DB.
    let output = env.run(["doctor"]);

    // Then: the DB was only opened read-only; no migrations/schema tables were created.
    assert_success(&output);
    assert_stdout_contains(&output, "Worklog database: ready");
    assert_eq!(sqlite_table_count(&database), tables_before);
    assert!(!database.with_extension("sqlite-wal").exists());
}

#[test]
fn doctor_reports_missing_opencode_readiness_with_actions_without_creating_paths() {
    // Given: no initialized my-worklog storage and no OpenCode install/source.
    let env = TestEnv::new();
    let missing_db = env.opencode_db.with_file_name("missing-opencode.db");

    // When: doctor runs against missing sandbox paths.
    let output = env.run_with_db(&missing_db, ["doctor"]);

    // Then: doctor stays read-only and prints actionable onboarding commands.
    assert_success(&output);
    assert_stdout_contains(&output, "OpenCode:");
    assert_stdout_contains(&output, "Plugin: missing");
    assert_stdout_contains(&output, "Helper tools: missing");
    assert_stdout_contains(&output, "my-worklog install opencode --global");
    assert_stdout_contains(&output, "my-worklog init");
    assert_stdout_contains(&output, "--opencode-db");
    assert_stdout_contains(&output, "--opencode-export");
    assert!(!env.home.exists());
    assert!(!env.opencode.exists());
    assert!(!missing_db.exists());
}

#[test]
fn import_missing_opencode_source_explains_source_options() {
    // Given: an isolated my-worklog home and a missing OpenCode DB override.
    let env = TestEnv::new();
    let missing_db = env.opencode_db.with_file_name("missing-opencode.db");

    // When: OpenCode import cannot discover local history.
    let output = env.run_with_db(&missing_db, ["import", "--opencode"]);

    // Then: the error distinguishes missing source from an import failure.
    assert_failure(&output);
    assert_stderr_contains(&output, "No OpenCode import source found");
    assert_stderr_contains(&output, "--opencode-db");
    assert_stderr_contains(&output, "--opencode-export");
}

#[test]
fn import_zero_event_export_guides_user_without_implying_failure() {
    // Given: an empty OpenCode export fixture.
    let env = TestEnv::new();
    let export = env.opencode_db.with_file_name("empty-export.json");
    fs::write(&export, r#"{"info":{"id":"empty"},"messages":[]}"#).expect("export");
    let export_path = export.to_str().expect("utf-8 path");

    // When: OpenCode import succeeds but imports zero messages.
    let output = env.run(["import", "--opencode", "--opencode-export", export_path]);

    // Then: zero events are framed as guidance, not as a silent success.
    assert_success(&output);
    assert_stdout_contains(&output, "Imported 0 OpenCode messages");
    assert_stdout_contains(&output, "No new OpenCode messages found");
    assert_stdout_contains(&output, "Open OpenCode and start a session");
    assert_stdout_contains(&output, "my-worklog status --period week --compact");
}

#[test]
fn import_successful_export_points_to_status_next_step() {
    // Given: a deterministic OpenCode export with one importable message.
    let env = TestEnv::new();
    let export = env.opencode_db.with_file_name("session-export.json");
    fs::write(
        &export,
        r#"{
          "info": {"id":"success", "directory":"/tmp/project"},
          "messages": [
            {"id":"msg_user", "role":"user", "created":"2026-07-24T09:00:00Z", "content":"Ship onboarding import guidance"}
          ]
        }"#,
    )
    .expect("export");
    let export_path = export.to_str().expect("utf-8 path");

    // When: OpenCode import succeeds with new messages.
    let output = env.run(["import", "--opencode", "--opencode-export", export_path]);

    // Then: successful import output points to the compact status command.
    assert_success(&output);
    assert_stdout_contains(&output, "Imported 1 OpenCode messages");
    assert_stdout_contains(&output, "Next: my-worklog status --period week --compact");
}

fn create_opencode_install_fixture(env: &TestEnv) {
    fs::create_dir_all(env.opencode.join("plugins")).expect("plugins dir");
    fs::create_dir_all(env.opencode.join("tools")).expect("tools dir");
    fs::write(env.opencode.join("plugins/my-worklog.ts"), "plugin").expect("plugin");
    for tool in TOOL_NAMES {
        fs::write(env.opencode.join("tools").join(tool), "tool").expect("tool");
    }
}

fn create_opencode_db_fixture(env: &TestEnv) {
    let connection = rusqlite::Connection::open(&env.opencode_db).expect("opencode db");
    connection
        .execute_batch(
            r#"
            CREATE TABLE message (id TEXT PRIMARY KEY, session_id TEXT NOT NULL, data TEXT NOT NULL);
            CREATE TABLE part (id TEXT PRIMARY KEY, message_id TEXT NOT NULL, data TEXT NOT NULL);
            "#,
        )
        .expect("opencode schema");
}

fn sqlite_table_count(path: &std::path::Path) -> i64 {
    let connection = rusqlite::Connection::open(path).expect("sqlite db");
    connection
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type IN ('table', 'virtual table')",
            [],
            |row| row.get(0),
        )
        .expect("table count")
}
