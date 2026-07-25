import { tool } from "@opencode-ai/plugin";
import { z } from "zod";
import { execFileSync } from "node:child_process";

export default tool({
  description: "Run `my-worklog week --compact` locally.",
  args: z.object({}),
  async execute() {
    return execFileSync("my-worklog", "week --compact".split(" "), { encoding: "utf8" });
  },
});
