## Why

Developers running multiple AI agent harness sessions (e.g., `claude`, `pi`)
across tmux need a quick way to see what is currently active. A `agentctl list`
subcommand surfaces all relevant sessions from a tmux server at a glance, giving
operators instant visibility without manually querying tmux.

## What Changes

- Add a `list` subcommand to `agentctl` that queries every pane across all tmux
  sessions via `tmux list-panes -a`.
- Identify agent harness panes by a two-step process:
  1. `#{pane_current_command}` is checked first — a value of `claude` is a
     direct match.
  2. Because `pi` is a Node.js application, tmux reports its foreground process
     as `node`. For any pane where `pane_current_command == "node"`, `agentctl`
     walks the child processes of `#{pane_pid}` (the pane's shell) using
     `ps -A -o pid=,ppid=,args=` and checks whether the argv\[0\] basename of
     any direct child equals `pi`. If so, the pane is reported as a `pi` pane.
- Output is columnar plain text with three left-aligned columns:
  `<agent-name>  <session-name>  <window-index>.<pane-index>`.

## Capabilities

### New Capabilities

- `list-sessions`: Discover and display panes running known agent harness
  processes (`claude`, `pi`) across all tmux sessions. `claude` is identified
  directly via `pane_current_command`; `pi` is identified by inspecting the
  argv\[0\] of the pane shell's child processes (since `pi` runs under `node`).
  Output shows agent name, session name, and `window.pane` identifier per
  result.

### Modified Capabilities

<!-- No existing spec-level behavior is changing. -->

## Impact

- `src/main.rs`: New `List` variant added to the `Commands` enum; routing added
  in `run()`.
- New source file(s) for the list subcommand implementation (tmux session
  querying logic).
- New dependency on a tmux interaction mechanism (shell-out to
  `tmux list-panes -a`).
- New dependency on `ps -A -o pid=,ppid=,args=` to resolve `node` panes to `pi`
  by inspecting process argv.
- No breaking changes to existing `help` subcommand or CLI surface.
