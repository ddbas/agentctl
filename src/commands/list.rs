//! `dv list` — enumerate tmux panes running known agent harness processes.
//!
//! Shells out to `tmux list-panes -a` and prints one line per pane whose
//! `pane_current_command` exactly matches a known agent name (`claude` or
//! `pi`).  Output is columnar plain text with three left-aligned columns:
//! `<agent-name>  <session-name>  <window-index>.<pane-index>`.

use std::collections::HashMap;
use std::io::ErrorKind;
use std::process::Command;

use anyhow::Result;

// ── Constants ─────────────────────────────────────────────────────────────────

/// Exact process names that are considered known agent harness processes.
const KNOWN_AGENTS: &[&str] = &["claude", "pi"];

// ── Data types ────────────────────────────────────────────────────────────────

/// A single tmux pane entry returned by `tmux list-panes -a`.
// Task 3.1
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
// Tasks 3.2, 3.3, 3.4
pub fn tmux_panes(server: Option<&str>) -> Result<Vec<PaneEntry>> {
    let mut cmd = Command::new("tmux");

    // Task 3.2: optional -S flag for a specific server socket
    if let Some(socket) = server {
        cmd.args(["-S", socket]);
    }

    cmd.args([
        "list-panes",
        "-a",
        "-F",
        "#{pane_current_command}\t#{session_name}\t#{window_index}\t#{pane_index}\t#{pane_pid}",
    ]);

    let output = cmd.output().map_err(|e| {
        // Task 3.4: friendly message when the tmux binary is missing
        if e.kind() == ErrorKind::NotFound {
            anyhow::anyhow!("tmux not found — is tmux installed and on PATH?")
        } else {
            anyhow::Error::from(e).context("failed to run tmux")
        }
    })?;

    // Task 3.3: non-zero exit means no server running → treat as empty list
    if !output.status.success() {
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut panes = Vec::new();

    for line in stdout.lines() {
        let parts: Vec<&str> = line.splitn(5, '\t').collect();
        if parts.len() < 5 {
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
// Task 6.2
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
        // argv[0] is the next token; rest are additional args we don't need.
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
// Task 6.3
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
// Task 6.4
fn resolve_node_agents(panes: &mut [PaneEntry]) {
    if !panes.iter().any(|p| p.agent == "node") {
        return;
    }
    let table = build_process_table();
    resolve_from_process_table(panes, &table);
}

// ── Filtering ─────────────────────────────────────────────────────────────────

/// Keep only panes whose `agent` exactly matches a known harness name.
// Task 3.5
pub fn filter_agent_panes(panes: &[PaneEntry]) -> Vec<&PaneEntry> {
    panes
        .iter()
        .filter(|p| KNOWN_AGENTS.contains(&p.agent.as_str()))
        .collect()
}

// ── Formatting ────────────────────────────────────────────────────────────────

/// Format a slice of agent pane references as columnar plain text.
///
/// Produces one line per pane with three left-aligned columns:
/// `agent`, `session`, `window_index.pane_index`.  Each column is padded to
/// the maximum value width in that column plus two spaces.  Returns an empty
/// string when `panes` is empty.
// Task 3.6
pub fn format_panes(panes: &[&PaneEntry]) -> String {
    if panes.is_empty() {
        return String::new();
    }

    // Pre-compute the window.pane strings so we can measure their width too.
    let window_pane: Vec<String> = panes
        .iter()
        .map(|p| format!("{}.{}", p.window_index, p.pane_index))
        .collect();

    let max_agent = panes.iter().map(|p| p.agent.len()).max().unwrap_or(0);
    let max_session = panes.iter().map(|p| p.session.len()).max().unwrap_or(0);

    let col1 = max_agent + 2;
    let col2 = max_session + 2;

    let mut output = String::new();
    for (pane, wp) in panes.iter().zip(window_pane.iter()) {
        output.push_str(&format!(
            "{:<col1$}{:<col2$}{}\n",
            pane.agent, pane.session, wp
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
// Task 3.7
pub fn run(server: Option<String>) -> Result<()> {
    // Allow the DV_TMUX_SOCKET env var to supply the socket when the flag is
    // not given.  This is consumed by TmuxEnv in integration tests.
    let server = server.or_else(|| std::env::var("DV_TMUX_SOCKET").ok());

    let panes = tmux_panes(server.as_deref())?;
    let agent_panes = filter_agent_panes(&panes);
    let output = format_panes(&agent_panes);
    print!("{output}");

    Ok(())
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn pane(agent: &str, session: &str, window: u32, pane: u32) -> PaneEntry {
        PaneEntry {
            agent: agent.to_owned(),
            session: session.to_owned(),
            window_index: window,
            pane_index: pane,
            pane_pid: 0,
        }
    }

    // Task 4.1 — known agents are kept; unknown agents are dropped
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

    // Task 4.2 — empty input returns empty output
    #[test]
    fn filter_empty_input_returns_empty() {
        let panes: Vec<PaneEntry> = vec![];
        assert!(filter_agent_panes(&panes).is_empty());
    }

    // Task 4.3 — near-matches are excluded (exact, case-sensitive match)
    #[test]
    fn filter_excludes_near_matches() {
        let panes = vec![
            pane("pilot", "dev", 0, 0),   // prefix match — must be excluded
            pane("Claude", "dev", 0, 1),  // wrong case
            pane("PI", "dev", 0, 2),      // wrong case
            pane("claude2", "dev", 0, 3), // trailing char
            pane("xpi", "dev", 0, 4),     // leading char
            pane("", "dev", 0, 5),        // empty string
        ];
        let filtered = filter_agent_panes(&panes);
        assert!(
            filtered.is_empty(),
            "expected no matches, got: {:?}",
            filtered
        );
    }

    // Task 4.4 — columnar formatting: left-aligned with correct per-column padding
    #[test]
    fn format_panes_left_aligned_correct_widths() {
        let data = vec![
            pane("claude", "my-session", 0, 0),
            pane("pi", "work-session", 1, 0),
            pane("pi", "work-session", 2, 3),
        ];
        let refs: Vec<&PaneEntry> = data.iter().collect();
        let output = format_panes(&refs);

        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 3, "expected 3 lines, got:\n{output}");

        // max agent = "claude" (6) → col width = 8
        // max session = "work-session" (12) → col width = 14
        assert_eq!(lines[0], "claude  my-session    0.0");
        assert_eq!(lines[1], "pi      work-session  1.0");
        assert_eq!(lines[2], "pi      work-session  2.3");
    }

    // Formatting with a single entry: no multi-row padding needed, still works.
    #[test]
    fn format_panes_single_entry() {
        let data = vec![pane("claude", "dev", 0, 0)];
        let refs: Vec<&PaneEntry> = data.iter().collect();
        let output = format_panes(&refs);
        assert_eq!(output, "claude  dev  0.0\n");
    }

    // Formatting with empty slice returns empty string.
    #[test]
    fn format_panes_empty_returns_empty_string() {
        let output = format_panes(&[]);
        assert_eq!(output, "");
    }

    // ── resolve_from_process_table (tasks 6.6 – 6.8) ──────────────────────

    fn make_table(entries: &[(u32, &str)]) -> HashMap<u32, Vec<String>> {
        let mut map: HashMap<u32, Vec<String>> = HashMap::new();
        for (ppid, argv0) in entries {
            map.entry(*ppid).or_default().push(argv0.to_string());
        }
        map
    }

    // Task 6.6 — a node pane whose shell PID has a child with argv0 "pi" is
    // resolved to "pi".
    #[test]
    fn resolve_node_pane_with_pi_child_becomes_pi() {
        let mut panes = vec![PaneEntry {
            agent: "node".to_owned(),
            session: "dev".to_owned(),
            window_index: 0,
            pane_index: 0,
            pane_pid: 100,
        }];
        // pid 200 is a child of shell 100, argv0 basename = "pi"
        let table = make_table(&[(100, "pi")]);
        resolve_from_process_table(&mut panes, &table);
        assert_eq!(panes[0].agent, "pi");
    }

    // Task 6.7 — a node pane with no matching child stays as "node".
    #[test]
    fn resolve_node_pane_with_no_pi_child_stays_node() {
        let mut panes = vec![PaneEntry {
            agent: "node".to_owned(),
            session: "dev".to_owned(),
            window_index: 0,
            pane_index: 0,
            pane_pid: 100,
        }];
        // child is some other node process, not pi
        let table = make_table(&[(100, "webpack")]);
        resolve_from_process_table(&mut panes, &table);
        assert_eq!(panes[0].agent, "node");
    }

    // Task 6.8 — claude panes are untouched by resolution.
    #[test]
    fn resolve_leaves_claude_panes_unchanged() {
        let mut panes = vec![PaneEntry {
            agent: "claude".to_owned(),
            session: "dev".to_owned(),
            window_index: 0,
            pane_index: 0,
            pane_pid: 200,
        }];
        // even if there's a pi child, claude pane must not change
        let table = make_table(&[(200, "pi")]);
        resolve_from_process_table(&mut panes, &table);
        assert_eq!(panes[0].agent, "claude");
    }
}
