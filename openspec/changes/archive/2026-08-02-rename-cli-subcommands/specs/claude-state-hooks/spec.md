## MODIFIED Requirements

### Requirement: SessionStart hook calls agentctl state set idle

The Claude `SessionStart` hook SHALL invoke
`agentctl state set idle --harness claude` when the session starts fresh,
resumes from a prior run, or is cleared. The hook SHALL be registered with
`matcher: "startup|resume|clear"` so it fires for all three sources.
`agentctl state set` exits 0 silently when `$TMUX_PANE` is not set.

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

### Requirement: UserPromptSubmit hook calls agentctl state set working

The Claude `UserPromptSubmit` hook SHALL invoke
`agentctl state set working --harness claude` (async).

#### Scenario: UserPromptSubmit fires inside tmux

- **WHEN** Claude fires the `UserPromptSubmit` hook and `$TMUX_PANE` is set
- **THEN** the agent's state file is updated to `"state": "working"`

### Requirement: PostToolUse hook calls agentctl state set working

The Claude `PostToolUse` hook SHALL invoke
`agentctl state set working --harness claude` (async) after any tool completes
successfully. This replaces the former `PreToolUse` general hook as the primary
"still working" refresh signal.

#### Scenario: PostToolUse fires inside tmux

- **WHEN** Claude fires the `PostToolUse` hook for any tool and `$TMUX_PANE` is
  set
- **THEN** the agent's state file is updated to `"state": "working"`

### Requirement: PreToolUse hook calls agentctl state set blocked for interactive tools

The Claude `PreToolUse` hook SHALL invoke
`agentctl state set blocked --harness claude` (async) when registered with
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

### Requirement: Stop hook calls agentctl state set done

The Claude `Stop` hook SHALL invoke `agentctl state set done --harness claude`
(async).

#### Scenario: Stop hook fires inside tmux

- **WHEN** Claude fires the `Stop` hook and `$TMUX_PANE` is set
- **THEN** the agent's state file is updated to `"state": "done"`

### Requirement: StopFailure hook calls agentctl state set done

The Claude `StopFailure` hook SHALL invoke
`agentctl state set done --harness claude` (async) so that API errors
(rate-limit, server overload, authentication failure, etc.) do not leave state
permanently stuck at "working".

#### Scenario: StopFailure fires inside tmux

- **WHEN** Claude's turn ends due to an API error, firing the `StopFailure`
  hook, and `$TMUX_PANE` is set
- **THEN** the agent's state file is updated to `"state": "done"`

### Requirement: Notification hook calls agentctl state set blocked on permission prompts

The Claude `Notification` hook SHALL invoke
`agentctl state set blocked --harness claude` (async) when registered with
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

### Requirement: SessionEnd hook calls agentctl state clear

The Claude `SessionEnd` hook SHALL invoke `agentctl state clear` so the state
file is removed when the session exits normally. `agentctl state clear` exits 0
silently when `$TMUX_PANE` is not set or when no state file exists.

#### Scenario: SessionEnd fires inside tmux

- **WHEN** Claude fires the `SessionEnd` hook and `$TMUX_PANE` is set
- **THEN** the state file at
  `$XDG_DATA_HOME/agentctl/agents/$TMUX_PANE/state.json` is deleted

#### Scenario: SessionEnd is a no-op when no state file exists

- **WHEN** Claude fires the `SessionEnd` hook but no state file exists for
  `$TMUX_PANE`
- **THEN** `agentctl state clear` exits 0 without error

### Requirement: agentctl state set and agentctl state clear are no-ops outside tmux

Both `agentctl state set <state>` and `agentctl state clear` SHALL exit with
code 0 and write nothing when `$TMUX_PANE` is not set in the environment and
`--pane` is not supplied.

#### Scenario: Hook fires outside tmux

- **WHEN** `agentctl state set` or `agentctl state clear` is invoked with
  `$TMUX_PANE` unset and no `--pane` flag
- **THEN** the command exits 0 and makes no changes to the filesystem

### Requirement: agentctl install claude registers handlers in user Claude settings

The `agentctl install claude` subcommand SHALL patch `~/.claude/settings.json`
to register eight Claude hook entries using `agentctl state set` and
`agentctl state clear`. `SessionStart` and `SessionEnd` are registered without
`async` so they complete before Claude proceeds; all other hooks use
`async: true`. The command SHALL be idempotent and SHALL merge into any existing
hook configuration without overwriting it. If `~/.claude/settings.json` already
contains hook entries registered by a pre-rename `agentctl` binary (using
`agentctl set-state`/`agentctl clear-state` commands), those entries SHALL be
recognized as agentctl-managed, removed as superseded, and replaced with the
equivalent `agentctl state set`/`agentctl state clear` entries.

The eight entries are:

| Event              | Matcher                         | Command                                       | Async |
| ------------------ | ------------------------------- | --------------------------------------------- | ----- |
| `SessionStart`     | `startup\|resume\|clear`        | `agentctl state set idle --harness claude`    | no    |
| `UserPromptSubmit` | —                               | `agentctl state set working --harness claude` | yes   |
| `PostToolUse`      | —                               | `agentctl state set working --harness claude` | yes   |
| `PreToolUse`       | `AskUserQuestion\|ExitPlanMode` | `agentctl state set blocked --harness claude` | yes   |
| `Stop`             | —                               | `agentctl state set done --harness claude`    | yes   |
| `StopFailure`      | —                               | `agentctl state set done --harness claude`    | yes   |
| `Notification`     | `permission_prompt`             | `agentctl state set blocked --harness claude` | yes   |
| `SessionEnd`       | —                               | `agentctl state clear`                        | no    |

#### Scenario: First-time install on a clean settings file

- **WHEN** the user runs `agentctl install claude` and `~/.claude/settings.json`
  contains no agentctl hook entries
- **THEN** the file is updated with all eight entries listed above

#### Scenario: Idempotent re-install

- **WHEN** the user runs `agentctl install claude` a second time
- **THEN** no duplicate entries are added and the file content is identical to
  after the first run

#### Scenario: Install merges with existing hooks

- **WHEN** `~/.claude/settings.json` already contains non-agentctl hook entries
- **THEN** those existing entries are preserved and the agentctl entries are
  added alongside them

#### Scenario: Install migrates pre-rename hook commands

- **WHEN** `~/.claude/settings.json` contains agentctl hook entries using the
  old `agentctl set-state ...` / `agentctl clear-state` commands (from a
  `agentctl` binary installed before this rename)
- **THEN** running `agentctl install claude` removes the old entries and adds
  the equivalent `agentctl state set ...` / `agentctl state clear` entries, with
  no duplicate or leftover old-style entries remaining
