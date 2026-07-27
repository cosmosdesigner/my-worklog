use std::collections::BTreeMap;
use std::path::Path;

use crate::db::repositories::StoredEvent;

#[derive(Debug)]
pub struct ProjectEvents<'a> {
    pub label: String,
    pub events: Vec<&'a StoredEvent>,
}

pub fn label(event: &StoredEvent) -> String {
    event
        .project_name
        .as_deref()
        .filter(|name| !name.trim().is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| event.project_root.as_deref().and_then(basename))
        .or_else(|| event.cwd.as_deref().and_then(basename))
        .unwrap_or_else(|| "Unknown project".to_string())
}

pub fn has_multiple_projects(events: &[StoredEvent]) -> bool {
    let mut labels = events.iter().map(label).collect::<Vec<_>>();
    labels.sort();
    labels.dedup();
    labels.len() > 1
}

pub fn groups(events: &[StoredEvent]) -> Vec<ProjectEvents<'_>> {
    let mut grouped = BTreeMap::<String, Vec<&StoredEvent>>::new();
    for event in events {
        grouped.entry(label(event)).or_default().push(event);
    }
    grouped
        .into_iter()
        .map(|(label, events)| ProjectEvents { label, events })
        .collect()
}

pub fn count_lines(events: &[StoredEvent]) -> Vec<String> {
    groups(events)
        .into_iter()
        .map(|group| format!("{}: {} events", group.label, group.events.len()))
        .collect()
}

fn basename(path: &str) -> Option<String> {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty())
        .map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use crate::db::repositories::StoredEvent;

    use super::label;

    fn event(
        project_name: Option<&str>,
        project_root: Option<&str>,
        cwd: Option<&str>,
    ) -> StoredEvent {
        StoredEvent {
            id: "event".to_string(),
            source_agent_id: "opencode".to_string(),
            session_id: "session".to_string(),
            event_type: "user_prompt".to_string(),
            title: None,
            content: None,
            timestamp: None,
            cwd: cwd.map(ToOwned::to_owned),
            project_root: project_root.map(ToOwned::to_owned),
            project_name: project_name.map(ToOwned::to_owned),
            command: None,
            file_path: None,
            duration_ms: None,
            raw_json: None,
        }
    }

    #[test]
    fn label_prefers_project_name_then_root_then_cwd_then_unknown() {
        assert_eq!(
            label(&event(Some("api"), Some("/workspace/web"), None)),
            "api"
        );
        assert_eq!(label(&event(None, Some("/workspace/web"), None)), "web");
        assert_eq!(label(&event(None, None, Some("/workspace/ops"))), "ops");
        assert_eq!(label(&event(None, None, None)), "Unknown project");
    }
}
