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
`$XDG_DATA_HOME/drovr/agents/<session-id>/state.json` (where `XDG_DATA_HOME`
defaults to `~/.local/share`). The file SHALL contain the fields `session_id`
(string, from the Claude hook payload), `pane_id` (string, from `$TMUX_PANE`,
omitted if not running inside tmux), `harness` (string), `state` (string
matching an `AgentState` variant in snake_case), and `updated_at` (RFC 3339
timestamp).

#### Scenario: State file written on state change

- **WHEN** an agent transitions to a new state
- **THEN** a valid JSON file exists at the expected path with all required
  fields populated and `updated_at` reflecting the time of the transition

#### Scenario: State file is atomically replaced

- **WHEN** a new state is written for an agent that already has a state file
- **THEN** the file is replaced atomically (write to a `.tmp` sibling then
  rename) so a concurrent reader never sees a partial write

#### Scenario: State file written without pane_id outside tmux

- **WHEN** a hook fires in an environment where `$TMUX_PANE` is not set
- **THEN** the state file is written without a `pane_id` field and the command
  exits 0

### Requirement: State read by CLI

The system SHALL provide a `read_state(session_id)` function that returns the
current `AgentState` for an agent. If the state file is absent, unreadable, or
contains an unrecognised value, the function SHALL return `Unknown`.

#### Scenario: State file exists and is valid

- **WHEN** a valid state file exists for the given session ID
- **THEN** `read_state` returns the state recorded in the file

#### Scenario: State file is missing

- **WHEN** no state file exists for the given session ID
- **THEN** `read_state` returns `Unknown`

### Requirement: Stale state cleanup

The system SHALL provide a `cleanup_stale(live_pane_ids)` function that deletes
every state file whose `pane_id` is not present in the provided set of live tmux
pane IDs. This function SHALL only be called when tmux is reachable; if the tmux
query fails, the cleanup step SHALL be skipped entirely to avoid deleting valid
state due to a transient error.

#### Scenario: Stale state file is removed

- **WHEN** `cleanup_stale` is called with a set of live pane IDs that does not
  include the pane ID stored in a given state file
- **THEN** that state file is deleted

#### Scenario: Live state file is preserved

- **WHEN** `cleanup_stale` is called with a set of live pane IDs that includes
  the pane ID stored in a given state file
- **THEN** that state file is not deleted

#### Scenario: Cleanup skipped when tmux is unreachable

- **WHEN** the tmux query fails before `cleanup_stale` would be called
- **THEN** no state files are deleted and the command continues normally
