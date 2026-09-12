//! `AgentRepository` — the dependency-inversion port for agent-state
//! persistence, implemented by `agentctl-state-fs::FileAgentRepository`.

use anyhow::Result;

use crate::{AgentInfo, AgentState, EventRecord};

/// Abstracts reading and writing agent-state persistence (`state.json` /
/// `events.jsonl`) behind a trait object, so callers never depend on a
/// concrete filesystem implementation.
pub trait AgentRepository {
    /// Read the full `AgentInfo` for `pane_id`, if it exists and is valid.
    fn read_agent_info(&self, pane_id: &str) -> Option<AgentInfo>;

    /// Atomically write (or overwrite) the state for `pane_id`, including an
    /// explicit `active_tools` value.
    fn write_state_with_tools(
        &self,
        pane_id: &str,
        harness: &str,
        status: AgentState,
        active_tools: u32,
    ) -> Result<()>;

    /// Delete the state for `pane_id`. No-op when absent.
    fn delete_state(&self, pane_id: &str) -> Result<()>;

    /// List all known `AgentInfo` values.
    fn list_agent_infos(&self) -> Vec<AgentInfo>;

    /// Append one event record for `pane_id`.
    fn append_event(&self, pane_id: &str, record: &EventRecord) -> Result<()>;
}
