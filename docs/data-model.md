# Data Model

MyWorklog stores source, project, session, event, activity, decision, todo, blocker, summary, manual entry, raw import, and optional FTS tables through embedded SQLite migrations. Manual entries are independently persisted and auditable; imported work events remain immutable.
