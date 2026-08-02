## ADDED Requirements

### Requirement: AgentState enum

The system SHALL define a standard `AgentState` enum with the following
variants: `Idle`, `Working`, `Done`, `Blocked`, `Unknown`. This enum SHALL be
the canonical representation of agent state throughout the codebase.

#### Scenario: All variants are defined

- **WHEN** the `AgentState` type is used anywhere in the codebase
- **THEN** it exposes exactly the variants `Idle`, `Working`, `Done`, `Blocked`,
  and `Unknown` with no additional variants

### Requirement: State file schema

The system SHALL persist agent state as a JSON file at
`$XDG_DATA_HOME/radr/agents/<pane-id>/state.json` (where `XDG_DATA_HOME`
defaults to `~/.local/share` and `<pane-id>` is the value of `$TMUX_PANE`, e.g.
`%12`). The file SHALL contain the fields `pane_id` (string), `harness`
(string), `state` (string matching an `AgentState` variant in snake_case), and
`updated_at` (RFC 3339 timestamp).

#### Scenario: State file written on state change

- **WHEN** an agent transitions to a new state
- **THEN** a valid JSON file exists at the expected path with all required
  fields populated and `updated_at` reflecting the time of the transition

#### Scenario: State file is atomically replaced

- **WHEN** a new state is written for an agent that already has a state file
- **THEN** the file is replaced atomically (write to a `.tmp` sibling then
  rename) so a concurrent reader never sees a partial write

### Requirement: State read by CLI

The system SHALL provide a `read_state(pane_id)` function that returns the
current `AgentState` for an agent. If the state file is absent, unreadable, or
contains an unrecognised value, the function SHALL return `Unknown`.

#### Scenario: State file exists and is valid

- **WHEN** a valid state file exists for the given pane ID
- **THEN** `read_state` returns the state recorded in the file

#### Scenario: State file is missing

- **WHEN** no state file exists for the given pane ID
- **THEN** `read_state` returns `Unknown`

### Requirement: State file deleted on session end

The system SHALL provide a `delete_state(pane_id)` function that removes the
state file and its parent directory. It SHALL be called by the `SessionEnd` hook
handler so that state is cleaned up when a session exits normally.

#### Scenario: State file removed on clean session exit

- **WHEN** `delete_state` is called for a pane that has a state file
- **THEN** the state file and its parent directory are removed from the
  filesystem

#### Scenario: delete_state is a no-op when no state file exists

- **WHEN** `delete_state` is called for a pane that has no state file
- **THEN** the call succeeds without error

### Requirement: Last-resort stale state validation by rd list

The system SHALL validate each state file when `rd list` runs and tmux is
reachable. A state file SHALL be deleted if either of the following is true:

- Its `pane_id` does not correspond to any currently live tmux pane.
- The live pane with that `pane_id` has a `pane_current_command` that does not
  match the `harness` field in the state file, **and** `harness` is not
  `"unknown"` (when `harness` is `"unknown"` the command-match check is skipped
  since there is no harness to validate against).

If the tmux query fails, validation SHALL be skipped entirely to avoid deleting
valid state due to a transient error.

#### Scenario: State file deleted when pane no longer exists

- **WHEN** `rd list` runs and a state file's `pane_id` is not in the live tmux
  pane set
- **THEN** that state file is deleted before the output is printed

#### Scenario: State file deleted when harness no longer matches

- **WHEN** `rd list` runs and a state file's `pane_id` exists but the pane's
  current command does not match the `harness` field
- **THEN** that state file is deleted before the output is printed

#### Scenario: State file preserved when pane and harness both match

- **WHEN** `rd list` runs and a state file's `pane_id` exists and its
  `pane_current_command` matches `harness`
- **THEN** the state file is not deleted

#### Scenario: State file preserved when harness is unknown

- **WHEN** `rd list` runs and a state file's `harness` field is `"unknown"`
  (regardless of what the pane's current command is)
- **THEN** the state file is not deleted on account of the harness check (only
  the pane-existence check still applies)

#### Scenario: Validation skipped when tmux is unreachable

- **WHEN** the tmux query fails
- **THEN** no state files are deleted and the command continues normally
