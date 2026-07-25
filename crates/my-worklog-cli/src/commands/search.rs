use anyhow::Result;
use clap::Args;
use my_worklog_core::WorklogDb;
use my_worklog_core::search::fts;

use crate::commands::Context;

#[derive(Debug, Args)]
pub struct SearchArgs {
    #[arg(help = "Text to search for in human-readable work events")]
    pub query: String,
}

pub fn run(context: &Context, args: &SearchArgs) -> Result<()> {
    let db = WorklogDb::open_existing(context.paths.database())?;
    println!("{}", fts::search_markdown(db.connection(), &args.query)?);
    Ok(())
}
