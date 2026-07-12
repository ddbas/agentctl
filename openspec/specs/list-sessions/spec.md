## Purpose

Provide a `list` subcommand that discovers and displays tmux panes running agent
harness processes (`claude`, `pi`) across all sessions.

## Requirements

### Requirement: List agent harness panes by running process

The CLI SHALL provide a `list` subcommand that queries every pane across all
tmux sessions via `tmux list-panes -a` and prints one line for each pane whose
`pane_current_command` exactly matches a known agent harness name (`claude` or
`pi`). Each line SHALL contain three left-aligned columns:
`<agent-name>  <session-name>  <window-index>.<pane-index>`.

#### Scenario: Agent panes exist across multiple sessions

- **WHEN** the user runs `dv list` and tmux has a pane in session `dev` running
  `claude` at window 0 pane 0, and a pane in session `work` running `pi` at
  window 1 pane 0, and other panes running `zsh` or `nvim`
- **THEN** the output contains exactly two lines — one for `claude` and one for
  `pi` — and panes running unrelated processes are omitted

#### Scenario: Output is columnar and left-aligned

- **WHEN** two or more matching panes are printed
- **THEN** all three columns (agent, session, window.pane) are left-aligned,
  each padded to the width of the longest value in that column plus two spaces

#### Scenario: Agent name appears in the first column

- **WHEN** matching panes are displayed
- **THEN** the first column contains the agent process name (`claude` or `pi`),
  the second column contains the tmux session name, and the third column
  contains the `window_index.pane_index` in the form `N.M`

#### Scenario: No matching panes

- **WHEN** the user runs `dv list` and no pane's `pane_current_command` matches
  `claude` or `pi`
- **THEN** the command exits with code 0 and produces no output

#### Scenario: No tmux server running

- **WHEN** the user runs `dv list` and no tmux server is active
- **THEN** the command exits with code 0 and produces no output (treats absence
  of a server as zero panes)

### Requirement: Target a specific tmux server socket

The `list` subcommand SHALL accept an optional `--server` (`-s`) flag whose
value is passed to `tmux -S <path>` to target a specific tmux server socket
file.

#### Scenario: Custom server socket provided

- **WHEN** the user runs `dv list --server /tmp/my-tmux.sock` and a pane on that
  socket is running `pi`
- **THEN** the output contains a line for that `pi` pane

#### Scenario: Default server used when flag is omitted

- **WHEN** the user runs `dv list` without `--server`
- **THEN** the command queries the default tmux server (no `-S` flag passed to
  tmux)

### Requirement: Clear error when tmux binary is missing

The `list` subcommand SHALL print a human-readable error message and exit
non-zero when the `tmux` binary cannot be found on PATH.

#### Scenario: tmux not installed

- **WHEN** the user runs `dv list` and `tmux` is not available on PATH
- **THEN** the command exits with a non-zero code and prints an error containing
  "tmux not found"
