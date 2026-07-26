# MyWorklog

`my-worklog` is a local-first developer work journal for coding-agent sessions.

Current commands include:

```bash
my-worklog init
my-worklog doctor
my-worklog import --spool
my-worklog today
my-worklog yesterday
my-worklog week
my-worklog search "database migration"
my-worklog status --period today
my-worklog decisions --period yesterday
my-worklog open-loops --period week
my-worklog blockers --period week
my-worklog files --period week
my-worklog commands --period yesterday
my-worklog agents --period week
my-worklog export events --jsonl
my-worklog share yesterday
```

## Install

Clone the repository and install the CLI binary with Cargo:

```bash
git clone git@github.com:cosmosdesigner/my-worklog.git
cd my-worklog
cargo install --path crates/my-worklog-cli
```

This installs the `my-worklog` command into Cargo's binary directory, usually
`~/.cargo/bin`. Make sure that directory is on your `PATH`:

```bash
my-worklog --help
```

Install the OpenCode integration globally so it is available from any OpenCode
project/session:

```bash
my-worklog install opencode --global
```

Preview the files before writing them:

```bash
my-worklog install opencode --global --dry-run
```

If an older installation already exists, overwrite it with timestamped backups:

```bash
my-worklog install opencode --global --force
```

Restart OpenCode after installation. The global install writes the plugin and
helper tools into OpenCode's global config directory, including:

```text
plugins/my-worklog.ts
tools/worklog_today.ts
tools/worklog_yesterday.ts
tools/worklog_week.ts
tools/worklog_status.ts
tools/worklog_decisions.ts
tools/worklog_open_loops.ts
tools/worklog_blockers.ts
tools/worklog_files.ts
tools/worklog_commands.ts
tools/worklog_agents.ts
```

Initialize and import local OpenCode history when needed:

```bash
my-worklog init
my-worklog import --opencode
my-worklog yesterday
```

It stores normalized, redacted local data in SQLite under `~/.my-worklog/` by default.
Set `MY_WORKLOG_HOME=/custom/path` to override the home directory.

Normal report commands are human-readable by default. `today`, `yesterday`, `week`, and
`search` summarize stored work events and hide raw provider payloads such as OpenCode
metadata JSON.

Local insight commands are deterministic and do not call an LLM:

```bash
my-worklog status --period today
my-worklog decisions --period today
my-worklog decisions --period yesterday
my-worklog decisions --period week
my-worklog open-loops --period week
my-worklog blockers --period week
my-worklog files --period week
my-worklog commands --period week
my-worklog agents --period week
```

`--period` supports `today`, `yesterday`, and `week`. The default is `week`.
`status` defaults to `today`; the other insight commands default to `week`.
These commands read local SQLite data, filter/group human-readable events, and
include available metrics such as event counts, total time, and token usage.
`status` gives a compact dashboard with blockers, decisions, open loops, file
activity, command activity, and source-agent counts.

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
