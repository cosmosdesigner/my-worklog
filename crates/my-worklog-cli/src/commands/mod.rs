use std::path::PathBuf;

use anyhow::Result;
use my_worklog_core::WorklogPaths;

pub mod cleanup;
pub mod doctor;
pub mod export;
pub mod import;
pub mod init;
pub mod install;
pub mod search;
pub mod share;
pub mod today;
pub mod week;
pub mod yesterday;

#[derive(Debug, Clone)]
pub struct Context {
    pub paths: WorklogPaths,
}

impl Context {
    pub fn resolve(home: Option<PathBuf>) -> Result<Self> {
        let paths = match home {
            Some(home) => WorklogPaths::from_home(home),
            None => WorklogPaths::resolve()?,
        };
        Ok(Self { paths })
    }
}
