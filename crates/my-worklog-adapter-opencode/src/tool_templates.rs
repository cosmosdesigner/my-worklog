use std::path::{Path, PathBuf};

use crate::install::PlannedFile;

pub fn tool_templates(target_dir: &Path) -> Vec<PlannedFile> {
    [
        ("worklog_today.ts", "today --compact"),
        ("worklog_yesterday.ts", "yesterday --compact"),
        ("worklog_week.ts", "week --compact"),
        ("worklog_decisions.ts", "decisions --compact"),
        ("worklog_open_loops.ts", "open-loops --compact"),
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
    }
}
