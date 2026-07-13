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
`$XDG_DATA_HOME/drovr/agents/<agent-id>/state.json` (where `XDG_DATA_HOME`
defaults to `~/.local/share`). The file SHALL contain the fields `agent_id`
(string UUID), `harness` (string), `state` (string matching an `AgentState`
variant in snake_case), and `updated_at` (RFC 3339 timestamp).

#### Scenario: State file written on state change

- **WHEN** an agent transitions to a new state
- **THEN** a valid JSON file exists at the expected path with all required
  fields populated and `updated_at` reflecting the time of the transition

#### Scenario: State file is atomically replaced

- **WHEN** a new state is written for an agent that already has a state file
- **THEN** the file is replaced atomically (write to a `.tmp` sibling then
  rename) so a concurrent reader never sees a partial write

### Requirement: Unique agent identifier via DROVR_AGENT_ID

The system SHALL assign each agent pane a UUID v4 stored in the tmux environment
variable `DROVR_AGENT_ID` when the pane is created by `dv`. Hook scripts SHALL
read `$DROVR_AGENT_ID` to determine the target state file path.

#### Scenario: Identifier set at pane creation

- **WHEN** `dv` creates a new tmux pane for an agent
- **THEN** `tmux show-environment -t <pane> DROVR_AGENT_ID` returns a valid UUID
  v4

#### Scenario: Hook script silently exits when identifier is absent

- **WHEN** a hook script fires in a pane where `DROVR_AGENT_ID` is not set
- **THEN** the script exits with code 0 and writes nothing

### Requirement: Initial idle state written at spawn

The system SHALL write an initial state of `Idle` for a new agent immediately
after setting `DROVR_AGENT_ID` in the pane's environment.

#### Scenario: New agent starts in idle state

- **WHEN** `dv` has just created a new agent pane
- **THEN** the state file for that agent contains `"state": "idle"`

### Requirement: State read by CLI

The system SHALL provide a `read_state(agent_id)` function that returns the
current `AgentState` for an agent. If the state file is absent, unreadable, or
contains an unrecognised value, the function SHALL return `Unknown`.

#### Scenario: State file exists and is valid

- **WHEN** a valid state file exists for the given agent ID
- **THEN** `read_state` returns the state recorded in the file

#### Scenario: State file is missing

- **WHEN** no state file exists for the given agent ID
- **THEN** `read_state` returns `Unknown`

### Requirement: Stale state cleanup

The system SHALL provide a `cleanup_stale(live_ids)` function that deletes every
state file whose `agent_id` is not present in the provided set of live agent
IDs. This function SHALL only be called when tmux is reachable; if the tmux
query fails, the cleanup step SHALL be skipped entirely to avoid deleting valid
state due to a transient error.

#### Scenario: Stale state file is removed

- **WHEN** `cleanup_stale` is called with a set of live IDs that does not
  include agent ID `abc-123`
- **THEN** the state file at `$XDG_DATA_HOME/drovr/agents/abc-123/state.json` is
  deleted

#### Scenario: Live state file is preserved

- **WHEN** `cleanup_stale` is called with a set of live IDs that includes agent
  ID `abc-123`
- **THEN** the state file for `abc-123` is not deleted

#### Scenario: Cleanup skipped when tmux is unreachable

- **WHEN** the tmux query fails before `cleanup_stale` would be called
- **THEN** no state files are deleted and the command continues normally
