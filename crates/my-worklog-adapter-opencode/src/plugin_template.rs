use std::path::Path;

pub fn plugin_template(worklog_home: &Path) -> String {
    let home = escape_js(&worklog_home.display().to_string());
    format!(
        r#"import {{ appendFileSync, mkdirSync }} from "node:fs";
import {{ dirname }} from "node:path";
import type {{ Plugin }} from "@opencode-ai/plugin";

const spool = "{home}/spool/opencode/events.jsonl";
const capturedEvents = new Set([
  "session.created",
  "session.updated",
  "session.idle",
  "session.compacted",
  "session.error",
  "message.updated",
  "file.edited",
  "todo.updated",
]);

function redact(value: unknown): unknown {{
  const text = JSON.stringify(value)
    .replace(/(OPENAI_API_KEY|ANTHROPIC_API_KEY|API_KEY|TOKEN|PASSWORD|SECRET)=([^\s"]+)/gi, "$1=[REDACTED]")
    .replace(/Authorization:\s*Bearer\s+[^\s"]+/gi, "Authorization: Bearer [REDACTED]")
    .replace(/postgres(?:ql)?:\/\/[^\s"]+/gi, "postgres://[REDACTED]")
    .replace(/eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+/g, "[REDACTED_JWT]");
  return JSON.parse(text);
}}

function write(type: string, payload: unknown) {{
  try {{
    mkdirSync(dirname(spool), {{ recursive: true }});
    appendFileSync(spool, JSON.stringify({{
      source_agent: "opencode",
      type,
      timestamp: new Date().toISOString(),
      content: JSON.stringify(redact(payload)),
    }}) + "\n");
  }} catch {{
    // Worklog capture must never block OpenCode.
  }}
}}

function captureEvent(input: {{ event?: {{ type?: string }} }}) {{
  const eventType = input.event?.type;
  if (eventType && capturedEvents.has(eventType)) {{
    write("event", input);
  }}
}}

export default (async () => {{
  return {{
    event: async (input: {{ event?: {{ type?: string }} }}) => captureEvent(input),
    "tool.execute.before": async (input: unknown) => write("tool_call", input),
    "tool.execute.after": async (input: unknown) => write("tool_result", input),
    "command.execute.before": async (input: unknown) => write("command", input),
  }};
}}) satisfies Plugin;
"#
    )
}

fn escape_js(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::plugin_template;

    #[test]
    fn plugin_uses_default_function_when_rendered() {
        let rendered = plugin_template(Path::new("/tmp/worklog"));
        assert!(rendered.contains("satisfies Plugin"));
        assert!(!rendered.contains("@opencode-ai/plugin/v2/promise"));
        assert!(rendered.contains("session.updated"));
        assert!(!rendered.contains("plugin.added"));
        assert!(rendered.contains("/tmp/worklog/spool/opencode/events.jsonl"));
    }
}
