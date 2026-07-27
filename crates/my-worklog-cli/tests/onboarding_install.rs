use std::fs;

mod common;

use common::{
    TestEnv, assert_failure, assert_stderr_contains, assert_stdout_contains, assert_success,
};

#[test]
fn install_dry_run_describes_files_tools_and_next_steps() {
    // Given: isolated my-worklog and OpenCode config directories.
    let env = TestEnv::new();
    let target_dir = env.opencode.to_str().expect("utf-8 path");

    // When: OpenCode install is previewed without writing files.
    let output = env.run([
        "install",
        "opencode",
        "--target-dir",
        target_dir,
        "--dry-run",
    ]);

    // Then: stdout is an onboarding contract, not just a raw file list.
    assert_success(&output);
    assert_stdout_contains(&output, "OpenCode install dry-run");
    assert_stdout_contains(&output, "plugins/my-worklog.ts");
    assert_stdout_contains(&output, "tools/worklog_status.ts");
    assert_stdout_contains(&output, "Helper tools: included");
    assert_stdout_contains(&output, "Next steps:");
    assert_stdout_contains(&output, "my-worklog init");
    assert_stdout_contains(&output, "Restart OpenCode");
    assert_stdout_contains(&output, "my-worklog import --opencode");
    assert_stdout_contains(&output, "my-worklog status --period week --compact");
}

#[test]
fn install_all_dry_run_means_all_currently_implemented_install_targets() {
    // Given: only the OpenCode installer is productized today.
    let env = TestEnv::new();
    let target_dir = env.opencode.to_str().expect("utf-8 path");

    // When: the aggregate install target is previewed.
    let output = env.run(["install", "all", "--target-dir", target_dir, "--dry-run"]);

    // Then: all means implemented install targets, not universal harness support.
    assert_success(&output);
    assert_stdout_contains(&output, "OpenCode install dry-run");
    assert_stdout_contains(&output, "plugins/my-worklog.ts");
    assert!(!output.stdout.contains("Codex"));
    assert!(!output.stdout.contains("Claude"));
}

#[test]
fn install_success_describes_written_files_and_next_steps() {
    // Given: an empty target OpenCode config directory in a sandbox.
    let env = TestEnv::new();
    let target_dir = env.opencode.to_str().expect("utf-8 path");

    // When: OpenCode install writes plugin and helper tools.
    let output = env.run(["install", "opencode", "--target-dir", target_dir]);

    // Then: the success copy explains what was installed and how to use it.
    assert_success(&output);
    assert_stdout_contains(&output, "Installed OpenCode integration");
    assert_stdout_contains(&output, "plugins/my-worklog.ts");
    assert_stdout_contains(&output, "tools/worklog_status.ts");
    assert_stdout_contains(&output, "Helper tools: included");
    assert_stdout_contains(&output, "Next steps:");
    assert_stdout_contains(&output, "my-worklog init");
    assert_stdout_contains(&output, "Restart OpenCode");
    assert_stdout_contains(&output, "my-worklog import --opencode");
    assert_stdout_contains(&output, "my-worklog status --period week --compact");
}

#[test]
fn install_existing_file_refusal_points_to_force_without_overwrite() {
    // Given: an existing plugin file that should not be overwritten by default.
    let env = TestEnv::new();
    let target_dir = env.opencode.to_str().expect("utf-8 path");
    let plugin = env.opencode.join("plugins/my-worklog.ts");
    fs::create_dir_all(plugin.parent().expect("plugin parent")).expect("plugin dir");
    fs::write(&plugin, "existing plugin").expect("existing plugin");

    // When: OpenCode install is run without --force.
    let output = env.run(["install", "opencode", "--target-dir", target_dir]);

    // Then: the refusal is actionable and the existing plugin remains intact.
    assert_failure(&output);
    assert_stderr_contains(&output, "refusing to overwrite existing file");
    assert_stderr_contains(&output, "Use --force to create timestamped backups");
    assert_eq!(
        fs::read_to_string(&plugin).expect("read plugin"),
        "existing plugin"
    );
}

#[test]
fn install_force_describes_backup_paths_and_next_steps() {
    // Given: an existing plugin file and --force installation.
    let env = TestEnv::new();
    let target_dir = env.opencode.to_str().expect("utf-8 path");
    let plugin = env.opencode.join("plugins/my-worklog.ts");
    fs::create_dir_all(plugin.parent().expect("plugin parent")).expect("plugin dir");
    fs::write(&plugin, "existing plugin").expect("existing plugin");

    // When: OpenCode install overwrites with backups enabled.
    let output = env.run(["install", "opencode", "--target-dir", target_dir, "--force"]);

    // Then: the user sees the backup and the same onboarding next steps.
    assert_success(&output);
    assert_stdout_contains(&output, "Backup created:");
    assert_stdout_contains(&output, "my-worklog.ts.bak.");
    assert_stdout_contains(&output, "Next steps:");
    assert_stdout_contains(&output, "my-worklog import --opencode");
}

#[test]
fn install_without_tools_says_helper_tools_were_skipped() {
    // Given: an isolated target where only the plugin should be installed.
    let env = TestEnv::new();
    let target_dir = env.opencode.to_str().expect("utf-8 path");

    // When: OpenCode install is run with --without-tools.
    let output = env.run([
        "install",
        "opencode",
        "--target-dir",
        target_dir,
        "--without-tools",
    ]);

    // Then: the copy does not imply helper tools were installed.
    assert_success(&output);
    assert_stdout_contains(&output, "plugins/my-worklog.ts");
    assert_stdout_contains(&output, "Helper tools: skipped (--without-tools)");
    assert!(!output.stdout.contains("tools/worklog_status.ts"));
}
