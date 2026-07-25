use std::fs;

use serde::{Deserialize, Serialize};

use crate::error::{WorklogError, WorklogResult};
use crate::paths::WorklogPaths;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub version: u32,
    pub home: String,
    pub database: String,
    pub privacy: PrivacyConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacyConfig {
    pub store_raw_events: bool,
    pub store_redacted_raw_events: bool,
    pub redact_secrets: bool,
    pub redact_home_path: bool,
    pub ignore_paths: Vec<String>,
}

impl Config {
    pub fn default_for(paths: &WorklogPaths) -> Self {
        Self {
            version: 1,
            home: paths.home().display().to_string(),
            database: paths.database().display().to_string(),
            privacy: PrivacyConfig {
                store_raw_events: false,
                store_redacted_raw_events: true,
                redact_secrets: true,
                redact_home_path: true,
                ignore_paths: vec![
                    "**/.env".to_owned(),
                    "**/.env.*".to_owned(),
                    "**/node_modules/**".to_owned(),
                    "**/.git/**".to_owned(),
                    "**/dist/**".to_owned(),
                    "**/build/**".to_owned(),
                    "**/target/**".to_owned(),
                ],
            },
        }
    }

    pub fn write_if_missing(&self, paths: &WorklogPaths) -> WorklogResult<()> {
        if paths.config().exists() {
            return Ok(());
        }
        let json = serde_json::to_string_pretty(self)?;
        fs::write(paths.config(), json).map_err(|source| WorklogError::Io {
            path: paths.config().display().to_string(),
            source,
        })
    }
}
