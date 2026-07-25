# Architecture

Agent session data flows from local JSONL spool files into a normalized SQLite worklog.
Reports and search read only from the local database.
