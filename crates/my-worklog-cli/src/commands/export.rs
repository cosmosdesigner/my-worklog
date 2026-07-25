use anyhow::{Result, bail};
use clap::{Args, Subcommand};
use my_worklog_core::WorklogDb;
use my_worklog_core::db::export::raw_events;

use crate::commands::Context;

#[derive(Debug, Args)]
pub struct ExportArgs {
    #[command(subcommand)]
    pub target: ExportTarget,
}

#[derive(Debug, Subcommand)]
pub enum ExportTarget {
    #[command(about = "Export raw stored work events")]
    Events(ExportEventsArgs),
}

#[derive(Debug, Args)]
pub struct ExportEventsArgs {
    #[arg(long, help = "Emit one raw event JSON object per line")]
    pub jsonl: bool,
}

pub fn run(context: &Context, args: &ExportArgs) -> Result<()> {
    match &args.target {
        ExportTarget::Events(events) => export_events(context, events),
    }
}

fn export_events(context: &Context, args: &ExportEventsArgs) -> Result<()> {
    if !args.jsonl {
        bail!("raw event export requires --jsonl");
    }
    let db = WorklogDb::open_existing(context.paths.database())?;
    for event in raw_events(db.connection())? {
        println!("{}", serde_json::to_string(&event)?);
    }
    Ok(())
}
