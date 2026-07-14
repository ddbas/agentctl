//! Agent state tracking — shared types and file-based state store.
//!
//! State files are stored at:
//!   `$XDG_DATA_HOME/drovr/agents/<pane-id>/state.json`
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
    Idle,
    Working,
    Done,
    Blocked,
    #[default]
    Unknown,
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
/// `<base>/drovr/agents/<pane-id>/`
pub fn agent_dir_in(base: &Path, pane_id: &str) -> PathBuf {
    base.join("drovr").join("agents").join(pane_id)
}

/// Return the path of the state file for the given pane under `base`.
///
/// `<base>/drovr/agents/<pane-id>/state.json`
pub fn state_file_path_in(base: &Path, pane_id: &str) -> PathBuf {
    agent_dir_in(base, pane_id).join("state.json")
}

/// Return the directory for the given pane's state files using the XDG base.
pub fn agent_dir(pane_id: &str) -> PathBuf {
    agent_dir_in(&xdg_data_home(), pane_id)
}

/// Return the path of the state file for the given pane using the XDG base.
pub fn state_file_path(pane_id: &str) -> PathBuf {
    state_file_path_in(&xdg_data_home(), pane_id)
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

/// Glob all `state.json` files under `<base>/drovr/agents/` and return parsed
/// [`AgentInfo`] values. Files that cannot be read or parsed are skipped.
pub fn list_agent_infos_in(base: &Path) -> Vec<AgentInfo> {
    let agents_dir = base.join("drovr").join("agents");
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
        let dir = tmp.path().join("drovr").join("agents").join("%bad");
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

    // ── AgentState Display ────────────────────────────────────────────────────

    #[test]
    fn agent_state_display() {
        assert_eq!(AgentState::Idle.to_string(), "idle");
        assert_eq!(AgentState::Working.to_string(), "working");
        assert_eq!(AgentState::Done.to_string(), "done");
        assert_eq!(AgentState::Blocked.to_string(), "blocked");
        assert_eq!(AgentState::Unknown.to_string(), "unknown");
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
