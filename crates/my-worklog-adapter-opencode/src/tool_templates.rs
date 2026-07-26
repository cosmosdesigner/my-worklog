use std::path::{Path, PathBuf};

use crate::install::PlannedFile;

pub fn tool_templates(target_dir: &Path) -> Vec<PlannedFile> {
    [
        ("worklog_today.ts", "today"),
        ("worklog_yesterday.ts", "yesterday"),
        ("worklog_week.ts", "week"),
        ("worklog_status.ts", "status --period today"),
        ("worklog_done.ts", "done --period week"),
        ("worklog_decisions.ts", "decisions --period week"),
        ("worklog_open_loops.ts", "open-loops --period week"),
        ("worklog_blockers.ts", "blockers --period week"),
        ("worklog_files.ts", "files --period week"),
        ("worklog_commands.ts", "commands --period week"),
        ("worklog_agents.ts", "agents --period week"),
    ]
    .into_iter()
    .map(|(name, command)| PlannedFile {
        path: tool_path(target_dir, name),
        contents: tool_template(command),
    })
    .collect()
}

fn tool_path(target_dir: &Path, name: &str) -> PathBuf {
    target_dir.join("tools").join(name)
}

fn tool_template(command: &str) -> String {
    format!(
        r#"import {{ tool }} from "@opencode-ai/plugin";
import {{ z }} from "zod";
import {{ execFileSync }} from "node:child_process";

export default tool({{
  description: "Run `my-worklog {command}` locally.",
  args: z.object({{}}),
  async execute() {{
    return execFileSync("my-worklog", "{command}".split(" "), {{ encoding: "utf8" }});
  }},
}});
"#
    )
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::tool_templates;

    #[test]
    fn tool_templates_include_today_tool() {
        let templates = tool_templates(Path::new(".opencode"));
        assert!(
            templates
                .iter()
                .any(|file| file.path.ends_with("worklog_today.ts"))
        );
        assert!(
            templates
                .iter()
                .any(|file| file.path.ends_with("worklog_status.ts"))
        );
    }
}
