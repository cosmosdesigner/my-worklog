use anyhow::{Result, bail};
use clap::{Args, Subcommand};
use my_worklog_core::WorklogDb;
use my_worklog_core::db::export::raw_events;
use my_worklog_core::manual;

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
    #[command(about = "Export manually recorded work")]
    Manual(ExportManualArgs),
}

#[derive(Debug, Args)]
pub struct ExportEventsArgs {
    #[arg(long, help = "Emit one raw event JSON object per line")]
    pub jsonl: bool,
}

#[derive(Debug, Args)]
pub struct ExportManualArgs {
    #[arg(long, help = "Emit one manual entry JSON object per line")]
    pub jsonl: bool,
}

pub fn run(context: &Context, args: &ExportArgs) -> Result<()> {
    match &args.target {
        ExportTarget::Events(events) => export_events(context, events),
        ExportTarget::Manual(manual_args) => export_manual(context, manual_args),
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

fn export_manual(context: &Context, args: &ExportManualArgs) -> Result<()> {
    if !args.jsonl {
        bail!("manual export requires --jsonl");
    }
    let db = WorklogDb::open_existing(context.paths.database())?;
    let start =
        chrono::DateTime::parse_from_rfc3339("0001-01-01T00:00:00Z")?.with_timezone(&chrono::Utc);
    let end =
        chrono::DateTime::parse_from_rfc3339("9999-12-31T23:59:59Z")?.with_timezone(&chrono::Utc);
    for entry in manual::list_between(db.connection(), start, end)? {
        println!("{}", serde_json::to_string(&entry)?);
    }
    Ok(())
}
