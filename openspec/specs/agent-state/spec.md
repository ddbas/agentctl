## Purpose

Define the canonical `AgentState` enum and the state file schema used to persist
agent state across tmux panes, along with read/write/delete helpers and
last-resort stale-state validation performed by `agentctl list`.

## Requirements

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
`$XDG_DATA_HOME/agentctl/agents/<pane-id>/state.json` (where `XDG_DATA_HOME`
defaults to `~/.local/share` and `<pane-id>` is the value of `$TMUX_PANE`, e.g.
`%12`). The file SHALL contain the fields `pane_id` (string), `harness`
(string), `status` (string matching an `AgentState` variant in snake_case),
`updated_at` (RFC 3339 timestamp), and `active_tools` (non-negative integer,
defaulting to `0`). There is no `blocked_prompts` field. The `status` field was
previously named `state`; it is renamed to `status` to disambiguate it from
other, independently tracked per-pane state such as `active_tools`.

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

#### Scenario: Old state files with a leftover blocked_prompts key still deserialize

- **WHEN** a `state.json` written by a version of `agentctl` that included a
  `blocked_prompts` field is read
- **THEN** it deserializes successfully, ignoring the unknown `blocked_prompts`
  key, and any freshly written state file omits it

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

### Requirement: Last-resort stale state validation by agentctl list

The system SHALL validate each state file when `agentctl list` runs and tmux is
reachable. A state file SHALL be deleted if either of the following is true:

- Its `pane_id` does not correspond to any currently live tmux pane.
- The live pane with that `pane_id` has a `pane_current_command` that does not
  match the `harness` field in the state file, **and** `harness` is not
  `"unknown"` (when `harness` is `"unknown"` the command-match check is skipped
  since there is no harness to validate against).

If the tmux query fails, validation SHALL be skipped entirely to avoid deleting
valid state due to a transient error.

#### Scenario: State file deleted when pane no longer exists

- **WHEN** `agentctl list` runs and a state file's `pane_id` is not in the live
  tmux pane set
- **THEN** that state file is deleted before the output is printed

#### Scenario: State file deleted when harness no longer matches

- **WHEN** `agentctl list` runs and a state file's `pane_id` exists but the
  pane's current command does not match the `harness` field
- **THEN** that state file is deleted before the output is printed

#### Scenario: State file preserved when pane and harness both match

- **WHEN** `agentctl list` runs and a state file's `pane_id` exists and its
  `pane_current_command` matches `harness`
- **THEN** the state file is not deleted

#### Scenario: State file preserved when harness is unknown

- **WHEN** `agentctl list` runs and a state file's `harness` field is
  `"unknown"` (regardless of what the pane's current command is)
- **THEN** the state file is not deleted on account of the harness check (only
  the pane-existence check still applies)

#### Scenario: Validation skipped when tmux is unreachable

- **WHEN** the tmux query fails
- **THEN** no state files are deleted and the command continues normally
