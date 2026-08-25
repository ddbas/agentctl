//! Append-only event log — records every dispatched `rd agent event` as one
//! JSON object per line at:
//!   `$XDG_DATA_HOME/radr/agents/<pane-id>/events.jsonl`
//!
//! Written for manual inspection/future replay only; nothing in `rd` reads
//! this file back today.

use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::state::agent_dir_in;

/// One line of `events.jsonl`: a single dispatched event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventRecord {
    /// Event name, e.g. `"SESSION_STARTED"`, `"RUN_STARTED"`, `"TOOL_CALL_START"`.
    pub event: String,
    /// Caller-supplied unique id (session id, run id, or tool-call id).
    pub id: String,
    /// Optional payload type (`"custom"` | `"elicitation"`), only present for
    /// `TOOL_CALL_START` / `TOOL_CALL_RESULT`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    /// Agent harness name (e.g. `claude`, `pi`), or `"unknown"` when not set.
    pub harness: String,
    /// Tmux pane ID (e.g. `%12`).
    pub pane_id: String,
    /// RFC 3339 timestamp of when the event was dispatched.
    pub timestamp: DateTime<Utc>,
}

/// Return the path of the events log file for the given pane under `base`.
///
/// `<base>/radr/agents/<pane-id>/events.jsonl`
pub fn events_file_path_in(base: &Path, pane_id: &str) -> std::path::PathBuf {
    agent_dir_in(base, pane_id).join("events.jsonl")
}

/// Append one JSON line describing `record` to
/// `<base>/radr/agents/<pane-id>/events.jsonl`, creating the file and its
/// parent directory if absent.
pub fn append_event_to(base: &Path, pane_id: &str, record: &EventRecord) -> Result<()> {
    let dir = agent_dir_in(base, pane_id);
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("create state dir: {}", dir.display()))?;

    let line = serde_json::to_string(record).context("serialise EventRecord")?;

    let path = events_file_path_in(base, pane_id);
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(&path)
        .with_context(|| format!("open events log: {}", path.display()))?;

    writeln!(file, "{line}")
        .with_context(|| format!("append to events log: {}", path.display()))?;

    Ok(())
}

/// Append an event using the XDG base directory.
pub fn append_event(pane_id: &str, record: &EventRecord) -> Result<()> {
    append_event_to(&crate::state::xdg_data_home(), pane_id, record)
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn record(event: &str, id: &str) -> EventRecord {
        EventRecord {
            event: event.to_owned(),
            id: id.to_owned(),
            r#type: None,
            harness: "claude".to_owned(),
            pane_id: "%12".to_owned(),
            timestamp: Utc::now(),
        }
    }

    #[test]
    fn first_event_creates_file_with_one_line() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = events_file_path_in(tmp.path(), "%12");
        assert!(!path.exists());

        append_event_to(tmp.path(), "%12", &record("SESSION_STARTED", "s1")).expect("append");

        assert!(path.exists());
        let content = std::fs::read_to_string(&path).expect("read");
        assert_eq!(content.lines().count(), 1);
        assert!(content.contains("SESSION_STARTED"));
    }

    #[test]
    fn second_event_appends_without_truncating() {
        let tmp = tempfile::tempdir().expect("tempdir");
        append_event_to(tmp.path(), "%12", &record("SESSION_STARTED", "s1")).expect("append 1");
        append_event_to(tmp.path(), "%12", &record("RUN_STARTED", "r1")).expect("append 2");

        let path = events_file_path_in(tmp.path(), "%12");
        let content = std::fs::read_to_string(&path).expect("read");
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("SESSION_STARTED"));
        assert!(lines[1].contains("RUN_STARTED"));
    }

    #[test]
    fn creates_dir_when_absent() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = agent_dir_in(tmp.path(), "%99");
        assert!(!dir.exists());

        append_event_to(tmp.path(), "%99", &record("SESSION_STARTED", "s1")).expect("append");

        assert!(dir.exists());
    }

    #[test]
    fn type_field_included_when_present() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut rec = record("TOOL_CALL_START", "t1");
        rec.r#type = Some("elicitation".to_owned());
        append_event_to(tmp.path(), "%12", &rec).expect("append");

        let path = events_file_path_in(tmp.path(), "%12");
        let content = std::fs::read_to_string(&path).expect("read");
        assert!(content.contains(r#""type":"elicitation""#));
    }

    #[test]
    fn type_field_omitted_when_absent() {
        let tmp = tempfile::tempdir().expect("tempdir");
        append_event_to(tmp.path(), "%12", &record("RUN_STARTED", "r1")).expect("append");

        let path = events_file_path_in(tmp.path(), "%12");
        let content = std::fs::read_to_string(&path).expect("read");
        assert!(!content.contains(r#""type""#));
    }
}
