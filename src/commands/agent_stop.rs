//! `rd agent stop` — write `done` or `snoozed` depending on whether a
//! background watch (e.g. Claude's Monitor tool) is currently armed for the
//! pane.
//!
//! Invoked from the `Stop` and `StopFailure` Claude hooks in place of the
//! previous unconditional `rd agent status done --harness claude`.

use anyhow::Result;

use crate::state::{AgentState, is_watch_armed, write_state};

/// Entry point for `rd agent stop [--harness <h>] [--pane <id>]`.
///
/// Resolution order for pane ID:
///   1. `--pane` flag
///   2. `$TMUX_PANE` environment variable
///   3. Exit 0 silently (not inside tmux)
///
/// Writes `AgentState::Snoozed` if an armed-watch marker is present for the
/// pane, otherwise writes `AgentState::Done` (unchanged prior behavior).
pub fn run(harness: Option<String>, pane: Option<String>) -> Result<()> {
    let pane_id = match pane.or_else(|| std::env::var("TMUX_PANE").ok()) {
        Some(id) if !id.is_empty() => id,
        _ => return Ok(()), // not in tmux — exit 0 silently
    };

    let harness_str = harness.unwrap_or_else(|| "unknown".to_owned());

    let state = if is_watch_armed(&pane_id) {
        AgentState::Snoozed
    } else {
        AgentState::Done
    };

    write_state(&pane_id, &harness_str, state)?;
    Ok(())
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{arm_watch_in, read_state_from};
    #[test]
    fn stop_with_no_marker_writes_done() {
        let _guard = crate::state::ENV_MUTEX
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let tmp = tempfile::tempdir().expect("tempdir");
        unsafe {
            std::env::set_var("XDG_DATA_HOME", tmp.path());
        }
        run(Some("claude".to_owned()), Some("%60".to_owned())).expect("run");
        assert_eq!(read_state_from(tmp.path(), "%60"), AgentState::Done);
        unsafe {
            std::env::remove_var("XDG_DATA_HOME");
        }
    }

    #[test]
    fn stop_with_marker_present_writes_snoozed() {
        let _guard = crate::state::ENV_MUTEX
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let tmp = tempfile::tempdir().expect("tempdir");
        unsafe {
            std::env::set_var("XDG_DATA_HOME", tmp.path());
        }
        arm_watch_in(tmp.path(), "%61").expect("arm");
        run(Some("claude".to_owned()), Some("%61".to_owned())).expect("run");
        assert_eq!(read_state_from(tmp.path(), "%61"), AgentState::Snoozed);
        unsafe {
            std::env::remove_var("XDG_DATA_HOME");
        }
    }

    #[test]
    fn stop_no_pane_exits_silently() {
        let result = run(None, None);
        let _ = result;
    }
}
