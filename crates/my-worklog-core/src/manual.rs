use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;

use crate::error::{WorklogError, WorklogResult};

pub const CATEGORIES: &[&str] = &[
    "meeting",
    "planning",
    "manual-development",
    "code-review",
    "qa",
    "communication",
    "learning",
    "other",
];

#[derive(Debug, Clone, Serialize)]
pub struct ManualEntry {
    pub id: String,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub project: String,
    pub category: String,
    pub description: String,
    pub tags: Option<String>,
    pub work_item: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct NewManualEntry {
    pub id: String,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub project: String,
    pub category: String,
    pub description: String,
    pub tags: Option<String>,
    pub work_item: Option<String>,
}

pub fn validate_range(start: DateTime<Utc>, end: DateTime<Utc>) -> WorklogResult<()> {
    if end <= start {
        return Err(WorklogError::InvalidManualEntry(
            "end must be after start".to_string(),
        ));
    }
    Ok(())
}

pub fn validate_fields(project: &str, category: &str, description: &str) -> WorklogResult<()> {
    if project.trim().is_empty() || description.trim().is_empty() {
        return Err(WorklogError::InvalidManualEntry(
            "project and description are required".to_string(),
        ));
    }
    if !CATEGORIES.contains(&category) {
        return Err(WorklogError::InvalidManualEntry(format!(
            "category must be one of: {}",
            CATEGORIES.join(", ")
        )));
    }
    Ok(())
}

pub fn create(conn: &Connection, entry: &NewManualEntry) -> WorklogResult<()> {
    validate_range(entry.start, entry.end)?;
    validate_fields(&entry.project, &entry.category, &entry.description)?;
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO manual_entry
         (id, start_at, end_at, project, category, description, tags, work_item, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
        params![
            entry.id,
            entry.start.to_rfc3339(),
            entry.end.to_rfc3339(),
            entry.project.trim(),
            entry.category,
            entry.description.trim(),
            entry.tags,
            entry.work_item,
            now,
        ],
    )?;
    Ok(())
}

pub fn list_between(
    conn: &Connection,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> WorklogResult<Vec<ManualEntry>> {
    let mut stmt = conn.prepare(
        "SELECT id, start_at, end_at, project, category, description, tags, work_item, created_at, updated_at
         FROM manual_entry
         WHERE end_at > ?1 AND start_at < ?2
         ORDER BY start_at ASC, id ASC",
    )?;
    let rows = stmt.query_map(params![start.to_rfc3339(), end.to_rfc3339()], row_to_entry)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

pub fn get(conn: &Connection, id: &str) -> WorklogResult<Option<ManualEntry>> {
    conn.query_row(
        "SELECT id, start_at, end_at, project, category, description, tags, work_item, created_at, updated_at
         FROM manual_entry WHERE id = ?1",
        params![id],
        row_to_entry,
    )
    .optional()
    .map_err(Into::into)
}

pub fn update(conn: &Connection, entry: &NewManualEntry) -> WorklogResult<()> {
    validate_range(entry.start, entry.end)?;
    validate_fields(&entry.project, &entry.category, &entry.description)?;
    let changed = conn.execute(
        "UPDATE manual_entry
         SET start_at = ?2, end_at = ?3, project = ?4, category = ?5, description = ?6,
             tags = ?7, work_item = ?8, updated_at = ?9
         WHERE id = ?1",
        params![
            entry.id,
            entry.start.to_rfc3339(),
            entry.end.to_rfc3339(),
            entry.project.trim(),
            entry.category,
            entry.description.trim(),
            entry.tags,
            entry.work_item,
            Utc::now().to_rfc3339(),
        ],
    )?;
    if changed == 0 {
        return Err(WorklogError::ManualEntryNotFound(entry.id.clone()));
    }
    Ok(())
}

pub fn delete(conn: &Connection, id: &str) -> WorklogResult<()> {
    let changed = conn.execute("DELETE FROM manual_entry WHERE id = ?1", params![id])?;
    if changed == 0 {
        return Err(WorklogError::ManualEntryNotFound(id.to_string()));
    }
    Ok(())
}

pub fn overlapping(conn: &Connection, entry: &ManualEntry) -> WorklogResult<Vec<ManualEntry>> {
    let mut stmt = conn.prepare(
        "SELECT id, start_at, end_at, project, category, description, tags, work_item, created_at, updated_at
         FROM manual_entry
         WHERE id != ?1 AND end_at > ?2 AND start_at < ?3
         ORDER BY start_at ASC, id ASC",
    )?;
    let rows = stmt.query_map(
        params![entry.id, entry.start.to_rfc3339(), entry.end.to_rfc3339()],
        row_to_entry,
    )?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

fn row_to_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<ManualEntry> {
    let start = row.get::<_, String>(1)?.parse().map_err(|_| {
        rusqlite::Error::InvalidColumnType(1, "start_at".to_string(), rusqlite::types::Type::Text)
    })?;
    let end = row.get::<_, String>(2)?.parse().map_err(|_| {
        rusqlite::Error::InvalidColumnType(2, "end_at".to_string(), rusqlite::types::Type::Text)
    })?;
    Ok(ManualEntry {
        id: row.get(0)?,
        start,
        end,
        project: row.get(3)?,
        category: row.get(4)?,
        description: row.get(5)?,
        tags: row.get(6)?,
        work_item: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}
