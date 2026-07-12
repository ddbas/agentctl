## ADDED Requirements

### Requirement: List agent harness sessions with window detail

The CLI SHALL provide a `list` subcommand that queries a tmux server and prints
one line per window for all sessions whose names begin with a known agent
harness prefix (`claude` or `pi`). Each line SHALL contain the session name,
window index, and window name in a readable columnar format.

#### Scenario: Sessions exist for known harnesses

- **WHEN** the user runs `dv list` and tmux has sessions named `claude` (windows
  0: zsh, 1: nvim), `pi-work` (window 0: bash), and `unrelated`
- **THEN** the output contains three lines — one for each window of `claude` and
  one for `pi-work` — each showing session name, window index, and window name;
  `unrelated` is omitted

#### Scenario: Output is columnar

- **WHEN** two or more windows are printed
- **THEN** session names are left-aligned in a consistent column width, window
  indices are right-aligned in a fixed-width field, and window names follow

#### Scenario: No matching sessions

- **WHEN** the user runs `dv list` and no tmux sessions match any known harness
  prefix
- **THEN** the command exits with code 0 and produces no output

#### Scenario: No tmux server running

- **WHEN** the user runs `dv list` and no tmux server is active
- **THEN** the command exits with code 0 and produces no output (treats absence
  of a server as zero sessions)

### Requirement: Target a specific tmux server socket

The `list` subcommand SHALL accept an optional `--server` (`-s`) flag whose
value is passed to `tmux -S <path>` to target a specific tmux server socket
file.

#### Scenario: Custom server socket provided

- **WHEN** the user runs `dv list --server /tmp/my-tmux.sock` and that socket
  has a session named `pi-custom`
- **THEN** the output contains `pi-custom`

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
