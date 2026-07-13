## ADDED Requirements

### Requirement: Claude PreToolUse hook writes working state

The system SHALL provide a hook script registered for Claude's `PreToolUse`
event that writes `AgentState::Working` to the agent's state file.

#### Scenario: PreToolUse fires while DROVR_AGENT_ID is set

- **WHEN** Claude fires the `PreToolUse` hook in a pane where `DROVR_AGENT_ID`
  is set
- **THEN** the agent's state file is updated to `"state": "working"` with a
  current `updated_at` timestamp

### Requirement: Claude Stop hook writes done state

The system SHALL provide a hook script registered for Claude's `Stop` event that
writes `AgentState::Done` to the agent's state file.

#### Scenario: Stop hook fires while DROVR_AGENT_ID is set

- **WHEN** Claude fires the `Stop` hook in a pane where `DROVR_AGENT_ID` is set
- **THEN** the agent's state file is updated to `"state": "done"` with a current
  `updated_at` timestamp

### Requirement: Hook scripts are no-ops when DROVR_AGENT_ID is absent

All Claude hook scripts SHALL exit with code 0 and produce no output when the
`DROVR_AGENT_ID` environment variable is empty or unset, so they are safe to
register globally in Claude's settings.

#### Scenario: Hook fires outside a dv-managed pane

- **WHEN** any Claude hook script fires in a pane where `DROVR_AGENT_ID` is not
  set or is empty
- **THEN** the script exits 0, writes nothing, and emits nothing to stdout or
  stderr

### Requirement: Hook scripts delivered in repo under hooks/claude/

The system SHALL commit the hook scripts at `hooks/claude/pre_tool_use.sh` and
`hooks/claude/stop.sh` in the repository so they can be referenced by users when
configuring Claude Code.

#### Scenario: Scripts are present after checkout

- **WHEN** the repository is cloned
- **THEN** `hooks/claude/pre_tool_use.sh` and `hooks/claude/stop.sh` exist and
  are executable (`chmod +x`)
