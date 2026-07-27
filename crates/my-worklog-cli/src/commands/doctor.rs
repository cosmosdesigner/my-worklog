use std::env;
use std::path::{Path, PathBuf};

use anyhow::{Context as AnyhowContext, Result};
use directories::BaseDirs;
use my_worklog_adapter_opencode::discover::{config_dir_from_env, default_db_path};
use my_worklog_adapter_opencode::tool_templates::tool_templates;
use rusqlite::{Connection, OpenFlags};

use crate::commands::Context;

#[derive(Debug, Clone, Copy)]
struct OpenCodeReadiness {
    plugin: bool,
    tools: bool,
    worklog_db: bool,
    import_source: bool,
}

pub fn run(context: &Context) -> Result<()> {
    let db_exists = context.paths.database().exists();
    let config_exists = context.paths.config().exists();
    let spool_exists = context.paths.spool().exists();
    if db_exists {
        check_database_readable(context.paths.database())?;
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
    println!("✓ OpenCode productized installer and import path");
    println!("✓ Spool contract source IDs: opencode, codex, claude");
    println!("! Codex and Claude are adapter contract sources, not installed integrations");
    print_opencode_readiness(context, db_exists);
    Ok(())
}

const fn mark(ok: bool) -> &'static str {
    if ok { "✓" } else { "!" }
}

fn check_database_readable(path: &Path) -> Result<()> {
    let conn =
        Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).with_context(|| {
            format!(
                "could not open worklog database read-only: {}",
                path.display()
            )
        })?;
    conn.pragma_query_value(None, "user_version", |row| row.get::<_, i32>(0))
        .with_context(|| {
            format!(
                "could not read worklog database metadata: {}",
                path.display()
            )
        })?;
    Ok(())
}

fn print_opencode_readiness(context: &Context, db_exists: bool) {
    let config_dir = opencode_config_dir();
    let plugin = config_dir.join("plugins").join("my-worklog.ts");
    let plugin_ready = plugin.exists();
    let tools_ready = tool_templates(&config_dir)
        .iter()
        .all(|file| file.path.exists());
    let import_source = default_db_path().filter(|path| path.exists());
    let opencode_readiness = OpenCodeReadiness {
        plugin: plugin_ready,
        tools: tools_ready,
        worklog_db: db_exists,
        import_source: import_source.is_some(),
    };
    let spool_parent_ready = context
        .paths
        .spool()
        .parent()
        .is_some_and(|parent| parent.exists());

    println!("\nOpenCode:");
    println!(
        "{} Plugin: {} ({})",
        mark(plugin_ready),
        readiness(plugin_ready),
        plugin.display()
    );
    println!(
        "{} Helper tools: {} ({})",
        mark(tools_ready),
        readiness(tools_ready),
        config_dir.join("tools").display()
    );
    println!(
        "{} Worklog database: {} ({})",
        mark(db_exists),
        readiness(db_exists),
        context.paths.database().display()
    );
    println!(
        "{} Spool path: {} ({})",
        mark(context.paths.spool().exists() || spool_parent_ready),
        if context.paths.spool().exists() {
            "ready"
        } else if spool_parent_ready {
            "parent ready"
        } else {
            "missing"
        },
        context.paths.spool().display()
    );
    match &import_source {
        Some(path) => println!("✓ Import source: ready ({})", path.display()),
        None => println!("! Import source: missing"),
    }
    print_opencode_actions(opencode_readiness);
}

fn print_opencode_actions(readiness: OpenCodeReadiness) {
    if readiness.plugin && readiness.tools && readiness.worklog_db && readiness.import_source {
        return;
    }
    println!("\nRecommended actions:");
    if !readiness.plugin || !readiness.tools {
        println!("- my-worklog install opencode --global");
        println!("- Restart OpenCode after installing the plugin.");
    }
    if !readiness.worklog_db {
        println!("- my-worklog init");
    }
    if !readiness.import_source {
        println!("- my-worklog import --opencode --opencode-db <path> or --opencode-export <path>");
    }
}

fn opencode_config_dir() -> PathBuf {
    if let Some(path) = config_dir_from_env() {
        return path;
    }
    if let Some(base) = BaseDirs::new() {
        return base.config_dir().join("opencode");
    }
    env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".opencode")
}

const fn readiness(ok: bool) -> &'static str {
    if ok { "ready" } else { "missing" }
}
