use anyhow::Result;
use my_worklog_core::WorklogDb;

use crate::commands::Context;

pub fn run(context: &Context) -> Result<()> {
    let db_exists = context.paths.database().exists();
    let config_exists = context.paths.config().exists();
    let spool_exists = context.paths.spool().exists();
    if db_exists {
        WorklogDb::open_existing(context.paths.database())?;
    }
    println!("MyWorklog Doctor\n");
    println!("Core:");
    println!("✓ Home: {}", context.paths.home().display());
    println!(
        "{} Database: {}",
        mark(db_exists),
        context.paths.database().display()
    );
    println!(
        "{} Config: {}",
        mark(config_exists),
        context.paths.config().display()
    );
    println!(
        "{} Spool: {}",
        mark(spool_exists),
        context.paths.spool().display()
    );
    println!("\nAgents:");
    println!("✓ OpenCode adapter available for spool import");
    println!("✓ Codex adapter available for spool import");
    println!("✓ Claude adapter available for spool import");
    Ok(())
}

const fn mark(ok: bool) -> &'static str {
    if ok { "✓" } else { "!" }
}
