# Adapter Contract

MyWorklog accepts local JSONL spool files from coding-agent harnesses. OpenCode is the only productized installer today. Other harnesses can integrate by writing this normalized spool contract and then running `my-worklog import --spool`.

## Path Convention

Write newline-delimited JSON records to:

```text
~/.my-worklog/spool/<agent>/events.jsonl
```

Supported source IDs are `opencode`, `codex`, and `claude`. The importer infers the source ID from the `<agent>` path component when a record does not provide one.

## Record Fields

Each line is a JSON object matching the normalized spool record. All fields are optional unless your adapter needs stable IDs or specific report details.

- `source_agent` or `agent`: source ID, one of `opencode`, `codex`, or `claude`.
- `source_session_id` or `session_id`: harness session ID. If omitted, MyWorklog derives a stable ID from the file and line number.
- `source_event_id` or `event_id`: harness event ID. If omitted, MyWorklog derives the stored event ID from the normalized record contents.
- `type`: event type, such as `message`, `tool_call`, `tool_result`, `command`, or `event`.
- `role`: message role when applicable.
- `timestamp`: RFC 3339 timestamp when available.
- `cwd`: current working directory associated with the event.
- `project_root`: project root associated with the event.
- `title`: short human-readable title.
- `content`: main text payload.
- `tool_name`: tool name for tool events.
- `command`: shell or harness command text.
- `file_path`: file path associated with a file event.
- `status`: event status when applicable.
- `duration_ms`: event or step duration in milliseconds when applicable. This is per-event timing, not total session time or working time.

Example:

```json
{"source_agent":"codex","source_session_id":"session-123","source_event_id":"event-1","type":"message","role":"assistant","timestamp":"2026-07-27T10:00:00Z","cwd":"/repo","project_root":"/repo","content":"Implemented the importer test."}
```

## Local-First and Redaction

Adapters should redact secrets before writing spool records. MyWorklog also redacts common secrets while importing normalized content, commands, titles, and redacted raw JSON. Raw transcript payloads should not be stored by default.

Spool import is local-only. `my-worklog import --spool` reads local JSONL files and writes the local SQLite database; it does not call external services or LLM providers.
