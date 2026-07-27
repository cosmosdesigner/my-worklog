use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

use tempfile::{TempDir, tempdir};

pub struct CliOutput {
    pub status: ExitStatus,
    pub stdout: String,
    pub stderr: String,
}

pub struct TestEnv {
    sandbox: TempDir,
    pub home: PathBuf,
    pub opencode: PathBuf,
    pub opencode_db: PathBuf,
}

impl TestEnv {
    pub fn new() -> Self {
        let sandbox = tempdir().expect("tempdir");
        let home = sandbox.path().join("worklog-home");
        let opencode = sandbox.path().join("opencode-config");
        let opencode_db = sandbox.path().join("default-opencode.db");
        Self {
            sandbox,
            home,
            opencode,
            opencode_db,
        }
    }

    pub fn run<const N: usize>(&self, args: [&str; N]) -> CliOutput {
        self.run_with_db(&self.opencode_db, args)
    }

    pub fn run_with_db<const N: usize>(&self, opencode_db: &Path, args: [&str; N]) -> CliOutput {
        let mut command = Command::new(env!("CARGO_BIN_EXE_my-worklog"));
        command
            .env("HOME", self.sandbox.path().join("home-env"))
            .env("XDG_CONFIG_HOME", self.sandbox.path().join("xdg-config"))
            .env("XDG_DATA_HOME", self.sandbox.path().join("xdg-data"))
            .env("MY_WORKLOG_HOME", &self.home)
            .env("OPENCODE_CONFIG_DIR", &self.opencode)
            .env("OPENCODE_DB", opencode_db)
            .arg("--home")
            .arg(&self.home);
        command.args(args.iter().map(OsStr::new));
        let output = command.output().expect("run my-worklog");
        CliOutput {
            status: output.status,
            stdout: String::from_utf8(output.stdout).expect("stdout utf-8"),
            stderr: String::from_utf8(output.stderr).expect("stderr utf-8"),
        }
    }
}

pub fn assert_success(output: &CliOutput) {
    assert!(
        output.status.success(),
        "expected success, got {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        output.stdout,
        output.stderr
    );
}

pub fn assert_failure(output: &CliOutput) {
    assert!(
        !output.status.success(),
        "expected failure\nstdout:\n{}\nstderr:\n{}",
        output.stdout,
        output.stderr
    );
}

pub fn assert_stdout_contains(output: &CliOutput, needle: &str) {
    assert!(
        output.stdout.contains(needle),
        "stdout missing {needle:?}\nstdout:\n{}\nstderr:\n{}",
        output.stdout,
        output.stderr
    );
}

pub fn assert_stderr_contains(output: &CliOutput, needle: &str) {
    assert!(
        output.stderr.contains(needle),
        "stderr missing {needle:?}\nstdout:\n{}\nstderr:\n{}",
        output.stdout,
        output.stderr
    );
}
