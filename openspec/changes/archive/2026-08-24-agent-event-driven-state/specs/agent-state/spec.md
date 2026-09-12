## MODIFIED Requirements

### Requirement: State file schema

The system SHALL persist agent state as a JSON file at
`$XDG_DATA_HOME/agentctl/agents/<pane-id>/state.json` (where `XDG_DATA_HOME`
defaults to `~/.local/share` and `<pane-id>` is the value of `$TMUX_PANE`, e.g.
`%12`). The file SHALL contain the fields `pane_id` (string), `harness`
(string), `status` (string matching an `AgentState` variant in snake_case), and
`updated_at` (RFC 3339 timestamp). The field was previously named `state`; it is
renamed to `status` to disambiguate it from other, independently tracked
per-pane state that future changes may add to this same file (e.g. a count of
active tool calls).

#### Scenario: State file written on status change

- **WHEN** an agent transitions to a new status
- **THEN** a valid JSON file exists at the expected path with all required
  fields populated, the `status` field set to the new status, and `updated_at`
  reflecting the time of the transition

#### Scenario: State file is atomically replaced

- **WHEN** a new status is written for an agent that already has a state file
- **THEN** the file is replaced atomically (write to a `.tmp` sibling then
  rename) so a concurrent reader never sees a partial write

#### Scenario: state.json no longer contains a state field

- **WHEN** a state file is written by any current `agentctl` command
- **THEN** the JSON object contains a `status` key and does NOT contain a
  `state` key
