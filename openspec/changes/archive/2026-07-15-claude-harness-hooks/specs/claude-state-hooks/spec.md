## ADDED Requirements

### Requirement: SessionStart hook calls rd set-state idle

The Claude `SessionStart` hook SHALL invoke `rd set-state idle --harness claude`
so that the agent's state file is written to
`$XDG_DATA_HOME/radr/agents/$TMUX_PANE/state.json` with `"state": "idle"`.
`rd set-state` exits 0 silently when `$TMUX_PANE` is not set.

#### Scenario: SessionStart fires inside tmux

- **WHEN** Claude fires the `SessionStart` hook with source `startup` and
  `$TMUX_PANE` is set
- **THEN** the state file at `$XDG_DATA_HOME/radr/agents/$TMUX_PANE/state.json`
  is written with `"state": "idle"` and the current `updated_at` timestamp

#### Scenario: SessionStart overwrites state from a prior session in the same pane

- **WHEN** a state file already exists for `$TMUX_PANE` (left by a previous
  session) and `SessionStart` fires again in that pane
- **THEN** the existing state file is overwritten with `"state": "idle"`

### Requirement: UserPromptSubmit hook calls rd set-state working

The Claude `UserPromptSubmit` hook SHALL invoke
`rd set-state working --harness claude` (async).

#### Scenario: UserPromptSubmit fires inside tmux

- **WHEN** Claude fires the `UserPromptSubmit` hook and `$TMUX_PANE` is set
- **THEN** the agent's state file is updated to `"state": "working"`

### Requirement: PreToolUse hook calls rd set-state working

The Claude `PreToolUse` hook SHALL invoke
`rd set-state working --harness claude` (async).

#### Scenario: PreToolUse fires inside tmux

- **WHEN** Claude fires the `PreToolUse` hook and `$TMUX_PANE` is set
- **THEN** the agent's state file is updated to `"state": "working"`

### Requirement: Stop hook calls rd set-state done

The Claude `Stop` hook SHALL invoke `rd set-state done --harness claude`
(async).

#### Scenario: Stop hook fires inside tmux

- **WHEN** Claude fires the `Stop` hook and `$TMUX_PANE` is set
- **THEN** the agent's state file is updated to `"state": "done"`

### Requirement: Notification hook calls rd set-state blocked on permission prompts

The Claude `Notification` hook SHALL invoke
`rd set-state blocked --harness claude` (async) when registered with
`matcher: "permission_prompt"`. Because Claude's hook system only fires the hook
when the matcher matches, no stdin-parsing logic is required inside `rd`.

#### Scenario: permission_prompt notification fires inside tmux

- **WHEN** Claude fires the `Notification` hook with `notification_type` of
  `permission_prompt` and `$TMUX_PANE` is set
- **THEN** the agent's state file is updated to `"state": "blocked"`

#### Scenario: Non-permission notification is not forwarded

- **WHEN** Claude fires the `Notification` hook with any `notification_type`
  other than `permission_prompt`
- **THEN** the hook is not triggered (filtered by Claude's `matcher`) and no
  state change occurs

### Requirement: SessionEnd hook calls rd clear-state

The Claude `SessionEnd` hook SHALL invoke `rd clear-state` so the state file is
removed when the session exits normally. `rd clear-state` exits 0 silently when
`$TMUX_PANE` is not set or when no state file exists.

#### Scenario: SessionEnd fires inside tmux

- **WHEN** Claude fires the `SessionEnd` hook and `$TMUX_PANE` is set
- **THEN** the state file at `$XDG_DATA_HOME/radr/agents/$TMUX_PANE/state.json`
  is deleted

#### Scenario: SessionEnd is a no-op when no state file exists

- **WHEN** Claude fires the `SessionEnd` hook but no state file exists for
  `$TMUX_PANE`
- **THEN** `rd clear-state` exits 0 without error

### Requirement: rd set-state and rd clear-state are no-ops outside tmux

Both `rd set-state <state>` and `rd clear-state` SHALL exit with code 0 and
write nothing when `$TMUX_PANE` is not set in the environment and `--pane` is
not supplied.

#### Scenario: Hook fires outside tmux

- **WHEN** `rd set-state` or `rd clear-state` is invoked with `$TMUX_PANE` unset
  and no `--pane` flag
- **THEN** the command exits 0 and makes no changes to the filesystem

### Requirement: rd hooks install claude registers handlers in user Claude settings

The `rd hooks install claude` subcommand SHALL patch `~/.claude/settings.json`
to register the six Claude hook entries using `rd set-state` and
`rd clear-state`. `SessionStart` and `SessionEnd` are registered without `async`
so they complete before Claude proceeds; all other hooks use `async: true`. The
command SHALL be idempotent and SHALL merge into any existing hook configuration
without overwriting it.

#### Scenario: First-time install on a clean settings file

- **WHEN** the user runs `rd hooks install claude` and `~/.claude/settings.json`
  contains no radr hook entries
- **THEN** the file is updated with entries for `SessionStart`,
  `UserPromptSubmit`, `PreToolUse`, `Stop`, `Notification`, and `SessionEnd`
  using `rd set-state` / `rd clear-state` commands

#### Scenario: Idempotent re-install

- **WHEN** the user runs `rd hooks install claude` a second time
- **THEN** no duplicate entries are added and the file content is identical to
  after the first run

#### Scenario: Install merges with existing hooks

- **WHEN** `~/.claude/settings.json` already contains non-radr hook entries
- **THEN** those existing entries are preserved and the radr entries are added
  alongside them
