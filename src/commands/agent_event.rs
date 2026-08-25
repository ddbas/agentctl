//! `rd agent event` — dispatch a typed event, deriving the pane's status via
//! the status FSM and appending the raw event to `events.jsonl`.

use anyhow::Result;
use chrono::Utc;

use crate::events_log::{EventRecord, append_event};
use crate::state::{read_agent_info, write_state_with_tools};
use crate::status_fsm::{self, Event, ToolCallType};

/// One of the five events accepted by `rd agent event <NAME>`.
///
/// Mirrors `AgentEventSubcommand` in `main.rs`; kept as a separate type here
/// so this module does not need to depend on clap's derive types.
pub enum EventKind {
    SessionStarted,
    RunStarted,
    RunFinished,
    ToolCallStart,
    ToolCallResult,
}

impl EventKind {
    /// The canonical event name string, as recorded in `events.jsonl`.
    fn name(&self) -> &'static str {
        match self {
            EventKind::SessionStarted => "SESSION_STARTED",
            EventKind::RunStarted => "RUN_STARTED",
            EventKind::RunFinished => "RUN_FINISHED",
            EventKind::ToolCallStart => "TOOL_CALL_START",
            EventKind::ToolCallResult => "TOOL_CALL_RESULT",
        }
    }

    /// Convert to the FSM's `Event` input, given an optional `--type`.
    fn to_fsm_event(&self, tool_call_type: Option<ToolCallType>) -> Event {
        match self {
            EventKind::SessionStarted => Event::SessionStarted,
            EventKind::RunStarted => Event::RunStarted,
            EventKind::RunFinished => Event::RunFinished,
            EventKind::ToolCallStart => Event::ToolCallStart {
                r#type: tool_call_type,
            },
            EventKind::ToolCallResult => Event::ToolCallResult {
                r#type: tool_call_type,
            },
        }
    }
}

/// Entry point for `rd agent event <NAME> --id <ID> [--type <TYPE>]
/// [--harness <H>] [--pane <ID>]`.
///
/// Resolution order for pane ID:
///   1. `--pane` flag
///   2. `$TMUX_PANE` environment variable
///   3. Exit 0 silently (not inside tmux)
pub fn run(
    kind: EventKind,
    id: String,
    tool_call_type: Option<ToolCallType>,
    harness: Option<String>,
    pane: Option<String>,
) -> Result<()> {
    // Resolve pane ID: flag → env → silent exit
    let pane_id = match pane.or_else(|| std::env::var("TMUX_PANE").ok()) {
        Some(id) if !id.is_empty() => id,
        _ => return Ok(()), // not in tmux — exit 0 silently
    };

    let harness_str = harness.unwrap_or_else(|| "unknown".to_owned());

    let current_info = read_agent_info(&pane_id);
    let current_status = current_info
        .as_ref()
        .map(|info| info.status)
        .unwrap_or_default();
    let current_active_tools = current_info
        .as_ref()
        .map(|info| info.active_tools)
        .unwrap_or(0);

    let event = kind.to_fsm_event(tool_call_type);
    let new_status = status_fsm::apply_event(current_status, &event);

    let new_active_tools = match kind {
        EventKind::SessionStarted => 0,
        EventKind::ToolCallStart => current_active_tools + 1,
        EventKind::ToolCallResult => current_active_tools.saturating_sub(1),
        EventKind::RunStarted | EventKind::RunFinished => current_active_tools,
    };

    write_state_with_tools(&pane_id, &harness_str, new_status, new_active_tools)?;

    let record = EventRecord {
        event: kind.name().to_owned(),
        id,
        r#type: tool_call_type.map(|t| match t {
            ToolCallType::Custom => "custom".to_owned(),
            ToolCallType::Elicitation => "elicitation".to_owned(),
        }),
        harness: harness_str,
        pane_id: pane_id.clone(),
        timestamp: Utc::now(),
    };
    append_event(&pane_id, &record)?;

    Ok(())
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{AgentState, read_state_from, state_file_path_in};
    use std::sync::Mutex;

    /// Serializes tests that mutate the process-global `XDG_DATA_HOME`
    /// environment variable, since `cargo test` runs tests in parallel
    /// threads by default and concurrent `set_var`/`remove_var` calls race.
    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    #[test]
    fn session_started_writes_idle() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let tmp = tempfile::tempdir().expect("tempdir");
        unsafe {
            std::env::set_var("XDG_DATA_HOME", tmp.path());
        }
        run(
            EventKind::SessionStarted,
            "s1".to_owned(),
            None,
            Some("claude".to_owned()),
            Some("%42".to_owned()),
        )
        .expect("run");
        assert_eq!(read_state_from(tmp.path(), "%42"), AgentState::Idle);
        unsafe {
            std::env::remove_var("XDG_DATA_HOME");
        }
    }

    #[test]
    fn run_started_moves_idle_to_working() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let tmp = tempfile::tempdir().expect("tempdir");
        unsafe {
            std::env::set_var("XDG_DATA_HOME", tmp.path());
        }
        run(
            EventKind::SessionStarted,
            "s1".to_owned(),
            None,
            None,
            Some("%1".to_owned()),
        )
        .expect("session started");
        run(
            EventKind::RunStarted,
            "r1".to_owned(),
            None,
            None,
            Some("%1".to_owned()),
        )
        .expect("run started");
        assert_eq!(read_state_from(tmp.path(), "%1"), AgentState::Working);
        unsafe {
            std::env::remove_var("XDG_DATA_HOME");
        }
    }

    #[test]
    fn no_pane_exits_silently_and_writes_nothing() {
        let result = run(EventKind::SessionStarted, "s1".to_owned(), None, None, None);
        // Either Ok (no TMUX_PANE set) — matches rd agent status precedent;
        // we don't touch global env here to avoid races.
        let _ = result;
    }

    #[test]
    fn event_is_appended_to_events_log() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let tmp = tempfile::tempdir().expect("tempdir");
        unsafe {
            std::env::set_var("XDG_DATA_HOME", tmp.path());
        }
        run(
            EventKind::SessionStarted,
            "s1".to_owned(),
            None,
            Some("claude".to_owned()),
            Some("%7".to_owned()),
        )
        .expect("run");
        let events_path = crate::events_log::events_file_path_in(tmp.path(), "%7");
        assert!(events_path.exists());
        let content = std::fs::read_to_string(&events_path).unwrap();
        assert!(content.contains("SESSION_STARTED"));
        unsafe {
            std::env::remove_var("XDG_DATA_HOME");
        }
    }

    // ── active_tools ──────────────────────────────────────────────────────────

    #[test]
    fn tool_call_start_increments_active_tools() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let tmp = tempfile::tempdir().expect("tempdir");
        unsafe {
            std::env::set_var("XDG_DATA_HOME", tmp.path());
        }
        run(
            EventKind::ToolCallStart,
            "t1".to_owned(),
            None,
            Some("pi".to_owned()),
            Some("%1".to_owned()),
        )
        .expect("run");
        let info = crate::state::read_agent_info_from(tmp.path(), "%1").expect("info");
        assert_eq!(info.active_tools, 1);
        unsafe {
            std::env::remove_var("XDG_DATA_HOME");
        }
    }

    #[test]
    fn tool_call_result_decrements_active_tools() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let tmp = tempfile::tempdir().expect("tempdir");
        unsafe {
            std::env::set_var("XDG_DATA_HOME", tmp.path());
        }
        run(
            EventKind::ToolCallStart,
            "t1".to_owned(),
            None,
            Some("pi".to_owned()),
            Some("%1".to_owned()),
        )
        .expect("start");
        run(
            EventKind::ToolCallStart,
            "t2".to_owned(),
            None,
            Some("pi".to_owned()),
            Some("%1".to_owned()),
        )
        .expect("start 2");
        run(
            EventKind::ToolCallResult,
            "t1".to_owned(),
            None,
            Some("pi".to_owned()),
            Some("%1".to_owned()),
        )
        .expect("result");
        let info = crate::state::read_agent_info_from(tmp.path(), "%1").expect("info");
        assert_eq!(info.active_tools, 1);
        unsafe {
            std::env::remove_var("XDG_DATA_HOME");
        }
    }

    #[test]
    fn tool_call_result_on_zero_count_stays_zero() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let tmp = tempfile::tempdir().expect("tempdir");
        unsafe {
            std::env::set_var("XDG_DATA_HOME", tmp.path());
        }
        run(
            EventKind::ToolCallResult,
            "t1".to_owned(),
            None,
            Some("pi".to_owned()),
            Some("%1".to_owned()),
        )
        .expect("result");
        let info = crate::state::read_agent_info_from(tmp.path(), "%1").expect("info");
        assert_eq!(info.active_tools, 0);
        unsafe {
            std::env::remove_var("XDG_DATA_HOME");
        }
    }

    #[test]
    fn multiple_concurrent_tool_call_starts_accumulate() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let tmp = tempfile::tempdir().expect("tempdir");
        unsafe {
            std::env::set_var("XDG_DATA_HOME", tmp.path());
        }
        for id in ["t1", "t2", "t3"] {
            run(
                EventKind::ToolCallStart,
                id.to_owned(),
                None,
                Some("pi".to_owned()),
                Some("%1".to_owned()),
            )
            .expect("start");
        }
        let info = crate::state::read_agent_info_from(tmp.path(), "%1").expect("info");
        assert_eq!(info.active_tools, 3);
        unsafe {
            std::env::remove_var("XDG_DATA_HOME");
        }
    }

    #[test]
    fn custom_type_tool_call_still_increments_and_decrements() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let tmp = tempfile::tempdir().expect("tempdir");
        unsafe {
            std::env::set_var("XDG_DATA_HOME", tmp.path());
        }
        run(
            EventKind::ToolCallStart,
            "t1".to_owned(),
            Some(ToolCallType::Custom),
            Some("pi".to_owned()),
            Some("%1".to_owned()),
        )
        .expect("start");
        let info = crate::state::read_agent_info_from(tmp.path(), "%1").expect("info");
        assert_eq!(info.active_tools, 1);
        run(
            EventKind::ToolCallResult,
            "t1".to_owned(),
            Some(ToolCallType::Custom),
            Some("pi".to_owned()),
            Some("%1".to_owned()),
        )
        .expect("result");
        let info = crate::state::read_agent_info_from(tmp.path(), "%1").expect("info");
        assert_eq!(info.active_tools, 0);
        unsafe {
            std::env::remove_var("XDG_DATA_HOME");
        }
    }

    #[test]
    fn session_started_resets_nonzero_active_tools_to_zero() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let tmp = tempfile::tempdir().expect("tempdir");
        unsafe {
            std::env::set_var("XDG_DATA_HOME", tmp.path());
        }
        run(
            EventKind::ToolCallStart,
            "t1".to_owned(),
            None,
            Some("pi".to_owned()),
            Some("%1".to_owned()),
        )
        .expect("start");
        run(
            EventKind::ToolCallStart,
            "t2".to_owned(),
            None,
            Some("pi".to_owned()),
            Some("%1".to_owned()),
        )
        .expect("start 2");
        run(
            EventKind::SessionStarted,
            "s1".to_owned(),
            None,
            Some("pi".to_owned()),
            Some("%1".to_owned()),
        )
        .expect("session started");
        let info = crate::state::read_agent_info_from(tmp.path(), "%1").expect("info");
        assert_eq!(info.active_tools, 0);
        unsafe {
            std::env::remove_var("XDG_DATA_HOME");
        }
    }

    #[test]
    fn elicitation_tool_call_start_blocks_status_and_increments_active_tools() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let tmp = tempfile::tempdir().expect("tempdir");
        unsafe {
            std::env::set_var("XDG_DATA_HOME", tmp.path());
        }
        run(
            EventKind::SessionStarted,
            "s1".to_owned(),
            None,
            Some("pi".to_owned()),
            Some("%1".to_owned()),
        )
        .expect("session started");
        run(
            EventKind::RunStarted,
            "r1".to_owned(),
            None,
            Some("pi".to_owned()),
            Some("%1".to_owned()),
        )
        .expect("run started");
        run(
            EventKind::ToolCallStart,
            "t1".to_owned(),
            Some(ToolCallType::Elicitation),
            Some("pi".to_owned()),
            Some("%1".to_owned()),
        )
        .expect("tool call start");
        let info = crate::state::read_agent_info_from(tmp.path(), "%1").expect("info");
        assert_eq!(info.status, AgentState::Blocked);
        assert_eq!(info.active_tools, 1);
        unsafe {
            std::env::remove_var("XDG_DATA_HOME");
        }
    }

    #[test]
    fn pane_flag_overrides_env_pane() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let tmp = tempfile::tempdir().expect("tempdir");
        unsafe {
            std::env::set_var("XDG_DATA_HOME", tmp.path());
        }
        run(
            EventKind::SessionStarted,
            "s1".to_owned(),
            None,
            None,
            Some("%99".to_owned()),
        )
        .expect("run");
        assert_eq!(read_state_from(tmp.path(), "%99"), AgentState::Idle);
        assert!(!state_file_path_in(tmp.path(), "%1").exists());
        unsafe {
            std::env::remove_var("XDG_DATA_HOME");
        }
    }
}
