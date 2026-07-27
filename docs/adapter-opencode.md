# OpenCode Adapter

OpenCode is the productized integration today. `my-worklog install opencode` writes the OpenCode plugin and helper tools, and `my-worklog install all` currently expands to that implemented OpenCode installer only.

The plugin writes redacted local events to `~/.my-worklog/spool/opencode/events.jsonl`. `my-worklog import --opencode` can also import local OpenCode history from the discovered OpenCode database or from an explicit OpenCode export path.

See `docs/adapter-contract.md` for the shared local spool JSONL contract.
