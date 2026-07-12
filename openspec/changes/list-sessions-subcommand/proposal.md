## Why

Developers running multiple AI agent harness sessions (e.g., `claude`, `pi`)
across tmux need a quick way to see what is currently active. A `dv list`
subcommand surfaces all relevant sessions from a tmux server at a glance, giving
operators instant visibility without manually querying tmux.

## What Changes

- Add a `list` subcommand to `dv` that queries every pane across all tmux
  sessions via `tmux list-panes -a`.
- Identify agent harness panes by their running process name
  (`#{pane_current_command}`), matching exactly `claude` or `pi` — no assumption
  is made about session naming.
- Output is columnar plain text with three left-aligned columns:
  `<agent-name>  <session-name>  <window-index>.<pane-index>`.

## Capabilities

### New Capabilities

- `list-sessions`: Discover and display panes running known agent harness
  processes (`claude`, `pi`) across all tmux sessions, identified by
  `pane_current_command`, showing agent name, session name, and `window.pane`
  identifier per result.

### Modified Capabilities

<!-- No existing spec-level behavior is changing. -->

## Impact

- `src/main.rs`: New `List` variant added to the `Commands` enum; routing added
  in `run()`.
- New source file(s) for the list subcommand implementation (tmux session
  querying logic).
- New dependency on a tmux interaction mechanism (shell-out to
  `tmux list-panes -a`).
- No breaking changes to existing `help` subcommand or CLI surface.
