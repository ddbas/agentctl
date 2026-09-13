//! Agent state domain types — `AgentState` and `AgentInfo`.
//!
//! Content shape of a `state.json` file for one agent pane, without any
//! filesystem access (see `agentctl-state-fs` for the file-backed store).

use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// The canonical set of states an agent can be in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AgentState {
    // NOTE: Ord is implemented manually below — do NOT derive it here.
    Idle,
    Working,
    Done,
    Blocked,
    #[default]
    Unknown,
}

impl AgentState {
    /// Canonical sort priority: `Blocked(0) < Done(1) < Working(2) < Idle(3) < Unknown(4)`.
    fn priority(self) -> u8 {
        match self {
            AgentState::Blocked => 0,
            AgentState::Done => 1,
            AgentState::Working => 2,
            AgentState::Idle => 3,
            AgentState::Unknown => 4,
        }
    }
}

impl PartialOrd for AgentState {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for AgentState {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.priority().cmp(&other.priority())
    }
}

impl fmt::Display for AgentState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AgentState::Idle => write!(f, "idle"),
            AgentState::Working => write!(f, "working"),
            AgentState::Done => write!(f, "done"),
            AgentState::Blocked => write!(f, "blocked"),
            AgentState::Unknown => write!(f, "unknown"),
        }
    }
}

/// Content of a `state.json` file for one agent pane.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    /// Tmux pane ID (e.g. `%12`), used as both directory name and correlation key.
    pub pane_id: String,
    /// Agent harness name (e.g. `claude`, `pi`), or `"unknown"` when not set.
    pub harness: String,
    /// Current status of the agent.
    pub status: AgentState,
    /// RFC 3339 timestamp of the last state change.
    pub updated_at: DateTime<Utc>,
    /// Count of tool calls currently in flight for this pane.
    #[serde(default)]
    pub active_tools: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── AgentState Ord ─────────────────────────────────────────────────────

    #[test]
    fn agent_state_blocked_is_minimum() {
        for other in [
            AgentState::Done,
            AgentState::Working,
            AgentState::Idle,
            AgentState::Unknown,
        ] {
            assert!(
                AgentState::Blocked < other,
                "Blocked should be less than {other:?}"
            );
        }
    }

    #[test]
    fn agent_state_unknown_is_maximum() {
        for other in [
            AgentState::Blocked,
            AgentState::Done,
            AgentState::Working,
            AgentState::Idle,
        ] {
            assert!(
                AgentState::Unknown > other,
                "Unknown should be greater than {other:?}"
            );
        }
    }

    #[test]
    fn agent_state_full_ascending_order() {
        let mut states = vec![
            AgentState::Unknown,
            AgentState::Idle,
            AgentState::Blocked,
            AgentState::Working,
            AgentState::Done,
        ];
        states.sort();
        assert_eq!(
            states,
            vec![
                AgentState::Blocked,
                AgentState::Done,
                AgentState::Working,
                AgentState::Idle,
                AgentState::Unknown,
            ]
        );
    }

    #[test]
    fn agent_state_display() {
        assert_eq!(AgentState::Idle.to_string(), "idle");
        assert_eq!(AgentState::Working.to_string(), "working");
        assert_eq!(AgentState::Done.to_string(), "done");
        assert_eq!(AgentState::Blocked.to_string(), "blocked");
        assert_eq!(AgentState::Unknown.to_string(), "unknown");
    }
}
