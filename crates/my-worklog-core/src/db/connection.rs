use std::fs;
use std::path::Path;

use rusqlite::Connection;

use crate::db::migrations::migrate;
use crate::error::{WorklogError, WorklogResult};

#[derive(Debug)]
pub struct WorklogDb {
    conn: Connection,
}

impl WorklogDb {
    pub fn open(path: &Path) -> WorklogResult<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| WorklogError::Io {
                path: parent.display().to_string(),
                source,
            })?;
        }
        let conn = Connection::open(path)?;
        migrate(&conn)?;
        Ok(Self { conn })
    }

    pub fn open_existing(path: &Path) -> WorklogResult<Self> {
        let conn = Connection::open(path)?;
        migrate(&conn)?;
        Ok(Self { conn })
    }

    pub fn connection(&self) -> &Connection {
        &self.conn
    }
}
