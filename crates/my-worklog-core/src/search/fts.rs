use rusqlite::Connection;

use crate::db::repositories::search_events;
use crate::error::WorklogResult;
use crate::report::display::event_label;

pub fn search_markdown(conn: &Connection, query: &str) -> WorklogResult<String> {
    let events = search_events(conn, query)?;
    let mut output = format!("# Search - {query}\n\n");
    if events.is_empty() {
        output.push_str("No matching work events found.\n");
        return Ok(output);
    }
    let mut shown = 0usize;
    for event in events {
        let Some(text) = event_label(&event) else {
            continue;
        };
        output.push_str(&format!("- {} [{}]\n", text, event.source_agent_id));
        shown += 1;
    }
    if shown == 0 {
        output.push_str("No human-readable matching work events found. Use an explicit raw/export command to inspect stored provider events.\n");
    }
    Ok(output)
}
