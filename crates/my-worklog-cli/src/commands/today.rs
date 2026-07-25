use anyhow::Result;
use my_worklog_core::WorklogDb;
use my_worklog_core::report::daily;

use crate::commands::Context;

pub fn run(context: &Context) -> Result<()> {
    let db = WorklogDb::open_existing(context.paths.database())?;
    println!("{}", daily::today(db.connection())?);
    Ok(())
}
