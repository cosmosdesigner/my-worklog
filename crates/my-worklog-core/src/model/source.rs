use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::WorklogError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceAgent {
    OpenCode,
    Codex,
    Claude,
}

impl SourceAgent {
    pub const fn id(self) -> &'static str {
        match self {
            Self::OpenCode => "opencode",
            Self::Codex => "codex",
            Self::Claude => "claude",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::OpenCode => "OpenCode",
            Self::Codex => "Codex",
            Self::Claude => "Claude",
        }
    }
}

impl fmt::Display for SourceAgent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.id())
    }
}

impl FromStr for SourceAgent {
    type Err = WorklogError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "opencode" | "OpenCode" => Ok(Self::OpenCode),
            "codex" | "Codex" => Ok(Self::Codex),
            "claude" | "Claude" => Ok(Self::Claude),
            other => Err(WorklogError::InvalidSourceAgent(other.to_owned())),
        }
    }
}
