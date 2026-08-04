use rusqlite::Connection;

use crate::db::schema::{CURRENT_VERSION, FTS_SQL, SCHEMA_SQL};
use crate::error::WorklogResult;

pub fn migrate(conn: &Connection) -> WorklogResult<()> {
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.busy_timeout(std::time::Duration::from_millis(5_000))?;
    let version: i32 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if version < CURRENT_VERSION {
        conn.execute_batch(SCHEMA_SQL)?;
    }
    if version < 2 {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS manual_entry (
                id TEXT PRIMARY KEY,
                start_at TEXT NOT NULL,
                end_at TEXT NOT NULL,
                project TEXT NOT NULL,
                category TEXT NOT NULL,
                description TEXT NOT NULL,
                tags TEXT,
                work_item TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_manual_entry_start_at ON manual_entry(start_at);
            PRAGMA user_version = 2;",
        )?;
    }
    if conn.execute_batch(FTS_SQL).is_err() {
        tracing::warn!("SQLite FTS5 unavailable; search will use LIKE fallback");
    }
    Ok(())
}
