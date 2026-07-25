use std::path::PathBuf;

use anyhow::{Result, bail};
use clap::Args;
use my_worklog_adapter_opencode::discover::default_db_path;
use my_worklog_adapter_opencode::import_db::import_opencode_db;
use my_worklog_adapter_opencode::import_export::import_opencode_export;
use my_worklog_core::WorklogDb;
use my_worklog_core::db::repositories::ImportOutcome;
use my_worklog_core::ingest::spool::import_spool;
use my_worklog_core::privacy::redact::Redactor;

use crate::commands::Context;

#[derive(Debug, Args)]
pub struct ImportArgs {
    #[arg(
        long,
        help = "Import JSONL records from the my-worklog spool directory"
    )]
    pub spool: bool,
    #[arg(long, help = "Override the spool directory to import from")]
    pub spool_dir: Option<PathBuf>,
    #[arg(
        long,
        help = "Import OpenCode transcript messages from local DB or export JSON"
    )]
    pub opencode: bool,
    #[arg(long, help = "Read OpenCode transcript messages from this SQLite DB")]
    pub opencode_db: Option<PathBuf>,
    #[arg(
        long,
        help = "Read OpenCode transcript messages from this export JSON file or directory"
    )]
    pub opencode_export: Option<PathBuf>,
}

pub fn run(context: &Context, args: &ImportArgs) -> Result<()> {
    if !args.spool && !args.opencode {
        println!("Use `my-worklog import --spool` or `my-worklog import --opencode`.");
        return Ok(());
    }
    let db = WorklogDb::open(context.paths.database())?;
    let redactor = Redactor::new(Some(context.paths.home().display().to_string()))?;
    if args.spool {
        let spool = args
            .spool_dir
            .as_deref()
            .unwrap_or_else(|| context.paths.spool());
        let outcome = import_spool(db.connection(), spool, &redactor)?;
        print_outcome("events", &outcome);
    }
    if args.opencode {
        let outcome = import_opencode(context, args, db.connection(), &redactor)?;
        print_outcome("OpenCode messages", &outcome);
    }
    Ok(())
}

fn import_opencode(
    _context: &Context,
    args: &ImportArgs,
    conn: &rusqlite::Connection,
    redactor: &Redactor,
) -> Result<ImportOutcome> {
    if let Some(export_path) = &args.opencode_export {
        return Ok(import_opencode_export(conn, export_path, redactor)?);
    }
    let db_path = match &args.opencode_db {
        Some(path) => path.clone(),
        None => default_db_path().unwrap_or_default(),
    };
    if db_path.as_os_str().is_empty() || !db_path.exists() {
        bail!("No OpenCode local state found. Pass --opencode-db or --opencode-export.");
    }
    Ok(import_opencode_db(conn, &db_path, redactor)?)
}

fn print_outcome(label: &str, outcome: &ImportOutcome) {
    println!(
        "Imported {} {} ({} duplicates, {} malformed, {} skipped noise).",
        outcome.imported, label, outcome.duplicates, outcome.malformed, outcome.skipped_noise
    );
}
