## Why

`agentctl list` output is currently plain text, so scanning many tmux panes for
agents that need attention (`blocked`) or are ready to hand off (`done`)
requires reading every line's state column carefully. Color-coding the state
column lets a user visually triage panes at a glance.

## What Changes

- The `<state>` column of `agentctl list` output SHALL be rendered with an ANSI
  foreground color based on the agent's status:
  - `idle` → grey
  - `working` → yellow
  - `blocked` → red
  - `done` → green
  - `unknown` remains uncolored (default terminal foreground)
- Column width/alignment computation SHALL continue to use the visible
  (uncolored) display width of each cell, so ANSI escape sequences do not throw
  off column padding.
- Color output SHALL be suppressed automatically when stdout is not a TTY (e.g.
  piped to a file or another command), so scripts/tests parsing plain text
  output are unaffected.
- No new CLI flag is introduced in this change; color is on by default when
  attached to a terminal.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `list-sessions`: the rendered `<state>` column now carries an ANSI foreground
  color keyed to the `AgentState` value, with automatic suppression when stdout
  is not a terminal; column alignment must remain based on visible width,
  unaffected by the added escape sequences.

## Impact

- Affected code: `src/commands/list.rs` (`format_panes`, and its width/padding
  calculations), likely a new small color-mapping helper.
- Affected dependencies: reuses the already-present `owo-colors` crate for ANSI
  coloring; TTY detection uses the standard library's `std::io::IsTerminal` (no
  new dependency needed).
- Affected tests: existing `format_panes` unit tests in `src/commands/list.rs`
  and `tests/list_sessions.rs` that assert exact plain-text output need
  updating/extending to account for ANSI sequences and TTY-detection behavior.
