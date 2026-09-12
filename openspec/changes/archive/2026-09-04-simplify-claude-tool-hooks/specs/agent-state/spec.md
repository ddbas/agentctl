## MODIFIED Requirements

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

## REMOVED Requirements

### Requirement: blocked_prompts tracks in-flight permission-prompt blocks

**Reason**: The `permission-prompt` event type this field tracked is removed
(see the `agent-events` spec); the field has no remaining writer or reader.

**Migration**: None required. Old `state.json` files with a `blocked_prompts`
key continue to deserialize successfully; the field is simply absent from any
newly written file.

### Requirement: SESSION_STARTED resets blocked_prompts to zero

**Reason**: Removed alongside `blocked_prompts` itself; see above.

**Migration**: None required.
