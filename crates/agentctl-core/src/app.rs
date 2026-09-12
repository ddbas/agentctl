//! `App` — the composition target for every current CLI command: `list`,
//! `install`, `uninstall`, `dispatch_event`, `clear`, `info`. Depends only on
//! trait objects (`AgentRepository`, `Harness`); the binary crate is the
//! composition root that supplies concrete implementations.

use std::io::IsTerminal;

use anyhow::{Result, anyhow};
use chrono::Utc;

use agentctl_harness::{Harness, tilde_collapse};
use agentctl_logging::LogLevel;
use agentctl_state::{AgentRepository, Event, EventRecord, ToolCallType, apply_event};
use agentctl_tmux::{filter_agent_panes, load_state_map, tmux_panes, validate_state_files};

use crate::event::EventKind;
use crate::list::{Field, format_panes, sort_panes};

/// The application/use-case layer. Generic over neither trait — instead
/// holds a `&dyn AgentRepository` and a heterogeneous `Vec<Box<dyn Harness>>`
/// since the harness set is inherently dynamic (pi, Claude, and future
/// harnesses).
pub struct App<'a> {
    repo: &'a dyn AgentRepository,
    harnesses: Vec<Box<dyn Harness>>,
}

impl<'a> App<'a> {
    /// Construct an `App` from a repository and the set of supported
    /// harnesses.
    pub fn new(repo: &'a dyn AgentRepository, harnesses: Vec<Box<dyn Harness>>) -> Self {
        Self { repo, harnesses }
    }

    /// `agentctl list` — enumerate tmux panes running known agent harness
    /// processes, resolve their state, sort, and render.
    ///
    /// Resolves the tmux server socket in priority order:
    /// 1. `server`
    /// 2. `AGENTCTL_TMUX_SOCKET` environment variable (used by the test harness)
    /// 3. Default tmux server (no `-S` flag)
    pub fn list(
        &self,
        server: Option<String>,
        sort_by: Field,
        reverse: bool,
        columns: Vec<Field>,
    ) -> Result<String> {
        let server = server.or_else(|| std::env::var("AGENTCTL_TMUX_SOCKET").ok());

        let panes = tmux_panes(server.as_deref())?;
        let mut agent_panes = filter_agent_panes(&panes);

        let mut state_map = load_state_map(self.repo);
        validate_state_files(self.repo, &mut state_map, &panes);

        sort_panes(&mut agent_panes, sort_by, reverse, &state_map);

        let use_color = std::io::stdout().is_terminal();
        Ok(format_panes(&agent_panes, &state_map, use_color, &columns))
    }

    /// `agentctl install <harness>` — install agentctl's integration for the
    /// named harness.
    pub fn install(&self, harness_name: &str) -> Result<String> {
        Ok(self.find_harness(harness_name)?.install()?.to_string())
    }

    /// `agentctl uninstall <harness>` — remove agentctl's integration for the
    /// named harness.
    pub fn uninstall(&self, harness_name: &str) -> Result<String> {
        Ok(self.find_harness(harness_name)?.uninstall()?.to_string())
    }

    fn find_harness(&self, name: &str) -> Result<&dyn Harness> {
        self.harnesses
            .iter()
            .find(|h| h.name() == name)
            .map(|h| h.as_ref())
            .ok_or_else(|| anyhow!("unknown harness: {name}"))
    }

    /// `agentctl agent event <NAME>` — dispatch a typed event, deriving the
    /// pane's status via the status FSM and appending the raw event to
    /// `events.jsonl`.
    ///
    /// Resolution order for pane ID:
    ///   1. `pane`
    ///   2. `$TMUX_PANE` environment variable
    ///   3. Exit 0 silently (not inside tmux)
    pub fn dispatch_event(
        &self,
        kind: EventKind,
        id: String,
        tool_call_type: Option<ToolCallType>,
        harness: Option<String>,
        pane: Option<String>,
    ) -> Result<()> {
        let pane_id = match pane.or_else(|| std::env::var("TMUX_PANE").ok()) {
            Some(id) if !id.is_empty() => id,
            _ => return Ok(()), // not in tmux — exit 0 silently
        };

        let harness_str = harness.unwrap_or_else(|| "unknown".to_owned());

        let current_info = self.repo.read_agent_info(&pane_id);
        let current_status = current_info
            .as_ref()
            .map(|info| info.status)
            .unwrap_or_default();
        let current_active_tools = current_info
            .as_ref()
            .map(|info| info.active_tools)
            .unwrap_or(0);

        let new_active_tools = match kind {
            EventKind::SessionStarted => 0,
            EventKind::ToolCallStart => current_active_tools + 1,
            EventKind::ToolCallResult => current_active_tools.saturating_sub(1),
            EventKind::RunFinished {
                active_tools: Some(n),
            } => n,
            EventKind::RunFinished { active_tools: None } | EventKind::RunStarted => {
                current_active_tools
            }
        };

        let event = match kind {
            EventKind::SessionStarted => Event::SessionStarted,
            EventKind::RunStarted => Event::RunStarted,
            EventKind::RunFinished { .. } => Event::RunFinished {
                active_tools: new_active_tools,
            },
            EventKind::ToolCallStart => Event::ToolCallStart {
                r#type: tool_call_type,
            },
            EventKind::ToolCallResult => Event::ToolCallResult {
                r#type: tool_call_type,
            },
        };
        let new_status = apply_event(current_status, &event);

        self.repo
            .write_state_with_tools(&pane_id, &harness_str, new_status, new_active_tools)?;

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
        self.repo.append_event(&pane_id, &record)?;

        Ok(())
    }

    /// `agentctl agent clear` — whole-session cleanup for the current agent
    /// pane.
    ///
    /// Resolution order for pane ID:
    ///   1. `pane`
    ///   2. `$TMUX_PANE` environment variable
    ///   3. Exit 0 silently (not inside tmux)
    pub fn clear(&self, pane: Option<String>) -> Result<()> {
        let pane_id = match pane.or_else(|| std::env::var("TMUX_PANE").ok()) {
            Some(id) if !id.is_empty() => id,
            _ => return Ok(()), // not in tmux — exit 0 silently
        };

        // delete_state is a no-op when absent.
        self.repo.delete_state(&pane_id)?;
        Ok(())
    }

    /// `agentctl info` — resolved paths (log directory, current log file)
    /// and metadata (version, effective log level) as `key: value` lines.
    /// Performs no filesystem writes.
    pub fn info(&self, log_level: LogLevel) -> Result<String> {
        let version = env!("CARGO_PKG_VERSION");

        let log_level_str = match log_level {
            LogLevel::Trace => "trace",
            LogLevel::Debug => "debug",
            LogLevel::Info => "info",
            LogLevel::Warn => "warn",
            LogLevel::Error => "error",
        };

        let log_dir = agentctl_logging::log_dir("agentctl")
            .ok_or_else(|| anyhow!("failed to resolve log directory"))?;
        let current_log_file = agentctl_logging::current_log_file(&log_dir, "agentctl");

        let log_dir_tilde = tilde_collapse(&log_dir);
        let current_log_file_tilde = tilde_collapse(&current_log_file);

        Ok(format!(
            "version: {version}\nlog level: {log_level_str}\nlog directory: {log_dir_tilde}\ncurrent log file: {current_log_file_tilde}\n"
        ))
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use agentctl_harness::{InstallSummary, UninstallSummary};
    use agentctl_state::AgentState;
    use agentctl_state_fs::FileAgentRepository;

    fn app(repo: &dyn AgentRepository) -> App<'_> {
        App::new(repo, Vec::new())
    }

    fn app_with_harnesses<'a>(
        repo: &'a dyn AgentRepository,
        harnesses: Vec<Box<dyn Harness>>,
    ) -> App<'a> {
        App::new(repo, harnesses)
    }

    // ── dispatch_event ────────────────────────────────────────────────────────

    #[test]
    fn session_started_writes_idle() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let repo = FileAgentRepository::new(tmp.path().to_path_buf());
        let a = app(&repo);
        a.dispatch_event(
            EventKind::SessionStarted,
            "s1".to_owned(),
            None,
            Some("claude".to_owned()),
            Some("%42".to_owned()),
        )
        .expect("run");
        assert_eq!(
            repo.read_agent_info("%42").map(|i| i.status),
            Some(AgentState::Idle)
        );
    }

    #[test]
    fn run_started_moves_idle_to_working() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let repo = FileAgentRepository::new(tmp.path().to_path_buf());
        let a = app(&repo);
        a.dispatch_event(
            EventKind::SessionStarted,
            "s1".to_owned(),
            None,
            None,
            Some("%1".to_owned()),
        )
        .expect("session started");
        a.dispatch_event(
            EventKind::RunStarted,
            "r1".to_owned(),
            None,
            None,
            Some("%1".to_owned()),
        )
        .expect("run started");
        assert_eq!(
            repo.read_agent_info("%1").map(|i| i.status),
            Some(AgentState::Working)
        );
    }

    #[test]
    fn no_pane_exits_silently_and_writes_nothing() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let repo = FileAgentRepository::new(tmp.path().to_path_buf());
        let a = app(&repo);
        // Without TMUX_PANE set and no `pane` override, dispatch_event is a
        // silent no-op (won't touch the repo unless the test process itself
        // happens to be running inside tmux).
        let result = a.dispatch_event(EventKind::SessionStarted, "s1".to_owned(), None, None, None);
        assert!(result.is_ok());
    }

    #[test]
    fn event_is_appended_to_events_log() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let repo = FileAgentRepository::new(tmp.path().to_path_buf());
        let a = app(&repo);
        a.dispatch_event(
            EventKind::SessionStarted,
            "s1".to_owned(),
            None,
            Some("claude".to_owned()),
            Some("%7".to_owned()),
        )
        .expect("run");
        let events_path = tmp
            .path()
            .join("agentctl")
            .join("agents")
            .join("%7")
            .join("events.jsonl");
        assert!(events_path.exists());
        let content = std::fs::read_to_string(&events_path).unwrap();
        assert!(content.contains("SESSION_STARTED"));
    }

    #[test]
    fn tool_call_start_increments_active_tools() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let repo = FileAgentRepository::new(tmp.path().to_path_buf());
        let a = app(&repo);
        a.dispatch_event(
            EventKind::ToolCallStart,
            "t1".to_owned(),
            None,
            Some("pi".to_owned()),
            Some("%1".to_owned()),
        )
        .expect("run");
        let info = repo.read_agent_info("%1").expect("info");
        assert_eq!(info.active_tools, 1);
    }

    #[test]
    fn tool_call_result_decrements_active_tools() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let repo = FileAgentRepository::new(tmp.path().to_path_buf());
        let a = app(&repo);
        a.dispatch_event(
            EventKind::ToolCallStart,
            "t1".to_owned(),
            None,
            Some("pi".to_owned()),
            Some("%1".to_owned()),
        )
        .expect("start");
        a.dispatch_event(
            EventKind::ToolCallStart,
            "t2".to_owned(),
            None,
            Some("pi".to_owned()),
            Some("%1".to_owned()),
        )
        .expect("start 2");
        a.dispatch_event(
            EventKind::ToolCallResult,
            "t1".to_owned(),
            None,
            Some("pi".to_owned()),
            Some("%1".to_owned()),
        )
        .expect("result");
        let info = repo.read_agent_info("%1").expect("info");
        assert_eq!(info.active_tools, 1);
    }

    #[test]
    fn tool_call_result_on_zero_count_stays_zero() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let repo = FileAgentRepository::new(tmp.path().to_path_buf());
        let a = app(&repo);
        a.dispatch_event(
            EventKind::ToolCallResult,
            "t1".to_owned(),
            None,
            Some("pi".to_owned()),
            Some("%1".to_owned()),
        )
        .expect("result");
        let info = repo.read_agent_info("%1").expect("info");
        assert_eq!(info.active_tools, 0);
    }

    #[test]
    fn multiple_concurrent_tool_call_starts_accumulate() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let repo = FileAgentRepository::new(tmp.path().to_path_buf());
        let a = app(&repo);
        for id in ["t1", "t2", "t3"] {
            a.dispatch_event(
                EventKind::ToolCallStart,
                id.to_owned(),
                None,
                Some("pi".to_owned()),
                Some("%1".to_owned()),
            )
            .expect("start");
        }
        let info = repo.read_agent_info("%1").expect("info");
        assert_eq!(info.active_tools, 3);
    }

    #[test]
    fn custom_type_tool_call_still_increments_and_decrements() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let repo = FileAgentRepository::new(tmp.path().to_path_buf());
        let a = app(&repo);
        a.dispatch_event(
            EventKind::ToolCallStart,
            "t1".to_owned(),
            Some(ToolCallType::Custom),
            Some("pi".to_owned()),
            Some("%1".to_owned()),
        )
        .expect("start");
        let info = repo.read_agent_info("%1").expect("info");
        assert_eq!(info.active_tools, 1);
        a.dispatch_event(
            EventKind::ToolCallResult,
            "t1".to_owned(),
            Some(ToolCallType::Custom),
            Some("pi".to_owned()),
            Some("%1".to_owned()),
        )
        .expect("result");
        let info = repo.read_agent_info("%1").expect("info");
        assert_eq!(info.active_tools, 0);
    }

    #[test]
    fn session_started_resets_nonzero_active_tools_to_zero() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let repo = FileAgentRepository::new(tmp.path().to_path_buf());
        let a = app(&repo);
        a.dispatch_event(
            EventKind::ToolCallStart,
            "t1".to_owned(),
            None,
            Some("pi".to_owned()),
            Some("%1".to_owned()),
        )
        .expect("start");
        a.dispatch_event(
            EventKind::ToolCallStart,
            "t2".to_owned(),
            None,
            Some("pi".to_owned()),
            Some("%1".to_owned()),
        )
        .expect("start 2");
        a.dispatch_event(
            EventKind::SessionStarted,
            "s1".to_owned(),
            None,
            Some("pi".to_owned()),
            Some("%1".to_owned()),
        )
        .expect("session started");
        let info = repo.read_agent_info("%1").expect("info");
        assert_eq!(info.active_tools, 0);
    }

    #[test]
    fn elicitation_tool_call_start_blocks_status_and_increments_active_tools() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let repo = FileAgentRepository::new(tmp.path().to_path_buf());
        let a = app(&repo);
        a.dispatch_event(
            EventKind::SessionStarted,
            "s1".to_owned(),
            None,
            Some("pi".to_owned()),
            Some("%1".to_owned()),
        )
        .expect("session started");
        a.dispatch_event(
            EventKind::RunStarted,
            "r1".to_owned(),
            None,
            Some("pi".to_owned()),
            Some("%1".to_owned()),
        )
        .expect("run started");
        a.dispatch_event(
            EventKind::ToolCallStart,
            "t1".to_owned(),
            Some(ToolCallType::Elicitation),
            Some("pi".to_owned()),
            Some("%1".to_owned()),
        )
        .expect("tool call start");
        let info = repo.read_agent_info("%1").expect("info");
        assert_eq!(info.status, AgentState::Blocked);
        assert_eq!(info.active_tools, 1);
    }

    #[test]
    fn run_finished_active_tools_override_reconciles_counter() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let repo = FileAgentRepository::new(tmp.path().to_path_buf());
        let a = app(&repo);
        a.dispatch_event(
            EventKind::SessionStarted,
            "s1".to_owned(),
            None,
            Some("claude".to_owned()),
            Some("%1".to_owned()),
        )
        .expect("session started");
        a.dispatch_event(
            EventKind::ToolCallStart,
            "t1".to_owned(),
            None,
            Some("claude".to_owned()),
            Some("%1".to_owned()),
        )
        .expect("tool call start");
        a.dispatch_event(
            EventKind::ToolCallResult,
            "t1".to_owned(),
            None,
            Some("claude".to_owned()),
            Some("%1".to_owned()),
        )
        .expect("tool call result");
        a.dispatch_event(
            EventKind::RunFinished {
                active_tools: Some(1),
            },
            "r1".to_owned(),
            None,
            Some("claude".to_owned()),
            Some("%1".to_owned()),
        )
        .expect("run finished");
        let info = repo.read_agent_info("%1").expect("info");
        assert_eq!(info.active_tools, 1);
    }

    #[test]
    fn run_finished_without_active_tools_leaves_counter_untouched() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let repo = FileAgentRepository::new(tmp.path().to_path_buf());
        let a = app(&repo);
        a.dispatch_event(
            EventKind::SessionStarted,
            "s1".to_owned(),
            None,
            Some("claude".to_owned()),
            Some("%1".to_owned()),
        )
        .expect("session started");
        a.dispatch_event(
            EventKind::ToolCallStart,
            "t1".to_owned(),
            None,
            Some("claude".to_owned()),
            Some("%1".to_owned()),
        )
        .expect("tool call start");
        a.dispatch_event(
            EventKind::RunFinished { active_tools: None },
            "r1".to_owned(),
            None,
            Some("claude".to_owned()),
            Some("%1".to_owned()),
        )
        .expect("run finished");
        let info = repo.read_agent_info("%1").expect("info");
        assert_eq!(info.active_tools, 1);
    }

    #[test]
    fn pane_flag_overrides_env_pane() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let repo = FileAgentRepository::new(tmp.path().to_path_buf());
        let a = app(&repo);
        a.dispatch_event(
            EventKind::SessionStarted,
            "s1".to_owned(),
            None,
            None,
            Some("%99".to_owned()),
        )
        .expect("run");
        assert_eq!(
            repo.read_agent_info("%99").map(|i| i.status),
            Some(AgentState::Idle)
        );
        assert!(repo.read_agent_info("%1").is_none());
    }

    #[test]
    fn tool_call_result_without_type_decrements_after_a_failed_call() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let repo = FileAgentRepository::new(tmp.path().to_path_buf());
        let a = app(&repo);
        a.dispatch_event(
            EventKind::ToolCallStart,
            "t1".to_owned(),
            None,
            Some("claude".to_owned()),
            Some("%1".to_owned()),
        )
        .expect("tool call start");
        a.dispatch_event(
            EventKind::ToolCallResult,
            "t1".to_owned(),
            None,
            Some("claude".to_owned()),
            Some("%1".to_owned()),
        )
        .expect("tool call result (failure path)");
        let info = repo.read_agent_info("%1").expect("info");
        assert_eq!(info.active_tools, 0);
    }

    #[test]
    fn run_finished_with_nonzero_active_tools_leaves_status_unchanged() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let repo = FileAgentRepository::new(tmp.path().to_path_buf());
        let a = app(&repo);
        a.dispatch_event(
            EventKind::SessionStarted,
            "s1".to_owned(),
            None,
            Some("claude".to_owned()),
            Some("%1".to_owned()),
        )
        .expect("session started");
        a.dispatch_event(
            EventKind::RunStarted,
            "r1".to_owned(),
            None,
            Some("claude".to_owned()),
            Some("%1".to_owned()),
        )
        .expect("run started");
        a.dispatch_event(
            EventKind::RunFinished {
                active_tools: Some(1),
            },
            "r1".to_owned(),
            None,
            Some("claude".to_owned()),
            Some("%1".to_owned()),
        )
        .expect("run finished");
        let info = repo.read_agent_info("%1").expect("info");
        assert_eq!(
            info.status,
            AgentState::Working,
            "still busy — status unchanged"
        );
        assert_eq!(info.active_tools, 1);
    }

    #[test]
    fn run_finished_from_blocked_with_zero_active_tools_drives_done() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let repo = FileAgentRepository::new(tmp.path().to_path_buf());
        let a = app(&repo);
        a.dispatch_event(
            EventKind::SessionStarted,
            "s1".to_owned(),
            None,
            Some("claude".to_owned()),
            Some("%1".to_owned()),
        )
        .expect("session started");
        a.dispatch_event(
            EventKind::RunStarted,
            "r1".to_owned(),
            None,
            Some("claude".to_owned()),
            Some("%1".to_owned()),
        )
        .expect("run started");
        a.dispatch_event(
            EventKind::ToolCallStart,
            "t1".to_owned(),
            Some(ToolCallType::Elicitation),
            Some("claude".to_owned()),
            Some("%1".to_owned()),
        )
        .expect("elicitation open (stale block, never resolved)");
        a.dispatch_event(
            EventKind::RunFinished {
                active_tools: Some(0),
            },
            "r1".to_owned(),
            None,
            Some("claude".to_owned()),
            Some("%1".to_owned()),
        )
        .expect("run finished");
        let info = repo.read_agent_info("%1").expect("info");
        assert_eq!(info.status, AgentState::Done);
    }

    // ── clear ─────────────────────────────────────────────────────────────────

    #[test]
    fn clear_state_removes_existing_file_via_pane_flag() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let repo = FileAgentRepository::new(tmp.path().to_path_buf());
        repo.write_state_with_tools("%10", "pi", AgentState::Idle, 0)
            .expect("write");
        let a = app(&repo);
        assert!(repo.read_agent_info("%10").is_some());
        a.clear(Some("%10".to_owned())).expect("clear");
        assert!(repo.read_agent_info("%10").is_none());
    }

    #[test]
    fn clear_state_absent_file_is_noop() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let repo = FileAgentRepository::new(tmp.path().to_path_buf());
        let a = app(&repo);
        a.clear(Some("%10".to_owned())).expect("clear absent");
    }

    #[test]
    fn clear_state_no_pane_returns_ok() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let repo = FileAgentRepository::new(tmp.path().to_path_buf());
        let a = app(&repo);
        let result = a.clear(None);
        assert!(result.is_ok());
    }

    // ── install / uninstall dispatch ─────────────────────────────────────────

    struct StubHarness {
        name: &'static str,
    }

    impl Harness for StubHarness {
        fn name(&self) -> &'static str {
            self.name
        }

        fn install(&self) -> Result<InstallSummary> {
            Ok(InstallSummary(format!("installed {}", self.name)))
        }

        fn uninstall(&self) -> Result<UninstallSummary> {
            Ok(UninstallSummary(format!("uninstalled {}", self.name)))
        }
    }

    #[test]
    fn install_dispatches_to_matching_harness_by_name() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let repo = FileAgentRepository::new(tmp.path().to_path_buf());
        let harnesses: Vec<Box<dyn Harness>> = vec![
            Box::new(StubHarness { name: "pi" }),
            Box::new(StubHarness { name: "claude" }),
        ];
        let a = app_with_harnesses(&repo, harnesses);
        let out = a.install("claude").expect("install");
        assert_eq!(out, "installed claude");
    }

    #[test]
    fn uninstall_dispatches_to_matching_harness_by_name() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let repo = FileAgentRepository::new(tmp.path().to_path_buf());
        let harnesses: Vec<Box<dyn Harness>> = vec![Box::new(StubHarness { name: "pi" })];
        let a = app_with_harnesses(&repo, harnesses);
        let out = a.uninstall("pi").expect("uninstall");
        assert_eq!(out, "uninstalled pi");
    }

    #[test]
    fn install_unknown_harness_is_an_error() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let repo = FileAgentRepository::new(tmp.path().to_path_buf());
        let a = app(&repo);
        assert!(a.install("nonexistent").is_err());
    }

    // ── info ──────────────────────────────────────────────────────────────────

    #[test]
    fn info_reports_version_and_log_level() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let repo = FileAgentRepository::new(tmp.path().to_path_buf());
        let a = app(&repo);
        let out = a.info(LogLevel::Info).expect("info");
        assert!(out.contains("version:"));
        assert!(out.contains("log level: info"));
        assert!(out.contains("log directory:"));
        assert!(out.contains("current log file:"));
    }
}
