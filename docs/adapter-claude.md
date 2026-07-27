# Claude Adapter

Claude is a supported source ID for the local spool contract, not an installed integration today. There is no `my-worklog install claude` target.

A Claude harness can integrate by writing normalized JSONL records to `~/.my-worklog/spool/claude/events.jsonl` and then running `my-worklog import --spool`.

See `docs/adapter-contract.md` for the record fields and privacy expectations.
