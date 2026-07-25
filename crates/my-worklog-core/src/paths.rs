use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use directories::BaseDirs;

use crate::error::{WorklogError, WorklogResult};

#[derive(Debug, Clone)]
pub struct WorklogPaths {
    home: PathBuf,
    database: PathBuf,
    config: PathBuf,
    spool: PathBuf,
    reports: PathBuf,
}

impl WorklogPaths {
    pub fn resolve() -> WorklogResult<Self> {
        let home = match env::var_os("MY_WORKLOG_HOME") {
            Some(value) => PathBuf::from(value),
            None => BaseDirs::new()
                .map(|dirs| dirs.home_dir().join(".my-worklog"))
                .ok_or(WorklogError::DataDirectoryUnavailable)?,
        };
        Ok(Self::from_home(home))
    }

    pub fn from_home(home: PathBuf) -> Self {
        Self {
            database: home.join("worklog.sqlite"),
            config: home.join("config.json"),
            spool: home.join("spool"),
            reports: home.join("reports"),
            home,
        }
    }

    pub fn ensure_dirs(&self) -> WorklogResult<()> {
        for path in [
            self.home.as_path(),
            self.spool.as_path(),
            &self.spool.join("opencode"),
            &self.spool.join("codex"),
            &self.spool.join("claude"),
            self.reports.as_path(),
        ] {
            create_dir(path)?;
        }
        Ok(())
    }

    pub fn home(&self) -> &Path {
        self.home.as_path()
    }

    pub fn database(&self) -> &Path {
        self.database.as_path()
    }

    pub fn config(&self) -> &Path {
        self.config.as_path()
    }

    pub fn spool(&self) -> &Path {
        self.spool.as_path()
    }
}

fn create_dir(path: &Path) -> WorklogResult<()> {
    fs::create_dir_all(path).map_err(|source| WorklogError::Io {
        path: path.display().to_string(),
        source,
    })
}
