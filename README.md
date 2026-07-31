# MyWorklog

MyWorklog turns your OpenCode sessions into a private work memory you can ask about.

Instead of digging through chat transcripts, terminal scrollback, and half-remembered task notes, install the OpenCode plugin and ask plain questions like:

```text
What did I do today?
What shipped this week?
What blockers came up?
What decisions did we make yesterday?
What files changed during the refactor?
What commands did I run while debugging?
```

The plugin records local, redacted work events while you code. The CLI imports those events into a local SQLite database and produces deterministic reports for daily standups, weekly reviews, handoffs, and status checks.

## Why It Helps

Coding-agent work moves fast. Important context often lives across prompts, tool calls, file edits, commands, and agent summaries. MyWorklog gives that work a durable shape:

- See what happened today, yesterday, or this week.
- Pull out completed work, blockers, decisions, and open loops.
- Review files, commands, and source-agent activity without reading raw transcripts.
- Give OpenCode compact answers that are short enough to paste into a chat.
- Turn a local report into a manager or client update only when you ask for it.

## Supported Integrations

- OpenCode: productized installer, plugin, helper tools, and import path are available today.
- Codex and Claude: supported source IDs for the local spool contract, but not install-ready integrations. There is no `install codex` or `install claude` target.
- Generic spool contract: any harness can write normalized JSONL to `~/.my-worklog/spool/<agent>/events.jsonl` and use `my-worklog import --spool`. See `docs/adapter-contract.md`.

## What the OpenCode Plugin Does

The OpenCode integration installs a plugin plus helper tools into OpenCode's global config directory. During OpenCode sessions, the plugin writes local redacted events to:

```text
~/.my-worklog/spool/opencode/events.jsonl
```

Then `my-worklog import --opencode` imports local OpenCode history into SQLite under `~/.my-worklog/` by default. Set `MY_WORKLOG_HOME=/custom/path` to use a different home directory.

Helper tools installed for OpenCode include:

```text
plugins/my-worklog.ts
tools/worklog_today.ts
tools/worklog_yesterday.ts
tools/worklog_week.ts
tools/worklog_status.ts
tools/worklog_done.ts
tools/worklog_decisions.ts
tools/worklog_open_loops.ts
tools/worklog_blockers.ts
tools/worklog_files.ts
tools/worklog_commands.ts
tools/worklog_agents.ts
```

`worklog_status` and `worklog_done` support compact answers for OpenCode, so questions like "what happened this week?" or "what got done today?" can stay brief.

## Local-First Privacy

Normal reports stay local and deterministic. Commands such as `today`, `week`, `status`, `done`, `decisions`, `open-loops`, `blockers`, `files`, `commands`, and `agents` read from local SQLite and don't call an LLM.

Raw event export is explicit:

```bash
my-worklog export events --jsonl
```

LLM wording is opt-in through `share`. `share` sends the already human-readable report text, not raw provider payloads, to the selected provider. Use it when you want a polished manager update, client note, or external summary.

## Quickstart

Clone the repository and install the CLI binary with Cargo:

```bash
git clone git@github.com:cosmosdesigner/my-worklog.git
cd my-worklog
cargo install --path crates/my-worklog-cli
```

This installs `my-worklog` into Cargo's binary directory, usually `~/.cargo/bin`. Make sure that directory is on your `PATH`:

```bash
my-worklog --help
```

Install the OpenCode integration globally so it is available from any OpenCode project or session:

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

Restart OpenCode after installation so the plugin and helper tools load. Then initialize MyWorklog, import local OpenCode history, and try a compact weekly status:

```bash
my-worklog init
my-worklog import --opencode
my-worklog status --period week --compact
```

Normal onboarding is local. The plugin records redacted events on disk, `my-worklog init` creates local state, and `my-worklog import --opencode` reads local OpenCode history. None of those steps call an LLM.

## Troubleshooting OpenCode Setup

- Install refuses to overwrite a file: rerun with `my-worklog install opencode --global --force` to create timestamped backups before replacing existing plugin or tool files. Use `my-worklog install opencode --global --dry-run` first if you want to preview the paths.
- OpenCode uses a different global config path: set `OPENCODE_CONFIG_DIR=/path/to/opencode` before running `my-worklog install opencode --global`, or pass `my-worklog install opencode --target-dir /path/to/opencode` for one install. `my-worklog doctor` prints the config directory it checks.
- OpenCode history is missing: run `my-worklog doctor`. If it reports a missing import source, pass the exact source with `my-worklog import --opencode --opencode-db <path>` or `my-worklog import --opencode --opencode-export <path>`.
- Import says zero OpenCode messages were imported: duplicates may already be in SQLite, or the session may contain only skipped noise. Open OpenCode, start a real session, then run `my-worklog import --opencode` again and check `my-worklog status --period week --compact`.
- LLM privacy boundary: normal onboarding and report commands don't call an LLM. Only `my-worklog share <period>` sends the generated report text to the selected provider, and `my-worklog share <period> --print-prompt` shows that text before sending.

## Daily Use

Use period reports for quick recall:

```bash
my-worklog today
my-worklog yesterday
my-worklog week
my-worklog search "database migration"
```

Use focused insight commands when you need a specific angle:

```bash
my-worklog status --period today
my-worklog status --period week --compact
my-worklog done --period week
my-worklog done --period week --compact
my-worklog decisions --period today
my-worklog decisions --period yesterday
my-worklog decisions --period week
my-worklog open-loops --period week
my-worklog blockers --period week
my-worklog files --period week
my-worklog commands --period week
my-worklog agents --period week
```

`--period` supports `today`, `yesterday`, and `week`. The default is `week`. `status` defaults to `today`; the other insight commands default to `week`.

These commands filter and group human-readable events, then include available metrics such as event counts, captured agent-session time, and token usage. Captured agent-session time is the time recorded in the session, not total work time. `done` answers what completed in the selected period. `status` gives a dashboard with blockers, decisions, open loops, completed work, file activity, command activity, and source-agent counts. Add `--compact` to `status` or `done` for shorter top-item lists and truncated bullets suitable for OpenCode answers.

When a period contains work from multiple captured project roots, local period and insight reports add a `Projects` summary and group the worked events under project subsections. Project labels come from captured project metadata or the basename of the captured root/cwd; MyWorklog does not scan parent folders or list sibling repositories that had no captured events.

## CLI Reference

Common commands:

```bash
my-worklog init
my-worklog doctor
my-worklog import --spool
my-worklog import --opencode
my-worklog today
my-worklog yesterday
my-worklog week
my-worklog search "database migration"
my-worklog status --period week --compact
my-worklog done --period week --compact
my-worklog decisions --period yesterday
my-worklog open-loops --period week
my-worklog blockers --period week
my-worklog files --period week
my-worklog commands --period yesterday
my-worklog agents --period week
my-worklog export events --jsonl
my-worklog share yesterday
```

Every command exposes CLI help:

```bash
my-worklog --help
my-worklog import --help
my-worklog share --help
```

## Optional LLM Share Reports

Normal report commands don't call an LLM. When you want polished wording, configure a provider and call `share`.

Recommended OpenAI setup:

```bash
export OPENAI_API_KEY="..."
my-worklog share yesterday
```

The default provider is OpenAI with `gpt-5.6`, chosen for concise report writing. DeepSeek is available through its OpenAI-compatible chat API with `deepseek-v4-pro`:

```bash
export DEEPSEEK_API_KEY="..."
my-worklog share yesterday --provider deep-seek
```

Defaults can be changed when needed:

```bash
my-worklog share today --audience client
my-worklog share week --provider open-ai --model gpt-5.6
my-worklog share yesterday --provider deep-seek --model deepseek-v4-pro
```

Before spending LLM tokens, preview the exact prompt that would be sent:

```bash
my-worklog share yesterday --print-prompt
my-worklog share today --provider deep-seek --audience client --print-prompt
```
