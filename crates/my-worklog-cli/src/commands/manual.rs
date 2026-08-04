use anyhow::{Result, bail};
use chrono::{DateTime, Datelike, Days, Duration, Local, TimeZone, Utc};
use clap::{Args, Subcommand};
use my_worklog_core::WorklogDb;
use my_worklog_core::ingest::normalize::stable_id;
use my_worklog_core::manual::{self, CATEGORIES, NewManualEntry};

use crate::commands::{Context, PeriodArg};

#[derive(Debug, Args)]
pub struct ManualArgs {
    #[command(subcommand)]
    pub command: ManualCommand,
}

#[derive(Debug, Subcommand)]
pub enum ManualCommand {
    Add(AddArgs),
    List(ListArgs),
    Edit(EditArgs),
    Delete(DeleteArgs),
}

#[derive(Debug, Args)]
pub struct AddArgs {
    #[arg(long, help = "RFC3339 start time")]
    pub start: String,
    #[arg(long, help = "RFC3339 end time; use this or --duration")]
    pub end: Option<String>,
    #[arg(long, help = "Duration such as 2h, 90m, or 3600s; use this or --end")]
    pub duration: Option<String>,
    #[arg(long)]
    pub project: String,
    #[arg(long, value_parser = parse_category)]
    pub category: String,
    #[arg(long)]
    pub description: String,
    #[arg(long)]
    pub tags: Option<String>,
    #[arg(long = "work-item")]
    pub work_item: Option<String>,
}

#[derive(Debug, Args)]
pub struct ListArgs {
    #[arg(long, value_enum, default_value_t = PeriodArg::Today)]
    pub period: PeriodArg,
}

#[derive(Debug, Args)]
pub struct EditArgs {
    pub id: String,
    #[arg(long)]
    pub start: Option<String>,
    #[arg(long)]
    pub end: Option<String>,
    #[arg(long)]
    pub duration: Option<String>,
    #[arg(long)]
    pub project: Option<String>,
    #[arg(long, value_parser = parse_category)]
    pub category: Option<String>,
    #[arg(long)]
    pub description: Option<String>,
    #[arg(long)]
    pub tags: Option<String>,
    #[arg(long = "work-item")]
    pub work_item: Option<String>,
}

#[derive(Debug, Args)]
pub struct DeleteArgs {
    pub id: String,
}

pub fn run(context: &Context, args: &ManualArgs) -> Result<()> {
    let db = match &args.command {
        ManualCommand::Add(_) => WorklogDb::open(context.paths.database())?,
        _ => WorklogDb::open_existing(context.paths.database())?,
    };
    match &args.command {
        ManualCommand::Add(args) => add(db.connection(), args),
        ManualCommand::List(args) => list(db.connection(), args),
        ManualCommand::Edit(args) => edit(db.connection(), args),
        ManualCommand::Delete(args) => delete(db.connection(), args),
    }
}

fn add(conn: &rusqlite::Connection, args: &AddArgs) -> Result<()> {
    let start = parse_time(&args.start)?;
    let end = end_from_args(start, args.end.as_deref(), args.duration.as_deref())?;
    let id = stable_id(&format!(
        "manual:{}:{}:{}:{}",
        Utc::now(),
        start,
        end,
        args.description
    ));
    let entry = NewManualEntry {
        id: id.clone(),
        start,
        end,
        project: args.project.clone(),
        category: args.category.clone(),
        description: args.description.clone(),
        tags: args.tags.clone(),
        work_item: args.work_item.clone(),
    };
    manual::create(conn, &entry)?;
    print_created(conn, &id)?;
    Ok(())
}

fn list(conn: &rusqlite::Connection, args: &ListArgs) -> Result<()> {
    let (start, end) = period_window(args.period);
    for entry in manual::list_between(conn, start, end)? {
        println!(
            "{} {} [{}] {} — {}",
            entry.id,
            entry.start.to_rfc3339(),
            entry.category,
            entry.project,
            entry.description
        );
    }
    Ok(())
}

fn edit(conn: &rusqlite::Connection, args: &EditArgs) -> Result<()> {
    let current = manual::get(conn, &args.id)?
        .ok_or_else(|| anyhow::anyhow!("manual entry not found: {}", args.id))?;
    let start = args
        .start
        .as_deref()
        .map(parse_time)
        .transpose()?
        .unwrap_or(current.start);
    let end = if let Some(end) = &args.end {
        parse_time(end)?
    } else if let Some(duration) = &args.duration {
        start + parse_duration(duration)?
    } else {
        current.end
    };
    let entry = NewManualEntry {
        id: current.id,
        start,
        end,
        project: args.project.clone().unwrap_or(current.project),
        category: args.category.clone().unwrap_or(current.category),
        description: args.description.clone().unwrap_or(current.description),
        tags: args.tags.clone().or(current.tags),
        work_item: args.work_item.clone().or(current.work_item),
    };
    manual::update(conn, &entry)?;
    print_created(conn, &entry.id)?;
    Ok(())
}

fn delete(conn: &rusqlite::Connection, args: &DeleteArgs) -> Result<()> {
    manual::delete(conn, &args.id)?;
    println!("Deleted manual entry {}", args.id);
    Ok(())
}

fn print_created(conn: &rusqlite::Connection, id: &str) -> Result<()> {
    let entry =
        manual::get(conn, id)?.ok_or_else(|| anyhow::anyhow!("manual entry not found: {id}"))?;
    let overlaps = manual::overlapping(conn, &entry)?;
    println!(
        "Recorded manual entry {} ({} minutes).",
        entry.id,
        (entry.end - entry.start).num_minutes()
    );
    if !overlaps.is_empty() {
        println!(
            "Warning: overlaps with {} existing manual entr{}.",
            overlaps.len(),
            if overlaps.len() == 1 { "y" } else { "ies" }
        );
    }
    Ok(())
}

fn parse_time(value: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|_| anyhow::anyhow!("invalid RFC3339 timestamp: {value}"))
}

fn end_from_args(
    start: DateTime<Utc>,
    end: Option<&str>,
    duration: Option<&str>,
) -> Result<DateTime<Utc>> {
    match (end, duration) {
        (Some(_), Some(_)) => bail!("use either --end or --duration, not both"),
        (Some(end), None) => parse_time(end),
        (None, Some(duration)) => Ok(start + parse_duration(duration)?),
        (None, None) => bail!("provide either --end or --duration"),
    }
}

fn parse_duration(value: &str) -> Result<Duration> {
    let (number, suffix) = value.trim().split_at(
        value
            .trim()
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(value.len()),
    );
    let number: i64 = number
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid duration: {value}"))?;
    if number <= 0 {
        bail!("duration must be positive");
    }
    match suffix {
        "s" => Ok(Duration::seconds(number)),
        "m" => Ok(Duration::minutes(number)),
        "h" => Ok(Duration::hours(number)),
        _ => bail!("duration must end in s, m, or h"),
    }
}

fn parse_category(value: &str) -> Result<String, String> {
    if CATEGORIES.contains(&value) {
        Ok(value.to_string())
    } else {
        Err(format!(
            "category must be one of: {}",
            CATEGORIES.join(", ")
        ))
    }
}

fn period_window(period: PeriodArg) -> (DateTime<Utc>, DateTime<Utc>) {
    let now = Local::now();
    let today = now.date_naive();
    let start_date = match period {
        PeriodArg::Today => today,
        PeriodArg::Yesterday => today - Days::new(1),
        PeriodArg::Week => today - Days::new(u64::from(today.weekday().num_days_from_monday())),
    };
    let start = Local
        .from_local_datetime(&start_date.and_time(chrono::NaiveTime::MIN))
        .single()
        .unwrap_or(now);
    let end = match period {
        PeriodArg::Yesterday => Local
            .from_local_datetime(&today.and_time(chrono::NaiveTime::MIN))
            .single()
            .unwrap_or(now),
        _ => now,
    };
    (start.with_timezone(&Utc), end.with_timezone(&Utc))
}
