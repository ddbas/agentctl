//! `status_fsm` — a `rust-fsm`-based state machine that derives an agent's
//! `status` from the stream of events dispatched via `agentctl agent event`.
//!
//! States are [`AgentState`]'s `Idle | Working | Blocked | Done` variants
//! (`Unknown` is never a target of any transition, but is accepted as a
//! starting state so a corrupt/missing state file behaves as "no matching
//! transition" rather than panicking).
//!
//! `Idle` is the machine's declared start state; `Done` is its declared end
//! state (reached at the close of each run). See `design.md` decision 3 for
//! the full transition table.

use rust_fsm::StateMachineImpl;

use crate::AgentState;

/// The optional payload type carried by `TOOL_CALL_START` / `TOOL_CALL_RESULT`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolCallType {
    Custom,
    Elicitation,
}

/// The five events accepted by `agentctl agent event`, as FSM inputs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    SessionStarted,
    RunStarted,
    /// A run finished. Always drives status to `Done`; the tool counter is
    /// deliberately not part of the status machine.
    RunFinished,
    ToolCallStart {
        r#type: Option<ToolCallType>,
    },
    ToolCallResult {
        r#type: Option<ToolCallType>,
    },
}

/// Marker type implementing [`StateMachineImpl`] for the agent status FSM.
pub struct StatusFsm;

impl StateMachineImpl for StatusFsm {
    type Input = Event;
    type State = AgentState;
    type Output = ();

    const INITIAL_STATE: Self::State = AgentState::Idle;

    fn transition(state: &Self::State, input: &Self::Input) -> Option<Self::State> {
        use AgentState::*;
        use ToolCallType::Elicitation;

        // Wildcard reset: SESSION_STARTED always moves to Idle, regardless of
        // the current status (including a stale Working/Blocked left over
        // from a crashed prior session, or Unknown from a missing/corrupt
        // file).
        if matches!(input, Event::SessionStarted) {
            return Some(Idle);
        }

        // Wildcard: RUN_FINISHED is authoritative from any current status
        // (Stop/StopFailure fired, the harness says the turn is over),
        // independent of any tool counter.
        if matches!(input, Event::RunFinished) {
            return Some(Done);
        }

        match (state, input) {
            (Idle, Event::RunStarted) => Some(Working),
            (Done, Event::RunStarted) => Some(Working),
            // A harness-initiated turn (e.g. background-task wake-up) self-corrects
            // at its first tool call.
            (
                Done,
                Event::ToolCallStart {
                    r#type: Some(Elicitation),
                },
            ) => Some(Blocked),
            (Done, Event::ToolCallStart { .. }) => Some(Working),
            (
                Working,
                Event::ToolCallStart {
                    r#type: Some(Elicitation),
                },
            ) => Some(Blocked),
            (
                Blocked,
                Event::ToolCallResult {
                    r#type: Some(Elicitation),
                },
            ) => Some(Working),
            // Every other combination (custom/no-type tool call events, or an
            // event that doesn't match the current status, e.g. a stray
            // TOOL_CALL_RESULT when not Blocked) is a no-op for status.
            _ => None,
        }
    }

    fn output(_state: &Self::State, _input: &Self::Input) -> Option<Self::Output> {
        None
    }
}

/// Apply `event` to `current_status`, returning the new status.
///
/// Runs the FSM's transition function and falls back to `current_status`
/// unchanged when no transition matches (lenient by design — every event is
/// still appended to `events.jsonl` by the caller regardless of whether it
/// drove a transition).
pub fn apply_event(current_status: AgentState, event: &Event) -> AgentState {
    StatusFsm::transition(&current_status, event).unwrap_or(current_status)
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use AgentState::*;

    #[test]
    fn session_started_resets_idle_to_idle() {
        assert_eq!(apply_event(Idle, &Event::SessionStarted), Idle);
    }

    #[test]
    fn session_started_resets_working_to_idle() {
        assert_eq!(apply_event(Working, &Event::SessionStarted), Idle);
    }

    #[test]
    fn session_started_resets_blocked_to_idle() {
        assert_eq!(apply_event(Blocked, &Event::SessionStarted), Idle);
    }

    #[test]
    fn session_started_resets_done_to_idle() {
        assert_eq!(apply_event(Done, &Event::SessionStarted), Idle);
    }

    #[test]
    fn session_started_resets_unknown_to_idle() {
        assert_eq!(apply_event(Unknown, &Event::SessionStarted), Idle);
    }

    #[test]
    fn run_started_moves_idle_to_working() {
        assert_eq!(apply_event(Idle, &Event::RunStarted), Working);
    }

    #[test]
    fn run_started_moves_done_to_working() {
        assert_eq!(apply_event(Done, &Event::RunStarted), Working);
    }

    #[test]
    fn run_finished_moves_working_to_done() {
        assert_eq!(apply_event(Working, &Event::RunFinished), Done);
    }

    #[test]
    fn run_finished_moves_idle_to_done() {
        assert_eq!(apply_event(Idle, &Event::RunFinished), Done);
    }

    #[test]
    fn run_finished_moves_blocked_to_done() {
        assert_eq!(apply_event(Blocked, &Event::RunFinished), Done);
    }

    #[test]
    fn run_finished_moves_done_to_done() {
        assert_eq!(apply_event(Done, &Event::RunFinished), Done);
    }

    #[test]
    fn run_finished_reaches_done_from_every_state() {
        for state in [Idle, Working, Blocked, Done, Unknown] {
            assert_eq!(apply_event(state, &Event::RunFinished), Done);
        }
    }

    #[test]
    fn tool_call_start_moves_done_to_working() {
        for t in [None, Some(ToolCallType::Custom)] {
            assert_eq!(
                apply_event(Done, &Event::ToolCallStart { r#type: t }),
                Working
            );
        }
    }

    #[test]
    fn elicitation_tool_call_start_moves_done_to_blocked() {
        let event = Event::ToolCallStart {
            r#type: Some(ToolCallType::Elicitation),
        };
        assert_eq!(apply_event(Done, &event), Blocked);
    }

    #[test]
    fn tool_call_result_does_not_move_done() {
        for t in [
            None,
            Some(ToolCallType::Custom),
            Some(ToolCallType::Elicitation),
        ] {
            assert_eq!(
                apply_event(Done, &Event::ToolCallResult { r#type: t }),
                Done
            );
        }
    }

    #[test]
    fn tool_call_start_does_not_move_idle() {
        assert_eq!(
            apply_event(Idle, &Event::ToolCallStart { r#type: None }),
            Idle
        );
    }

    #[test]
    fn elicitation_tool_call_start_moves_working_to_blocked() {
        let event = Event::ToolCallStart {
            r#type: Some(ToolCallType::Elicitation),
        };
        assert_eq!(apply_event(Working, &event), Blocked);
    }

    #[test]
    fn elicitation_tool_call_result_moves_blocked_to_working() {
        let event = Event::ToolCallResult {
            r#type: Some(ToolCallType::Elicitation),
        };
        assert_eq!(apply_event(Blocked, &event), Working);
    }

    #[test]
    fn custom_type_tool_call_start_is_noop() {
        let event = Event::ToolCallStart {
            r#type: Some(ToolCallType::Custom),
        };
        assert_eq!(apply_event(Working, &event), Working);
    }

    #[test]
    fn custom_type_tool_call_result_is_noop() {
        let event = Event::ToolCallResult {
            r#type: Some(ToolCallType::Custom),
        };
        assert_eq!(apply_event(Working, &event), Working);
    }

    #[test]
    fn no_type_tool_call_start_is_noop() {
        let event = Event::ToolCallStart { r#type: None };
        assert_eq!(apply_event(Working, &event), Working);
    }

    #[test]
    fn no_type_tool_call_result_is_noop() {
        let event = Event::ToolCallResult { r#type: None };
        assert_eq!(apply_event(Working, &event), Working);
    }

    #[test]
    fn unmatched_event_for_current_state_is_noop() {
        // Idle has no defined transition for an elicitation tool call result.
        let event = Event::ToolCallResult {
            r#type: Some(ToolCallType::Elicitation),
        };
        assert_eq!(apply_event(Idle, &event), Idle);
    }

    #[test]
    fn blocked_run_started_is_noop() {
        // Not part of the defined transition table.
        assert_eq!(apply_event(Blocked, &Event::RunStarted), Blocked);
    }

    #[test]
    fn done_run_finished_is_noop_when_already_done() {
        assert_eq!(apply_event(Done, &Event::RunFinished), Done);
    }
}
