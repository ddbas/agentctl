//! `EventKind` — the five events accepted by `agentctl agent event <NAME>`,
//! decoupled from `clap`'s derive types.

/// One of the five events accepted by `agentctl agent event <NAME>`.
#[derive(Debug, Clone, Copy)]
pub enum EventKind {
    SessionStarted,
    RunStarted,
    /// A run (agent turn) finished. `active_tools`, when set, overrides the
    /// pane's `activeTools` counter instead of leaving it untouched — used
    /// to reconcile against a harness's authoritative in-flight
    /// background-task count.
    RunFinished {
        active_tools: Option<u32>,
    },
    ToolCallStart,
    ToolCallResult,
}

impl EventKind {
    /// The canonical event name string, as recorded in `events.jsonl`.
    pub(crate) fn name(&self) -> &'static str {
        match self {
            EventKind::SessionStarted => "SESSION_STARTED",
            EventKind::RunStarted => "RUN_STARTED",
            EventKind::RunFinished { .. } => "RUN_FINISHED",
            EventKind::ToolCallStart => "TOOL_CALL_START",
            EventKind::ToolCallResult => "TOOL_CALL_RESULT",
        }
    }
}
