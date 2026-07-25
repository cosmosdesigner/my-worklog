# MyWorklog

`my-worklog` is a local-first developer work journal for coding-agent sessions.

Phase 1 implements:

```bash
my-worklog init
my-worklog doctor
my-worklog import --spool
my-worklog today
my-worklog yesterday
my-worklog week
my-worklog search "database migration"
my-worklog export events --jsonl
my-worklog share yesterday
```

It stores normalized, redacted local data in SQLite under `~/.my-worklog/` by default.
Set `MY_WORKLOG_HOME=/custom/path` to override the home directory.

Normal report commands are human-readable by default. `today`, `yesterday`, `week`, and
`search` summarize stored work events and hide raw provider payloads such as OpenCode
metadata JSON.

Raw event data remains available explicitly for debugging or machine export:

```bash
my-worklog export events --jsonl
```

LLM summaries are opt-in through `share`. Normal reports stay local and deterministic.
`share` sends the already human-readable report text, not raw provider payloads, to the
selected provider.

Recommended setup:

```bash
export OPENAI_API_KEY="..."
my-worklog share yesterday
```

The default provider is OpenAI with `gpt-5.6`, chosen for high-quality concise writing
when turning a worklog into a manager/client update. DeepSeek is available through its
OpenAI-compatible chat API with `deepseek-v4-pro`:

```bash
export DEEPSEEK_API_KEY="..."
my-worklog share yesterday --provider deep-seek
```

Defaults can be overridden when needed:

```bash
my-worklog share today --audience client
my-worklog share week --provider open-ai --model gpt-5.6
my-worklog share yesterday --provider deep-seek --model deepseek-v4-pro
```

Every command exposes CLI help:

```bash
my-worklog --help
my-worklog import --help
my-worklog share --help
```

Before spending LLM tokens, preview the exact prompt that would be sent:

```bash
my-worklog share yesterday --print-prompt
my-worklog share today --provider deep-seek --audience client --print-prompt
```
