//! Agent state tracking — shared types and file-based state store.
//!
//! State files are stored at:
//!   `$XDG_DATA_HOME/radr/agents/<pane-id>/state.json`
//! where `XDG_DATA_HOME` defaults to `~/.local/share`.

use std::fmt;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ── AgentState ────────────────────────────────────────────────────────────────

/// The canonical set of states an agent can be in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AgentState {
    // NOTE: Ord is implemented manually below — do NOT derive it here.
    Idle,
    Working,
    Done,
    Blocked,
    /// Not currently producing visible output, but an armed, self-resuming
    /// mechanism (e.g. a background watch or listener) will wake it up
    /// automatically without new human input.
    Snoozed,
    #[default]
    Unknown,
}

impl AgentState {
    /// Canonical sort priority:
    /// `Blocked(0) < Working(1) < Snoozed(2) < Done(3) < Idle(4) < Unknown(5)`.
    fn priority(self) -> u8 {
        match self {
            AgentState::Blocked => 0,
            AgentState::Working => 1,
            AgentState::Snoozed => 2,
            AgentState::Done => 3,
            AgentState::Idle => 4,
            AgentState::Unknown => 5,
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
            AgentState::Snoozed => write!(f, "snoozed"),
            AgentState::Unknown => write!(f, "unknown"),
        }
    }
}

// ── AgentInfo ─────────────────────────────────────────────────────────────────

/// Content of a `state.json` file for one agent pane.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    /// Tmux pane ID (e.g. `%12`), used as both directory name and correlation key.
    pub pane_id: String,
    /// Agent harness name (e.g. `claude`, `pi`), or `"unknown"` when not set.
    pub harness: String,
    /// Current state of the agent.
    pub state: AgentState,
    /// RFC 3339 timestamp of the last state change.
    pub updated_at: DateTime<Utc>,
}

// ── XDG helpers ───────────────────────────────────────────────────────────────

/// Resolve the XDG data home directory.
///
/// Returns `$XDG_DATA_HOME` if set and non-empty, otherwise `~/.local/share`.
pub fn xdg_data_home() -> PathBuf {
    if let Ok(val) = std::env::var("XDG_DATA_HOME")
        && !val.is_empty()
    {
        return PathBuf::from(val);
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_owned());
    PathBuf::from(home).join(".local").join("share")
}

/// Return the directory for the given pane's state files under `base`.
///
/// `<base>/radr/agents/<pane-id>/`
pub fn agent_dir_in(base: &Path, pane_id: &str) -> PathBuf {
    base.join("radr").join("agents").join(pane_id)
}

/// Return the path of the state file for the given pane under `base`.
///
/// `<base>/radr/agents/<pane-id>/state.json`
pub fn state_file_path_in(base: &Path, pane_id: &str) -> PathBuf {
    agent_dir_in(base, pane_id).join("state.json")
}

/// Return the path of the armed-watch marker file for the given pane under
/// `base`.
///
/// `<base>/radr/agents/<pane-id>/monitor.json`
pub fn watch_marker_path_in(base: &Path, pane_id: &str) -> PathBuf {
    agent_dir_in(base, pane_id).join("monitor.json")
}

/// Return the directory for the given pane's state files using the XDG base.
pub fn agent_dir(pane_id: &str) -> PathBuf {
    agent_dir_in(&xdg_data_home(), pane_id)
}

/// Return the path of the state file for the given pane using the XDG base.
pub fn state_file_path(pane_id: &str) -> PathBuf {
    state_file_path_in(&xdg_data_home(), pane_id)
}

/// Return the path of the armed-watch marker file for the given pane using
/// the XDG base.
pub fn watch_marker_path(pane_id: &str) -> PathBuf {
    watch_marker_path_in(&xdg_data_home(), pane_id)
}

// ── StateStore API (explicit base) ────────────────────────────────────────────

/// Atomically write (or overwrite) the state file for `pane_id` under `base`.
///
/// Writes to a `.tmp` sibling then renames so concurrent readers never see a
/// partial file.
pub fn write_state_to(base: &Path, pane_id: &str, harness: &str, state: AgentState) -> Result<()> {
    let dir = agent_dir_in(base, pane_id);
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("create state dir: {}", dir.display()))?;

    let info = AgentInfo {
        pane_id: pane_id.to_owned(),
        harness: harness.to_owned(),
        state,
        updated_at: Utc::now(),
    };

    let json = serde_json::to_string_pretty(&info).context("serialise AgentInfo")?;

    let final_path = dir.join("state.json");
    let tmp_path = dir.join("state.json.tmp");

    std::fs::write(&tmp_path, &json)
        .with_context(|| format!("write tmp state file: {}", tmp_path.display()))?;
    std::fs::rename(&tmp_path, &final_path)
        .with_context(|| format!("rename state file: {}", final_path.display()))?;

    Ok(())
}

/// Read and return the current `AgentState` for `pane_id` under `base`.
///
/// Returns [`AgentState::Unknown`] on any error.
pub fn read_state_from(base: &Path, pane_id: &str) -> AgentState {
    let path = state_file_path_in(base, pane_id);
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(_) => return AgentState::Unknown,
    };
    match serde_json::from_slice::<AgentInfo>(&bytes) {
        Ok(info) => info.state,
        Err(_) => AgentState::Unknown,
    }
}

/// Read and return the full [`AgentInfo`] for `pane_id` under `base`, if it
/// exists and is valid.
pub fn read_agent_info_from(base: &Path, pane_id: &str) -> Option<AgentInfo> {
    let path = state_file_path_in(base, pane_id);
    let bytes = std::fs::read(&path).ok()?;
    serde_json::from_slice::<AgentInfo>(&bytes).ok()
}

/// Delete the state file and its parent directory for `pane_id` under `base`.
///
/// No-op when the file or directory does not exist.
pub fn delete_state_in(base: &Path, pane_id: &str) -> Result<()> {
    let dir = agent_dir_in(base, pane_id);
    if dir.exists() {
        std::fs::remove_dir_all(&dir)
            .with_context(|| format!("remove agent dir: {}", dir.display()))?;
    }
    Ok(())
}

// ── Armed-watch marker API (explicit base) ────────────────────────────────────

/// Record that a background watch (e.g. Claude's Monitor tool) is armed for
/// `pane_id` under `base`. Creates the pane's state directory if needed and
/// writes (or overwrites) the marker file. Idempotent: calling this again
/// while already armed is a no-op in effect.
pub fn arm_watch_in(base: &Path, pane_id: &str) -> Result<()> {
    let dir = agent_dir_in(base, pane_id);
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("create state dir: {}", dir.display()))?;

    let marker = json_marker_body();
    let final_path = watch_marker_path_in(base, pane_id);
    let tmp_path = agent_dir_in(base, pane_id).join("monitor.json.tmp");

    std::fs::write(&tmp_path, &marker)
        .with_context(|| format!("write tmp marker file: {}", tmp_path.display()))?;
    std::fs::rename(&tmp_path, &final_path)
        .with_context(|| format!("rename marker file: {}", final_path.display()))?;

    Ok(())
}

fn json_marker_body() -> String {
    format!("{{\"armed_at\":\"{}\"}}", Utc::now().to_rfc3339())
}

/// Returns `true` if a background watch is currently armed for `pane_id`
/// under `base` (i.e. the marker file exists).
pub fn is_watch_armed_in(base: &Path, pane_id: &str) -> bool {
    watch_marker_path_in(base, pane_id).exists()
}

/// Clear the armed-watch marker for `pane_id` under `base`. No-op when the
/// marker does not exist.
pub fn clear_watch_marker_in(base: &Path, pane_id: &str) -> Result<()> {
    let path = watch_marker_path_in(base, pane_id);
    if path.exists() {
        std::fs::remove_file(&path)
            .with_context(|| format!("remove marker file: {}", path.display()))?;
    }
    Ok(())
}

// ── Armed-watch marker API (XDG base) ─────────────────────────────────────────

/// Record that a background watch is armed for `pane_id` using the XDG base.
pub fn arm_watch(pane_id: &str) -> Result<()> {
    arm_watch_in(&xdg_data_home(), pane_id)
}

/// Returns `true` if a background watch is currently armed for `pane_id`
/// using the XDG base.
pub fn is_watch_armed(pane_id: &str) -> bool {
    is_watch_armed_in(&xdg_data_home(), pane_id)
}

/// Clear the armed-watch marker for `pane_id` using the XDG base.
pub fn clear_watch_marker(pane_id: &str) -> Result<()> {
    clear_watch_marker_in(&xdg_data_home(), pane_id)
}

/// Glob all `state.json` files under `<base>/radr/agents/` and return parsed
/// [`AgentInfo`] values. Files that cannot be read or parsed are skipped.
pub fn list_agent_infos_in(base: &Path) -> Vec<AgentInfo> {
    let agents_dir = base.join("radr").join("agents");
    let read_dir = match std::fs::read_dir(&agents_dir) {
        Ok(rd) => rd,
        Err(_) => return Vec::new(),
    };

    let mut infos = Vec::new();
    for entry in read_dir.flatten() {
        let state_path = entry.path().join("state.json");
        if let Ok(bytes) = std::fs::read(&state_path)
            && let Ok(info) = serde_json::from_slice::<AgentInfo>(&bytes)
        {
            infos.push(info);
        }
    }
    infos
}

// ── StateStore API (XDG base) ─────────────────────────────────────────────────

/// Atomically write (or overwrite) the state file for `pane_id` using the XDG
/// base directory.
pub fn write_state(pane_id: &str, harness: &str, state: AgentState) -> Result<()> {
    write_state_to(&xdg_data_home(), pane_id, harness, state)
}

/// Read and return the current `AgentState` for `pane_id` using the XDG base.
///
/// Returns [`AgentState::Unknown`] on any error.
pub fn read_state(pane_id: &str) -> AgentState {
    read_state_from(&xdg_data_home(), pane_id)
}

/// Read and return the full [`AgentInfo`] for `pane_id` using the XDG base.
pub fn read_agent_info(pane_id: &str) -> Option<AgentInfo> {
    read_agent_info_from(&xdg_data_home(), pane_id)
}

/// Delete the state file and its parent directory for `pane_id` using the XDG
/// base. No-op when absent.
pub fn delete_state(pane_id: &str) -> Result<()> {
    delete_state_in(&xdg_data_home(), pane_id)
}

/// Glob all `state.json` files using the XDG base directory.
pub fn list_agent_infos() -> Vec<AgentInfo> {
    list_agent_infos_in(&xdg_data_home())
}

// ── Unit tests ────────────────────────────────────────────────────────────────

// ── Shared test env mutex ───────────────────────────────────────────

/// Serializes tests across the whole crate that mutate process-global
/// environment variables (`XDG_DATA_HOME`, `TMUX_PANE`), since `cargo test`
/// runs tests from every module in the same process by default and
/// concurrent `set_var`/`remove_var` calls from different modules would
/// otherwise race. Any test module that calls `std::env::set_var` on a
/// shared variable must lock this mutex for the duration of the mutation.
#[cfg(test)]
pub static ENV_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
mod tests {
    use super::*;

    // ── write / read round-trip ───────────────────────────────────────────────

    #[test]
    fn write_and_read_state_round_trip() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_state_to(tmp.path(), "%12", "claude", AgentState::Working).expect("write_state");
        assert_eq!(read_state_from(tmp.path(), "%12"), AgentState::Working);
    }

    #[test]
    fn write_overwrites_previous_state() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_state_to(tmp.path(), "%12", "claude", AgentState::Working).expect("write first");
        write_state_to(tmp.path(), "%12", "claude", AgentState::Done).expect("write second");
        assert_eq!(read_state_from(tmp.path(), "%12"), AgentState::Done);
    }

    #[test]
    fn write_state_persists_harness_field() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_state_to(tmp.path(), "%5", "pi", AgentState::Idle).expect("write_state");
        let info = read_agent_info_from(tmp.path(), "%5").expect("read info");
        assert_eq!(info.harness, "pi");
        assert_eq!(info.pane_id, "%5");
    }

    // ── read_state missing / corrupt ─────────────────────────────────────────

    #[test]
    fn read_state_missing_file_returns_unknown() {
        let tmp = tempfile::tempdir().expect("tempdir");
        assert_eq!(read_state_from(tmp.path(), "%99"), AgentState::Unknown);
    }

    #[test]
    fn read_state_corrupt_file_returns_unknown() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = tmp.path().join("radr").join("agents").join("%bad");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("state.json"), b"not-json").unwrap();
        assert_eq!(read_state_from(tmp.path(), "%bad"), AgentState::Unknown);
    }

    // ── delete_state ──────────────────────────────────────────────────────────

    #[test]
    fn delete_state_removes_file_and_dir() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_state_to(tmp.path(), "%12", "claude", AgentState::Idle).expect("write");
        assert!(state_file_path_in(tmp.path(), "%12").exists());
        delete_state_in(tmp.path(), "%12").expect("delete");
        assert!(!agent_dir_in(tmp.path(), "%12").exists());
    }

    #[test]
    fn delete_state_noop_when_absent() {
        let tmp = tempfile::tempdir().expect("tempdir");
        delete_state_in(tmp.path(), "%99").expect("delete absent");
    }

    // ── AgentState Ord ─────────────────────────────────────────────────────

    #[test]
    fn agent_state_blocked_is_minimum() {
        for other in [
            AgentState::Working,
            AgentState::Snoozed,
            AgentState::Done,
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
            AgentState::Working,
            AgentState::Snoozed,
            AgentState::Done,
            AgentState::Idle,
        ] {
            assert!(
                AgentState::Unknown > other,
                "Unknown should be greater than {other:?}"
            );
        }
    }

    #[test]
    fn agent_state_snoozed_sorts_below_working_and_blocked() {
        assert!(AgentState::Snoozed > AgentState::Working);
        assert!(AgentState::Snoozed > AgentState::Blocked);
    }

    #[test]
    fn agent_state_snoozed_sorts_above_done_and_idle() {
        assert!(AgentState::Snoozed < AgentState::Done);
        assert!(AgentState::Snoozed < AgentState::Idle);
    }

    #[test]
    fn agent_state_full_ascending_order() {
        let mut states = vec![
            AgentState::Unknown,
            AgentState::Idle,
            AgentState::Blocked,
            AgentState::Snoozed,
            AgentState::Working,
            AgentState::Done,
        ];
        states.sort();
        assert_eq!(
            states,
            vec![
                AgentState::Blocked,
                AgentState::Working,
                AgentState::Snoozed,
                AgentState::Done,
                AgentState::Idle,
                AgentState::Unknown,
            ]
        );
    }

    // ── AgentState Display ────────────────────────────────────────────────────

    #[test]
    fn agent_state_display() {
        assert_eq!(AgentState::Idle.to_string(), "idle");
        assert_eq!(AgentState::Working.to_string(), "working");
        assert_eq!(AgentState::Done.to_string(), "done");
        assert_eq!(AgentState::Blocked.to_string(), "blocked");
        assert_eq!(AgentState::Snoozed.to_string(), "snoozed");
        assert_eq!(AgentState::Unknown.to_string(), "unknown");
    }

    // ── AgentState serde ──────────────────────────────────────────────────────

    #[test]
    fn agent_state_snoozed_serde_round_trip() {
        let json = serde_json::to_string(&AgentState::Snoozed).expect("serialize");
        assert_eq!(json, "\"snoozed\"");
        let parsed: AgentState = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed, AgentState::Snoozed);
    }

    #[test]
    fn write_and_read_snoozed_state_round_trip() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_state_to(tmp.path(), "%13", "claude", AgentState::Snoozed).expect("write_state");
        assert_eq!(read_state_from(tmp.path(), "%13"), AgentState::Snoozed);
    }

    #[test]
    fn snoozed_state_is_overwritten_by_working_transition() {
        // Simulates a subsequent PostToolUse/UserPromptSubmit hook firing
        // after a Stop wrote `snoozed`: the pane's state is overwritten with
        // `working`, so `Snoozed` is never a stuck/terminal state.
        let tmp = tempfile::tempdir().expect("tempdir");
        write_state_to(tmp.path(), "%14", "claude", AgentState::Snoozed).expect("write snoozed");
        assert_eq!(read_state_from(tmp.path(), "%14"), AgentState::Snoozed);
        write_state_to(tmp.path(), "%14", "claude", AgentState::Working).expect("write working");
        assert_eq!(read_state_from(tmp.path(), "%14"), AgentState::Working);
    }

    // ── Armed-watch marker ────────────────────────────────────────────────────

    #[test]
    fn arm_watch_creates_marker_file() {
        let tmp = tempfile::tempdir().expect("tempdir");
        assert!(!is_watch_armed_in(tmp.path(), "%20"));
        arm_watch_in(tmp.path(), "%20").expect("arm_watch");
        assert!(is_watch_armed_in(tmp.path(), "%20"));
        assert!(watch_marker_path_in(tmp.path(), "%20").exists());
    }

    #[test]
    fn arm_watch_is_idempotent() {
        let tmp = tempfile::tempdir().expect("tempdir");
        arm_watch_in(tmp.path(), "%20").expect("arm first");
        arm_watch_in(tmp.path(), "%20").expect("arm second");
        assert!(is_watch_armed_in(tmp.path(), "%20"));
    }

    #[test]
    fn clear_watch_marker_removes_marker() {
        let tmp = tempfile::tempdir().expect("tempdir");
        arm_watch_in(tmp.path(), "%20").expect("arm");
        assert!(is_watch_armed_in(tmp.path(), "%20"));
        clear_watch_marker_in(tmp.path(), "%20").expect("clear");
        assert!(!is_watch_armed_in(tmp.path(), "%20"));
    }

    #[test]
    fn clear_watch_marker_noop_when_absent() {
        let tmp = tempfile::tempdir().expect("tempdir");
        clear_watch_marker_in(tmp.path(), "%99").expect("clear absent");
    }

    #[test]
    fn delete_state_removes_watch_marker_too() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_state_to(tmp.path(), "%21", "claude", AgentState::Snoozed).expect("write");
        arm_watch_in(tmp.path(), "%21").expect("arm");
        assert!(is_watch_armed_in(tmp.path(), "%21"));
        delete_state_in(tmp.path(), "%21").expect("delete");
        assert!(!is_watch_armed_in(tmp.path(), "%21"));
        assert!(!state_file_path_in(tmp.path(), "%21").exists());
    }

    // ── list_agent_infos ──────────────────────────────────────────────────────

    #[test]
    fn list_agent_infos_empty_dir() {
        let tmp = tempfile::tempdir().expect("tempdir");
        assert!(list_agent_infos_in(tmp.path()).is_empty());
    }

    #[test]
    fn list_agent_infos_returns_all_valid_files() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_state_to(tmp.path(), "%1", "claude", AgentState::Idle).unwrap();
        write_state_to(tmp.path(), "%2", "pi", AgentState::Working).unwrap();
        let infos = list_agent_infos_in(tmp.path());
        assert_eq!(infos.len(), 2);
    }
}
