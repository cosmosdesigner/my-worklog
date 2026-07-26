use my_worklog_adapter_opencode::install::{InstallOptions, InstallPlan};
use tempfile::tempdir;

#[test]
fn dry_run_reports_files_without_writing() {
    let dir = tempdir().expect("tempdir");
    let options = InstallOptions {
        target_dir: dir.path().join(".opencode"),
        worklog_home: dir.path().join("worklog"),
        dry_run: true,
        force: false,
        include_tools: true,
    };

    let plan = InstallPlan::build(&options);
    let report = plan.apply(&options).expect("dry run");

    assert!(report.dry_run);
    assert!(
        report
            .files
            .iter()
            .any(|path| path.ends_with("plugins/my-worklog.ts"))
    );
    assert!(!dir.path().join(".opencode/plugins/my-worklog.ts").exists());
}

#[test]
fn install_writes_plugin_and_tools() {
    let dir = tempdir().expect("tempdir");
    let options = InstallOptions {
        target_dir: dir.path().join(".opencode"),
        worklog_home: dir.path().join("worklog"),
        dry_run: false,
        force: false,
        include_tools: true,
    };

    let plan = InstallPlan::build(&options);
    let report = plan.apply(&options).expect("install");

    assert!(!report.dry_run);
    assert!(dir.path().join(".opencode/plugins/my-worklog.ts").exists());
    assert!(dir.path().join(".opencode/tools/worklog_today.ts").exists());
    assert!(dir.path().join(".opencode/tools/worklog_done.ts").exists());
    assert!(
        dir.path()
            .join(".opencode/tools/worklog_status.ts")
            .exists()
    );
    assert!(
        dir.path()
            .join(".opencode/tools/worklog_blockers.ts")
            .exists()
    );
    assert!(dir.path().join(".opencode/tools/worklog_files.ts").exists());
    assert!(
        dir.path()
            .join(".opencode/tools/worklog_commands.ts")
            .exists()
    );
    assert!(
        dir.path()
            .join(".opencode/tools/worklog_agents.ts")
            .exists()
    );

    let status_tool = std::fs::read_to_string(dir.path().join(".opencode/tools/worklog_status.ts"))
        .expect("read status tool");
    assert!(status_tool.contains("status --period today"));

    let command_tool =
        std::fs::read_to_string(dir.path().join(".opencode/tools/worklog_commands.ts"))
            .expect("read commands tool");
    let done_tool = std::fs::read_to_string(dir.path().join(".opencode/tools/worklog_done.ts"))
        .expect("read done tool");
    assert!(done_tool.contains("done --period week"));
    assert!(command_tool.contains("commands --period week"));
    assert!(!status_tool.contains("--compact"));
    assert!(!command_tool.contains("--compact"));
}

#[test]
fn install_refuses_existing_file_without_force() {
    let dir = tempdir().expect("tempdir");
    let plugin = dir.path().join(".opencode/plugins");
    std::fs::create_dir_all(&plugin).expect("plugin dir");
    std::fs::write(plugin.join("my-worklog.ts"), "existing").expect("existing file");
    let options = InstallOptions {
        target_dir: dir.path().join(".opencode"),
        worklog_home: dir.path().join("worklog"),
        dry_run: false,
        force: false,
        include_tools: false,
    };

    let plan = InstallPlan::build(&options);
    let error = plan.apply(&options).expect_err("overwrite should fail");

    assert!(error.to_string().contains("refusing to overwrite"));
}
