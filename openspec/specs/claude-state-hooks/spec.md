# Claude State Hooks

## Purpose

Specification for agentctl's Claude harness integration: the eight Claude
lifecycle hook entries (`SessionStart`, `UserPromptSubmit`, `PostToolUse`,
`PreToolUse`, `Stop`, `StopFailure`, `Notification`, `SessionEnd`) that keep the
agent state file in sync with Claude's current activity, and the
`agentctl hooks install claude` command that registers them.

## Requirements

### Requirement: SessionStart hook calls agentctl set-state idle

The Claude `SessionStart` hook SHALL invoke
`agentctl set-state idle --harness claude` when the session starts fresh,
resumes from a prior run, or is cleared. The hook SHALL be registered with
`matcher: "startup|resume|clear"` so it fires for all three sources.
`agentctl set-state` exits 0 silently when `$TMUX_PANE` is not set.

#### Scenario: SessionStart fires on new session inside tmux

- **WHEN** Claude fires the `SessionStart` hook with source `startup` and
  `$TMUX_PANE` is set
- **THEN** the state file at
  `$XDG_DATA_HOME/agentctl/agents/$TMUX_PANE/state.json` is written with
  `"state": "idle"` and the current `updated_at` timestamp

#### Scenario: SessionStart fires on resumed session inside tmux

- **WHEN** a Claude session is killed and later resumed (via `--resume`,
  `--continue`, or `/resume`), causing `SessionStart` to fire with source
  `resume`, and `$TMUX_PANE` is set
- **THEN** the state file is written with `"state": "idle"`, replacing any
  previous state (including "unknown" from a missing file)

#### Scenario: SessionStart fires after /clear inside tmux

- **WHEN** the user runs `/clear` in Claude, causing `SessionStart` to fire with
  source `clear`, and `$TMUX_PANE` is set
- **THEN** the state file is written with `"state": "idle"`

#### Scenario: SessionStart does not fire on compaction

- **WHEN** Claude auto-compacts or the user runs `/compact`, causing
  `SessionStart` to fire with source `compact`
- **THEN** the hook does NOT fire (filtered by Claude's `matcher`) and no state
  change occurs

### Requirement: UserPromptSubmit hook calls agentctl set-state working

The Claude `UserPromptSubmit` hook SHALL invoke
`agentctl set-state working --harness claude` (async).

#### Scenario: UserPromptSubmit fires inside tmux

- **WHEN** Claude fires the `UserPromptSubmit` hook and `$TMUX_PANE` is set
- **THEN** the agent's state file is updated to `"state": "working"`

### Requirement: PostToolUse hook calls agentctl set-state working

The Claude `PostToolUse` hook SHALL invoke
`agentctl set-state working --harness claude` (async) after any tool completes
successfully. This replaces the former `PreToolUse` general hook as the primary
"still working" refresh signal.

#### Scenario: PostToolUse fires inside tmux

- **WHEN** Claude fires the `PostToolUse` hook for any tool and `$TMUX_PANE` is
  set
- **THEN** the agent's state file is updated to `"state": "working"`

### Requirement: PreToolUse hook calls agentctl set-state blocked for interactive tools

The Claude `PreToolUse` hook SHALL invoke
`agentctl set-state blocked --harness claude` (async) when registered with
`matcher: "AskUserQuestion|ExitPlanMode"`. This covers tools that show a
blocking interactive dialog and wait for user input before continuing.

#### Scenario: AskUserQuestion tool fires inside tmux

- **WHEN** Claude calls the `AskUserQuestion` tool and `$TMUX_PANE` is set
- **THEN** the agent's state file is updated to `"state": "blocked"` before the
  interactive dialog is shown

#### Scenario: ExitPlanMode tool fires inside tmux

- **WHEN** Claude calls the `ExitPlanMode` tool (plan approval dialog) and
  `$TMUX_PANE` is set
- **THEN** the agent's state file is updated to `"state": "blocked"` before the
  plan approval dialog is shown

#### Scenario: Non-interactive PreToolUse does not set blocked

- **WHEN** Claude calls any tool other than `AskUserQuestion` or `ExitPlanMode`
- **THEN** this hook does NOT fire (filtered by Claude's `matcher`) and no
  "blocked" state change occurs

### Requirement: Stop hook calls agentctl set-state done

The Claude `Stop` hook SHALL invoke `agentctl set-state done --harness claude`
(async).

#### Scenario: Stop hook fires inside tmux

- **WHEN** Claude fires the `Stop` hook and `$TMUX_PANE` is set
- **THEN** the agent's state file is updated to `"state": "done"`

### Requirement: StopFailure hook calls agentctl set-state done

The Claude `StopFailure` hook SHALL invoke
`agentctl set-state done --harness claude` (async) so that API errors
(rate-limit, server overload, authentication failure, etc.) do not leave state
permanently stuck at "working".

#### Scenario: StopFailure fires inside tmux

- **WHEN** Claude's turn ends due to an API error, firing the `StopFailure`
  hook, and `$TMUX_PANE` is set
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
`~/.claude/settings.json` to register eight Claude hook entries using
`agentctl set-state` and `agentctl clear-state`. `SessionStart` and `SessionEnd`
are registered without `async` so they complete before Claude proceeds; all
other hooks use `async: true`. The command SHALL be idempotent and SHALL merge
into any existing hook configuration without overwriting it.

The eight entries are:

| Event              | Matcher                         | Command                                       | Async |
| ------------------ | ------------------------------- | --------------------------------------------- | ----- |
| `SessionStart`     | `startup\|resume\|clear`        | `agentctl set-state idle --harness claude`    | no    |
| `UserPromptSubmit` | —                               | `agentctl set-state working --harness claude` | yes   |
| `PostToolUse`      | —                               | `agentctl set-state working --harness claude` | yes   |
| `PreToolUse`       | `AskUserQuestion\|ExitPlanMode` | `agentctl set-state blocked --harness claude` | yes   |
| `Stop`             | —                               | `agentctl set-state done --harness claude`    | yes   |
| `StopFailure`      | —                               | `agentctl set-state done --harness claude`    | yes   |
| `Notification`     | `permission_prompt`             | `agentctl set-state blocked --harness claude` | yes   |
| `SessionEnd`       | —                               | `agentctl clear-state`                        | no    |

#### Scenario: First-time install on a clean settings file

- **WHEN** the user runs `agentctl hooks install claude` and
  `~/.claude/settings.json` contains no agentctl hook entries
- **THEN** the file is updated with all eight entries listed above

#### Scenario: Idempotent re-install

- **WHEN** the user runs `agentctl hooks install claude` a second time
- **THEN** no duplicate entries are added and the file content is identical to
  after the first run

#### Scenario: Install merges with existing hooks

- **WHEN** `~/.claude/settings.json` already contains non-agentctl hook entries
- **THEN** those existing entries are preserved and the agentctl entries are
  added alongside them
