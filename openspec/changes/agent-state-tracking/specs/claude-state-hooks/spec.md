## ADDED Requirements

### Requirement: dv hook session-start writes idle state

The `dv hook session-start` subcommand SHALL write `AgentState::Idle` to the
agent's state file when invoked by Claude's `SessionStart` (startup) hook.

#### Scenario: SessionStart fires while DROVR_AGENT_ID is set

- **WHEN** Claude fires the `SessionStart` hook with source `startup` in a pane
  where `DROVR_AGENT_ID` is set
- **THEN** the agent's state file is updated to `"state": "idle"` with a current
  `updated_at` timestamp

### Requirement: dv hook user-prompt-submit writes working state

The `dv hook user-prompt-submit` subcommand SHALL write `AgentState::Working` to
the agent's state file when invoked by Claude's `UserPromptSubmit` hook.

#### Scenario: UserPromptSubmit fires while DROVR_AGENT_ID is set

- **WHEN** Claude fires the `UserPromptSubmit` hook in a pane where
  `DROVR_AGENT_ID` is set
- **THEN** the agent's state file is updated to `"state": "working"` with a
  current `updated_at` timestamp

### Requirement: dv hook pre-tool-use writes working state

The `dv hook pre-tool-use` subcommand SHALL write `AgentState::Working` to the
agent's state file when invoked by Claude's `PreToolUse` hook.

#### Scenario: PreToolUse fires while DROVR_AGENT_ID is set

- **WHEN** Claude fires the `PreToolUse` hook in a pane where `DROVR_AGENT_ID`
  is set
- **THEN** the agent's state file is updated to `"state": "working"` with a
  current `updated_at` timestamp

### Requirement: dv hook stop writes done state

The `dv hook stop` subcommand SHALL write `AgentState::Done` to the agent's
state file when invoked by Claude's `Stop` hook.

#### Scenario: Stop hook fires while DROVR_AGENT_ID is set

- **WHEN** Claude fires the `Stop` hook in a pane where `DROVR_AGENT_ID` is set
- **THEN** the agent's state file is updated to `"state": "done"` with a current
  `updated_at` timestamp

### Requirement: dv hook notification writes blocked state

The `dv hook notification` subcommand SHALL write `AgentState::Blocked` to the
agent's state file when invoked by Claude's `Notification` hook with type
`permission_prompt`.

#### Scenario: permission_prompt notification fires while DROVR_AGENT_ID is set

- **WHEN** Claude fires the `Notification` hook with `notification_type` of
  `permission_prompt` in a pane where `DROVR_AGENT_ID` is set
- **THEN** the agent's state file is updated to `"state": "blocked"` with a
  current `updated_at` timestamp

### Requirement: Hook handlers are no-ops when DROVR_AGENT_ID is absent

All `dv hook <event>` subcommands SHALL exit with code 0 and write nothing when
`DROVR_AGENT_ID` is not set in the environment, so they are safe to register
globally in `~/.claude/settings.json`.

#### Scenario: Hook fires outside a dv-managed pane

- **WHEN** any `dv hook <event>` subcommand fires in an environment where
  `DROVR_AGENT_ID` is not set or is empty
- **THEN** the command exits 0 and makes no changes to the filesystem

### Requirement: dv hooks install registers handlers in user Claude settings

The `dv hooks install` subcommand SHALL patch `~/.claude/settings.json` to
register all five `dv hook <event>` handlers as async Claude hooks. The command
SHALL be idempotent: running it multiple times produces the same configuration
without duplication. It SHALL merge into any existing hook configuration rather
than overwriting the file.

#### Scenario: First-time install on a clean settings file

- **WHEN** the user runs `dv hooks install` and `~/.claude/settings.json`
  contains no drovr hook entries
- **THEN** the file is updated with entries for `SessionStart`,
  `UserPromptSubmit`, `PreToolUse`, `Stop`, and `Notification`, each using
  `async: true`

#### Scenario: Idempotent re-install

- **WHEN** the user runs `dv hooks install` a second time
- **THEN** no duplicate entries are added and the file content is identical to
  after the first run

#### Scenario: Install merges with existing hooks

- **WHEN** `~/.claude/settings.json` already contains non-drovr hook entries
- **THEN** those existing entries are preserved and the drovr entries are added
  alongside them
