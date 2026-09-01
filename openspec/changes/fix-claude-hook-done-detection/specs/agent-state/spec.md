## MODIFIED Requirements

### Requirement: State file schema

The system SHALL persist agent state as a JSON file at
`$XDG_DATA_HOME/radr/agents/<pane-id>/state.json` (where `XDG_DATA_HOME`
defaults to `~/.local/share` and `<pane-id>` is the value of `$TMUX_PANE`, e.g.
`%12`). The file SHALL contain the fields `pane_id` (string), `harness`
(string), `status` (string matching an `AgentState` variant in snake_case),
`updated_at` (RFC 3339 timestamp), `active_tools` (non-negative integer,
defaulting to `0`), and `blocked_prompts` (non-negative integer, defaulting to
`0`). The field was previously named `state`; it is renamed to `status` to
disambiguate it from other, independently tracked per-pane state such as
`active_tools` and `blocked_prompts`.

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

- **WHEN** a state file is written by any current `rd` command
- **THEN** the JSON object contains a `status` key and does NOT contain a
  `state` key

#### Scenario: New state file defaults active_tools to zero

- **WHEN** a state file is written for a pane that has never had a
  `TOOL_CALL_START` dispatched
- **THEN** the file's `active_tools` field is `0`

#### Scenario: New state file defaults blocked_prompts to zero

- **WHEN** a state file is written for a pane that has never had a
  `permission-prompt`-typed `TOOL_CALL_START` dispatched
- **THEN** the file's `blocked_prompts` field is `0`

#### Scenario: Old state files without active_tools deserialize with zero

- **WHEN** a `state.json` written by a version of `rd` that predates the
  `active_tools` field is read
- **THEN** it deserializes successfully with `active_tools` treated as `0`

#### Scenario: Old state files without blocked_prompts deserialize with zero

- **WHEN** a `state.json` written by a version of `rd` that predates the
  `blocked_prompts` field is read
- **THEN** it deserializes successfully with `blocked_prompts` treated as `0`

### Requirement: State read by CLI

The system SHALL provide a `read_state(pane_id)` function that returns the
current `AgentState` for an agent, and a `read_agent_info(pane_id)` function
that returns the full `AgentInfo` (including `active_tools` and
`blocked_prompts`). If the state file is absent, unreadable, or contains an
unrecognised value, `read_state` SHALL return `Unknown`.

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

#### Scenario: read_agent_info exposes blocked_prompts

- **WHEN** a valid state file with `blocked_prompts: 2` exists for a pane
- **THEN** `read_agent_info` returns an `AgentInfo` whose `blocked_prompts`
  field is `2`

## ADDED Requirements

### Requirement: blocked_prompts tracks in-flight permission-prompt blocks

The system SHALL track, per pane, a non-negative count of permission-prompt
blocks currently open as the `blocked_prompts` field of `state.json`,
independently of `active_tools`. This count SHALL be incremented by one whenever
`rd agent event TOOL_CALL_START --type permission-prompt` is dispatched for that
pane, and decremented by one (never below `0`) whenever
`rd agent event TOOL_CALL_RESULT --type permission-prompt` is dispatched for
that pane. No other event type mutates `blocked_prompts`.

#### Scenario: permission-prompt TOOL_CALL_START increments blocked_prompts

- **WHEN** a pane's `blocked_prompts` is `0` and
  `rd agent event TOOL_CALL_START --id p1 --type permission-prompt` is
  dispatched
- **THEN** the pane's `blocked_prompts` becomes `1`

#### Scenario: permission-prompt TOOL_CALL_RESULT decrements blocked_prompts

- **WHEN** a pane's `blocked_prompts` is `1` and
  `rd agent event TOOL_CALL_RESULT --id p1 --type permission-prompt` is
  dispatched
- **THEN** the pane's `blocked_prompts` becomes `0`

#### Scenario: blocked_prompts never decrements below zero

- **WHEN** a pane's `blocked_prompts` is `0` and
  `rd agent event TOOL_CALL_RESULT --id p1 --type permission-prompt` is
  dispatched (no matching prior open)
- **THEN** the pane's `blocked_prompts` remains `0`

#### Scenario: Other tool-call types do not mutate blocked_prompts

- **WHEN** a pane's `blocked_prompts` is `0` and
  `rd agent event TOOL_CALL_START --id t1 --type elicitation` (or
  `--type custom`, or no `--type`) is dispatched
- **THEN** the pane's `blocked_prompts` remains `0`

### Requirement: SESSION_STARTED resets blocked_prompts to zero

`rd agent event SESSION_STARTED` SHALL reset the pane's `blocked_prompts` to
`0`, in addition to resetting `status` to `idle` and `active_tools` to `0`.

#### Scenario: SESSION_STARTED clears a nonzero blocked_prompts count

- **WHEN** a pane's `blocked_prompts` is `2` and
  `rd agent event SESSION_STARTED --id s1` is dispatched
- **THEN** the pane's `blocked_prompts` becomes `0`
