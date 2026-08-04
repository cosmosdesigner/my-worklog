mod commands;

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing::Level;
use tracing_subscriber::EnvFilter;

use crate::commands::Context;

#[derive(Debug, Parser)]
#[command(
    name = "my-worklog",
    version,
    about = "Local-first coding-agent work journal",
    long_about = "Local-first coding-agent work journal. Normal commands are human-readable; raw provider data is available through explicit export commands."
)]
struct Cli {
    #[arg(
        long,
        global = true,
        env = "MY_WORKLOG_HOME",
        help = "Override the my-worklog home directory"
    )]
    home: Option<PathBuf>,
    #[arg(short, long, action = clap::ArgAction::Count, global = true, help = "Increase log verbosity (-v, -vv, -vvv)")]
    verbose: u8,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    #[command(about = "Initialize local my-worklog storage")]
    Init,
    #[command(about = "Check local my-worklog configuration and database health")]
    Doctor,
    #[command(about = "Clean imported noise from local storage")]
    Cleanup(commands::cleanup::CleanupArgs),
    #[command(about = "Export raw stored data explicitly")]
    Export(commands::export::ExportArgs),
    #[command(about = "Install coding-agent integrations")]
    Install(commands::install::InstallArgs),
    #[command(about = "Import captured work events or agent transcripts")]
    Import(commands::import::ImportArgs),
    #[command(about = "Show today's captured work events")]
    Today,
    #[command(about = "Show yesterday's captured work events")]
    Yesterday,
    #[command(about = "Show this week's captured work events")]
    Week,
    #[command(about = "Search human-readable work events")]
    Search(commands::search::SearchArgs),
    #[command(about = "Use an LLM to turn a report into shareable prose")]
    Share(commands::share::ShareArgs),
    #[command(about = "Show a compact local status dashboard")]
    Status(commands::status::Args),
    #[command(about = "Show completed captured work events")]
    Done(commands::done::Args),
    #[command(about = "Show decisions found in local work events")]
    Decisions(commands::decisions::Args),
    #[command(
        name = "open-loops",
        about = "Show open loops found in local work events"
    )]
    OpenLoops(commands::open_loops::Args),
    #[command(about = "Show blockers found in local work events")]
    Blockers(commands::blockers::Args),
    #[command(about = "Show file activity found in local work events")]
    Files(commands::files::Args),
    #[command(about = "Show command activity found in local work events")]
    Commands(commands::command_activity::Args),
    #[command(about = "Show source-agent activity counts")]
    Agents(commands::agents::Args),
    #[command(about = "Manage manually recorded work")]
    Manual(commands::manual::ManualArgs),
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    init_tracing(cli.verbose);
    let context = Context::resolve(cli.home)?;
    match cli.command {
        Command::Init => commands::init::run(&context),
        Command::Doctor => commands::doctor::run(&context),
        Command::Cleanup(args) => commands::cleanup::run(&context, &args),
        Command::Export(args) => commands::export::run(&context, &args),
        Command::Install(args) => commands::install::run(&context, &args),
        Command::Import(args) => commands::import::run(&context, &args),
        Command::Today => commands::today::run(&context),
        Command::Yesterday => commands::yesterday::run(&context),
        Command::Week => commands::week::run(&context),
        Command::Search(args) => commands::search::run(&context, &args),
        Command::Share(args) => commands::share::run(&context, &args),
        Command::Status(args) => commands::status::run(&context, &args),
        Command::Done(args) => commands::done::run(&context, &args),
        Command::Decisions(args) => commands::decisions::run(&context, &args),
        Command::OpenLoops(args) => commands::open_loops::run(&context, &args),
        Command::Blockers(args) => commands::blockers::run(&context, &args),
        Command::Files(args) => commands::files::run(&context, &args),
        Command::Commands(args) => commands::command_activity::run(&context, &args),
        Command::Agents(args) => commands::agents::run(&context, &args),
        Command::Manual(args) => commands::manual::run(&context, &args),
    }
}

fn init_tracing(verbose: u8) {
    let level = match verbose {
        0 => Level::WARN,
        1 => Level::INFO,
        2 => Level::DEBUG,
        _ => Level::TRACE,
    };
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(format!("my_worklog={level}")));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .without_time()
        .compact()
        .with_writer(std::io::stderr)
        .init();
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::Cli;

    #[test]
    fn report_commands_describe_captured_work_events() {
        let command = Cli::command();
        for name in ["today", "yesterday", "week", "done"] {
            let about = command
                .find_subcommand(name)
                .expect("subcommand")
                .get_about()
                .expect("about")
                .to_string();
            assert!(about.contains("captured work events"));
            assert!(!about.contains("human-readable work report"));
        }
    }
}
