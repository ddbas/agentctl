//! `agentctl-state` — domain types, the status FSM, and the `AgentRepository`
//! trait port. No filesystem access lives here; that is the sole
//! responsibility of `agentctl-state-fs`.

mod agent_info;
mod events;
mod repository;
mod status_fsm;

pub use agent_info::{AgentInfo, AgentState};
pub use events::EventRecord;
pub use repository::AgentRepository;
pub use status_fsm::{Event, StatusFsm, ToolCallType, apply_event};
