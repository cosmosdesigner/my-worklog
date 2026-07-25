use std::env;
use std::path::PathBuf;

use anyhow::{Result, bail};
use clap::{Args, ValueEnum};
use directories::BaseDirs;
use my_worklog_adapter_opencode::install::{InstallOptions, InstallPlan, default_project_target};

use crate::commands::Context;

#[derive(Debug, Args)]
pub struct InstallArgs {
    #[arg(value_enum, help = "Integration target to install")]
    pub target: InstallTarget,
    #[arg(long, help = "Install into the global OpenCode config directory")]
    pub global: bool,
    #[arg(long, help = "Project root where .opencode files should be installed")]
    pub project: Option<PathBuf>,
    #[arg(long, help = "Exact OpenCode config directory to write into")]
    pub target_dir: Option<PathBuf>,
    #[arg(long, help = "Print files that would be written without writing them")]
    pub dry_run: bool,
    #[arg(
        long,
        help = "Overwrite existing integration files after creating backups"
    )]
    pub force: bool,
    #[arg(long, help = "Install only the plugin, without helper tools")]
    pub without_tools: bool,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum InstallTarget {
    #[value(name = "opencode")]
    OpenCode,
    All,
}

pub fn run(context: &Context, args: &InstallArgs) -> Result<()> {
    match args.target {
        InstallTarget::OpenCode | InstallTarget::All => install_opencode(context, args),
    }
}

fn install_opencode(context: &Context, args: &InstallArgs) -> Result<()> {
    let target_dir = resolve_target_dir(args)?;
    let options = InstallOptions {
        target_dir,
        worklog_home: context.paths.home().to_path_buf(),
        dry_run: args.dry_run,
        force: args.force,
        include_tools: !args.without_tools,
    };
    let plan = InstallPlan::build(&options);
    let report = plan.apply(&options)?;
    if report.dry_run {
        println!("OpenCode install dry-run. Files that would be written:");
        for file in &plan.files {
            println!("- {}", file.path.display());
        }
        println!(
            "\nAdd this plugin in your OpenCode config if your local plugin directory is not auto-loaded:"
        );
        println!("plugin: [\"./plugins/my-worklog.ts\"]");
        return Ok(());
    }
    println!("Installed OpenCode integration:");
    for file in report.files {
        println!("- {}", file.display());
    }
    if !report.backups.is_empty() {
        println!("\nBackups:");
        for backup in report.backups {
            println!("- {}", backup.display());
        }
    }
    Ok(())
}

fn resolve_target_dir(args: &InstallArgs) -> Result<PathBuf> {
    if let Some(target_dir) = &args.target_dir {
        return Ok(target_dir.clone());
    }
    if args.global {
        return global_opencode_dir();
    }
    let project = match &args.project {
        Some(project) => project.clone(),
        None => env::current_dir()?,
    };
    Ok(default_project_target(&project))
}

fn global_opencode_dir() -> Result<PathBuf> {
    if let Some(value) = env::var_os("OPENCODE_CONFIG_DIR") {
        return Ok(PathBuf::from(value));
    }
    let Some(base) = BaseDirs::new() else {
        bail!("could not resolve home directory for global OpenCode config")
    };
    Ok(base.config_dir().join("opencode"))
}
