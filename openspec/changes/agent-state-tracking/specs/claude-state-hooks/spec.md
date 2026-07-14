## ADDED Requirements

### Requirement: dv hook session-start writes idle state

The `dv hook session-start` subcommand SHALL write `AgentState::Idle` to the
agent's state file when invoked by Claude's `SessionStart` (startup) hook. It
SHALL exit 0 silently if `$TMUX_PANE` is not set.

#### Scenario: SessionStart fires inside tmux

- **WHEN** Claude fires the `SessionStart` hook with source `startup` and
  `$TMUX_PANE` is set
- **THEN** the state file at `$XDG_DATA_HOME/drovr/agents/$TMUX_PANE/state.json`
  is written with `"state": "idle"` and the current `updated_at` timestamp

#### Scenario: SessionStart overwrites state from a prior session in the same pane

- **WHEN** a state file already exists for `$TMUX_PANE` (left by a previous
  session) and `SessionStart` fires again in that pane
- **THEN** the existing state file is overwritten with `"state": "idle"`

### Requirement: dv hook user-prompt-submit writes working state

The `dv hook user-prompt-submit` subcommand SHALL write `AgentState::Working` to
the agent's state file when invoked by Claude's `UserPromptSubmit` hook.

#### Scenario: UserPromptSubmit fires inside tmux

- **WHEN** Claude fires the `UserPromptSubmit` hook and `$TMUX_PANE` is set
- **THEN** the agent's state file is updated to `"state": "working"`

### Requirement: dv hook pre-tool-use writes working state

The `dv hook pre-tool-use` subcommand SHALL write `AgentState::Working` to the
agent's state file when invoked by Claude's `PreToolUse` hook.

#### Scenario: PreToolUse fires inside tmux

- **WHEN** Claude fires the `PreToolUse` hook and `$TMUX_PANE` is set
- **THEN** the agent's state file is updated to `"state": "working"`

### Requirement: dv hook stop writes done state

The `dv hook stop` subcommand SHALL write `AgentState::Done` to the agent's
state file when invoked by Claude's `Stop` hook.

#### Scenario: Stop hook fires inside tmux

- **WHEN** Claude fires the `Stop` hook and `$TMUX_PANE` is set
- **THEN** the agent's state file is updated to `"state": "done"`

### Requirement: dv hook notification writes blocked state

The `dv hook notification` subcommand SHALL write `AgentState::Blocked` to the
agent's state file when invoked by Claude's `Notification` hook with type
`permission_prompt`. It SHALL exit 0 silently for all other notification types.

#### Scenario: permission_prompt notification fires inside tmux

- **WHEN** Claude fires the `Notification` hook with `notification_type` of
  `permission_prompt` and `$TMUX_PANE` is set
- **THEN** the agent's state file is updated to `"state": "blocked"`

#### Scenario: Non-permission notification is ignored

- **WHEN** Claude fires the `Notification` hook with any `notification_type`
  other than `permission_prompt`
- **THEN** the command exits 0 and makes no changes to the filesystem

### Requirement: dv hook session-end deletes state file

The `dv hook session-end` subcommand SHALL delete the agent's state file when
invoked by Claude's `SessionEnd` hook. It SHALL exit 0 silently if `$TMUX_PANE`
is not set or if no state file exists for the pane.

#### Scenario: SessionEnd fires inside tmux

- **WHEN** Claude fires the `SessionEnd` hook and `$TMUX_PANE` is set
- **THEN** the state file at `$XDG_DATA_HOME/drovr/agents/$TMUX_PANE/state.json`
  is deleted

#### Scenario: SessionEnd is a no-op when no state file exists

- **WHEN** Claude fires the `SessionEnd` hook but no state file exists for
  `$TMUX_PANE`
- **THEN** the command exits 0 without error

### Requirement: Hook handlers are no-ops outside tmux

All `dv hook <event>` subcommands SHALL exit with code 0 and write nothing when
`$TMUX_PANE` is not set in the environment.

#### Scenario: Hook fires outside tmux

- **WHEN** any `dv hook <event>` subcommand is invoked with `$TMUX_PANE` unset
- **THEN** the command exits 0 and makes no changes to the filesystem

### Requirement: dv hooks install registers handlers in user Claude settings

The `dv hooks install` subcommand SHALL patch `~/.claude/settings.json` to
register all six `dv hook <event>` handlers. `SessionStart` and `SessionEnd` are
registered without `async` so they complete before Claude proceeds. All other
handlers use `async: true`. The command SHALL be idempotent and SHALL merge into
any existing hook configuration without overwriting it.

#### Scenario: First-time install on a clean settings file

- **WHEN** the user runs `dv hooks install` and `~/.claude/settings.json`
  contains no drovr hook entries
- **THEN** the file is updated with entries for `SessionStart`,
  `UserPromptSubmit`, `PreToolUse`, `Stop`, `Notification`, and `SessionEnd`

#### Scenario: Idempotent re-install

- **WHEN** the user runs `dv hooks install` a second time
- **THEN** no duplicate entries are added and the file content is identical to
  after the first run

#### Scenario: Install merges with existing hooks

- **WHEN** `~/.claude/settings.json` already contains non-drovr hook entries
- **THEN** those existing entries are preserved and the drovr entries are added
  alongside them
