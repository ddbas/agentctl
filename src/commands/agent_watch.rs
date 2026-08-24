//! `rd agent watch arm` / `rd agent watch resolve` — armed-watch marker
//! plumbing for self-resuming background mechanisms (e.g. Claude Code's
//! Monitor tool).
//!
//! `arm` is invoked from a `PreToolUse` hook matched to the arming tool
//! (e.g. `Monitor`) and simply records that a watch is now armed for the
//! current pane. `resolve` is invoked from the corresponding `PostToolUse`
//! hook and reads the hook's JSON payload from stdin to decide whether the
//! watch has already ended (in which case the marker is cleared) or is
//! still ambiguous/running (in which case the marker is left in place,
//! biasing toward `Snoozed` over `Done` at the next `Stop`).

use std::io::Read;

use anyhow::Result;
use serde_json::Value;

use crate::state::{arm_watch, clear_watch_marker};

/// Entry point for `rd agent watch arm [--pane <id>]`.
///
/// Resolution order for pane ID:
///   1. `--pane` flag
///   2. `$TMUX_PANE` environment variable
///   3. Exit 0 silently (not inside tmux)
pub fn run_arm(pane: Option<String>) -> Result<()> {
    let pane_id = match resolve_pane(pane) {
        Some(id) => id,
        None => return Ok(()),
    };
    arm_watch(&pane_id)?;
    Ok(())
}

/// Entry point for `rd agent watch resolve [--pane <id>]`.
///
/// Reads the hook's JSON payload from stdin and inspects `tool_response` to
/// decide whether the watch has ended. Ambiguous or unparsable payloads
/// leave the marker in place (conservative default).
pub fn run_resolve(pane: Option<String>) -> Result<()> {
    let pane_id = match resolve_pane(pane) {
        Some(id) => id,
        None => return Ok(()),
    };

    let mut input = String::new();
    // Errors reading stdin are treated the same as ambiguous input: leave
    // the marker armed rather than fail the hook.
    let _ = std::io::stdin().read_to_string(&mut input);

    if watch_ended(&input) {
        clear_watch_marker(&pane_id)?;
    }

    Ok(())
}

fn resolve_pane(pane: Option<String>) -> Option<String> {
    match pane.or_else(|| std::env::var("TMUX_PANE").ok()) {
        Some(id) if !id.is_empty() => Some(id),
        _ => None,
    }
}

/// Returns `true` if the hook payload's `tool_response` clearly indicates
/// the watch has ended (timed out, errored, was denied, or completed).
/// Returns `false` (ambiguous / still armed) for any other shape, including
/// unparsable JSON or a missing `tool_response`.
fn watch_ended(payload: &str) -> bool {
    let parsed: Value = match serde_json::from_str(payload) {
        Ok(v) => v,
        Err(_) => return false,
    };

    let Some(tool_response) = parsed.get("tool_response") else {
        return false;
    };

    // Explicit boolean signals.
    if tool_response.get("ended").and_then(Value::as_bool) == Some(true) {
        return true;
    }
    if tool_response.get("is_error").and_then(Value::as_bool) == Some(true) {
        return true;
    }

    // String status signals.
    const ENDED_STATUSES: &[&str] = &[
        "timeout",
        "timed_out",
        "error",
        "denied",
        "cancelled",
        "canceled",
        "completed",
        "ended",
        "stopped",
    ];
    if let Some(status) = tool_response.get("status").and_then(Value::as_str)
        && ENDED_STATUSES.contains(&status)
    {
        return true;
    }

    false
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{is_watch_armed_in, read_state_from, watch_marker_path_in};
    #[test]
    fn arm_writes_marker_for_pane() {
        let _guard = crate::state::ENV_MUTEX
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let tmp = tempfile::tempdir().expect("tempdir");
        unsafe {
            std::env::set_var("XDG_DATA_HOME", tmp.path());
        }
        run_arm(Some("%50".to_owned())).expect("run_arm");
        assert!(watch_marker_path_in(tmp.path(), "%50").exists());
        unsafe {
            std::env::remove_var("XDG_DATA_HOME");
        }
    }

    #[test]
    fn arm_no_pane_exits_silently() {
        let result = run_arm(None);
        let _ = result;
    }

    #[test]
    fn watch_ended_true_for_timeout_status() {
        assert!(watch_ended(r#"{"tool_response":{"status":"timeout"}}"#));
    }

    #[test]
    fn watch_ended_true_for_error_status() {
        assert!(watch_ended(r#"{"tool_response":{"status":"error"}}"#));
    }

    #[test]
    fn watch_ended_true_for_is_error_flag() {
        assert!(watch_ended(r#"{"tool_response":{"is_error":true}}"#));
    }

    #[test]
    fn watch_ended_false_for_running_status() {
        assert!(!watch_ended(r#"{"tool_response":{"status":"running"}}"#));
    }

    #[test]
    fn watch_ended_false_for_missing_tool_response() {
        assert!(!watch_ended(r#"{"tool_name":"Monitor"}"#));
    }

    #[test]
    fn watch_ended_false_for_unparsable_input() {
        assert!(!watch_ended("not json"));
    }

    #[test]
    fn watch_ended_false_for_empty_input() {
        assert!(!watch_ended(""));
    }

    #[test]
    fn resolve_clears_marker_when_ended() {
        let _guard = crate::state::ENV_MUTEX
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let tmp = tempfile::tempdir().expect("tempdir");
        unsafe {
            std::env::set_var("XDG_DATA_HOME", tmp.path());
        }
        run_arm(Some("%51".to_owned())).expect("arm");
        assert!(is_watch_armed_in(tmp.path(), "%51"));

        // resolve() reads stdin directly, so exercise the pure logic + clear
        // helper instead of piping stdin in-process.
        if watch_ended(r#"{"tool_response":{"status":"timeout"}}"#) {
            crate::state::clear_watch_marker_in(tmp.path(), "%51").expect("clear");
        }
        assert!(!is_watch_armed_in(tmp.path(), "%51"));
        unsafe {
            std::env::remove_var("XDG_DATA_HOME");
        }
    }

    #[test]
    fn resolve_leaves_marker_when_ambiguous() {
        let _guard = crate::state::ENV_MUTEX
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let tmp = tempfile::tempdir().expect("tempdir");
        unsafe {
            std::env::set_var("XDG_DATA_HOME", tmp.path());
        }
        run_arm(Some("%52".to_owned())).expect("arm");
        assert!(is_watch_armed_in(tmp.path(), "%52"));

        if watch_ended(r#"{"tool_response":{"status":"running"}}"#) {
            crate::state::clear_watch_marker_in(tmp.path(), "%52").expect("clear");
        }
        assert!(
            is_watch_armed_in(tmp.path(), "%52"),
            "ambiguous response should leave the marker armed"
        );
        // read_state_from unrelated pane should stay unknown (sanity check
        // that arming the watch doesn't affect visible state).
        assert_eq!(
            read_state_from(tmp.path(), "%52"),
            crate::state::AgentState::Unknown
        );
        unsafe {
            std::env::remove_var("XDG_DATA_HOME");
        }
    }
}
