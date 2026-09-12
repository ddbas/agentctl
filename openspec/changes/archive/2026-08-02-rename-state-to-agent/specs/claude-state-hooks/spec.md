## MODIFIED Requirements

### Requirement: SessionStart hook calls agentctl agent status idle

The Claude `SessionStart` hook SHALL invoke
`agentctl agent status idle --harness claude` when the session starts fresh,
resumes from a prior run, or is cleared. The hook SHALL be registered with
`matcher: "startup|resume|clear"` so it fires for all three sources.
`agentctl agent status` exits 0 silently when `$TMUX_PANE` is not set.

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

### Requirement: UserPromptSubmit hook calls agentctl agent status working

The Claude `UserPromptSubmit` hook SHALL invoke
`agentctl agent status working --harness claude` (async).

#### Scenario: UserPromptSubmit fires inside tmux

- **WHEN** Claude fires the `UserPromptSubmit` hook and `$TMUX_PANE` is set
- **THEN** the agent's state file is updated to `"state": "working"`

### Requirement: PostToolUse hook calls agentctl agent status working

The Claude `PostToolUse` hook SHALL invoke
`agentctl agent status working --harness claude` (async) after any tool
completes successfully. This replaces the former `PreToolUse` general hook as
the primary "still working" refresh signal.

#### Scenario: PostToolUse fires inside tmux

- **WHEN** Claude fires the `PostToolUse` hook for any tool and `$TMUX_PANE` is
  set
- **THEN** the agent's state file is updated to `"state": "working"`

### Requirement: PreToolUse hook calls agentctl agent status blocked for interactive tools

The Claude `PreToolUse` hook SHALL invoke
`agentctl agent status blocked --harness claude` (async) when registered with
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

### Requirement: Stop hook calls agentctl agent status done

The Claude `Stop` hook SHALL invoke
`agentctl agent status done --harness claude` (async).

#### Scenario: Stop hook fires inside tmux

- **WHEN** Claude fires the `Stop` hook and `$TMUX_PANE` is set
- **THEN** the agent's state file is updated to `"state": "done"`

### Requirement: StopFailure hook calls agentctl agent status done

The Claude `StopFailure` hook SHALL invoke
`agentctl agent status done --harness claude` (async) so that API errors
(rate-limit, server overload, authentication failure, etc.) do not leave state
permanently stuck at "working".

#### Scenario: StopFailure fires inside tmux

- **WHEN** Claude's turn ends due to an API error, firing the `StopFailure`
  hook, and `$TMUX_PANE` is set
- **THEN** the agent's state file is updated to `"state": "done"`

### Requirement: Notification hook calls agentctl agent status blocked on permission prompts

The Claude `Notification` hook SHALL invoke
`agentctl agent status blocked --harness claude` (async) when registered with
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

### Requirement: SessionEnd hook calls agentctl agent clear

The Claude `SessionEnd` hook SHALL invoke `agentctl agent clear` so the state
file is removed when the session exits normally. `agentctl agent clear` exits 0
silently when `$TMUX_PANE` is not set or when no state file exists.

#### Scenario: SessionEnd fires inside tmux

- **WHEN** Claude fires the `SessionEnd` hook and `$TMUX_PANE` is set
- **THEN** the state file at
  `$XDG_DATA_HOME/agentctl/agents/$TMUX_PANE/state.json` is deleted

#### Scenario: SessionEnd is a no-op when no state file exists

- **WHEN** Claude fires the `SessionEnd` hook but no state file exists for
  `$TMUX_PANE`
- **THEN** `agentctl agent clear` exits 0 without error

### Requirement: agentctl agent status and agentctl agent clear are no-ops outside tmux

Both `agentctl agent status <state>` and `agentctl agent clear` SHALL exit with
code 0 and write nothing when `$TMUX_PANE` is not set in the environment and
`--pane` is not supplied.

#### Scenario: Hook fires outside tmux

- **WHEN** `agentctl agent status` or `agentctl agent clear` is invoked with
  `$TMUX_PANE` unset and no `--pane` flag
- **THEN** the command exits 0 and makes no changes to the filesystem

### Requirement: agentctl install claude registers handlers in user Claude settings

The `agentctl install claude` subcommand SHALL patch `~/.claude/settings.json`
to register eight Claude hook entries using `agentctl agent status` and
`agentctl agent clear`. `SessionStart` and `SessionEnd` are registered without
`async` so they complete before Claude proceeds; all other hooks use
`async: true`. The command SHALL be idempotent and SHALL merge into any existing
hook configuration without overwriting it. If `~/.claude/settings.json` already
contains hook entries registered by an older `agentctl` binary using either the
pre-rename `agentctl set-state`/`agentctl clear-state` commands or the
subsequently renamed `agentctl state set`/`agentctl state clear` commands, those
entries SHALL be recognized as agentctl-managed, removed as superseded, and
replaced with the equivalent `agentctl agent status`/`agentctl agent clear`
entries.

The eight entries are:

| Event              | Matcher                         | Command                                          | Async |
| ------------------ | ------------------------------- | ------------------------------------------------ | ----- |
| `SessionStart`     | `startup\|resume\|clear`        | `agentctl agent status idle --harness claude`    | no    |
| `UserPromptSubmit` | —                               | `agentctl agent status working --harness claude` | yes   |
| `PostToolUse`      | —                               | `agentctl agent status working --harness claude` | yes   |
| `PreToolUse`       | `AskUserQuestion\|ExitPlanMode` | `agentctl agent status blocked --harness claude` | yes   |
| `Stop`             | —                               | `agentctl agent status done --harness claude`    | yes   |
| `StopFailure`      | —                               | `agentctl agent status done --harness claude`    | yes   |
| `Notification`     | `permission_prompt`             | `agentctl agent status blocked --harness claude` | yes   |
| `SessionEnd`       | —                               | `agentctl agent clear`                           | no    |

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
  `agentctl` binary installed before either rename)
- **THEN** running `agentctl install claude` removes the old entries and adds
  the equivalent `agentctl agent status ...` / `agentctl agent clear` entries,
  with no duplicate or leftover old-style entries remaining

#### Scenario: Install migrates agentctl state to agentctl agent hook commands

- **WHEN** `~/.claude/settings.json` contains agentctl hook entries using the
  intermediate `agentctl state set ...` / `agentctl state clear` commands (from
  a `agentctl` binary installed after the `set-state`→`state set` rename but
  before this `state`→`agent` rename)
- **THEN** running `agentctl install claude` removes the old entries and adds
  the equivalent `agentctl agent status ...` / `agentctl agent clear` entries,
  with no duplicate or leftover old-style entries remaining

### Requirement: agentctl uninstall claude removes only agentctl-managed hook entries

The `agentctl uninstall claude` subcommand SHALL remove agentctl-managed hook
entries from `~/.claude/settings.json`, recognizing the current-form commands
(`agentctl agent status ...` / `agentctl agent clear`) as well as both prior
legacy forms (`agentctl state set ...` / `agentctl state clear`, and the earlier
`agentctl set-state ...` / `agentctl clear-state`) using the same recognition
logic as `agentctl install claude`'s migration path. All non-agentctl content in
the file, including non-agentctl hook entries under the same event and any other
top-level settings keys, SHALL be preserved unchanged. The command SHALL be
idempotent: running it again after all agentctl entries are removed SHALL make
no further changes.

#### Scenario: Uninstall removes all eight agentctl hook entries

- **WHEN** the user runs `agentctl uninstall claude` and
  `~/.claude/settings.json` contains the eight agentctl-managed hook entries
  installed by `agentctl install claude`
- **THEN** all eight entries are removed and the command prints a summary
  listing each removed entry (prefixed with `-`)

#### Scenario: Uninstall preserves non-agentctl hooks under the same event

- **WHEN** `~/.claude/settings.json` contains a non-agentctl hook entry under
  `UserPromptSubmit` alongside the agentctl-managed entry for that event
- **THEN** `agentctl uninstall claude` removes only the agentctl-managed entry
  and leaves the non-agentctl entry in place

#### Scenario: Uninstall removes legacy pre-rename entries

- **WHEN** `~/.claude/settings.json` contains agentctl hook entries using either
  the earliest `agentctl set-state ...` / `agentctl clear-state` command strings
  or the intermediate `agentctl state set ...` / `agentctl state clear` command
  strings (from an older `agentctl` binary, not yet migrated by
  `agentctl install claude`)
- **THEN** `agentctl uninstall claude` recognizes and removes those entries as
  agentctl-managed

#### Scenario: Uninstall prunes empty event arrays and the hooks key

- **WHEN** removing agentctl entries leaves an event's array empty
- **THEN** that event key is removed from `hooks`; if `hooks` becomes an empty
  object, the `hooks` key is removed from the settings file; the settings file
  itself is never deleted

#### Scenario: Uninstall is a no-op when no agentctl entries are present

- **WHEN** the user runs `agentctl uninstall claude` and
  `~/.claude/settings.json` contains no agentctl-managed hook entries (including
  when the file does not exist)
- **THEN** no filesystem write occurs, the command prints a summary indicating
  there was nothing to remove (prefixed with `=`), and exits 0

#### Scenario: Uninstall is idempotent

- **WHEN** the user runs `agentctl uninstall claude` twice in succession
- **THEN** the second run makes no further changes and reports nothing left to
  remove

#### Scenario: Uninstall does not touch multi-command hook entries

- **WHEN** a single hook entry's `hooks` array contains both a agentctl-managed
  command and a non-agentctl command mixed together
- **THEN** `agentctl uninstall claude` leaves that entry entirely untouched,
  since it cannot be safely attributed as solely agentctl-owned
