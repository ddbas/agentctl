## ADDED Requirements

### Requirement: SessionStart hook calls agentctl set-state idle

The Claude `SessionStart` hook SHALL invoke
`agentctl set-state idle --harness claude` so that the agent's state file is
written to `$XDG_DATA_HOME/agentctl/agents/$TMUX_PANE/state.json` with
`"state": "idle"`. `agentctl set-state` exits 0 silently when `$TMUX_PANE` is
not set.

#### Scenario: SessionStart fires inside tmux

- **WHEN** Claude fires the `SessionStart` hook with source `startup` and
  `$TMUX_PANE` is set
- **THEN** the state file at
  `$XDG_DATA_HOME/agentctl/agents/$TMUX_PANE/state.json` is written with
  `"state": "idle"` and the current `updated_at` timestamp

#### Scenario: SessionStart overwrites state from a prior session in the same pane

- **WHEN** a state file already exists for `$TMUX_PANE` (left by a previous
  session) and `SessionStart` fires again in that pane
- **THEN** the existing state file is overwritten with `"state": "idle"`

### Requirement: UserPromptSubmit hook calls agentctl set-state working

The Claude `UserPromptSubmit` hook SHALL invoke
`agentctl set-state working --harness claude` (async).

#### Scenario: UserPromptSubmit fires inside tmux

- **WHEN** Claude fires the `UserPromptSubmit` hook and `$TMUX_PANE` is set
- **THEN** the agent's state file is updated to `"state": "working"`

### Requirement: PreToolUse hook calls agentctl set-state working

The Claude `PreToolUse` hook SHALL invoke
`agentctl set-state working --harness claude` (async).

#### Scenario: PreToolUse fires inside tmux

- **WHEN** Claude fires the `PreToolUse` hook and `$TMUX_PANE` is set
- **THEN** the agent's state file is updated to `"state": "working"`

### Requirement: Stop hook calls agentctl set-state done

The Claude `Stop` hook SHALL invoke `agentctl set-state done --harness claude`
(async).

#### Scenario: Stop hook fires inside tmux

- **WHEN** Claude fires the `Stop` hook and `$TMUX_PANE` is set
- **THEN** the agent's state file is updated to `"state": "done"`

### Requirement: Notification hook calls agentctl set-state blocked on permission prompts

The Claude `Notification` hook SHALL invoke
`agentctl set-state blocked --harness claude` (async) when registered with
`matcher: "permission_prompt"`. Because Claude's hook system only fires the hook
when the matcher matches, no stdin-parsing logic is required inside `agentctl`.

#### Scenario: permission_prompt notification fires inside tmux

- **WHEN** Claude fires the `Notification` hook with `notification_type` of
  `permission_prompt` and `$TMUX_PANE` is set
- **THEN** the agent's state file is updated to `"state": "blocked"`

#### Scenario: Non-permission notification is not forwarded

- **WHEN** Claude fires the `Notification` hook with any `notification_type`
  other than `permission_prompt`
- **THEN** the hook is not triggered (filtered by Claude's `matcher`) and no
  state change occurs

### Requirement: SessionEnd hook calls agentctl clear-state

The Claude `SessionEnd` hook SHALL invoke `agentctl clear-state` so the state
file is removed when the session exits normally. `agentctl clear-state` exits 0
silently when `$TMUX_PANE` is not set or when no state file exists.

#### Scenario: SessionEnd fires inside tmux

- **WHEN** Claude fires the `SessionEnd` hook and `$TMUX_PANE` is set
- **THEN** the state file at
  `$XDG_DATA_HOME/agentctl/agents/$TMUX_PANE/state.json` is deleted

#### Scenario: SessionEnd is a no-op when no state file exists

- **WHEN** Claude fires the `SessionEnd` hook but no state file exists for
  `$TMUX_PANE`
- **THEN** `agentctl clear-state` exits 0 without error

### Requirement: agentctl set-state and agentctl clear-state are no-ops outside tmux

Both `agentctl set-state <state>` and `agentctl clear-state` SHALL exit with
code 0 and write nothing when `$TMUX_PANE` is not set in the environment and
`--pane` is not supplied.

#### Scenario: Hook fires outside tmux

- **WHEN** `agentctl set-state` or `agentctl clear-state` is invoked with
  `$TMUX_PANE` unset and no `--pane` flag
- **THEN** the command exits 0 and makes no changes to the filesystem

### Requirement: agentctl hooks install claude registers handlers in user Claude settings

The `agentctl hooks install claude` subcommand SHALL patch
`~/.claude/settings.json` to register the six Claude hook entries using
`agentctl set-state` and `agentctl clear-state`. `SessionStart` and `SessionEnd`
are registered without `async` so they complete before Claude proceeds; all
other hooks use `async: true`. The command SHALL be idempotent and SHALL merge
into any existing hook configuration without overwriting it.

#### Scenario: First-time install on a clean settings file

- **WHEN** the user runs `agentctl hooks install claude` and
  `~/.claude/settings.json` contains no agentctl hook entries
- **THEN** the file is updated with entries for `SessionStart`,
  `UserPromptSubmit`, `PreToolUse`, `Stop`, `Notification`, and `SessionEnd`
  using `agentctl set-state` / `agentctl clear-state` commands

#### Scenario: Idempotent re-install

- **WHEN** the user runs `agentctl hooks install claude` a second time
- **THEN** no duplicate entries are added and the file content is identical to
  after the first run

#### Scenario: Install merges with existing hooks

- **WHEN** `~/.claude/settings.json` already contains non-agentctl hook entries
- **THEN** those existing entries are preserved and the agentctl entries are
  added alongside them
