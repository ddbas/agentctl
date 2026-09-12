//! `agentctl-state-fs` — the sole crate that reads or writes agent-state
//! persistence (`state.json` / `events.jsonl`) on disk. Implements
//! `agentctl_state::AgentRepository`.
//!
//! State files are stored at:
//!   `$XDG_DATA_HOME/agentctl/agents/<pane-id>/state.json`
//! Events are appended to:
//!   `$XDG_DATA_HOME/agentctl/agents/<pane-id>/events.jsonl`
//! where `XDG_DATA_HOME` defaults to `~/.local/share`.

use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::Utc;

use agentctl_state::{AgentInfo, AgentRepository, AgentState, EventRecord};

// ── XDG helpers ───────────────────────────────────────────────────────────────

/// Resolve the XDG data home directory.
///
/// Returns `$XDG_DATA_HOME` if set and non-empty, otherwise `~/.local/share`.
fn xdg_data_home() -> PathBuf {
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
/// `<base>/agentctl/agents/<pane-id>/`
fn agent_dir_in(base: &Path, pane_id: &str) -> PathBuf {
    base.join("agentctl").join("agents").join(pane_id)
}

/// Return the path of the state file for the given pane under `base`.
fn state_file_path_in(base: &Path, pane_id: &str) -> PathBuf {
    agent_dir_in(base, pane_id).join("state.json")
}

/// Return the path of the events log file for the given pane under `base`.
fn events_file_path_in(base: &Path, pane_id: &str) -> PathBuf {
    agent_dir_in(base, pane_id).join("events.jsonl")
}

// ── FileAgentRepository ───────────────────────────────────────────────────────

/// A plain, explicitly-constructed `AgentRepository` backed by the
/// filesystem. Not a lazy-initialized singleton — callers choose the base
/// directory via [`FileAgentRepository::new`] or [`FileAgentRepository::xdg`].
pub struct FileAgentRepository {
    base: PathBuf,
}

impl FileAgentRepository {
    /// Construct a repository rooted at an explicit `base` directory.
    pub fn new(base: PathBuf) -> Self {
        Self { base }
    }

    /// Construct a repository rooted at the XDG data home
    /// (`$XDG_DATA_HOME`, defaulting to `~/.local/share`).
    pub fn xdg() -> Self {
        Self {
            base: xdg_data_home(),
        }
    }
}

impl AgentRepository for FileAgentRepository {
    fn read_agent_info(&self, pane_id: &str) -> Option<AgentInfo> {
        let path = state_file_path_in(&self.base, pane_id);
        let bytes = std::fs::read(&path).ok()?;
        serde_json::from_slice::<AgentInfo>(&bytes).ok()
    }

    fn write_state_with_tools(
        &self,
        pane_id: &str,
        harness: &str,
        status: AgentState,
        active_tools: u32,
    ) -> Result<()> {
        let dir = agent_dir_in(&self.base, pane_id);
        std::fs::create_dir_all(&dir)
            .with_context(|| format!("create state dir: {}", dir.display()))?;

        let info = AgentInfo {
            pane_id: pane_id.to_owned(),
            harness: harness.to_owned(),
            status,
            updated_at: Utc::now(),
            active_tools,
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

    fn delete_state(&self, pane_id: &str) -> Result<()> {
        let dir = agent_dir_in(&self.base, pane_id);
        if dir.exists() {
            std::fs::remove_dir_all(&dir)
                .with_context(|| format!("remove agent dir: {}", dir.display()))?;
        }
        Ok(())
    }

    fn list_agent_infos(&self) -> Vec<AgentInfo> {
        let agents_dir = self.base.join("agentctl").join("agents");
        let read_dir = match std::fs::read_dir(&agents_dir) {
            Ok(agentctl) => agentctl,
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

    fn append_event(&self, pane_id: &str, record: &EventRecord) -> Result<()> {
        let dir = agent_dir_in(&self.base, pane_id);
        std::fs::create_dir_all(&dir)
            .with_context(|| format!("create state dir: {}", dir.display()))?;

        let line = serde_json::to_string(record).context("serialise EventRecord")?;

        let path = events_file_path_in(&self.base, pane_id);
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&path)
            .with_context(|| format!("open events log: {}", path.display()))?;

        writeln!(file, "{line}")
            .with_context(|| format!("append to events log: {}", path.display()))?;

        Ok(())
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn repo(base: &Path) -> FileAgentRepository {
        FileAgentRepository::new(base.to_path_buf())
    }

    // ── write / read round-trip ───────────────────────────────────────────────

    #[test]
    fn write_and_read_state_round_trip() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let r = repo(tmp.path());
        r.write_state_with_tools("%12", "claude", AgentState::Working, 0)
            .expect("write_state");
        assert_eq!(
            r.read_agent_info("%12").map(|i| i.status),
            Some(AgentState::Working)
        );
    }

    #[test]
    fn write_overwrites_previous_state() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let r = repo(tmp.path());
        r.write_state_with_tools("%12", "claude", AgentState::Working, 0)
            .expect("write first");
        r.write_state_with_tools("%12", "claude", AgentState::Done, 0)
            .expect("write second");
        assert_eq!(
            r.read_agent_info("%12").map(|i| i.status),
            Some(AgentState::Done)
        );
    }

    #[test]
    fn write_state_persists_harness_field() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let r = repo(tmp.path());
        r.write_state_with_tools("%5", "pi", AgentState::Idle, 0)
            .expect("write_state");
        let info = r.read_agent_info("%5").expect("read info");
        assert_eq!(info.harness, "pi");
        assert_eq!(info.pane_id, "%5");
    }

    // ── read missing / corrupt ─────────────────────────────────────────

    #[test]
    fn read_agent_info_missing_file_returns_none() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let r = repo(tmp.path());
        assert!(r.read_agent_info("%99").is_none());
    }

    #[test]
    fn read_agent_info_corrupt_file_returns_none() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let r = repo(tmp.path());
        let dir = tmp.path().join("agentctl").join("agents").join("%bad");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("state.json"), b"not-json").unwrap();
        assert!(r.read_agent_info("%bad").is_none());
    }

    // ── delete_state ──────────────────────────────────────────────────────────

    #[test]
    fn delete_state_removes_file_and_dir() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let r = repo(tmp.path());
        r.write_state_with_tools("%12", "claude", AgentState::Idle, 0)
            .expect("write");
        assert!(state_file_path_in(tmp.path(), "%12").exists());
        r.delete_state("%12").expect("delete");
        assert!(!agent_dir_in(tmp.path(), "%12").exists());
    }

    #[test]
    fn delete_state_noop_when_absent() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let r = repo(tmp.path());
        r.delete_state("%99").expect("delete absent");
    }

    // ── list_agent_infos ──────────────────────────────────────────────────────

    #[test]
    fn list_agent_infos_empty_dir() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let r = repo(tmp.path());
        assert!(r.list_agent_infos().is_empty());
    }

    #[test]
    fn list_agent_infos_returns_all_valid_files() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let r = repo(tmp.path());
        r.write_state_with_tools("%1", "claude", AgentState::Idle, 0)
            .unwrap();
        r.write_state_with_tools("%2", "pi", AgentState::Working, 0)
            .unwrap();
        assert_eq!(r.list_agent_infos().len(), 2);
    }

    // ── active_tools ──────────────────────────────────────────────────────────

    #[test]
    fn write_and_read_active_tools_round_trip() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let r = repo(tmp.path());
        r.write_state_with_tools("%12", "claude", AgentState::Working, 3)
            .expect("write_state_with_tools");
        let info = r.read_agent_info("%12").expect("read info");
        assert_eq!(info.active_tools, 3);
    }

    #[test]
    fn write_state_defaults_active_tools_to_zero() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let r = repo(tmp.path());
        r.write_state_with_tools("%13", "claude", AgentState::Idle, 0)
            .expect("write_state");
        let info = r.read_agent_info("%13").expect("read info");
        assert_eq!(info.active_tools, 0);
    }

    #[test]
    fn legacy_state_file_without_active_tools_deserializes_as_zero() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let r = repo(tmp.path());
        let dir = tmp.path().join("agentctl").join("agents").join("%legacy");
        std::fs::create_dir_all(&dir).unwrap();
        let legacy_json = r#"{
            "pane_id": "%legacy",
            "harness": "claude",
            "status": "idle",
            "updated_at": "2024-01-01T00:00:00Z"
        }"#;
        std::fs::write(dir.join("state.json"), legacy_json).unwrap();
        let info = r.read_agent_info("%legacy").expect("read legacy info");
        assert_eq!(info.active_tools, 0);
    }

    // ── blocked_prompts (legacy field) ─────────────────────────────────

    #[test]
    fn legacy_state_file_with_leftover_blocked_prompts_key_still_deserializes() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let r = repo(tmp.path());
        let dir = tmp.path().join("agentctl").join("agents").join("%legacy2");
        std::fs::create_dir_all(&dir).unwrap();
        let legacy_json = r#"{
            "pane_id": "%legacy2",
            "harness": "claude",
            "status": "idle",
            "updated_at": "2024-01-01T00:00:00Z",
            "active_tools": 1,
            "blocked_prompts": 2
        }"#;
        std::fs::write(dir.join("state.json"), legacy_json).unwrap();
        let info = r.read_agent_info("%legacy2").expect("read legacy info");
        assert_eq!(info.active_tools, 1);
    }

    // ── events log ──────────────────────────────────────────────────────────

    fn record(event: &str, id: &str) -> EventRecord {
        EventRecord {
            event: event.to_owned(),
            id: id.to_owned(),
            r#type: None,
            harness: "claude".to_owned(),
            pane_id: "%12".to_owned(),
            timestamp: Utc::now(),
        }
    }

    #[test]
    fn first_event_creates_file_with_one_line() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let r = repo(tmp.path());
        let path = events_file_path_in(tmp.path(), "%12");
        assert!(!path.exists());

        r.append_event("%12", &record("SESSION_STARTED", "s1"))
            .expect("append");

        assert!(path.exists());
        let content = std::fs::read_to_string(&path).expect("read");
        assert_eq!(content.lines().count(), 1);
        assert!(content.contains("SESSION_STARTED"));
    }

    #[test]
    fn second_event_appends_without_truncating() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let r = repo(tmp.path());
        r.append_event("%12", &record("SESSION_STARTED", "s1"))
            .expect("append 1");
        r.append_event("%12", &record("RUN_STARTED", "r1"))
            .expect("append 2");

        let path = events_file_path_in(tmp.path(), "%12");
        let content = std::fs::read_to_string(&path).expect("read");
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("SESSION_STARTED"));
        assert!(lines[1].contains("RUN_STARTED"));
    }

    #[test]
    fn creates_dir_when_absent() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let r = repo(tmp.path());
        let dir = agent_dir_in(tmp.path(), "%99");
        assert!(!dir.exists());

        r.append_event("%99", &record("SESSION_STARTED", "s1"))
            .expect("append");

        assert!(dir.exists());
    }

    #[test]
    fn type_field_included_when_present() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let r = repo(tmp.path());
        let mut rec = record("TOOL_CALL_START", "t1");
        rec.r#type = Some("elicitation".to_owned());
        r.append_event("%12", &rec).expect("append");

        let path = events_file_path_in(tmp.path(), "%12");
        let content = std::fs::read_to_string(&path).expect("read");
        assert!(content.contains(r#""type":"elicitation""#));
    }

    #[test]
    fn type_field_omitted_when_absent() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let r = repo(tmp.path());
        r.append_event("%12", &record("RUN_STARTED", "r1"))
            .expect("append");

        let path = events_file_path_in(tmp.path(), "%12");
        let content = std::fs::read_to_string(&path).expect("read");
        assert!(!content.contains(r#""type""#));
    }

    // ── constructors ──────────────────────────────────────────────────────────

    #[test]
    fn new_constructs_with_explicit_base() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let r = FileAgentRepository::new(tmp.path().to_path_buf());
        r.write_state_with_tools("%1", "claude", AgentState::Idle, 0)
            .unwrap();
        assert!(r.read_agent_info("%1").is_some());
    }
}
