# Architecture

Agent session data flows from local JSONL spool files into a normalized SQLite worklog. OpenCode is the only productized installer today; Codex and Claude can participate through the same local spool contract as source IDs.

The default spool path is `~/.my-worklog/spool/<agent>/events.jsonl`, where `<agent>` is `opencode`, `codex`, or `claude`. Import normalizes records, redacts common secrets, and stores reportable events in SQLite.

Reports and search read only from the local database. Normal report commands do not call external services.
