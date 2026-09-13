//! `agentctl-tmux` — tmux/`ps`-based pane discovery, `node` → `pi` process
//! resolution, and agent-state validation against the live pane set.
//!
//! Shells out to `tmux list-panes -a` and (when needed) `ps` to resolve
//! `node` panes running the `pi` coding agent to their real agent name.

use std::collections::HashMap;
use std::io::ErrorKind;
use std::process::Command;

use anyhow::Result;

use agentctl_state::{AgentInfo, AgentRepository};

// ── Constants ─────────────────────────────────────────────────────────────────

/// Exact process names that are considered known agent harness processes.
const KNOWN_AGENTS: &[&str] = &["claude", "pi"];

// ── Data types ────────────────────────────────────────────────────────────────

/// A single tmux pane entry returned by `tmux list-panes -a`.
#[derive(Debug, Clone, PartialEq)]
pub struct PaneEntry {
    /// Foreground process name as reported by `#{pane_current_command}`.
    /// For `pi` panes this is initially `"node"` and is resolved to `"pi"`
    /// by [`resolve_node_agents`] after querying the process table.
    pub agent: String,
    /// Tmux session name.
    pub session: String,
    /// Window index within the session.
    pub window_index: u32,
    /// Pane index within the window.
    pub pane_index: u32,
    /// PID of the pane's shell process (used to resolve `node` → `pi`).
    pub pane_pid: u32,
    /// Tmux pane ID (e.g. `%12`), used to correlate with state files.
    pub pane_id: String,
}

// ── tmux interaction ──────────────────────────────────────────────────────────

/// Query every pane across all tmux sessions via `tmux list-panes -a`.
///
/// - Returns an empty [`Vec`] if no tmux server is running (tmux exits
///   non-zero).
/// - Returns an [`anyhow::Error`] containing "tmux not found" if the `tmux`
///   binary cannot be found on `PATH`.
///
/// If `server` is [`Some`] the value is forwarded as `tmux -S <socket>` so
/// callers can target a specific server socket.
pub fn tmux_panes(server: Option<&str>) -> Result<Vec<PaneEntry>> {
    let mut cmd = Command::new("tmux");

    if let Some(socket) = server {
        cmd.args(["-S", socket]);
    }

    cmd.args([
        "list-panes",
        "-a",
        "-F",
        "#{pane_current_command}\t#{session_name}\t#{window_index}\t#{pane_index}\t#{pane_pid}\t#{pane_id}",
    ]);

    let output = cmd.output().map_err(|e| {
        if e.kind() == ErrorKind::NotFound {
            anyhow::anyhow!("tmux not found — is tmux installed and on PATH?")
        } else {
            anyhow::Error::from(e).context("failed to run tmux")
        }
    })?;

    // Non-zero exit means no server running → treat as empty list
    if !output.status.success() {
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut panes = Vec::new();

    for line in stdout.lines() {
        let parts: Vec<&str> = line.splitn(6, '\t').collect();
        if parts.len() < 6 {
            continue;
        }
        let window_index = parts[2].parse::<u32>().unwrap_or(0);
        let pane_index = parts[3].parse::<u32>().unwrap_or(0);
        let pane_pid = parts[4].parse::<u32>().unwrap_or(0);
        panes.push(PaneEntry {
            agent: parts[0].to_owned(),
            session: parts[1].to_owned(),
            window_index,
            pane_index,
            pane_pid,
            pane_id: parts[5].to_owned(),
        });
    }

    resolve_node_agents(&mut panes);

    Ok(panes)
}

// ── node → pi resolution ─────────────────────────────────────────────────────

/// Build a map of parent-PID → argv[0] basenames of its direct children by
/// running `ps -A -o pid=,ppid=,args=` once.
///
/// Returns an empty map if `ps` cannot be run or exits non-zero (best-effort;
/// `node` panes simply remain unresolved in that case).
pub fn build_process_table() -> HashMap<u32, Vec<String>> {
    let output = Command::new("ps")
        .args(["-A", "-o", "pid=,ppid=,args="])
        .output();

    let output = match output {
        Ok(o) if o.status.success() => o,
        _ => return HashMap::new(),
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut table: HashMap<u32, Vec<String>> = HashMap::new();

    for line in stdout.lines() {
        let mut tokens = line.split_whitespace();
        let pid = tokens.next().and_then(|s| s.parse::<u32>().ok());
        let ppid = tokens.next().and_then(|s| s.parse::<u32>().ok());
        let argv0 = tokens.next().unwrap_or("");
        let basename = argv0.rsplit('/').next().unwrap_or(argv0);

        if let (Some(_pid), Some(ppid)) = (pid, ppid) {
            table.entry(ppid).or_default().push(basename.to_owned());
        }
    }

    table
}

/// Mutate any `PaneEntry` whose `agent == "node"` to the resolved agent name
/// when the pane's shell PID has a direct child whose argv[0] basename matches
/// a known agent (`pi`).
///
/// This is a pure function over a pre-built `table`; use
/// [`resolve_node_agents`] for the full workflow.
pub fn resolve_from_process_table(panes: &mut [PaneEntry], table: &HashMap<u32, Vec<String>>) {
    for pane in panes.iter_mut() {
        if pane.agent != "node" {
            continue;
        }
        if let Some(children) = table.get(&pane.pane_pid)
            && children.iter().any(|a| a == "pi")
        {
            pane.agent = "pi".to_owned();
        }
    }
}

/// Resolve `node` panes to their real agent name.
///
/// Skips the `ps` call entirely when no `node` panes are present.
fn resolve_node_agents(panes: &mut [PaneEntry]) {
    if !panes.iter().any(|p| p.agent == "node") {
        return;
    }
    let table = build_process_table();
    resolve_from_process_table(panes, &table);
}

// ── Filtering ─────────────────────────────────────────────────────────────────

/// Keep only panes whose `agent` exactly matches a known harness name.
pub fn filter_agent_panes(panes: &[PaneEntry]) -> Vec<&PaneEntry> {
    panes
        .iter()
        .filter(|p| KNOWN_AGENTS.contains(&p.agent.as_str()))
        .collect()
}

// ── State file validation / stale cleanup ─────────────────────────────────────

/// Build a `pane_id → AgentInfo` lookup map from every agent-state entry
/// known to `repo`.
pub fn load_state_map(repo: &dyn AgentRepository) -> HashMap<String, AgentInfo> {
    repo.list_agent_infos()
        .into_iter()
        .map(|info| (info.pane_id.clone(), info))
        .collect()
}

/// Validate state entries against the live pane set and delete stale ones
/// via `repo`.
///
/// A state entry is deleted when:
///   - Its `pane_id` is not in the live pane set (orphaned).
///   - Its `pane_id` is in the live set **and** the live pane's
///     `pane_current_command` does not match `harness`, **and** `harness` is
///     not `"unknown"` (contradictory state from a different harness).
///
/// Entries whose `harness` is `"unknown"` are only deleted when their pane no
/// longer exists — the harness-match check is skipped.
pub fn validate_state_files(
    repo: &dyn AgentRepository,
    state_map: &mut HashMap<String, AgentInfo>,
    live_panes: &[PaneEntry],
) {
    let live: HashMap<&str, &str> = live_panes
        .iter()
        .map(|p| (p.pane_id.as_str(), p.agent.as_str()))
        .collect();

    let stale_ids: Vec<String> = state_map
        .values()
        .filter(|info| match live.get(info.pane_id.as_str()) {
            None => true,
            Some(cmd) => info.harness != "unknown" && *cmd != info.harness.as_str(),
        })
        .map(|info| info.pane_id.clone())
        .collect();

    for pane_id in &stale_ids {
        let _ = repo.delete_state(pane_id);
        state_map.remove(pane_id);
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use agentctl_state::AgentState;
    use agentctl_state_fs::FileAgentRepository;

    fn pane(agent: &str, session: &str, window: u32, pane: u32) -> PaneEntry {
        PaneEntry {
            agent: agent.to_owned(),
            session: session.to_owned(),
            window_index: window,
            pane_index: pane,
            pane_pid: 0,
            pane_id: format!("%{window}{pane}"),
        }
    }

    fn pane_with_id(agent: &str, session: &str, window: u32, p: u32, id: &str) -> PaneEntry {
        PaneEntry {
            agent: agent.to_owned(),
            session: session.to_owned(),
            window_index: window,
            pane_index: p,
            pane_pid: 0,
            pane_id: id.to_owned(),
        }
    }

    // ── filter_agent_panes ─────────────────────────────────────────────────

    #[test]
    fn filter_keeps_claude_and_pi_drops_others() {
        let panes = vec![
            pane("claude", "dev", 0, 0),
            pane("pi", "work", 1, 0),
            pane("zsh", "dev", 0, 1),
            pane("nvim", "dev", 0, 2),
            pane("bash", "other", 2, 0),
        ];
        let filtered = filter_agent_panes(&panes);
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].agent, "claude");
        assert_eq!(filtered[1].agent, "pi");
    }

    #[test]
    fn filter_empty_input_returns_empty() {
        let panes: Vec<PaneEntry> = vec![];
        assert!(filter_agent_panes(&panes).is_empty());
    }

    #[test]
    fn filter_excludes_near_matches() {
        let panes = vec![
            pane("pilot", "dev", 0, 0),
            pane("Claude", "dev", 0, 1),
            pane("PI", "dev", 0, 2),
            pane("claude2", "dev", 0, 3),
            pane("xpi", "dev", 0, 4),
            pane("", "dev", 0, 5),
        ];
        let filtered = filter_agent_panes(&panes);
        assert!(
            filtered.is_empty(),
            "expected no matches, got: {:?}",
            filtered
        );
    }

    // ── resolve_from_process_table ───────────────────────────────────────────

    fn make_table(entries: &[(u32, &str)]) -> HashMap<u32, Vec<String>> {
        let mut map: HashMap<u32, Vec<String>> = HashMap::new();
        for (ppid, argv0) in entries {
            map.entry(*ppid).or_default().push(argv0.to_string());
        }
        map
    }

    #[test]
    fn resolve_node_pane_with_pi_child_becomes_pi() {
        let mut panes = vec![PaneEntry {
            agent: "node".to_owned(),
            session: "dev".to_owned(),
            window_index: 0,
            pane_index: 0,
            pane_pid: 100,
            pane_id: "%1".to_owned(),
        }];
        let table = make_table(&[(100, "pi")]);
        resolve_from_process_table(&mut panes, &table);
        assert_eq!(panes[0].agent, "pi");
    }

    #[test]
    fn resolve_node_pane_with_no_pi_child_stays_node() {
        let mut panes = vec![PaneEntry {
            agent: "node".to_owned(),
            session: "dev".to_owned(),
            window_index: 0,
            pane_index: 0,
            pane_pid: 100,
            pane_id: "%1".to_owned(),
        }];
        let table = make_table(&[(100, "webpack")]);
        resolve_from_process_table(&mut panes, &table);
        assert_eq!(panes[0].agent, "node");
    }

    #[test]
    fn resolve_leaves_claude_panes_unchanged() {
        let mut panes = vec![PaneEntry {
            agent: "claude".to_owned(),
            session: "dev".to_owned(),
            window_index: 0,
            pane_index: 0,
            pane_pid: 200,
            pane_id: "%2".to_owned(),
        }];
        let table = make_table(&[(200, "pi")]);
        resolve_from_process_table(&mut panes, &table);
        assert_eq!(panes[0].agent, "claude");
    }

    // ── validate_state_files ───────────────────────────────────────────────

    #[test]
    fn validate_deletes_orphaned_state_file() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let repo = FileAgentRepository::new(tmp.path().to_path_buf());
        repo.write_state_with_tools("%orphan", "claude", AgentState::Idle, 0)
            .unwrap();
        let mut map = load_state_map(&repo);
        assert!(map.contains_key("%orphan"));

        // No live panes → all state files are orphaned
        validate_state_files(&repo, &mut map, &[]);
        assert!(!map.contains_key("%orphan"));
        assert!(repo.read_agent_info("%orphan").is_none());
    }

    #[test]
    fn validate_deletes_contradictory_harness() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let repo = FileAgentRepository::new(tmp.path().to_path_buf());
        repo.write_state_with_tools("%1", "claude", AgentState::Working, 0)
            .unwrap();
        let mut map = load_state_map(&repo);

        let live = vec![pane_with_id("pi", "dev", 0, 0, "%1")];
        validate_state_files(&repo, &mut map, &live);
        assert!(!map.contains_key("%1"));
    }

    #[test]
    fn validate_keeps_matching_state_file() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let repo = FileAgentRepository::new(tmp.path().to_path_buf());
        repo.write_state_with_tools("%2", "claude", AgentState::Done, 0)
            .unwrap();
        let mut map = load_state_map(&repo);

        let live = vec![pane_with_id("claude", "dev", 0, 0, "%2")];
        validate_state_files(&repo, &mut map, &live);
        assert!(map.contains_key("%2"));
    }

    #[test]
    fn validate_keeps_unknown_harness_even_if_command_differs() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let repo = FileAgentRepository::new(tmp.path().to_path_buf());
        repo.write_state_with_tools("%3", "unknown", AgentState::Idle, 0)
            .unwrap();
        let mut map = load_state_map(&repo);

        let live = vec![pane_with_id("claude", "dev", 0, 0, "%3")];
        validate_state_files(&repo, &mut map, &live);
        assert!(map.contains_key("%3"));
    }
}
