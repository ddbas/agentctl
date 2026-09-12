# Claude State Hooks

## Purpose

Specification for agentctl's Claude harness integration: the eleven Claude
lifecycle hook entries (`SessionStart`, `UserPromptSubmit`, `PreToolUse`
(unmatched and matched), `PostToolUse` (unmatched and matched),
`PostToolUseFailure` (unmatched and matched), `Stop`, `StopFailure`,
`SessionEnd`) that keep the agent state file in sync with Claude's current
activity, and the `agentctl install claude` command that registers them.

## Requirements

### Requirement: SessionStart hook calls agentctl agent event SESSION_STARTED

The Claude `SessionStart` hook SHALL invoke a command that reads the hook's JSON
payload from stdin, extracts `session_id`, and invokes
`agentctl agent event SESSION_STARTED --id <session_id> --harness claude` when
the session starts fresh, resumes from a prior run, or is cleared. The hook
SHALL be registered with `matcher: "startup|resume|clear"` so it fires for all
three sources. `agentctl agent event` exits 0 silently when `$TMUX_PANE` is not
set.

#### Scenario: SessionStart fires on new session inside tmux

- **WHEN** Claude fires the `SessionStart` hook with source `startup` and
  `$TMUX_PANE` is set
- **THEN** the state file at
  `$XDG_DATA_HOME/agentctl/agents/$TMUX_PANE/state.json` is written with
  `"status": "idle"` and the current `updated_at` timestamp

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

### Requirement: UserPromptSubmit hook calls agentctl agent event RUN_STARTED

The Claude `UserPromptSubmit` hook SHALL invoke a command that reads the hook's
JSON payload from stdin, extracts `session_id`, and invokes
`agentctl agent event RUN_STARTED --id <session_id> --harness claude` (async).

#### Scenario: UserPromptSubmit fires inside tmux

- **WHEN** Claude fires the `UserPromptSubmit` hook and `$TMUX_PANE` is set
- **THEN** the agent's state file is updated to `"status": "working"`

### Requirement: PreToolUse hook calls agentctl agent event TOOL_CALL_START for every tool call

The Claude `PreToolUse` hook SHALL additionally be registered without a tool
matcher (firing for every tool) to invoke a command that reads the hook's JSON
payload from stdin, extracts `tool_use_id`, and invokes
`agentctl agent event TOOL_CALL_START --id <tool_use_id> --harness claude`
(async, no `--type`), incrementing the pane's `active_tools` count for every
tool call, mirroring the pi extension's `tool_call` handler.

#### Scenario: Any tool call increments active_tools

- **WHEN** Claude fires the unmatched `PreToolUse` hook for any tool call and
  `$TMUX_PANE` is set
- **THEN** the state file's `active_tools` count is incremented by one

#### Scenario: Concurrent tool calls each get a distinct id

- **WHEN** Claude fires the unmatched `PreToolUse` hook for two tool calls
  running concurrently
- **THEN** each invocation uses its own `tool_use_id`, so both increments are
  recorded independently

### Requirement: PreToolUse hook calls agentctl agent event TOOL_CALL_START for interactive tools

The Claude `PreToolUse` hook SHALL invoke a command that reads the hook's JSON
payload from stdin, extracts `tool_use_id`, and invokes
`agentctl agent event TOOL_CALL_START --id <tool_use_id> --type elicitation --harness claude`
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

### Requirement: PostToolUse hook calls agentctl agent event TOOL_CALL_RESULT for every tool call

The Claude `PostToolUse` hook SHALL additionally be registered without a tool
matcher (firing for every tool) to invoke a command that reads the hook's JSON
payload from stdin, extracts `tool_use_id`, and invokes
`agentctl agent event TOOL_CALL_RESULT --id <tool_use_id> --harness claude`
(async, no `--type`), decrementing the pane's `active_tools` count for every
completed tool call, mirroring the pi extension's `tool_result` handler. This
entry only fires for tool calls that succeed; failed tool calls are covered by
the matching unmatched `PostToolUseFailure` registration instead, since Claude
Code fires exactly one of the two per tool call.

#### Scenario: Any successful tool call completion decrements active_tools

- **WHEN** Claude fires the unmatched `PostToolUse` hook for any tool call and
  `$TMUX_PANE` is set
- **THEN** the state file's `active_tools` count is decremented by one (never
  below zero)

#### Scenario: Interactive tool calls still track active_tools alongside blocked status

- **WHEN** Claude fires `PreToolUse`/`PostToolUse` for `AskUserQuestion` or
  `ExitPlanMode`
- **THEN** both the elicitation-typed entries (driving `blocked` status) and the
  general untyped entries (driving `active_tools`) fire for that same tool call,
  each independently

### Requirement: PostToolUse hook calls agentctl agent event TOOL_CALL_RESULT for interactive tools

The Claude `PostToolUse` hook SHALL invoke a command that reads the hook's JSON
payload from stdin, extracts `tool_use_id`, and invokes
`agentctl agent event TOOL_CALL_RESULT --id <tool_use_id> --type elicitation --harness claude`
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

### Requirement: PostToolUseFailure hook calls agentctl agent event TOOL_CALL_RESULT for every failed tool call

The Claude `PostToolUseFailure` hook SHALL be registered without a tool matcher
(firing for every tool whose call fails) to invoke a command that reads the
hook's JSON payload from stdin, extracts `tool_use_id`, and invokes
`agentctl agent event TOOL_CALL_RESULT --id <tool_use_id> --harness claude`
(async, no `--type`), decrementing the pane's `active_tools` count for every
failed tool call, exactly mirroring the untyped `PostToolUse` registration. This
closes out the `TOOL_CALL_START` opened by the unmatched `PreToolUse`
registration for tool calls that fail rather than succeed, since Claude Code
fires exactly one of `PostToolUse`/`PostToolUseFailure` per tool call, never
both.

#### Scenario: A failed tool call still decrements active_tools

- **WHEN** Claude fires the unmatched `PostToolUseFailure` hook for a tool call
  that errored (e.g. a nonzero Bash exit or an MCP tool error) and `$TMUX_PANE`
  is set
- **THEN** the state file's `active_tools` count is decremented by one (never
  below zero)

#### Scenario: Success and failure never both fire for the same tool call

- **WHEN** a single tool call completes
- **THEN** Claude Code fires exactly one of the unmatched `PostToolUse` or
  unmatched `PostToolUseFailure` hooks for that `tool_use_id`, never both, so
  `active_tools` is decremented exactly once per completed tool call regardless
  of outcome

### Requirement: PostToolUseFailure hook calls agentctl agent event TOOL_CALL_RESULT for failed interactive tools

The Claude `PostToolUseFailure` hook SHALL invoke a command that reads the
hook's JSON payload from stdin, extracts `tool_use_id`, and invokes
`agentctl agent event TOOL_CALL_RESULT --id <tool_use_id> --type elicitation --harness claude`
(async) when registered with `matcher: "AskUserQuestion|ExitPlanMode"`, closing
out the `blocked` status opened by the matching `PreToolUse` registration for
the same tool call, for the (rare) case where an interactive tool call itself
fails rather than completing normally.

#### Scenario: AskUserQuestion tool call fails inside tmux

- **WHEN** Claude's `AskUserQuestion` tool call fails and Claude fires the
  matched `PostToolUseFailure` hook, and `$TMUX_PANE` is set
- **THEN** the agent's state file is updated to `"status": "working"`

### Requirement: Stop hook calls agentctl agent event RUN_FINISHED

The Claude `Stop` hook SHALL invoke a command that reads the hook's JSON payload
from stdin, extracts `session_id`, and invokes
`agentctl agent event RUN_FINISHED --id <session_id> --harness claude` (async).
This is authoritative regardless of the pane's current status: it drives the
pane to `done` from any status (`idle`, `working`, `blocked`, or `done` itself),
provided the reconciled `active_tools` count is `0` (see the `agent-events`
spec's `RUN_FINISHED` transition).

#### Scenario: Stop hook fires inside tmux

- **WHEN** Claude fires the `Stop` hook, the pane's `status` is `working` with
  `active_tools` `0`, and `$TMUX_PANE` is set
- **THEN** the agent's state file is updated to `"status": "done"`

#### Scenario: Stop hook clears a stale blocked status

- **WHEN** Claude fires the `Stop` hook, the pane's `status` is `blocked` (e.g.
  a permission-prompt block that was never explicitly cleared) with
  `active_tools` `0`, and `$TMUX_PANE` is set
- **THEN** the agent's state file is updated to `"status": "done"`

#### Scenario: Stop hook does not force done while a background task is still running

- **WHEN** Claude fires the `Stop` hook with a non-empty `background_tasks`
  array (so `--active-tools` reconciles the pane's `active_tools` to a value
  greater than `0`) and `$TMUX_PANE` is set
- **THEN** the agent's state file's `status` is left unchanged (not forced to
  `done`)

### Requirement: StopFailure hook calls agentctl agent event RUN_FINISHED

The Claude `StopFailure` hook SHALL invoke a command that reads the hook's JSON
payload from stdin, extracts `session_id`, and invokes
`agentctl agent event RUN_FINISHED --id <session_id> --harness claude` (async)
so that API errors (rate-limit, server overload, authentication failure, etc.)
do not leave status permanently stuck at "working". This is authoritative
regardless of the pane's current status, gated on the reconciled `active_tools`
count being `0`, identically to the `Stop` hook.

#### Scenario: StopFailure fires inside tmux

- **WHEN** Claude's turn ends due to an API error, firing the `StopFailure`
  hook, the pane's `active_tools` is `0`, and `$TMUX_PANE` is set
- **THEN** the agent's state file is updated to `"status": "done"`

#### Scenario: StopFailure clears a stale blocked status

- **WHEN** Claude's turn ends due to an API error while the pane's `status` is
  `blocked`, `active_tools` is `0`, and `$TMUX_PANE` is set
- **THEN** the agent's state file is updated to `"status": "done"`

### Requirement: SessionEnd hook calls agentctl agent clear

The Claude `SessionEnd` hook SHALL invoke `agentctl agent clear` so the state
file (and its `events.jsonl`) is removed when the session exits normally.
`agentctl agent clear` exits 0 silently when `$TMUX_PANE` is not set or when no
state file exists.

#### Scenario: SessionEnd fires inside tmux

- **WHEN** Claude fires the `SessionEnd` hook and `$TMUX_PANE` is set
- **THEN** the state file at
  `$XDG_DATA_HOME/agentctl/agents/$TMUX_PANE/state.json` and its `events.jsonl`
  sibling are deleted

#### Scenario: SessionEnd is a no-op when no state file exists

- **WHEN** Claude fires the `SessionEnd` hook but no state file exists for
  `$TMUX_PANE`
- **THEN** `agentctl agent clear` exits 0 without error

### Requirement: agentctl agent event and agentctl agent clear are no-ops outside tmux

Every `agentctl agent event <NAME>` subcommand and `agentctl agent clear` SHALL
exit with code 0 and write nothing when `$TMUX_PANE` is not set in the
environment and `--pane` is not supplied.

#### Scenario: Hook fires outside tmux

- **WHEN** `agentctl agent event <NAME>` or `agentctl agent clear` is invoked
  with `$TMUX_PANE` unset and no `--pane` flag
- **THEN** the command exits 0 and makes no changes to the filesystem

### Requirement: agentctl install claude registers handlers via a Claude Code plugin

The `agentctl install claude` subcommand SHALL install agentctl's eleven hook
entries as a Claude Code plugin under `~/.claude/skills/agentctl/`, consisting
of a plugin manifest at `~/.claude/skills/agentctl/.claude-plugin/plugin.json`
and a hooks configuration at `~/.claude/skills/agentctl/hooks/hooks.json`. The
command SHALL:

1. Create `~/.claude/skills/` if it does not already exist.
2. Delete `~/.claude/skills/agentctl/` and its full contents if it already
   exists (`fs::remove_dir_all` or equivalent) — no diffing or in-place patching
   of an existing plugin installation.
3. Write `.claude-plugin/plugin.json` and `hooks/hooks.json` fresh, containing
   the same eleven hook entries the prior `~/.claude/settings.json`-based
   implementation registered.

`hooks/hooks.json` SHALL use the plugin hooks wrapper format
(`{ "hooks": { "<Event>": [ { "matcher"?: ..., "hooks": [ { "type": "command", "command": ..., "async"?: true } ] } ] } }`)
and SHALL register the same event/matcher/command/async combinations as before:
`SessionStart` and `SessionEnd` without `async` (so they complete before Claude
proceeds); `UserPromptSubmit`, both `PreToolUse` entries, both `PostToolUse`
entries, both `PostToolUseFailure` entries, `Stop`, and `StopFailure` with
`async: true`. The eleven entries are: `SessionStart`, `UserPromptSubmit`,
`PreToolUse` (unmatched and matched), `PostToolUse` (unmatched and matched),
`PostToolUseFailure` (unmatched and matched), `Stop`, `StopFailure`, and
`SessionEnd`. There is no `Notification` registration.

This command SHALL NOT read from, write to, or otherwise inspect
`~/.claude/settings.json`. It SHALL NOT attempt to detect, migrate, or clean up
any agentctl hook entries a previous `agentctl` version may have registered in
`~/.claude/settings.json`.

#### Scenario: First-time install with no prior plugin present

- **WHEN** the user runs `agentctl install claude` and neither
  `~/.claude/skills/` nor `~/.claude/skills/agentctl/` exists
- **THEN** `~/.claude/skills/` is created, `~/.claude/skills/agentctl/` is
  created containing `.claude-plugin/plugin.json` and `hooks/hooks.json`, and
  `hooks/hooks.json` registers all eleven agentctl hook entries, including both
  `PostToolUseFailure` entries and no `Notification` entry

#### Scenario: Re-install replaces the existing plugin directory

- **WHEN** the user runs `agentctl install claude` and
  `~/.claude/skills/agentctl/` already exists (from a prior install, with any
  contents, including files not written by `agentctl`)
- **THEN** the existing `~/.claude/skills/agentctl/` directory and all its
  contents are deleted, and a fresh `~/.claude/skills/agentctl/` is written
  containing exactly `.claude-plugin/plugin.json` and `hooks/hooks.json` with
  the current eleven hook entries

#### Scenario: Install does not touch settings.json

- **WHEN** the user runs `agentctl install claude` and `~/.claude/settings.json`
  exists with unrelated user configuration or with agentctl hook entries from a
  previous `agentctl` version
- **THEN** `~/.claude/settings.json` is not read or modified in any way

#### Scenario: Install creates the skills directory when absent

- **WHEN** the user runs `agentctl install claude` and `~/.claude/skills/` does
  not exist
- **THEN** `~/.claude/skills/` is created before the `agentctl` plugin directory
  is written

### Requirement: agentctl uninstall claude removes the agentctl plugin directory

The `agentctl uninstall claude` subcommand SHALL remove the
`~/.claude/skills/agentctl/` directory and all its contents if it exists. It
SHALL NOT read from, write to, or otherwise inspect `~/.claude/settings.json`,
and SHALL NOT attempt to remove any agentctl hook entries a previous `agentctl`
version may have registered there. The command SHALL be idempotent: running it
again after the directory has been removed (or when it never existed) SHALL make
no further filesystem changes and SHALL exit 0.

#### Scenario: Uninstall removes an existing plugin directory

- **WHEN** the user runs `agentctl uninstall claude` and
  `~/.claude/skills/agentctl/` exists
- **THEN** `~/.claude/skills/agentctl/` and all its contents are deleted, and
  the command exits 0

#### Scenario: Uninstall is a no-op when the plugin is not installed

- **WHEN** the user runs `agentctl uninstall claude` and
  `~/.claude/skills/agentctl/` does not exist (whether or not
  `~/.claude/skills/` itself exists)
- **THEN** no filesystem changes occur and the command exits 0

#### Scenario: Uninstall does not touch settings.json

- **WHEN** the user runs `agentctl uninstall claude` and
  `~/.claude/settings.json` contains agentctl hook entries from a previous
  `agentctl` version
- **THEN** `~/.claude/settings.json` is not read or modified in any way

#### Scenario: Uninstall leaves the skills directory itself in place

- **WHEN** the user runs `agentctl uninstall claude` and `~/.claude/skills/`
  contains other, non-agentctl entries alongside `agentctl/`
- **THEN** only `~/.claude/skills/agentctl/` is removed; `~/.claude/skills/` and
  its other contents are left untouched
