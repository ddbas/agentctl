# Claude State Hooks

## Purpose

Specification for radr's Claude harness integration: the nine Claude lifecycle
hook entries (`SessionStart`, `UserPromptSubmit`, `PreToolUse`, `PostToolUse`
(matched and unmatched), `Stop`, `StopFailure`, `Notification`, `SessionEnd`)
that keep the agent state file in sync with Claude's current activity, and the
`rd install claude` command that registers them.

## Requirements

### Requirement: SessionStart hook calls rd agent event SESSION_STARTED

The Claude `SessionStart` hook SHALL invoke a command that reads the hook's JSON
payload from stdin, extracts `session_id`, and invokes
`rd agent event SESSION_STARTED --id <session_id> --harness claude` when the
session starts fresh, resumes from a prior run, or is cleared. The hook SHALL be
registered with `matcher: "startup|resume|clear"` so it fires for all three
sources. `rd agent event` exits 0 silently when `$TMUX_PANE` is not set.

#### Scenario: SessionStart fires on new session inside tmux

- **WHEN** Claude fires the `SessionStart` hook with source `startup` and
  `$TMUX_PANE` is set
- **THEN** the state file at `$XDG_DATA_HOME/radr/agents/$TMUX_PANE/state.json`
  is written with `"status": "idle"` and the current `updated_at` timestamp

#### Scenario: SessionStart fires on resumed session inside tmux

- **WHEN** a Claude session is killed and later resumed (via `--resume`,
  `--continue`, or `/resume`), causing `SessionStart` to fire with source
  `resume`, and `$TMUX_PANE` is set
- **THEN** the state file is written with `"status": "idle"`, replacing any
  previous status (including "unknown" from a missing file)

#### Scenario: SessionStart fires after /clear inside tmux

- **WHEN** the user runs `/clear` in Claude, causing `SessionStart` to fire with
  source `clear`, and `$TMUX_PANE` is set
- **THEN** the state file is written with `"status": "idle"`

#### Scenario: SessionStart does not fire on compaction

- **WHEN** Claude auto-compacts or the user runs `/compact`, causing
  `SessionStart` to fire with source `compact`
- **THEN** the hook does NOT fire (filtered by Claude's `matcher`) and no status
  change occurs

### Requirement: UserPromptSubmit hook calls rd agent event RUN_STARTED

The Claude `UserPromptSubmit` hook SHALL invoke a command that reads the hook's
JSON payload from stdin, extracts `session_id`, and invokes
`rd agent event RUN_STARTED --id <session_id> --harness claude` (async).

#### Scenario: UserPromptSubmit fires inside tmux

- **WHEN** Claude fires the `UserPromptSubmit` hook and `$TMUX_PANE` is set
- **THEN** the agent's state file is updated to `"status": "working"`

### Requirement: PreToolUse hook calls rd agent event TOOL_CALL_START for interactive tools

The Claude `PreToolUse` hook SHALL invoke a command that reads the hook's JSON
payload from stdin, extracts `tool_use_id`, and invokes
`rd agent event TOOL_CALL_START --id <tool_use_id> --type elicitation --harness claude`
(async) when registered with `matcher: "AskUserQuestion|ExitPlanMode"`. This
covers tools that show a blocking interactive dialog and wait for user input
before continuing.

#### Scenario: AskUserQuestion tool fires inside tmux

- **WHEN** Claude calls the `AskUserQuestion` tool and `$TMUX_PANE` is set
- **THEN** the agent's state file is updated to `"status": "blocked"` before the
  interactive dialog is shown

#### Scenario: ExitPlanMode tool fires inside tmux

- **WHEN** Claude calls the `ExitPlanMode` tool (plan approval dialog) and
  `$TMUX_PANE` is set
- **THEN** the agent's state file is updated to `"status": "blocked"` before the
  plan approval dialog is shown

#### Scenario: Non-interactive PreToolUse does not set blocked

- **WHEN** Claude calls any tool other than `AskUserQuestion` or `ExitPlanMode`
- **THEN** this hook does NOT fire (filtered by Claude's `matcher`) and no
  `TOOL_CALL_START --type elicitation` event is dispatched from this
  registration

### Requirement: PostToolUse hook calls rd agent event TOOL_CALL_RESULT for interactive tools

The Claude `PostToolUse` hook SHALL invoke a command that reads the hook's JSON
payload from stdin, extracts `tool_use_id`, and invokes
`rd agent event TOOL_CALL_RESULT --id <tool_use_id> --type elicitation --harness claude`
(async) when registered with `matcher: "AskUserQuestion|ExitPlanMode"`, closing
out the `blocked` status opened by the matching `PreToolUse` registration for
the same tool call.

#### Scenario: AskUserQuestion tool answered inside tmux

- **WHEN** the user answers Claude's `AskUserQuestion` dialog, Claude's
  `PostToolUse` hook fires for that tool call, and `$TMUX_PANE` is set
- **THEN** the agent's state file is updated to `"status": "working"`

#### Scenario: ExitPlanMode tool approved inside tmux

- **WHEN** the user approves Claude's plan via `ExitPlanMode`, Claude's
  `PostToolUse` hook fires for that tool call, and `$TMUX_PANE` is set
- **THEN** the agent's state file is updated to `"status": "working"`

### Requirement: Unmatched PostToolUse hook closes out permission-prompt blocks

The Claude `PostToolUse` hook SHALL additionally be registered without a tool
matcher (firing for every tool) to invoke a command that reads the hook's JSON
payload from stdin, extracts `session_id`, and invokes
`rd agent event TOOL_CALL_RESULT --id "perm-<session_id>" --type elicitation --harness claude`
(async). This closes out the `blocked` status opened by the `Notification`
hook's `permission_prompt` registration, without unconditionally forcing
`working` on every tool completion the way the prior unmatched `PostToolUse`
registration did.

#### Scenario: Tool call after a resolved permission prompt clears blocked

- **WHEN** a `permission_prompt` `Notification` set the pane's status to
  `blocked`, the user grants permission, the tool executes, and Claude fires the
  unmatched `PostToolUse` hook for that tool call
- **THEN** the agent's state file is updated to `"status": "working"`

#### Scenario: Tool call with no outstanding permission prompt is a no-op for status

- **WHEN** no `permission_prompt` `Notification` is outstanding for the current
  session and Claude fires the unmatched `PostToolUse` hook after any tool call
- **THEN** the `TOOL_CALL_RESULT` event is dispatched and logged but the agent's
  status is unchanged (no matching `blocked` transition exists)

### Requirement: Stop hook calls rd agent event RUN_FINISHED

The Claude `Stop` hook SHALL invoke a command that reads the hook's JSON payload
from stdin, extracts `session_id`, and invokes
`rd agent event RUN_FINISHED --id <session_id> --harness claude` (async).

#### Scenario: Stop hook fires inside tmux

- **WHEN** Claude fires the `Stop` hook and `$TMUX_PANE` is set
- **THEN** the agent's state file is updated to `"status": "done"`

### Requirement: StopFailure hook calls rd agent event RUN_FINISHED

The Claude `StopFailure` hook SHALL invoke a command that reads the hook's JSON
payload from stdin, extracts `session_id`, and invokes
`rd agent event RUN_FINISHED --id <session_id> --harness claude` (async) so that
API errors (rate-limit, server overload, authentication failure, etc.) do not
leave status permanently stuck at "working".

#### Scenario: StopFailure fires inside tmux

- **WHEN** Claude's turn ends due to an API error, firing the `StopFailure`
  hook, and `$TMUX_PANE` is set
- **THEN** the agent's state file is updated to `"status": "done"`

### Requirement: Notification hook calls rd agent event TOOL_CALL_START on permission prompts

The Claude `Notification` hook SHALL invoke a command that reads the hook's JSON
payload from stdin, extracts `session_id`, and invokes
`rd agent event TOOL_CALL_START --id "perm-<session_id>" --type elicitation --harness claude`
(async) when registered with `matcher: "permission_prompt"`. Because Claude's
hook system only fires the hook when the matcher matches, no notification-type
branching logic is required inside `rd`.

#### Scenario: permission_prompt notification fires inside tmux

- **WHEN** Claude fires the `Notification` hook with `notification_type` of
  `permission_prompt` and `$TMUX_PANE` is set
- **THEN** the agent's state file is updated to `"status": "blocked"`

#### Scenario: Non-permission notification is not forwarded

- **WHEN** Claude fires the `Notification` hook with any `notification_type`
  other than `permission_prompt`
- **THEN** the hook is not triggered (filtered by Claude's `matcher`) and no
  status change occurs

### Requirement: SessionEnd hook calls rd agent clear

The Claude `SessionEnd` hook SHALL invoke `rd agent clear` so the state file
(and its `events.jsonl`) is removed when the session exits normally.
`rd agent clear` exits 0 silently when `$TMUX_PANE` is not set or when no state
file exists.

#### Scenario: SessionEnd fires inside tmux

- **WHEN** Claude fires the `SessionEnd` hook and `$TMUX_PANE` is set
- **THEN** the state file at `$XDG_DATA_HOME/radr/agents/$TMUX_PANE/state.json`
  and its `events.jsonl` sibling are deleted

#### Scenario: SessionEnd is a no-op when no state file exists

- **WHEN** Claude fires the `SessionEnd` hook but no state file exists for
  `$TMUX_PANE`
- **THEN** `rd agent clear` exits 0 without error

### Requirement: rd agent event and rd agent clear are no-ops outside tmux

Every `rd agent event <NAME>` subcommand and `rd agent clear` SHALL exit with
code 0 and write nothing when `$TMUX_PANE` is not set in the environment and
`--pane` is not supplied.

#### Scenario: Hook fires outside tmux

- **WHEN** `rd agent event <NAME>` or `rd agent clear` is invoked with
  `$TMUX_PANE` unset and no `--pane` flag
- **THEN** the command exits 0 and makes no changes to the filesystem

### Requirement: rd install claude registers handlers in user Claude settings

The `rd install claude` subcommand SHALL patch `~/.claude/settings.json` to
register nine Claude hook entries using `rd agent event` and `rd agent clear`.
`SessionStart` and `SessionEnd` are registered without `async` so they complete
before Claude proceeds; all other hooks use `async: true`. The command SHALL be
idempotent and SHALL merge into any existing hook configuration without
overwriting it. If `~/.claude/settings.json` already contains hook entries
registered by an older `rd` binary using the pre-rename
`rd set-state`/`rd clear-state` commands, the subsequently renamed
`rd state set`/`rd state clear` commands, or the subsequently renamed
`rd agent status`/`rd agent clear` commands, those entries SHALL be recognized
as radr-managed, removed as superseded, and replaced with the equivalent
`rd agent event`/`rd agent clear` entries. If `~/.claude/settings.json` contains
hook entries using `rd agent watch arm`, `rd agent watch resolve`, or
`rd agent stop` command strings (from a build of `rd` implementing a feature
never shipped on the mainline), those entries SHALL be recognized as
radr-managed and removed as superseded, with no replacement entries added since
no current hook set corresponds to them.

The nine entries are:

| Event              | Matcher                         | Command                                                                                                                           | Async |
| ------------------ | ------------------------------- | --------------------------------------------------------------------------------------------------------------------------------- | ----- |
| `SessionStart`     | `startup\|resume\|clear`        | reads stdin for `session_id`, then `rd agent event SESSION_STARTED --id <session_id> --harness claude`                            | no    |
| `UserPromptSubmit` | —                               | reads stdin for `session_id`, then `rd agent event RUN_STARTED --id <session_id> --harness claude`                                | yes   |
| `PreToolUse`       | `AskUserQuestion\|ExitPlanMode` | reads stdin for `tool_use_id`, then `rd agent event TOOL_CALL_START --id <tool_use_id> --type elicitation --harness claude`       | yes   |
| `PostToolUse`      | `AskUserQuestion\|ExitPlanMode` | reads stdin for `tool_use_id`, then `rd agent event TOOL_CALL_RESULT --id <tool_use_id> --type elicitation --harness claude`      | yes   |
| `PostToolUse`      | — (unmatched)                   | reads stdin for `session_id`, then `rd agent event TOOL_CALL_RESULT --id "perm-<session_id>" --type elicitation --harness claude` | yes   |
| `Notification`     | `permission_prompt`             | reads stdin for `session_id`, then `rd agent event TOOL_CALL_START --id "perm-<session_id>" --type elicitation --harness claude`  | yes   |
| `Stop`             | —                               | reads stdin for `session_id`, then `rd agent event RUN_FINISHED --id <session_id> --harness claude`                               | yes   |
| `StopFailure`      | —                               | reads stdin for `session_id`, then `rd agent event RUN_FINISHED --id <session_id> --harness claude`                               | yes   |
| `SessionEnd`       | —                               | `rd agent clear`                                                                                                                  | no    |

#### Scenario: First-time install on a clean settings file

- **WHEN** the user runs `rd install claude` and `~/.claude/settings.json`
  contains no radr hook entries
- **THEN** the file is updated with all nine entries listed above

#### Scenario: Idempotent re-install

- **WHEN** the user runs `rd install claude` a second time
- **THEN** no duplicate entries are added and the file content is identical to
  after the first run

#### Scenario: Install merges with existing hooks

- **WHEN** `~/.claude/settings.json` already contains non-radr hook entries
- **THEN** those existing entries are preserved and the radr entries are added
  alongside them

#### Scenario: Install migrates pre-rename hook commands

- **WHEN** `~/.claude/settings.json` contains radr hook entries using the old
  `rd set-state ...` / `rd clear-state` commands (from a `rd` binary installed
  before any of the renames)
- **THEN** running `rd install claude` removes the old entries and adds the
  equivalent new-form entries, with no duplicate or leftover old-style entries
  remaining

#### Scenario: Install migrates rd state to rd agent hook commands

- **WHEN** `~/.claude/settings.json` contains radr hook entries using the
  intermediate `rd state set ...` / `rd state clear` commands
- **THEN** running `rd install claude` removes the old entries and adds the
  equivalent new-form entries, with no duplicate or leftover old-style entries
  remaining

#### Scenario: Install migrates rd agent status to rd agent event hook commands

- **WHEN** `~/.claude/settings.json` contains radr hook entries using the
  `rd agent status ...` / `rd agent clear` commands (from a `rd` binary
  installed after the `state`→`agent` rename but before this `status`→`event`
  rename)
- **THEN** running `rd install claude` removes the old entries and adds the
  equivalent `rd agent event ...` / `rd agent clear` entries, with no duplicate
  or leftover old-style entries remaining

#### Scenario: Install removes orphaned Monitor-watch and stop entries with no replacement

- **WHEN** `~/.claude/settings.json` contains a `PreToolUse` entry (matcher
  `Monitor`) invoking `rd agent watch arm --harness claude`, a `PostToolUse`
  entry (matcher `Monitor`) invoking `rd agent watch resolve --harness claude`,
  and `Stop`/`StopFailure` entries invoking `rd agent stop --harness claude`
  (left over from a build of `rd` implementing a feature never shipped on the
  mainline)
- **THEN** running `rd install claude` removes all four entries, reports each in
  the summary's `[removed]` list, adds no replacement entries for them, and
  still adds/leaves-unchanged the nine current entries listed above

### Requirement: rd uninstall claude removes only radr-managed hook entries

The `rd uninstall claude` subcommand SHALL remove radr-managed hook entries from
`~/.claude/settings.json`, recognizing the current-form commands
(`rd agent event ...` / `rd agent clear`) as well as all prior legacy forms
(`rd agent status ...` / `rd agent clear`, `rd state set ...` /
`rd state clear`, the earlier `rd set-state ...` / `rd clear-state`, and the
never-shipped `rd agent watch arm`/`rd agent watch resolve`/`rd agent stop`
forms) using the same recognition logic as `rd install claude`'s migration path.
All non-radr content in the file, including non-radr hook entries under the same
event and any other top-level settings keys, SHALL be preserved unchanged. The
command SHALL be idempotent: running it again after all radr entries are removed
SHALL make no further changes.

#### Scenario: Uninstall removes all nine radr hook entries

- **WHEN** the user runs `rd uninstall claude` and `~/.claude/settings.json`
  contains the nine radr-managed hook entries installed by `rd install claude`
- **THEN** all nine entries are removed and the command prints a summary listing
  each removed entry (prefixed with `-`)

#### Scenario: Uninstall preserves non-radr hooks under the same event

- **WHEN** `~/.claude/settings.json` contains a non-radr hook entry under
  `UserPromptSubmit` alongside the radr-managed entry for that event
- **THEN** `rd uninstall claude` removes only the radr-managed entry and leaves
  the non-radr entry in place

#### Scenario: Uninstall removes legacy pre-rename entries

- **WHEN** `~/.claude/settings.json` contains radr hook entries using any of the
  earlier `rd set-state ...` / `rd clear-state`, `rd state set ...` /
  `rd state clear`, or `rd agent status ...` / `rd agent clear` command strings
  (from an older `rd` binary, not yet migrated by `rd install claude`)
- **THEN** `rd uninstall claude` recognizes and removes those entries as
  radr-managed

#### Scenario: Uninstall removes orphaned Monitor-watch and stop entries

- **WHEN** `~/.claude/settings.json` contains hook entries invoking
  `rd agent watch arm --harness claude`,
  `rd agent watch resolve --harness claude`, or `rd agent stop --harness claude`
- **THEN** `rd uninstall claude` recognizes and removes those entries as
  radr-managed, alongside any other radr entries present

#### Scenario: Uninstall prunes empty event arrays and the hooks key

- **WHEN** removing radr entries leaves an event's array empty
- **THEN** that event key is removed from `hooks`; if `hooks` becomes an empty
  object, the `hooks` key is removed from the settings file; the settings file
  itself is never deleted

#### Scenario: Uninstall is a no-op when no radr entries are present

- **WHEN** the user runs `rd uninstall claude` and `~/.claude/settings.json`
  contains no radr-managed hook entries (including when the file does not exist)
- **THEN** no filesystem write occurs, the command prints a summary indicating
  there was nothing to remove (prefixed with `=`), and exits 0

#### Scenario: Uninstall is idempotent

- **WHEN** the user runs `rd uninstall claude` twice in succession
- **THEN** the second run makes no further changes and reports nothing left to
  remove

#### Scenario: Uninstall does not touch multi-command hook entries

- **WHEN** a single hook entry's `hooks` array contains both a radr-managed
  command and a non-radr command mixed together
- **THEN** `rd uninstall claude` leaves that entry entirely untouched, since it
  cannot be safely attributed as solely radr-owned
