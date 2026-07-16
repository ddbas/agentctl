//! `dv list` — enumerate tmux panes running known agent harness processes.
//!
//! Shells out to `tmux list-panes -a` and prints one line per pane whose
//! `pane_current_command` exactly matches a known agent name (`claude` or
//! `pi`).  Output is columnar plain text with four left-aligned columns:
//! `<agent-name>  <session-name>  <window-index>.<pane-index>  <state>`.

use std::collections::HashMap;
use std::io::ErrorKind;
use std::process::Command;

use unicode_width::UnicodeWidthStr;

use anyhow::Result;

use crate::state::{AgentInfo, AgentState, delete_state_in, list_agent_infos_in};

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

/// Build a `pane_id → AgentInfo` lookup map from all state files under `base`.
pub fn load_state_map_from(base: &std::path::Path) -> HashMap<String, AgentInfo> {
    list_agent_infos_in(base)
        .into_iter()
        .map(|info| (info.pane_id.clone(), info))
        .collect()
}

/// Build a `pane_id → AgentInfo` lookup map using the XDG base directory.
pub fn load_state_map() -> HashMap<String, AgentInfo> {
    load_state_map_from(&crate::state::xdg_data_home())
}

/// Validate state files against the live pane set under `base` and delete stale ones.
pub fn validate_state_files_in(
    base: &std::path::Path,
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
        let _ = delete_state_in(base, pane_id);
        state_map.remove(pane_id);
    }
}

/// Validate state files against the live pane set (XDG base) and delete stale ones.
///
/// A state file is deleted when:
///   - Its `pane_id` is not in the live pane set (orphaned).
///   - Its `pane_id` is in the live set **and** the live pane's
///     `pane_current_command` does not match `harness`, **and** `harness` is
///     not `"unknown"` (contradictory state from a different harness).
///
/// Files whose `harness` is `"unknown"` are only deleted when their pane no
/// longer exists — the harness-match check is skipped.
pub fn validate_state_files(state_map: &mut HashMap<String, AgentInfo>, live_panes: &[PaneEntry]) {
    validate_state_files_in(&crate::state::xdg_data_home(), state_map, live_panes);
}

// ── Formatting ────────────────────────────────────────────────────────────────

/// Format a slice of agent pane references as columnar plain text.
///
/// Produces one line per pane with four left-aligned columns:
/// `agent`, `session`, `window_index.pane_index`, `state`.  Each column is
/// padded to the maximum value width in that column plus two spaces.  Returns
/// an empty string when `panes` is empty.
pub fn format_panes(panes: &[&PaneEntry], state_map: &HashMap<String, AgentInfo>) -> String {
    if panes.is_empty() {
        return String::new();
    }

    // Pre-compute the window.pane strings so we can measure their width too.
    let window_pane: Vec<String> = panes
        .iter()
        .map(|p| format!("{}.{}", p.window_index, p.pane_index))
        .collect();

    // Resolve each pane's state string.
    let states: Vec<String> = panes
        .iter()
        .map(|p| {
            state_map
                .get(&p.pane_id)
                .map(|info| info.state.to_string())
                .unwrap_or_else(|| AgentState::Unknown.to_string())
        })
        .collect();

    let max_agent = panes.iter().map(|p| p.agent.width()).max().unwrap_or(0);
    let max_session = panes.iter().map(|p| p.session.width()).max().unwrap_or(0);
    let max_wp = window_pane.iter().map(|s| s.width()).max().unwrap_or(0);

    let col1 = max_agent + 2;
    let col2 = max_session + 2;
    let col3 = max_wp + 2;

    let mut output = String::new();
    for ((pane, wp), state) in panes.iter().zip(window_pane.iter()).zip(states.iter()) {
        let pad1 = col1 - pane.agent.width();
        let pad2 = col2 - pane.session.width();
        let pad3 = col3 - wp.width();
        output.push_str(&format!(
            "{}{}{}{}{}{}{}\n",
            pane.agent,
            " ".repeat(pad1),
            pane.session,
            " ".repeat(pad2),
            wp,
            " ".repeat(pad3),
            state
        ));
    }
    output
}

// ── Subcommand entry point ────────────────────────────────────────────────────

/// Entry point for the `dv list` subcommand.
///
/// Resolves the tmux server socket in priority order:
/// 1. `--server` / `-s` CLI flag
/// 2. `DV_TMUX_SOCKET` environment variable (used by the test harness)
/// 3. Default tmux server (no `-S` flag)
pub fn run(server: Option<String>) -> Result<()> {
    let server = server.or_else(|| std::env::var("DV_TMUX_SOCKET").ok());

    let panes = tmux_panes(server.as_deref())?;
    let agent_panes = filter_agent_panes(&panes);

    // Load state files and validate / clean up stale ones (best effort; only
    // when tmux was reachable, which is guaranteed at this point since
    // tmux_panes returned Ok).
    let mut state_map = load_state_map();
    validate_state_files(&mut state_map, &panes);

    let output = format_panes(&agent_panes, &state_map);
    print!("{output}");

    Ok(())
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{AgentState, write_state_to};

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

    fn empty_state_map() -> HashMap<String, AgentInfo> {
        HashMap::new()
    }

    // ── filter_agent_panes ─────────────────────────────────────────────────────────────────

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

    // ── format_panes ────────────────────────────────────────────────────────────────────

    #[test]
    fn format_panes_aligns_columns_with_wide_unicode() {
        // 🌳 is a wide character (2 terminal columns, 4 UTF-8 bytes, 1 scalar)
        let data = vec![
            pane_with_id("pi", "dotfiles", 0, 0, "%1"),
            pane_with_id("pi", "🌳 drovr (claude-hooks)", 0, 0, "%2"),
            pane_with_id("pi", "🌳 drovr (list-sort)", 0, 0, "%3"),
        ];
        let refs: Vec<&PaneEntry> = data.iter().collect();
        let output = format_panes(&refs, &empty_state_map());

        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 3);

        // Find the byte offset of the state column start on each line.
        // Because the session column is padded to the same display width,
        // the state token should appear at the same display-column position.
        // We check this by measuring the display width of everything before
        // the final whitespace-delimited token on each line.
        fn state_display_col(line: &str) -> usize {
            // The state is the last whitespace-delimited word.
            let state_start = line.rfind(|c: char| c == ' ').map(|i| i + 1).unwrap_or(0);
            UnicodeWidthStr::width(&line[..state_start])
        }

        let col0 = state_display_col(lines[0]);
        let col1 = state_display_col(lines[1]);
        let col2 = state_display_col(lines[2]);

        assert_eq!(
            col0, col1,
            "state column misaligned between ASCII and emoji rows:\n{output}"
        );
        assert_eq!(
            col0, col2,
            "state column misaligned between ASCII and emoji rows:\n{output}"
        );
    }

    #[test]
    fn format_panes_left_aligned_correct_widths() {
        let data = vec![
            pane_with_id("claude", "my-session", 0, 0, "%1"),
            pane_with_id("pi", "work-session", 1, 0, "%2"),
            pane_with_id("pi", "work-session", 2, 3, "%3"),
        ];
        let refs: Vec<&PaneEntry> = data.iter().collect();
        let output = format_panes(&refs, &empty_state_map());

        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 3, "expected 3 lines, got:\n{output}");

        assert_eq!(lines[0], "claude  my-session    0.0  unknown");
        assert_eq!(lines[1], "pi      work-session  1.0  unknown");
        assert_eq!(lines[2], "pi      work-session  2.3  unknown");
    }

    #[test]
    fn format_panes_single_entry() {
        let data = vec![pane_with_id("claude", "dev", 0, 0, "%1")];
        let refs: Vec<&PaneEntry> = data.iter().collect();
        let output = format_panes(&refs, &empty_state_map());
        assert_eq!(output, "claude  dev  0.0  unknown\n");
    }

    #[test]
    fn format_panes_empty_returns_empty_string() {
        let output = format_panes(&[], &empty_state_map());
        assert_eq!(output, "");
    }

    #[test]
    fn format_panes_shows_state_from_map() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_state_to(tmp.path(), "%5", "claude", AgentState::Working).unwrap();
        let state_map = load_state_map_from(tmp.path());

        let data = vec![pane_with_id("claude", "dev", 0, 0, "%5")];
        let refs: Vec<&PaneEntry> = data.iter().collect();
        let output = format_panes(&refs, &state_map);
        assert!(
            output.contains("working"),
            "expected 'working' in: {output}"
        );
    }

    // ── validate_state_files ───────────────────────────────────────────────────────────

    #[test]
    fn validate_deletes_orphaned_state_file() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_state_to(tmp.path(), "%orphan", "claude", AgentState::Idle).unwrap();
        let mut map = load_state_map_from(tmp.path());
        assert!(map.contains_key("%orphan"));

        // No live panes → all state files are orphaned
        validate_state_files_in(tmp.path(), &mut map, &[]);
        assert!(!map.contains_key("%orphan"));
        assert!(!crate::state::state_file_path_in(tmp.path(), "%orphan").exists());
    }

    #[test]
    fn validate_deletes_contradictory_harness() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_state_to(tmp.path(), "%1", "claude", AgentState::Working).unwrap();
        let mut map = load_state_map_from(tmp.path());

        let live = vec![pane_with_id("pi", "dev", 0, 0, "%1")];
        validate_state_files_in(tmp.path(), &mut map, &live);
        assert!(!map.contains_key("%1"));
    }

    #[test]
    fn validate_keeps_matching_state_file() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_state_to(tmp.path(), "%2", "claude", AgentState::Done).unwrap();
        let mut map = load_state_map_from(tmp.path());

        let live = vec![pane_with_id("claude", "dev", 0, 0, "%2")];
        validate_state_files_in(tmp.path(), &mut map, &live);
        assert!(map.contains_key("%2"));
    }

    #[test]
    fn validate_keeps_unknown_harness_even_if_command_differs() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_state_to(tmp.path(), "%3", "unknown", AgentState::Idle).unwrap();
        let mut map = load_state_map_from(tmp.path());

        let live = vec![pane_with_id("claude", "dev", 0, 0, "%3")];
        validate_state_files_in(tmp.path(), &mut map, &live);
        assert!(map.contains_key("%3"));
    }

    // ── resolve_from_process_table ───────────────────────────────────────────────────

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
}
