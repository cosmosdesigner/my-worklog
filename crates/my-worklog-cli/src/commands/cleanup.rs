use anyhow::Result;
use clap::{Args, ValueEnum};
use my_worklog_core::WorklogDb;
use my_worklog_core::db::cleanup::cleanup_opencode_noise;

use crate::commands::Context;

#[derive(Debug, Args)]
pub struct CleanupArgs {
    #[arg(value_enum, help = "Cleanup target to run")]
    pub target: CleanupTarget,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum CleanupTarget {
    #[value(name = "opencode-noise")]
    OpenCodeNoise,
}

pub fn run(context: &Context, args: &CleanupArgs) -> Result<()> {
    match args.target {
        CleanupTarget::OpenCodeNoise => {
            let db = WorklogDb::open_existing(context.paths.database())?;
            let deleted = cleanup_opencode_noise(db.connection())?;
            println!("Removed {deleted} noisy OpenCode events.");
            Ok(())
        }
    }
}
