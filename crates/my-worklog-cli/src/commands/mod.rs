use std::path::PathBuf;

use anyhow::Result;
use clap::ValueEnum;
use my_worklog_core::WorklogPaths;
use my_worklog_core::report::insights::ReportPeriod;

pub mod agents;
pub mod blockers;
pub mod cleanup;
pub mod command_activity;
pub mod decisions;
pub mod doctor;
pub mod done;
pub mod export;
pub mod files;
pub mod import;
pub mod init;
pub mod install;
pub mod open_loops;
pub mod search;
pub mod share;
pub mod status;
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

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum PeriodArg {
    Today,
    Yesterday,
    Week,
}

impl From<PeriodArg> for ReportPeriod {
    fn from(value: PeriodArg) -> Self {
        match value {
            PeriodArg::Today => Self::Today,
            PeriodArg::Yesterday => Self::Yesterday,
            PeriodArg::Week => Self::Week,
        }
    }
}
