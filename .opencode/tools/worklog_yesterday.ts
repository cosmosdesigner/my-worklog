import { tool } from "@opencode-ai/plugin";
import { z } from "zod";
import { execFileSync } from "node:child_process";

export default tool({
  description: "Run `my-worklog yesterday --compact` locally.",
  args: z.object({}),
  async execute() {
    return execFileSync("my-worklog", "yesterday --compact".split(" "), { encoding: "utf8" });
  },
});
