# Codex Adapter

Codex is a supported source ID for the local spool contract, not an installed integration today. There is no `my-worklog install codex` target.

A Codex harness can integrate by writing normalized JSONL records to `~/.my-worklog/spool/codex/events.jsonl` and then running `my-worklog import --spool`.

See `docs/adapter-contract.md` for the record fields and privacy expectations.
