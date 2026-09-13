//! Sorting, filtering-adjacent rendering, and the shared `Field` vocabulary
//! for `agentctl list`. Moved here (out of the former CLI-facing
//! `commands/list.rs`) because sorting rules and text/color rendering are
//! business rules a future non-CLI adapter should still be able to reach.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use owo_colors::OwoColorize;

use agentctl_state::{AgentInfo, AgentState};
use agentctl_tmux::PaneEntry;

// ── Field ────────────────────────────────────────────────────────────────────

/// The shared vocabulary of properties accepted by both `--sort-by` and
/// `--columns`. Every value accepted by one flag is accepted by the other,
/// using the identical camelCase spelling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
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
    UpdatedAt,
    /// In-flight tool-call count. Busiest first by default when sorted.
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
/// [`DEFAULT_COLUMNS_STR`] (the bin's `--columns` clap default).
#[allow(dead_code)]
pub const DEFAULT_COLUMNS: &[Field] = &[
    Field::Agent,
    Field::Session,
    Field::Pane,
    Field::Status,
    Field::ActiveTools,
];

/// The literal, comma-delimited string form of [`DEFAULT_COLUMNS`], used by
/// the bin as the `--columns` flag's clap `default_value` so `--help` shows
/// the actual default. Kept in sync with `DEFAULT_COLUMNS` by the
/// `default_columns_str_matches_default_columns_const` test.
pub const DEFAULT_COLUMNS_STR: &str = "agent,session,pane,status,activeTools";

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
/// padding, and every row always has `columns.len() - 1` tab separators —
/// including when a trailing column's value is empty (e.g. an empty
/// `activeTools` or `updatedAt` cell) — so downstream tools splitting on
/// `\t` (e.g. `cut -f`, `awk -F'\t'`) see a consistent field count across
/// all rows. Returns an empty string when `panes` is empty.
///
/// When `use_color` is `true`, whichever column position(s) hold
/// [`Field::Status`] are wrapped in an ANSI foreground color escape sequence
/// keyed to the agent's state (see `colorize_state`).
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

    let mut output = String::new();
    for (row, status) in rows.iter().zip(statuses.iter()) {
        let rendered_cells: Vec<String> = columns
            .iter()
            .enumerate()
            .map(|(i, field)| {
                let cell = &row[i];
                if *field == Field::Status {
                    colorize_state(*status, cell, use_color)
                } else {
                    cell.clone()
                }
            })
            .collect();
        output.push_str(&rendered_cells.join("\t"));
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

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

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

    fn state_map_with(
        pane_id: &str,
        harness: &str,
        status: AgentState,
        active_tools: u32,
    ) -> HashMap<String, AgentInfo> {
        let mut map = HashMap::new();
        map.insert(
            pane_id.to_string(),
            AgentInfo {
                pane_id: pane_id.to_string(),
                harness: harness.to_string(),
                status,
                updated_at: Utc::now(),
                active_tools,
            },
        );
        map
    }

    // ── format_panes ────────────────────────────────────────────────────────

    #[test]
    fn format_panes_handles_wide_unicode_with_tabs() {
        let data = vec![
            pane_with_id("pi", "dotfiles", 0, 0, "%1"),
            pane_with_id("pi", "🌳 agentctl (claude-hooks)", 0, 0, "%2"),
            pane_with_id("pi", "🌳 agentctl (list-sort)", 0, 0, "%3"),
        ];
        let refs: Vec<&PaneEntry> = data.iter().collect();
        let output = format_panes(&refs, &empty_state_map(), false, DEFAULT_COLUMNS);

        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 3);

        assert_eq!(lines[0], "pi\tdotfiles\t0.0\tunknown\t");
        assert_eq!(lines[1], "pi\t🌳 agentctl (claude-hooks)\t0.0\tunknown\t");
        assert_eq!(lines[2], "pi\t🌳 agentctl (list-sort)\t0.0\tunknown\t");
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

        assert_eq!(lines[0], "claude\tmy-session\t0.0\tunknown\t");
        assert_eq!(lines[1], "pi\twork-session\t1.0\tunknown\t");
        assert_eq!(lines[2], "pi\twork-session\t2.3\tunknown\t");
    }

    #[test]
    fn format_panes_single_entry() {
        let data = vec![pane_with_id("claude", "dev", 0, 0, "%1")];
        let refs: Vec<&PaneEntry> = data.iter().collect();
        let output = format_panes(&refs, &empty_state_map(), false, DEFAULT_COLUMNS);
        assert_eq!(output, "claude\tdev\t0.0\tunknown\t\n");
    }

    #[test]
    fn format_panes_empty_returns_empty_string() {
        let output = format_panes(&[], &empty_state_map(), false, DEFAULT_COLUMNS);
        assert_eq!(output, "");
    }

    #[test]
    fn format_panes_shows_state_from_map() {
        let state_map = state_map_with("%5", "claude", AgentState::Working, 0);
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
        let state_map = state_map_with("%5", "claude", AgentState::Working, 1);
        let data = vec![pane_with_id("claude", "dev", 0, 0, "%5")];
        let refs: Vec<&PaneEntry> = data.iter().collect();
        let output = format_panes(&refs, &state_map, false, DEFAULT_COLUMNS);
        assert_eq!(output, "claude\tdev\t0.0\tworking\t1 tool\n");
    }

    #[test]
    fn format_panes_plural_tool_column() {
        let state_map = state_map_with("%5", "claude", AgentState::Working, 3);
        let data = vec![pane_with_id("claude", "dev", 0, 0, "%5")];
        let refs: Vec<&PaneEntry> = data.iter().collect();
        let output = format_panes(&refs, &state_map, false, DEFAULT_COLUMNS);
        assert_eq!(output, "claude\tdev\t0.0\tworking\t3 tools\n");
    }

    #[test]
    fn format_panes_empty_tool_column_when_zero() {
        let state_map = state_map_with("%5", "claude", AgentState::Working, 0);
        let data = vec![pane_with_id("claude", "dev", 0, 0, "%5")];
        let refs: Vec<&PaneEntry> = data.iter().collect();
        let output = format_panes(&refs, &state_map, false, DEFAULT_COLUMNS);
        assert_eq!(output, "claude	dev	0.0	working	\n");
    }

    #[test]
    fn format_panes_unknown_state_never_shows_tool_count() {
        let state_map = empty_state_map();
        let data = vec![pane_with_id("claude", "dev", 0, 0, "%missing")];
        let refs: Vec<&PaneEntry> = data.iter().collect();
        let output = format_panes(&refs, &state_map, false, DEFAULT_COLUMNS);
        assert_eq!(output, "claude	dev	0.0	unknown	\n");
    }

    #[test]
    fn format_panes_column_alignment_mixed_tool_counts() {
        let mut state_map = state_map_with("%1", "claude", AgentState::Working, 1);
        state_map.extend(state_map_with("%2", "pi", AgentState::Idle, 0));

        let data = vec![
            pane_with_id("claude", "dev", 0, 0, "%1"),
            pane_with_id("pi", "dev", 0, 1, "%2"),
        ];
        let refs: Vec<&PaneEntry> = data.iter().collect();
        let output = format_panes(&refs, &state_map, false, DEFAULT_COLUMNS);
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "claude\tdev\t0.0\tworking\t1 tool");
        assert_eq!(lines[1], "pi\tdev\t0.1\tidle\t");
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

    // ── --columns: subset, order, duplicates ────────────────

    #[test]
    fn format_panes_columns_subset_and_custom_order() {
        let state_map = state_map_with("%1", "claude", AgentState::Working, 0);
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

    // ── updatedAt rendering ───────────────────────────────────────────────────

    #[test]
    fn format_panes_updated_at_renders_rfc3339_when_state_file_exists() {
        let state_map = state_map_with("%1", "claude", AgentState::Working, 0);
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
        assert_eq!(output, "claude\t\n");
    }

    // ── generalized status coloring at arbitrary positions ──────────────────

    #[test]
    fn format_panes_status_colorized_when_not_in_default_position() {
        let state_map = state_map_with("%1", "claude", AgentState::Working, 0);
        let data = vec![pane_with_id("claude", "dev", 0, 0, "%1")];
        let refs: Vec<&PaneEntry> = data.iter().collect();
        let output = format_panes(&refs, &state_map, true, &[Field::Status, Field::Agent]);
        assert!(output.starts_with("\u{1b}[33mworking\u{1b}[39m\t"));
        let after_status = output.trim_end().split('\t').last().unwrap();
        assert_eq!(after_status, "claude");
    }

    #[test]
    fn format_panes_repeated_status_columns_each_colorized() {
        let state_map = state_map_with("%1", "claude", AgentState::Blocked, 0);
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

    // ── generalized fixed field count for a custom last column ──────────────

    #[test]
    fn format_panes_keeps_fixed_field_count_for_custom_last_column() {
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
            let fields: Vec<&str> = line.split('\t').collect();
            assert_eq!(
                fields.len(),
                2,
                "expected 2 tab-separated fields, got {fields:?} from line: {line:?}"
            );
        }
    }

    // ── alignment regression for arbitrary --columns selections ─────────────

    #[test]
    fn format_panes_alignment_holds_for_custom_columns_selection() {
        let data = vec![
            pane_with_id("claude", "dotfiles", 0, 0, "%1"),
            pane_with_id("pi", "🌳 agentctl (claude-hooks)", 0, 0, "%2"),
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

        assert_eq!(lines[0], "dotfiles\tclaude");
        assert_eq!(lines[1], "🌳 agentctl (claude-hooks)\tpi");
    }

    // ── colorize_state ───────────────────────────────────────────────────────

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

    // ── format_panes color wiring ────────────────────────────────────────────

    #[test]
    fn format_panes_use_color_true_wraps_state_in_ansi_codes() {
        let mut state_map = state_map_with("%1", "claude", AgentState::Idle, 0);
        state_map.extend(state_map_with("%2", "claude", AgentState::Working, 0));
        state_map.extend(state_map_with("%3", "claude", AgentState::Blocked, 0));
        state_map.extend(state_map_with("%4", "claude", AgentState::Done, 0));

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
        let unknown_line = output.lines().last().unwrap();
        assert!(unknown_line.ends_with("unknown\t"));
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
        assert_eq!(lines[0], "claude\tmy-session\t0.0\tunknown\t");
        assert_eq!(lines[1], "pi\twork-session\t1.0\tunknown\t");
        assert_eq!(lines[2], "pi\twork-session\t2.3\tunknown\t");
        assert!(!output.contains("\u{1b}["));
    }

    #[test]
    fn format_panes_color_does_not_break_alignment() {
        let mut state_map = state_map_with("%1", "claude", AgentState::Idle, 0);
        state_map.extend(state_map_with("%2", "claude", AgentState::Blocked, 0));

        let p1 = pane_with_id("claude", "dev", 0, 0, "%1");
        let p2 = pane_with_id("pi", "dev", 0, 1, "%2");
        let refs: Vec<&PaneEntry> = vec![&p1, &p2];

        let colored = format_panes(&refs, &state_map, true, DEFAULT_COLUMNS);
        let plain = format_panes(&refs, &state_map, false, DEFAULT_COLUMNS);

        fn strip_ansi(s: &str) -> String {
            let mut out = String::new();
            let mut chars = s.chars().peekable();
            while let Some(c) = chars.next() {
                if c == '\u{1b}' {
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

    // ── sort_panes ───────────────────────────────────────────────────────────

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
                };
                (id.to_string(), info)
            })
            .collect()
    }

    fn t(secs: i64) -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::from_timestamp(secs, 0).expect("valid timestamp")
    }

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
        assert_eq!(refs[0].pane_id, "%1");
        assert_eq!(refs[1].pane_id, "%2");
    }

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

    #[test]
    fn sort_panes_by_pane_window_then_pane_index() {
        let p1 = pane_with_id("claude", "dev", 1, 2, "%1");
        let p2 = pane_with_id("pi", "dev", 0, 5, "%2");
        let p3 = pane_with_id("claude", "dev", 1, 0, "%3");
        let p4 = pane_with_id("pi", "dev", 0, 0, "%4");
        let mut refs: Vec<&PaneEntry> = vec![&p1, &p2, &p3, &p4];
        sort_panes(&mut refs, Field::Pane, false, &empty_state_map());
        assert_eq!(
            refs.iter()
                .map(|p| (p.window_index, p.pane_index))
                .collect::<Vec<_>>(),
            vec![(0, 0), (0, 5), (1, 0), (1, 2)]
        );
    }

    #[test]
    fn sort_panes_state_ascending_blocked_first() {
        let mut map = state_map_with("%1", "claude", AgentState::Unknown, 0);
        map.extend(state_map_with("%2", "claude", AgentState::Idle, 0));
        map.extend(state_map_with("%3", "claude", AgentState::Blocked, 0));
        map.extend(state_map_with("%4", "claude", AgentState::Working, 0));
        map.extend(state_map_with("%5", "claude", AgentState::Done, 0));

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
        let mut map = state_map_with("%1", "claude", AgentState::Blocked, 0);
        map.extend(state_map_with("%2", "claude", AgentState::Unknown, 0));
        map.extend(state_map_with("%3", "claude", AgentState::Idle, 0));

        let p1 = pane_with_id("claude", "dev", 0, 0, "%1");
        let p2 = pane_with_id("claude", "dev", 0, 1, "%2");
        let p3 = pane_with_id("claude", "dev", 0, 2, "%3");
        let mut refs: Vec<&PaneEntry> = vec![&p1, &p2, &p3];
        sort_panes(&mut refs, Field::Status, true, &map);

        assert_eq!(map[refs[0].pane_id.as_str()].status, AgentState::Unknown);
        assert_eq!(map[refs[2].pane_id.as_str()].status, AgentState::Blocked);
    }

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
        let map = state_map_with_tools(&[
            ("%1", AgentState::Working, 1),
            ("%2", AgentState::Unknown, 9),
        ]);
        let mut refs: Vec<&PaneEntry> = vec![&p1, &p2];
        sort_panes(&mut refs, Field::ActiveTools, false, &map);
        assert_eq!(refs[0].pane_id, "%1");
        assert_eq!(refs[1].pane_id, "%2");
    }

    #[test]
    fn sort_panes_stable_equal_keys_preserve_order() {
        let p1 = pane_with_id("claude", "dev", 0, 0, "%1");
        let p2 = pane_with_id("pi", "dev", 0, 1, "%2");
        let p3 = pane_with_id("claude", "dev", 0, 2, "%3");
        let mut refs: Vec<&PaneEntry> = vec![&p1, &p2, &p3];
        sort_panes(&mut refs, Field::Session, false, &empty_state_map());
        assert_eq!(
            refs.iter().map(|p| p.pane_id.as_str()).collect::<Vec<_>>(),
            vec!["%1", "%2", "%3"]
        );
    }
}
