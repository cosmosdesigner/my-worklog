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
    if conn.execute_batch(FTS_SQL).is_err() {
        tracing::warn!("SQLite FTS5 unavailable; search will use LIKE fallback");
    }
    Ok(())
}
