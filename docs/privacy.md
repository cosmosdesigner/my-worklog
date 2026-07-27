# Privacy

MyWorklog is local-first. OpenCode installation, spool import, OpenCode import, reports, and search read and write local files only.

The spool pipeline redacts common secrets before storing normalized content, titles, commands, and redacted raw event JSON. Adapter authors should also redact before writing `~/.my-worklog/spool/<agent>/events.jsonl`, because spool files live on disk before import.

Raw transcript payloads are not stored by default. LLM calls happen only when a user explicitly runs a share command for polished external wording.
