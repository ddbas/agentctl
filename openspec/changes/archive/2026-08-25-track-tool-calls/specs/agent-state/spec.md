## MODIFIED Requirements

### Requirement: State file schema

The system SHALL persist agent state as a JSON file at
`$XDG_DATA_HOME/agentctl/agents/<pane-id>/state.json` (where `XDG_DATA_HOME`
defaults to `~/.local/share` and `<pane-id>` is the value of `$TMUX_PANE`, e.g.
`%12`). The file SHALL contain the fields `pane_id` (string), `harness`
(string), `status` (string matching an `AgentState` variant in snake_case),
`updated_at` (RFC 3339 timestamp), and `active_tools` (non-negative integer,
defaulting to `0`). The field was previously named `state`; it is renamed to
`status` to disambiguate it from other, independently tracked per-pane state
such as `active_tools`.

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

#### Scenario: New state file defaults active_tools to zero

- **WHEN** a state file is written for a pane that has never had a
  `TOOL_CALL_START` dispatched
- **THEN** the file's `active_tools` field is `0`

#### Scenario: Old state files without active_tools deserialize with zero

- **WHEN** a `state.json` written by a version of `agentctl` that predates the
  `active_tools` field is read
- **THEN** it deserializes successfully with `active_tools` treated as `0`

### Requirement: State read by CLI

The system SHALL provide a `read_state(pane_id)` function that returns the
current `AgentState` for an agent, and a `read_agent_info(pane_id)` function
that returns the full `AgentInfo` (including `active_tools`). If the state file
is absent, unreadable, or contains an unrecognised value, `read_state` SHALL
return `Unknown`.

#### Scenario: State file exists and is valid

- **WHEN** a valid state file exists for the given pane ID
- **THEN** `read_state` returns the state recorded in the file

#### Scenario: State file is missing

- **WHEN** no state file exists for the given pane ID
- **THEN** `read_state` returns `Unknown`

#### Scenario: read_agent_info exposes active_tools

- **WHEN** a valid state file with `active_tools: 3` exists for a pane
- **THEN** `read_agent_info` returns an `AgentInfo` whose `active_tools` field
  is `3`

## ADDED Requirements

### Requirement: active_tools tracks in-flight tool calls

The system SHALL track, per pane, a non-negative count of tool calls currently
in flight as the `active_tools` field of `state.json`. This count SHALL be
incremented by one whenever `agentctl agent event TOOL_CALL_START` is dispatched
for that pane, and decremented by one (never below `0`) whenever
`agentctl agent event TOOL_CALL_RESULT` is dispatched for that pane, regardless
of the event's `--type` value (`custom`, `elicitation`, or omitted).

#### Scenario: TOOL_CALL_START increments active_tools

- **WHEN** a pane's `active_tools` is `0` and
  `agentctl agent event TOOL_CALL_START --id t1` is dispatched
- **THEN** the pane's `active_tools` becomes `1`

#### Scenario: Multiple concurrent TOOL_CALL_START calls accumulate

- **WHEN** a pane's `active_tools` is `0` and three `TOOL_CALL_START` events are
  dispatched in sequence with distinct `--id` values
- **THEN** the pane's `active_tools` becomes `3`

#### Scenario: TOOL_CALL_RESULT decrements active_tools

- **WHEN** a pane's `active_tools` is `2` and
  `agentctl agent event TOOL_CALL_RESULT --id t1` is dispatched
- **THEN** the pane's `active_tools` becomes `1`

#### Scenario: TOOL_CALL_RESULT never decrements below zero

- **WHEN** a pane's `active_tools` is `0` and
  `agentctl agent event TOOL_CALL_RESULT --id t1` is dispatched (no matching
  prior `TOOL_CALL_START`)
- **THEN** the pane's `active_tools` remains `0`

#### Scenario: Custom-typed tool calls still affect active_tools

- **WHEN** `agentctl agent event TOOL_CALL_START --id t2 --type custom` is
  dispatched
- **THEN** `active_tools` is incremented, exactly as for `--type elicitation` or
  no `--type`

### Requirement: SESSION_STARTED resets active_tools to zero

`agentctl agent event SESSION_STARTED` SHALL reset the pane's `active_tools` to
`0`, in addition to resetting `status` to `idle`.

#### Scenario: SESSION_STARTED clears a nonzero active_tools count

- **WHEN** a pane's `active_tools` is `2` and
  `agentctl agent event SESSION_STARTED --id s1` is dispatched
- **THEN** the pane's `active_tools` becomes `0`
