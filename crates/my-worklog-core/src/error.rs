use thiserror::Error;

pub type WorklogResult<T> = Result<T, WorklogError>;

#[derive(Debug, Error)]
pub enum WorklogError {
    #[error("could not resolve a local data directory")]
    DataDirectoryUnavailable,
    #[error("I/O error at {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid source agent: {0}")]
    InvalidSourceAgent(String),
    #[error("invalid manual entry: {0}")]
    InvalidManualEntry(String),
    #[error("manual entry not found: {0}")]
    ManualEntryNotFound(String),
}
