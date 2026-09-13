//! `EventRecord` — the shape of one line of `events.jsonl`.
//!
//! File I/O for this record lives in `agentctl-state-fs`; this crate only
//! owns the data shape.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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
