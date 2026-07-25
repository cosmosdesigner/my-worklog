# Agent Notes

This repository is a Rust workspace for `my-worklog`.

- Keep adapter-specific assumptions isolated in adapter crates.
- Do not upload transcripts or call external LLMs in v1.
- Do not store raw transcript payloads by default.
- Prefer deterministic, testable report generation.
- When the user asks natural-language questions about their work history inside OpenCode, such as "what did I do yesterday?", "what did I do today?", "what did I do this week?", or asks for a status/update for a specific supported period, answer directly by using local `my-worklog` data for that period. Use the matching local report command (`my-worklog <period>`) and summarize it yourself without inventing details. Do not call DeepSeek/OpenAI for normal in-session answers.
- Use the share-style LLM report (`my-worklog share <period> --provider deep-seek` or another provider) only when the user explicitly asks for an external/shareable report, boss/client update, polished message, or otherwise wants LLM-generated wording outside OpenCode. Supported periods currently include `today`, `yesterday`, and `week`.
