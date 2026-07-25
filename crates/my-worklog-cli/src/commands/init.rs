use anyhow::Result;
use my_worklog_core::{Config, WorklogDb};

use crate::commands::Context;

pub fn run(context: &Context) -> Result<()> {
    context.paths.ensure_dirs()?;
    Config::default_for(&context.paths).write_if_missing(&context.paths)?;
    WorklogDb::open(context.paths.database())?;
    println!(
        "Initialized MyWorklog at {}",
        context.paths.home().display()
    );
    Ok(())
}
