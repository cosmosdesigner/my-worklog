pub const CURRENT_VERSION: i32 = 2;

pub const SCHEMA_SQL: &str = r#"
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS source_agent (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  version TEXT,
  source_type TEXT NOT NULL,
  created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS project (
  id TEXT PRIMARY KEY,
  root_path TEXT NOT NULL UNIQUE,
  name TEXT,
  git_remote TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS work_session (
  id TEXT PRIMARY KEY,
  source_agent_id TEXT NOT NULL,
  source_session_id TEXT NOT NULL,
  project_id TEXT,
  title TEXT,
  started_at TEXT,
  ended_at TEXT,
  last_seen_at TEXT,
  cwd TEXT,
  git_branch TEXT,
  git_commit_start TEXT,
  git_commit_end TEXT,
  status TEXT,
  summary TEXT,
  raw_ref TEXT,
  imported_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  UNIQUE(source_agent_id, source_session_id),
  FOREIGN KEY(source_agent_id) REFERENCES source_agent(id),
  FOREIGN KEY(project_id) REFERENCES project(id)
);

CREATE TABLE IF NOT EXISTS work_event (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  source_agent_id TEXT NOT NULL,
  source_event_id TEXT,
  type TEXT NOT NULL,
  role TEXT,
  timestamp TEXT,
  cwd TEXT,
  title TEXT,
  content TEXT,
  normalized_content TEXT,
  tool_name TEXT,
  command TEXT,
  file_path TEXT,
  status TEXT,
  duration_ms INTEGER,
  raw_json TEXT,
  redacted INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  FOREIGN KEY(session_id) REFERENCES work_session(id)
);

CREATE TABLE IF NOT EXISTS file_activity (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  project_id TEXT,
  file_path TEXT NOT NULL,
  action TEXT NOT NULL,
  language TEXT,
  lines_added INTEGER,
  lines_removed INTEGER,
  timestamp TEXT,
  created_at TEXT NOT NULL,
  FOREIGN KEY(session_id) REFERENCES work_session(id),
  FOREIGN KEY(project_id) REFERENCES project(id)
);

CREATE TABLE IF NOT EXISTS command_activity (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  project_id TEXT,
  command TEXT NOT NULL,
  category TEXT,
  exit_code INTEGER,
  status TEXT,
  duration_ms INTEGER,
  timestamp TEXT,
  created_at TEXT NOT NULL,
  FOREIGN KEY(session_id) REFERENCES work_session(id),
  FOREIGN KEY(project_id) REFERENCES project(id)
);

CREATE TABLE IF NOT EXISTS work_decision (
  id TEXT PRIMARY KEY,
  session_id TEXT,
  project_id TEXT,
  decision TEXT NOT NULL,
  rationale TEXT,
  confidence REAL,
  timestamp TEXT,
  created_at TEXT NOT NULL,
  FOREIGN KEY(session_id) REFERENCES work_session(id),
  FOREIGN KEY(project_id) REFERENCES project(id)
);

CREATE TABLE IF NOT EXISTS work_todo (
  id TEXT PRIMARY KEY,
  session_id TEXT,
  project_id TEXT,
  task TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'open',
  source TEXT,
  timestamp TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY(session_id) REFERENCES work_session(id),
  FOREIGN KEY(project_id) REFERENCES project(id)
);

CREATE TABLE IF NOT EXISTS work_blocker (
  id TEXT PRIMARY KEY,
  session_id TEXT,
  project_id TEXT,
  blocker TEXT NOT NULL,
  evidence TEXT,
  status TEXT NOT NULL DEFAULT 'open',
  timestamp TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY(session_id) REFERENCES work_session(id),
  FOREIGN KEY(project_id) REFERENCES project(id)
);

CREATE TABLE IF NOT EXISTS summary (
  id TEXT PRIMARY KEY,
  scope TEXT NOT NULL,
  scope_key TEXT NOT NULL,
  date_start TEXT NOT NULL,
  date_end TEXT NOT NULL,
  title TEXT,
  markdown TEXT NOT NULL,
  json TEXT,
  generated_by TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS raw_import (
  id TEXT PRIMARY KEY,
  source_agent_id TEXT NOT NULL,
  source_ref TEXT NOT NULL,
  source_hash TEXT NOT NULL,
  imported_at TEXT NOT NULL,
  status TEXT NOT NULL,
  error TEXT,
  UNIQUE(source_agent_id, source_ref, source_hash)
);

CREATE INDEX IF NOT EXISTS idx_work_session_started_at ON work_session(started_at);
CREATE INDEX IF NOT EXISTS idx_work_event_timestamp ON work_event(timestamp);
CREATE INDEX IF NOT EXISTS idx_work_event_content ON work_event(normalized_content);
CREATE INDEX IF NOT EXISTS idx_work_event_source_agent ON work_event(source_agent_id);

INSERT OR IGNORE INTO source_agent (id, name, version, source_type, created_at)
VALUES
  ('opencode', 'OpenCode', NULL, 'coding_agent', datetime('now')),
  ('codex', 'Codex', NULL, 'coding_agent', datetime('now')),
  ('claude', 'Claude Code', NULL, 'coding_agent', datetime('now'));

PRAGMA user_version = 1;
"#;

pub const FTS_SQL: &str = r#"
CREATE VIRTUAL TABLE IF NOT EXISTS search_index USING fts5(
  entity_type,
  entity_id,
  title,
  body,
  project_path,
  source_agent,
  timestamp
);
"#;
