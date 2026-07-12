## Why

Developers running multiple AI agent harness sessions (e.g., `claude`, `pi`)
across tmux need a quick way to see what is currently active. A `dv list`
subcommand surfaces all relevant sessions from a tmux server at a glance, giving
operators instant visibility without manually querying tmux.

## What Changes

- Add a `list` subcommand to `dv` that queries a tmux server for running
  sessions.
- Filter and display sessions whose names match known agent harness prefixes:
  `claude` and `pi`.
- For each matching session, list every window with its index and name — one
  window per output line.
- Output is columnar plain text: session name, window index, window name (no
  deeper status metadata in this iteration).

## Capabilities

### New Capabilities

- `list-sessions`: Discover and display agent harness sessions running on a tmux
  server, filtered by known harness names (`claude`, `pi`), with per-window
  detail (index and name) for each session.

### Modified Capabilities

<!-- No existing spec-level behavior is changing. -->

## Impact

- `src/main.rs`: New `List` variant added to the `Commands` enum; routing added
  in `run()`.
- New source file(s) for the list subcommand implementation (tmux session
  querying logic).
- New dependency on a tmux interaction mechanism (shell-out to
  `tmux list-sessions` or a tmux crate).
- No breaking changes to existing `help` subcommand or CLI surface.
