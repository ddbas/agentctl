//! `rd list` — enumerate tmux panes running known agent harness processes.
//!
//! Shells out to `tmux list-panes -a` and prints one line per pane whose
//! `pane_current_command` exactly matches a known agent name (`claude` or
//! `pi`).  Output is columnar plain text; by default five left-aligned
//! columns are shown: `<agent-name>  <session-name>  <window-index>.<pane-index>
//!  <status>  <active-tools>`.  The rendered columns can be customized via
//! `--columns`/`-c` (see [`Field`] and [`DEFAULT_COLUMNS`]).

use std::collections::HashMap;
use std::io::ErrorKind;
use std::io::IsTerminal;
use std::process::Command;

use owo_colors::OwoColorize;

use anyhow::Result;
use chrono::{DateTime, Utc};

use crate::state::{AgentInfo, AgentState, delete_state_in, list_agent_infos_in};

// ── Field ────────────────────────────────────────────────────────────────────

/// The shared vocabulary of properties accepted by both `--sort-by` and
/// `--columns`. Every value accepted by one flag is accepted by the other,
/// using the identical camelCase spelling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, Default)]
pub enum Field {
    /// Agent harness name, lexicographically.
    Agent,
    /// Tmux session name, lexicographically.
    Session,
    /// Pane position: window index first, then pane index.
    Pane,
    /// Agent state priority: Blocked < Done < Working < Idle < Unknown.
    Status,
    /// Last state-file update timestamp. Newest first by default when sorted.
    #[default]
    #[value(name = "updatedAt")]
    UpdatedAt,
    /// In-flight tool-call count. Busiest first by default when sorted.
    #[value(name = "activeTools")]
    ActiveTools,
}

impl std::fmt::Display for Field {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Field::Agent => write!(f, "agent"),
            Field::Session => write!(f, "session"),
            Field::Pane => write!(f, "pane"),
            Field::Status => write!(f, "status"),
            Field::UpdatedAt => write!(f, "updatedAt"),
            Field::ActiveTools => write!(f, "activeTools"),
        }
    }
}

/// The columns rendered when `--columns` is omitted, matching the output
/// shipped prior to the introduction of `--columns` (aside from the
/// `state` → `status` and unnamed-fifth-column → `activeTools` renames).
///
/// Exposed for tests and as the source of truth kept in sync with
/// [`DEFAULT_COLUMNS_STR`] (the clap `--columns` default). The CLI's actual
/// default column set at runtime is enforced via `--columns`'s clap
/// `default_value`, which is derived from this same set.
#[allow(dead_code)]
pub const DEFAULT_COLUMNS: &[Field] = &[
    Field::Agent,
    Field::Session,
    Field::Pane,
    Field::Status,
    Field::ActiveTools,
];

/// The literal, comma-delimited string form of [`DEFAULT_COLUMNS`], used as
/// the `--columns` flag's clap `default_value` so `--help` shows the actual
/// default. Kept in sync with `DEFAULT_COLUMNS` by the
/// `default_columns_str_matches_default_columns_const` test.
pub const DEFAULT_COLUMNS_STR: &str = "agent,session,pane,status,activeTools";

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

/// Wrap `text` (the plain display text for `state`) in an ANSI foreground
/// color escape sequence keyed to the `AgentState` value, or return it
/// unchanged when `use_color` is `false` or `state` is [`AgentState::Unknown`].
///
/// Color mapping: `Idle` → bright black (grey), `Working` → yellow,
/// `Blocked` → red, `Done` → green, `Unknown` → uncolored.
fn colorize_state(state: AgentState, text: &str, use_color: bool) -> String {
    if !use_color {
        return text.to_string();
    }
    match state {
        AgentState::Idle => text.fg::<owo_colors::colors::BrightBlack>().to_string(),
        AgentState::Working => text.fg::<owo_colors::colors::Yellow>().to_string(),
        AgentState::Blocked => text.fg::<owo_colors::colors::Red>().to_string(),
        AgentState::Done => text.fg::<owo_colors::colors::Green>().to_string(),
        AgentState::Unknown => text.to_string(),
    }
}

/// Resolve a pane's `active_tools` count, treating panes with no state file
/// or whose status is `unknown` as `0`.
fn active_tools_of(pane: &PaneEntry, state_map: &HashMap<String, AgentInfo>) -> u32 {
    match state_map.get(&pane.pane_id) {
        Some(info) if info.status != AgentState::Unknown => info.active_tools,
        _ => 0,
    }
}

/// Resolve the plain (uncolored) cell value for `field` for a single pane.
fn cell_value(
    pane: &PaneEntry,
    status: AgentState,
    state_map: &HashMap<String, AgentInfo>,
    field: Field,
) -> String {
    match field {
        Field::Agent => pane.agent.clone(),
        Field::Session => pane.session.clone(),
        Field::Pane => format!("{}.{}", pane.window_index, pane.pane_index),
        Field::Status => status.to_string(),
        Field::UpdatedAt => state_map
            .get(&pane.pane_id)
            .map(|info| info.updated_at.to_rfc3339())
            .unwrap_or_default(),
        Field::ActiveTools => {
            if status == AgentState::Unknown {
                return String::new();
            }
            match active_tools_of(pane, state_map) {
                0 => String::new(),
                1 => "1 tool".to_owned(),
                n => format!("{n} tools"),
            }
        }
    }
}

/// Format a slice of agent pane references as tab-delimited plain text.
///
/// Produces one line per pane, rendering exactly the columns listed in
/// `columns`, in the given order (duplicates rendered as repeated columns,
/// no deduplication). Columns are separated by tab characters (`\t`) without
/// padding. Any trailing whitespace is trimmed from each row's line.
/// Returns an empty string when `panes` is empty.
///
/// When `use_color` is `true`, whichever column position(s) hold
/// [`Field::Status`] are wrapped in an ANSI foreground color escape sequence
/// keyed to the agent's state (see [`colorize_state`]).
pub fn format_panes(
    panes: &[&PaneEntry],
    state_map: &HashMap<String, AgentInfo>,
    use_color: bool,
    columns: &[Field],
) -> String {
    if panes.is_empty() {
        return String::new();
    }

    // Resolve each pane's status once (used for the `status` column, the
    // `activeTools` column, and status colorization).
    let statuses: Vec<AgentState> = panes
        .iter()
        .map(|p| {
            state_map
                .get(&p.pane_id)
                .map(|info| info.status)
                .unwrap_or(AgentState::Unknown)
        })
        .collect();

    // Compute each row's plain (uncolored) cell values for every requested column.
    let rows: Vec<Vec<String>> = panes
        .iter()
        .zip(statuses.iter())
        .map(|(pane, status)| {
            columns
                .iter()
                .map(|field| cell_value(pane, *status, state_map, *field))
                .collect()
        })
        .collect();

    // With tab delimiters, we don't need to compute column widths for padding.

    let mut output = String::new();
    for (row, status) in rows.iter().zip(statuses.iter()) {
        let mut line = String::new();
        for (i, field) in columns.iter().enumerate() {
            let cell = &row[i];
            let rendered = if *field == Field::Status {
                colorize_state(*status, cell, use_color)
            } else {
                cell.clone()
            };
            line.push_str(&rendered);
            if i + 1 < columns.len() {
                // Use tab as column delimiter instead of 2 spaces
                line.push('\t');
            }
        }
        output.push_str(line.trim_end());
        output.push('\n');
    }
    output
}

// ── Sorting ───────────────────────────────────────────────────────────────────

/// Sort `panes` in-place by `key`.
///
/// The natural direction for each key is ascending (lowest value first),
/// except [`Field::UpdatedAt`] and [`Field::ActiveTools`], whose natural
/// direction is descending (newest/busiest first) to match the idiomatic
/// behaviour of time- and activity-sorted CLI tools (`ls -t`, `git log`).
///
/// Passing `reverse = true` flips whatever the natural direction is.
///
/// Uses a stable sort so that equal-keyed entries preserve their original
/// relative order.
pub fn sort_panes(
    panes: &mut Vec<&PaneEntry>,
    key: Field,
    reverse: bool,
    state_map: &HashMap<String, AgentInfo>,
) {
    panes.sort_by(|a, b| match key {
        Field::Status => {
            let sa = state_map
                .get(&a.pane_id)
                .map(|i| i.status)
                .unwrap_or(AgentState::Unknown);
            let sb = state_map
                .get(&b.pane_id)
                .map(|i| i.status)
                .unwrap_or(AgentState::Unknown);
            sa.cmp(&sb)
        }
        Field::Agent => a.agent.cmp(&b.agent),
        Field::Session => a.session.cmp(&b.session),
        Field::Pane => a
            .window_index
            .cmp(&b.window_index)
            .then(a.pane_index.cmp(&b.pane_index)),
        Field::UpdatedAt => {
            // Natural direction: newest first (descending by timestamp).
            let ta = state_map
                .get(&a.pane_id)
                .map(|i| i.updated_at)
                .unwrap_or(DateTime::<Utc>::MIN_UTC);
            let tb = state_map
                .get(&b.pane_id)
                .map(|i| i.updated_at)
                .unwrap_or(DateTime::<Utc>::MIN_UTC);
            tb.cmp(&ta) // note: tb before ta = descending
        }
        Field::ActiveTools => {
            // Natural direction: busiest first (descending by active_tools).
            let ca = active_tools_of(a, state_map);
            let cb = active_tools_of(b, state_map);
            cb.cmp(&ca) // note: cb before ca = descending
        }
    });
    if reverse {
        panes.reverse();
    }
}

// ── Subcommand entry point ────────────────────────────────────────────────────

/// Entry point for the `rd list` subcommand.
///
/// Resolves the tmux server socket in priority order:
/// 1. `--server` / `-s` CLI flag
/// 2. `RD_TMUX_SOCKET` environment variable (used by the test harness)
/// 3. Default tmux server (no `-S` flag)
pub fn run(
    server: Option<String>,
    sort_by: Field,
    reverse: bool,
    columns: Vec<Field>,
) -> Result<()> {
    let server = server.or_else(|| std::env::var("RD_TMUX_SOCKET").ok());

    let panes = tmux_panes(server.as_deref())?;
    let mut agent_panes = filter_agent_panes(&panes);

    // Load state files and validate / clean up stale ones (best effort; only
    // when tmux was reachable, which is guaranteed at this point since
    // tmux_panes returned Ok).
    let mut state_map = load_state_map();
    validate_state_files(&mut state_map, &panes);

    sort_panes(&mut agent_panes, sort_by, reverse, &state_map);

    let use_color = std::io::stdout().is_terminal();
    let output = format_panes(&agent_panes, &state_map, use_color, &columns);
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
    fn format_panes_handles_wide_unicode_with_tabs() {
        // With tab delimiters, wide unicode characters (like 🌳) don't affect column alignment
        // since tabs provide the delimiter, not padding.
        let data = vec![
            pane_with_id("pi", "dotfiles", 0, 0, "%1"),
            pane_with_id("pi", "🌳 radr (claude-hooks)", 0, 0, "%2"),
            pane_with_id("pi", "🌳 radr (list-sort)", 0, 0, "%3"),
        ];
        let refs: Vec<&PaneEntry> = data.iter().collect();
        let output = format_panes(&refs, &empty_state_map(), false, DEFAULT_COLUMNS);

        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 3);

        // All lines should have tab-separated columns (5 columns with 4 tabs)
        assert_eq!(lines[0], "pi\tdotfiles\t0.0\tunknown");
        assert_eq!(lines[1], "pi\t🌳 radr (claude-hooks)\t0.0\tunknown");
        assert_eq!(lines[2], "pi\t🌳 radr (list-sort)\t0.0\tunknown");
    }

    #[test]
    fn format_panes_left_aligned_correct_widths() {
        let data = vec![
            pane_with_id("claude", "my-session", 0, 0, "%1"),
            pane_with_id("pi", "work-session", 1, 0, "%2"),
            pane_with_id("pi", "work-session", 2, 3, "%3"),
        ];
        let refs: Vec<&PaneEntry> = data.iter().collect();
        let output = format_panes(&refs, &empty_state_map(), false, DEFAULT_COLUMNS);

        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 3, "expected 3 lines, got:\n{output}");

        assert_eq!(lines[0], "claude	my-session	0.0	unknown");
        assert_eq!(lines[1], "pi	work-session	1.0	unknown");
        assert_eq!(lines[2], "pi	work-session	2.3	unknown");
    }

    #[test]
    fn format_panes_single_entry() {
        let data = vec![pane_with_id("claude", "dev", 0, 0, "%1")];
        let refs: Vec<&PaneEntry> = data.iter().collect();
        let output = format_panes(&refs, &empty_state_map(), false, DEFAULT_COLUMNS);
        assert_eq!(
            output,
            "claude	dev	0.0	unknown
"
        );
    }

    #[test]
    fn format_panes_empty_returns_empty_string() {
        let output = format_panes(&[], &empty_state_map(), false, DEFAULT_COLUMNS);
        assert_eq!(output, "");
    }

    #[test]
    fn format_panes_shows_state_from_map() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_state_to(tmp.path(), "%5", "claude", AgentState::Working).unwrap();
        let state_map = load_state_map_from(tmp.path());

        let data = vec![pane_with_id("claude", "dev", 0, 0, "%5")];
        let refs: Vec<&PaneEntry> = data.iter().collect();
        let output = format_panes(&refs, &state_map, false, DEFAULT_COLUMNS);
        assert!(
            output.contains("working"),
            "expected 'working' in: {output}"
        );
    }

    // ── active-tools fifth column ──────────────────────────────────────────

    #[test]
    fn format_panes_singular_tool_column() {
        let tmp = tempfile::tempdir().expect("tempdir");
        crate::state::write_state_with_tools_to(tmp.path(), "%5", "claude", AgentState::Working, 1)
            .unwrap();
        let state_map = load_state_map_from(tmp.path());

        let data = vec![pane_with_id("claude", "dev", 0, 0, "%5")];
        let refs: Vec<&PaneEntry> = data.iter().collect();
        let output = format_panes(&refs, &state_map, false, DEFAULT_COLUMNS);
        assert_eq!(output, "claude\tdev\t0.0\tworking\t1 tool\n");
    }

    #[test]
    fn format_panes_plural_tool_column() {
        let tmp = tempfile::tempdir().expect("tempdir");
        crate::state::write_state_with_tools_to(tmp.path(), "%5", "claude", AgentState::Working, 3)
            .unwrap();
        let state_map = load_state_map_from(tmp.path());

        let data = vec![pane_with_id("claude", "dev", 0, 0, "%5")];
        let refs: Vec<&PaneEntry> = data.iter().collect();
        let output = format_panes(&refs, &state_map, false, DEFAULT_COLUMNS);
        assert_eq!(output, "claude\tdev\t0.0\tworking\t3 tools\n");
    }

    #[test]
    fn format_panes_empty_tool_column_when_zero() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_state_to(tmp.path(), "%5", "claude", AgentState::Working).unwrap();
        let state_map = load_state_map_from(tmp.path());

        let data = vec![pane_with_id("claude", "dev", 0, 0, "%5")];
        let refs: Vec<&PaneEntry> = data.iter().collect();
        let output = format_panes(&refs, &state_map, false, DEFAULT_COLUMNS);
        assert_eq!(
            output,
            "claude	dev	0.0	working
"
        );
    }

    #[test]
    fn format_panes_unknown_state_never_shows_tool_count() {
        // No state file for this pane at all → unknown state, no tool count.
        let state_map = empty_state_map();
        let data = vec![pane_with_id("claude", "dev", 0, 0, "%missing")];
        let refs: Vec<&PaneEntry> = data.iter().collect();
        let output = format_panes(&refs, &state_map, false, DEFAULT_COLUMNS);
        assert_eq!(
            output,
            "claude	dev	0.0	unknown
"
        );
    }

    #[test]
    fn format_panes_column_alignment_mixed_tool_counts() {
        let tmp = tempfile::tempdir().expect("tempdir");
        crate::state::write_state_with_tools_to(tmp.path(), "%1", "claude", AgentState::Working, 1)
            .unwrap();
        write_state_to(tmp.path(), "%2", "pi", AgentState::Idle).unwrap();
        let state_map = load_state_map_from(tmp.path());

        let data = vec![
            pane_with_id("claude", "dev", 0, 0, "%1"),
            pane_with_id("pi", "dev", 0, 1, "%2"),
        ];
        let refs: Vec<&PaneEntry> = data.iter().collect();
        let output = format_panes(&refs, &state_map, false, DEFAULT_COLUMNS);
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "claude\tdev\t0.0\tworking\t1 tool");
        assert_eq!(lines[1], "pi\tdev\t0.1\tidle");
    }

    // ── DEFAULT_COLUMNS_STR / DEFAULT_COLUMNS consistency ─────────────────

    #[test]
    fn default_columns_str_matches_default_columns_const() {
        let computed: String = DEFAULT_COLUMNS
            .iter()
            .map(|f| f.to_string())
            .collect::<Vec<_>>()
            .join(",");
        assert_eq!(computed, DEFAULT_COLUMNS_STR);
    }

    // ── --columns: subset, order, duplicates, invalid values ────────────────

    #[test]
    fn format_panes_columns_subset_and_custom_order() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_state_to(tmp.path(), "%1", "claude", AgentState::Working).unwrap();
        let state_map = load_state_map_from(tmp.path());

        let data = vec![pane_with_id("claude", "dev", 0, 0, "%1")];
        let refs: Vec<&PaneEntry> = data.iter().collect();
        let output = format_panes(&refs, &state_map, false, &[Field::Status, Field::Agent]);
        assert_eq!(output, "working\tclaude\n");
    }

    #[test]
    fn format_panes_columns_single_column() {
        let data = vec![pane_with_id("claude", "dev", 0, 0, "%1")];
        let refs: Vec<&PaneEntry> = data.iter().collect();
        let output = format_panes(&refs, &empty_state_map(), false, &[Field::Agent]);
        assert_eq!(output, "claude\n");
    }

    #[test]
    fn format_panes_columns_duplicate_field_renders_twice() {
        let data = vec![pane_with_id("claude", "dev", 0, 0, "%1")];
        let refs: Vec<&PaneEntry> = data.iter().collect();
        let output = format_panes(
            &refs,
            &empty_state_map(),
            false,
            &[Field::Agent, Field::Agent],
        );
        assert_eq!(
            output,
            "claude	claude
"
        );
    }

    #[test]
    fn invalid_columns_value_rejected_by_clap() {
        let result =
            <crate::Cli as clap::Parser>::try_parse_from(["rd", "list", "--columns", "bogus"]);
        assert!(
            result.is_err(),
            "expected clap error for unknown --columns value"
        );
    }

    #[test]
    fn columns_and_sort_by_share_identical_vocabulary() {
        for value in [
            "agent",
            "session",
            "pane",
            "status",
            "updatedAt",
            "activeTools",
        ] {
            let sort_result =
                <crate::Cli as clap::Parser>::try_parse_from(["rd", "list", "--sort-by", value]);
            let columns_result =
                <crate::Cli as clap::Parser>::try_parse_from(["rd", "list", "--columns", value]);
            assert!(sort_result.is_ok(), "--sort-by {value} should be accepted");
            assert!(
                columns_result.is_ok(),
                "--columns {value} should be accepted"
            );
        }
    }

    // ── updatedAt rendering ───────────────────────────────────────────────────

    #[test]
    fn format_panes_updated_at_renders_rfc3339_when_state_file_exists() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_state_to(tmp.path(), "%1", "claude", AgentState::Working).unwrap();
        let state_map = load_state_map_from(tmp.path());
        let expected = state_map["%1"].updated_at.to_rfc3339();

        let data = vec![pane_with_id("claude", "dev", 0, 0, "%1")];
        let refs: Vec<&PaneEntry> = data.iter().collect();
        let output = format_panes(&refs, &state_map, false, &[Field::Agent, Field::UpdatedAt]);
        assert_eq!(
            output,
            format!(
                "claude	{}
",
                expected
            )
        );
    }

    #[test]
    fn format_panes_updated_at_empty_when_no_state_file() {
        let data = vec![pane_with_id("claude", "dev", 0, 0, "%missing")];
        let refs: Vec<&PaneEntry> = data.iter().collect();
        let output = format_panes(
            &refs,
            &empty_state_map(),
            false,
            &[Field::Agent, Field::UpdatedAt],
        );
        // Trailing whitespace trim removes the empty last column entirely.
        assert_eq!(output, "claude\n");
    }

    // ── generalized status coloring at arbitrary positions ──────────────────

    #[test]
    fn format_panes_status_colorized_when_not_in_default_position() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_state_to(tmp.path(), "%1", "claude", AgentState::Working).unwrap();
        let state_map = load_state_map_from(tmp.path());

        let data = vec![pane_with_id("claude", "dev", 0, 0, "%1")];
        let refs: Vec<&PaneEntry> = data.iter().collect();
        let output = format_panes(&refs, &state_map, true, &[Field::Status, Field::Agent]);
        assert!(output.starts_with("\u{1b}[33mworking\u{1b}[39m\t"));
        // Only the status column is colorized; agent is never colorized.
        let after_status = output.trim_end().split('\t').last().unwrap();
        assert_eq!(after_status, "claude");
    }

    #[test]
    fn format_panes_repeated_status_columns_each_colorized() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_state_to(tmp.path(), "%1", "claude", AgentState::Blocked).unwrap();
        let state_map = load_state_map_from(tmp.path());

        let data = vec![pane_with_id("claude", "dev", 0, 0, "%1")];
        let refs: Vec<&PaneEntry> = data.iter().collect();
        let output = format_panes(
            &refs,
            &state_map,
            true,
            &[Field::Status, Field::Agent, Field::Status],
        );
        let occurrences = output.matches("\u{1b}[31mblocked\u{1b}[39m").count();
        assert_eq!(
            occurrences, 2,
            "expected both status columns colorized: {output}"
        );
    }

    // ── generalized trailing whitespace trim for a custom last column ───────

    #[test]
    fn format_panes_trims_trailing_whitespace_for_custom_last_column() {
        let data = vec![
            pane_with_id("claude", "dev", 0, 0, "%missing1"),
            pane_with_id("pi", "dev", 0, 1, "%missing2"),
        ];
        let refs: Vec<&PaneEntry> = data.iter().collect();
        let output = format_panes(
            &refs,
            &empty_state_map(),
            false,
            &[Field::Agent, Field::UpdatedAt],
        );
        for line in output.lines() {
            assert_eq!(
                line,
                line.trim_end(),
                "line has trailing whitespace: {line:?}"
            );
        }
    }

    // ── alignment regression for arbitrary --columns selections ─────────────

    #[test]
    fn format_panes_alignment_holds_for_custom_columns_selection() {
        let data = vec![
            pane_with_id("claude", "dotfiles", 0, 0, "%1"),
            pane_with_id("pi", "🌳 radr (claude-hooks)", 0, 0, "%2"),
        ];
        let refs: Vec<&PaneEntry> = data.iter().collect();
        let output = format_panes(
            &refs,
            &empty_state_map(),
            false,
            &[Field::Session, Field::Agent],
        );
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 2);

        // Tab-delimited format should have session and agent separated by tabs
        assert_eq!(lines[0], "dotfiles\tclaude");
        assert_eq!(lines[1], "🌳 radr (claude-hooks)\tpi");
    }

    // ── colorize_state ───────────────────────────────────────────────────────────────────

    #[test]
    fn colorize_state_use_color_false_returns_plain_text() {
        for state in [
            AgentState::Idle,
            AgentState::Working,
            AgentState::Blocked,
            AgentState::Done,
            AgentState::Unknown,
        ] {
            let text = state.to_string();
            assert_eq!(colorize_state(state, &text, false), text);
        }
    }

    #[test]
    fn colorize_state_idle_is_bright_black() {
        let out = colorize_state(AgentState::Idle, "idle", true);
        assert_eq!(out, "\u{1b}[90midle\u{1b}[39m");
    }

    #[test]
    fn colorize_state_working_is_yellow() {
        let out = colorize_state(AgentState::Working, "working", true);
        assert_eq!(out, "\u{1b}[33mworking\u{1b}[39m");
    }

    #[test]
    fn colorize_state_blocked_is_red() {
        let out = colorize_state(AgentState::Blocked, "blocked", true);
        assert_eq!(out, "\u{1b}[31mblocked\u{1b}[39m");
    }

    #[test]
    fn colorize_state_done_is_green() {
        let out = colorize_state(AgentState::Done, "done", true);
        assert_eq!(out, "\u{1b}[32mdone\u{1b}[39m");
    }

    #[test]
    fn colorize_state_unknown_is_uncolored_even_with_use_color_true() {
        let out = colorize_state(AgentState::Unknown, "unknown", true);
        assert_eq!(out, "unknown");
    }

    // ── format_panes color wiring ────────────────────────────────────────────────────────

    #[test]
    fn format_panes_use_color_true_wraps_state_in_ansi_codes() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_state_to(tmp.path(), "%1", "claude", AgentState::Idle).unwrap();
        write_state_to(tmp.path(), "%2", "claude", AgentState::Working).unwrap();
        write_state_to(tmp.path(), "%3", "claude", AgentState::Blocked).unwrap();
        write_state_to(tmp.path(), "%4", "claude", AgentState::Done).unwrap();
        let state_map = load_state_map_from(tmp.path());

        let p1 = pane_with_id("claude", "dev", 0, 0, "%1");
        let p2 = pane_with_id("claude", "dev", 0, 1, "%2");
        let p3 = pane_with_id("claude", "dev", 0, 2, "%3");
        let p4 = pane_with_id("claude", "dev", 0, 3, "%4");
        let p5 = pane_with_id("claude", "dev", 0, 4, "%5"); // no state file → unknown
        let refs: Vec<&PaneEntry> = vec![&p1, &p2, &p3, &p4, &p5];
        let output = format_panes(&refs, &state_map, true, DEFAULT_COLUMNS);

        assert!(output.contains("\u{1b}[90midle\u{1b}[39m"));
        assert!(output.contains("\u{1b}[33mworking\u{1b}[39m"));
        assert!(output.contains("\u{1b}[31mblocked\u{1b}[39m"));
        assert!(output.contains("\u{1b}[32mdone\u{1b}[39m"));
        // unknown remains uncolored
        let unknown_line = output.lines().last().unwrap();
        assert!(unknown_line.ends_with("unknown"));
        assert!(!unknown_line.contains("\u{1b}["));
    }

    #[test]
    fn format_panes_use_color_false_matches_pre_change_plain_output() {
        let data = vec![
            pane_with_id("claude", "my-session", 0, 0, "%1"),
            pane_with_id("pi", "work-session", 1, 0, "%2"),
            pane_with_id("pi", "work-session", 2, 3, "%3"),
        ];
        let refs: Vec<&PaneEntry> = data.iter().collect();
        let output = format_panes(&refs, &empty_state_map(), false, DEFAULT_COLUMNS);

        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines[0], "claude	my-session	0.0	unknown");
        assert_eq!(lines[1], "pi	work-session	1.0	unknown");
        assert_eq!(lines[2], "pi	work-session	2.3	unknown");
        assert!(!output.contains("\u{1b}["));
    }

    #[test]
    fn format_panes_color_does_not_break_alignment() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_state_to(tmp.path(), "%1", "claude", AgentState::Idle).unwrap();
        write_state_to(tmp.path(), "%2", "claude", AgentState::Blocked).unwrap();
        let state_map = load_state_map_from(tmp.path());

        let p1 = pane_with_id("claude", "dev", 0, 0, "%1");
        let p2 = pane_with_id("pi", "dev", 0, 1, "%2");
        let refs: Vec<&PaneEntry> = vec![&p1, &p2];

        let colored = format_panes(&refs, &state_map, true, DEFAULT_COLUMNS);
        let plain = format_panes(&refs, &state_map, false, DEFAULT_COLUMNS);

        // Strip ANSI escape sequences from the colored output and compare
        // display-column start offsets against the plain-text version.
        fn strip_ansi(s: &str) -> String {
            let mut out = String::new();
            let mut chars = s.chars().peekable();
            while let Some(c) = chars.next() {
                if c == '\u{1b}' {
                    // Skip until 'm' terminator.
                    for c2 in chars.by_ref() {
                        if c2 == 'm' {
                            break;
                        }
                    }
                } else {
                    out.push(c);
                }
            }
            out
        }

        let colored_lines: Vec<&str> = colored.lines().collect();
        let plain_lines: Vec<&str> = plain.lines().collect();
        assert_eq!(colored_lines.len(), plain_lines.len());
        for (colored_line, plain_line) in colored_lines.iter().zip(plain_lines.iter()) {
            let stripped = strip_ansi(colored_line);
            assert_eq!(&stripped, plain_line);
        }
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

    // ── sort_panes ───────────────────────────────────────────────────────────────────────

    /// Build a state map from (pane_id, state, updated_at) tuples.
    fn state_map_with_times(
        entries: &[(&str, AgentState, chrono::DateTime<chrono::Utc>)],
    ) -> HashMap<String, AgentInfo> {
        entries
            .iter()
            .map(|(id, state, ts)| {
                let info = AgentInfo {
                    pane_id: id.to_string(),
                    harness: "claude".to_string(),
                    status: *state,
                    updated_at: *ts,
                    active_tools: 0,
                    blocked_prompts: 0,
                };
                (id.to_string(), info)
            })
            .collect()
    }

    fn t(secs: i64) -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::from_timestamp(secs, 0).expect("valid timestamp")
    }

    // ── 5.1: default sort = updated-at descending (newest first) ─────────────

    #[test]
    fn sort_panes_default_updated_at_newest_first() {
        let p1 = pane_with_id("claude", "dev", 0, 0, "%1");
        let p2 = pane_with_id("pi", "dev", 0, 1, "%2");
        let p3 = pane_with_id("claude", "dev", 0, 2, "%3");
        let map = state_map_with_times(&[
            ("%1", AgentState::Idle, t(1000)),
            ("%2", AgentState::Working, t(3000)),
            ("%3", AgentState::Done, t(2000)),
        ]);
        let mut refs: Vec<&PaneEntry> = vec![&p1, &p2, &p3];
        sort_panes(&mut refs, Field::UpdatedAt, false, &map);
        // Newest first: p2 (3000) > p3 (2000) > p1 (1000)
        assert_eq!(
            refs.iter().map(|p| p.pane_id.as_str()).collect::<Vec<_>>(),
            vec!["%2", "%3", "%1"]
        );
    }

    #[test]
    fn sort_panes_updated_at_no_state_file_sorts_last() {
        let p1 = pane_with_id("claude", "dev", 0, 0, "%1");
        let p2 = pane_with_id("pi", "dev", 0, 1, "%2"); // no state file
        let map = state_map_with_times(&[("%1", AgentState::Idle, t(1000))]);
        let mut refs: Vec<&PaneEntry> = vec![&p1, &p2];
        sort_panes(&mut refs, Field::UpdatedAt, false, &map);
        // p1 has a real timestamp; p2 gets MIN_UTC → sorts last
        assert_eq!(refs[0].pane_id, "%1");
        assert_eq!(refs[1].pane_id, "%2");
    }

    #[test]
    fn sort_panes_updated_at_reverse_oldest_first() {
        let p1 = pane_with_id("claude", "dev", 0, 0, "%1");
        let p2 = pane_with_id("pi", "dev", 0, 1, "%2");
        let map = state_map_with_times(&[
            ("%1", AgentState::Idle, t(1000)),
            ("%2", AgentState::Idle, t(3000)),
        ]);
        let mut refs: Vec<&PaneEntry> = vec![&p1, &p2];
        sort_panes(&mut refs, Field::UpdatedAt, true, &map);
        // Reversed: oldest first → p1 (1000) before p2 (3000)
        assert_eq!(refs[0].pane_id, "%1");
        assert_eq!(refs[1].pane_id, "%2");
    }

    // ── 5.2: --sort-by agent ──────────────────────────────────────────────────────

    #[test]
    fn sort_panes_by_agent_alphabetical() {
        let p1 = pane_with_id("pi", "dev", 0, 0, "%1");
        let p2 = pane_with_id("claude", "dev", 0, 1, "%2");
        let mut refs: Vec<&PaneEntry> = vec![&p1, &p2];
        sort_panes(&mut refs, Field::Agent, false, &empty_state_map());
        assert_eq!(refs[0].agent, "claude");
        assert_eq!(refs[1].agent, "pi");
    }

    #[test]
    fn sort_panes_by_agent_reversed() {
        let p1 = pane_with_id("claude", "dev", 0, 0, "%1");
        let p2 = pane_with_id("pi", "dev", 0, 1, "%2");
        let mut refs: Vec<&PaneEntry> = vec![&p1, &p2];
        sort_panes(&mut refs, Field::Agent, true, &empty_state_map());
        assert_eq!(refs[0].agent, "pi");
        assert_eq!(refs[1].agent, "claude");
    }

    // ── 5.3: --sort-by session ───────────────────────────────────────────────────

    #[test]
    fn sort_panes_by_session_alphabetical() {
        let p1 = pane_with_id("claude", "work", 0, 0, "%1");
        let p2 = pane_with_id("claude", "dev", 0, 1, "%2");
        let p3 = pane_with_id("pi", "alpha", 0, 2, "%3");
        let mut refs: Vec<&PaneEntry> = vec![&p1, &p2, &p3];
        sort_panes(&mut refs, Field::Session, false, &empty_state_map());
        assert_eq!(
            refs.iter().map(|p| p.session.as_str()).collect::<Vec<_>>(),
            vec!["alpha", "dev", "work"]
        );
    }

    // ── 5.4: --sort-by pane ───────────────────────────────────────────────────────

    #[test]
    fn sort_panes_by_pane_window_then_pane_index() {
        let p1 = pane_with_id("claude", "dev", 1, 2, "%1");
        let p2 = pane_with_id("pi", "dev", 0, 5, "%2");
        let p3 = pane_with_id("claude", "dev", 1, 0, "%3");
        let p4 = pane_with_id("pi", "dev", 0, 0, "%4");
        let mut refs: Vec<&PaneEntry> = vec![&p1, &p2, &p3, &p4];
        sort_panes(&mut refs, Field::Pane, false, &empty_state_map());
        // Expected: window 0 pane 0, window 0 pane 5, window 1 pane 0, window 1 pane 2
        assert_eq!(
            refs.iter()
                .map(|p| (p.window_index, p.pane_index))
                .collect::<Vec<_>>(),
            vec![(0, 0), (0, 5), (1, 0), (1, 2)]
        );
    }

    // ── 5.5: --sort-by state --reverse ─────────────────────────────────────────

    #[test]
    fn sort_panes_state_ascending_blocked_first() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_state_to(tmp.path(), "%1", "claude", AgentState::Unknown).unwrap();
        write_state_to(tmp.path(), "%2", "claude", AgentState::Idle).unwrap();
        write_state_to(tmp.path(), "%3", "claude", AgentState::Blocked).unwrap();
        write_state_to(tmp.path(), "%4", "claude", AgentState::Working).unwrap();
        write_state_to(tmp.path(), "%5", "claude", AgentState::Done).unwrap();
        let map = load_state_map_from(tmp.path());

        let p1 = pane_with_id("claude", "dev", 0, 0, "%1");
        let p2 = pane_with_id("claude", "dev", 0, 1, "%2");
        let p3 = pane_with_id("claude", "dev", 0, 2, "%3");
        let p4 = pane_with_id("claude", "dev", 0, 3, "%4");
        let p5 = pane_with_id("claude", "dev", 0, 4, "%5");
        let mut refs: Vec<&PaneEntry> = vec![&p1, &p2, &p3, &p4, &p5];
        sort_panes(&mut refs, Field::Status, false, &map);

        let states: Vec<AgentState> = refs.iter().map(|p| map[&p.pane_id].status).collect();
        assert_eq!(
            states,
            vec![
                AgentState::Blocked,
                AgentState::Done,
                AgentState::Working,
                AgentState::Idle,
                AgentState::Unknown,
            ]
        );
    }

    #[test]
    fn sort_panes_state_reverse_unknown_first() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_state_to(tmp.path(), "%1", "claude", AgentState::Blocked).unwrap();
        write_state_to(tmp.path(), "%2", "claude", AgentState::Unknown).unwrap();
        write_state_to(tmp.path(), "%3", "claude", AgentState::Idle).unwrap();
        let map = load_state_map_from(tmp.path());

        let p1 = pane_with_id("claude", "dev", 0, 0, "%1");
        let p2 = pane_with_id("claude", "dev", 0, 1, "%2");
        let p3 = pane_with_id("claude", "dev", 0, 2, "%3");
        let mut refs: Vec<&PaneEntry> = vec![&p1, &p2, &p3];
        sort_panes(&mut refs, Field::Status, true, &map);

        assert_eq!(map[refs[0].pane_id.as_str()].status, AgentState::Unknown);
        assert_eq!(map[refs[2].pane_id.as_str()].status, AgentState::Blocked);
    }

    // ── 5.7: --sort-by activeTools ────────────────────────────────────────────

    /// Build a state map from (pane_id, state, active_tools) tuples.
    fn state_map_with_tools(entries: &[(&str, AgentState, u32)]) -> HashMap<String, AgentInfo> {
        entries
            .iter()
            .map(|(id, state, active_tools)| {
                let info = AgentInfo {
                    pane_id: id.to_string(),
                    harness: "claude".to_string(),
                    status: *state,
                    updated_at: chrono::Utc::now(),
                    active_tools: *active_tools,
                    blocked_prompts: 0,
                };
                (id.to_string(), info)
            })
            .collect()
    }

    #[test]
    fn sort_panes_active_tools_descending_busiest_first() {
        let p1 = pane_with_id("claude", "dev", 0, 0, "%1");
        let p2 = pane_with_id("pi", "dev", 0, 1, "%2");
        let p3 = pane_with_id("claude", "dev", 0, 2, "%3"); // no state file
        let map = state_map_with_tools(&[
            ("%1", AgentState::Working, 1),
            ("%2", AgentState::Working, 5),
        ]);
        let mut refs: Vec<&PaneEntry> = vec![&p1, &p2, &p3];
        sort_panes(&mut refs, Field::ActiveTools, false, &map);
        // Busiest first: %2 (5) > %1 (1) > %3 (no state file → 0)
        assert_eq!(
            refs.iter().map(|p| p.pane_id.as_str()).collect::<Vec<_>>(),
            vec!["%2", "%1", "%3"]
        );
    }

    #[test]
    fn sort_panes_active_tools_reverse_quietest_first() {
        let p1 = pane_with_id("claude", "dev", 0, 0, "%1");
        let p2 = pane_with_id("pi", "dev", 0, 1, "%2");
        let map = state_map_with_tools(&[
            ("%1", AgentState::Working, 1),
            ("%2", AgentState::Working, 5),
        ]);
        let mut refs: Vec<&PaneEntry> = vec![&p1, &p2];
        sort_panes(&mut refs, Field::ActiveTools, true, &map);
        assert_eq!(refs[0].pane_id, "%1");
        assert_eq!(refs[1].pane_id, "%2");
    }

    #[test]
    fn sort_panes_active_tools_unknown_status_treated_as_zero() {
        let p1 = pane_with_id("claude", "dev", 0, 0, "%1");
        let p2 = pane_with_id("pi", "dev", 0, 1, "%2");
        // %2 has active_tools set, but status unknown → treated as 0.
        let map = state_map_with_tools(&[
            ("%1", AgentState::Working, 1),
            ("%2", AgentState::Unknown, 9),
        ]);
        let mut refs: Vec<&PaneEntry> = vec![&p1, &p2];
        sort_panes(&mut refs, Field::ActiveTools, false, &map);
        assert_eq!(refs[0].pane_id, "%1");
        assert_eq!(refs[1].pane_id, "%2");
    }

    // ── Stability ───────────────────────────────────────────────────────────────────

    #[test]
    fn sort_panes_stable_equal_keys_preserve_order() {
        // All panes have the same session name — relative order must be preserved.
        let p1 = pane_with_id("claude", "dev", 0, 0, "%1");
        let p2 = pane_with_id("pi", "dev", 0, 1, "%2");
        let p3 = pane_with_id("claude", "dev", 0, 2, "%3");
        let mut refs: Vec<&PaneEntry> = vec![&p1, &p2, &p3];
        sort_panes(&mut refs, Field::Session, false, &empty_state_map());
        // All "dev" — original order preserved
        assert_eq!(
            refs.iter().map(|p| p.pane_id.as_str()).collect::<Vec<_>>(),
            vec!["%1", "%2", "%3"]
        );
    }

    // ── 5.6: invalid --sort-by value ───────────────────────────────────────────

    #[test]
    fn invalid_sort_by_rejected_by_clap() {
        // Clap's ValueEnum parsing rejects unknown values at argument parse time.
        // We verify this by using try_parse_from on the CLI root.
        let result =
            <crate::Cli as clap::Parser>::try_parse_from(["rd", "list", "--sort-by", "foobar"]);
        assert!(
            result.is_err(),
            "expected clap error for unknown sort-by value"
        );
    }
}
