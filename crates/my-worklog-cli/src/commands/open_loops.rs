use anyhow::Result;
use clap::Args as ClapArgs;
use my_worklog_core::WorklogDb;
use my_worklog_core::report::insights;

use crate::commands::{Context, PeriodArg};

#[derive(Debug, ClapArgs)]
pub struct Args {
    #[arg(long, value_enum, default_value_t = PeriodArg::Week)]
    pub period: PeriodArg,
}

pub fn run(context: &Context, args: &Args) -> Result<()> {
    let db = WorklogDb::open_existing(context.paths.database())?;
    println!(
        "{}",
        insights::open_loops(db.connection(), args.period.into())?
    );
    Ok(())
}
