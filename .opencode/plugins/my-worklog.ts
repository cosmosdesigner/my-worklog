import { appendFileSync, mkdirSync } from "node:fs";
import { dirname } from "node:path";
import type { Plugin } from "@opencode-ai/plugin";

const spool = "/Users/carlosalmeida/.my-worklog/spool/opencode/events.jsonl";
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

function redact(value: unknown): unknown {
  const text = JSON.stringify(value)
    .replace(/(OPENAI_API_KEY|ANTHROPIC_API_KEY|API_KEY|TOKEN|PASSWORD|SECRET)=([^\s"]+)/gi, "$1=[REDACTED]")
    .replace(/Authorization:\s*Bearer\s+[^\s"]+/gi, "Authorization: Bearer [REDACTED]")
    .replace(/postgres(?:ql)?:\/\/[^\s"]+/gi, "postgres://[REDACTED]")
    .replace(/eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+/g, "[REDACTED_JWT]");
  return JSON.parse(text);
}

function write(type: string, payload: unknown) {
  try {
    mkdirSync(dirname(spool), { recursive: true });
    appendFileSync(spool, JSON.stringify({
      source_agent: "opencode",
      type,
      timestamp: new Date().toISOString(),
      content: JSON.stringify(redact(payload)),
    }) + "\n");
  } catch {
    // Worklog capture must never block OpenCode.
  }
}

function captureEvent(input: { event?: { type?: string } }) {
  const eventType = input.event?.type;
  if (eventType && capturedEvents.has(eventType)) {
    write("event", input);
  }
}

export default (async () => {
  return {
    event: async (input: { event?: { type?: string } }) => captureEvent(input),
    "tool.execute.before": async (input: unknown) => write("tool_call", input),
    "tool.execute.after": async (input: unknown) => write("tool_result", input),
    "command.execute.before": async (input: unknown) => write("command", input),
  };
}) satisfies Plugin;
