# Claude State Hooks

## Purpose

Specification for radr's Claude harness integration: the eleven Claude lifecycle
hook entries (`SessionStart`, `UserPromptSubmit`, `PreToolUse` (unmatched and
matched), `PostToolUse` (unmatched untyped, matched, and unmatched "perm-" id),
`Stop`, `StopFailure`, `Notification`, `SessionEnd`) that keep the agent state
file in sync with Claude's current activity, and the `rd install claude` command
that registers them.

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

### Requirement: PreToolUse hook calls rd agent event TOOL_CALL_START for every tool call

The Claude `PreToolUse` hook SHALL additionally be registered without a tool
matcher (firing for every tool) to invoke a command that reads the hook's JSON
payload from stdin, extracts `tool_use_id`, and invokes
`rd agent event TOOL_CALL_START --id <tool_use_id> --harness claude` (async, no
`--type`), incrementing the pane's `active_tools` count for every tool call,
mirroring the pi extension's `tool_call` handler.

#### Scenario: Any tool call increments active_tools

- **WHEN** Claude fires the unmatched `PreToolUse` hook for any tool call and
  `$TMUX_PANE` is set
- **THEN** the state file's `active_tools` count is incremented by one

#### Scenario: Concurrent tool calls each get a distinct id

- **WHEN** Claude fires the unmatched `PreToolUse` hook for two tool calls
  running concurrently
- **THEN** each invocation uses its own `tool_use_id`, so both increments are
  recorded independently

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

### Requirement: PostToolUse hook calls rd agent event TOOL_CALL_RESULT for every tool call

The Claude `PostToolUse` hook SHALL additionally be registered without a tool
matcher (firing for every tool) to invoke a command that reads the hook's JSON
payload from stdin, extracts `tool_use_id`, and invokes
`rd agent event TOOL_CALL_RESULT --id <tool_use_id> --harness claude` (async, no
`--type`), decrementing the pane's `active_tools` count for every completed tool
call, mirroring the pi extension's `tool_result` handler. This entry is
independent of the unmatched `PostToolUse` →
`TOOL_CALL_RESULT --id "perm-<session_id>-<nonce>" --type permission-prompt`
entry, which continues to serve only permission-prompt block closure and is
unaffected by this requirement.

#### Scenario: Any tool call completion decrements active_tools

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
`rd agent event TOOL_CALL_RESULT --id "perm-<session_id>-<nonce>" --type permission-prompt --harness claude`
(async), where `<nonce>` is a per-invocation unique value (e.g. a random number)
distinguishing this invocation's `events.jsonl` entry from other
permission-prompt events in the same session. This closes out a `blocked` status
opened by the `Notification` hook's `permission_prompt` registration,
decrementing the pane's `blocked_prompts` counter by one; `blocked` is only
cleared once `blocked_prompts` returns to `0`, so a still-pending, concurrent
permission prompt opened separately is not prematurely cleared. Unlike the
previous `--type elicitation` form, this entry does NOT mutate the pane's
`active_tools` counter (see the `agent-events` spec's `blocked_prompts`
requirement).

#### Scenario: Tool call after a resolved permission prompt clears blocked

- **WHEN** a `permission_prompt` `Notification` set the pane's status to
  `blocked` (with `blocked_prompts` `1`), the user grants permission, the tool
  executes, and Claude fires the unmatched `PostToolUse` hook for that tool call
- **THEN** the agent's state file is updated to `"status": "working"` and
  `blocked_prompts` becomes `0`

#### Scenario: Tool call with no outstanding permission prompt is a no-op for status

- **WHEN** no `permission_prompt` `Notification` is outstanding for the current
  session (`blocked_prompts` is `0`) and Claude fires the unmatched
  `PostToolUse` hook after any tool call
- **THEN** the `TOOL_CALL_RESULT` event is dispatched and logged but the agent's
  status is unchanged (`blocked_prompts` remains `0`, clamped)

#### Scenario: Resolving one of two concurrent permission prompts does not clear blocked

- **WHEN** two `permission_prompt` `Notification`s have each set the pane's
  status to `blocked` (`blocked_prompts` is `2`), and Claude fires the unmatched
  `PostToolUse` hook once (closing one of the two)
- **THEN** the agent's state file's `status` remains `"blocked"` and
  `blocked_prompts` becomes `1`

#### Scenario: Unmatched PostToolUse close does not mutate active_tools

- **WHEN** a pane's `active_tools` is `1` (a real tool call genuinely in flight)
  and Claude fires the unmatched `PostToolUse` permission-close hook
- **THEN** `active_tools` remains `1`

### Requirement: Notification hook calls rd agent event TOOL_CALL_START on permission prompts

The Claude `Notification` hook SHALL invoke a command that reads the hook's JSON
payload from stdin, extracts `session_id`, and invokes
`rd agent event TOOL_CALL_START --id "perm-<session_id>-<nonce>" --type permission-prompt --harness claude`
(async) when registered with `matcher: "permission_prompt"`, where `<nonce>` is
a per-invocation unique value (e.g. a random number) distinguishing this
invocation's `events.jsonl` entry from other permission-prompt events in the
same session. Because Claude's hook system only fires the hook when the matcher
matches, no notification-type branching logic is required inside `rd`. This
increments the pane's `blocked_prompts` counter by one (see the `agent-events`
spec) and does NOT mutate `active_tools`.

#### Scenario: permission_prompt notification fires inside tmux

- **WHEN** Claude fires the `Notification` hook with `notification_type` of
  `permission_prompt` and `$TMUX_PANE` is set
- **THEN** the agent's state file is updated to `"status": "blocked"` and
  `blocked_prompts` is incremented by one

#### Scenario: Non-permission notification is not forwarded

- **WHEN** Claude fires the `Notification` hook with any `notification_type`
  other than `permission_prompt`
- **THEN** the hook is not triggered (filtered by Claude's `matcher`) and no
  status change occurs

#### Scenario: A second concurrent permission_prompt notification opens independently

- **WHEN** a pane's status is already `blocked` with `blocked_prompts` `1`, and
  Claude fires a second `Notification` hook with `notification_type` of
  `permission_prompt` (for a different, concurrent tool awaiting approval)
- **THEN** the agent's state file's `status` remains `"blocked"` and
  `blocked_prompts` becomes `2`

### Requirement: Stop hook calls rd agent event RUN_FINISHED

The Claude `Stop` hook SHALL invoke a command that reads the hook's JSON payload
from stdin, extracts `session_id`, and invokes
`rd agent event RUN_FINISHED --id <session_id> --harness claude` (async). This
is authoritative regardless of the pane's current status: it drives the pane to
`done` from any status (`idle`, `working`, `blocked`, or `done` itself),
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

### Requirement: StopFailure hook calls rd agent event RUN_FINISHED

The Claude `StopFailure` hook SHALL invoke a command that reads the hook's JSON
payload from stdin, extracts `session_id`, and invokes
`rd agent event RUN_FINISHED --id <session_id> --harness claude` (async) so that
API errors (rate-limit, server overload, authentication failure, etc.) do not
leave status permanently stuck at "working". This is authoritative regardless of
the pane's current status, gated on the reconciled `active_tools` count being
`0`, identically to the `Stop` hook.

#### Scenario: StopFailure fires inside tmux

- **WHEN** Claude's turn ends due to an API error, firing the `StopFailure`
  hook, the pane's `active_tools` is `0`, and `$TMUX_PANE` is set
- **THEN** the agent's state file is updated to `"status": "done"`

#### Scenario: StopFailure clears a stale blocked status

- **WHEN** Claude's turn ends due to an API error while the pane's `status` is
  `blocked`, `active_tools` is `0`, and `$TMUX_PANE` is set
- **THEN** the agent's state file is updated to `"status": "done"`

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

### Requirement: rd install claude registers handlers via a Claude Code plugin

The `rd install claude` subcommand SHALL install radr's eleven hook entries as a
Claude Code plugin under `~/.claude/skills/radr/`, consisting of a plugin
manifest at `~/.claude/skills/radr/.claude-plugin/plugin.json` and a hooks
configuration at `~/.claude/skills/radr/hooks/hooks.json`. The command SHALL:

1. Create `~/.claude/skills/` if it does not already exist.
2. Delete `~/.claude/skills/radr/` and its full contents if it already exists
   (`fs::remove_dir_all` or equivalent) — no diffing or in-place patching of an
   existing plugin installation.
3. Write `.claude-plugin/plugin.json` and `hooks/hooks.json` fresh, containing
   the same eleven hook entries the prior `~/.claude/settings.json`-based
   implementation registered.

`hooks/hooks.json` SHALL use the plugin hooks wrapper format
(`{ "hooks": { "<Event>": [ { "matcher"?: ..., "hooks": [ { "type": "command", "command": ..., "async"?: true } ] } ] } }`)
and SHALL register the same event/matcher/command/async combinations as before:
`SessionStart` and `SessionEnd` without `async` (so they complete before Claude
proceeds); `UserPromptSubmit`, both `PreToolUse` entries, all three
`PostToolUse` entries, `Notification`, `Stop`, and `StopFailure` with
`async: true`.

This command SHALL NOT read from, write to, or otherwise inspect
`~/.claude/settings.json`. It SHALL NOT attempt to detect, migrate, or clean up
any radr hook entries a previous `rd` version may have registered in
`~/.claude/settings.json`.

#### Scenario: First-time install with no prior plugin present

- **WHEN** the user runs `rd install claude` and neither `~/.claude/skills/` nor
  `~/.claude/skills/radr/` exists
- **THEN** `~/.claude/skills/` is created, `~/.claude/skills/radr/` is created
  containing `.claude-plugin/plugin.json` and `hooks/hooks.json`, and
  `hooks/hooks.json` registers all eleven radr hook entries

#### Scenario: Re-install replaces the existing plugin directory

- **WHEN** the user runs `rd install claude` and `~/.claude/skills/radr/`
  already exists (from a prior install, with any contents, including files not
  written by `rd`)
- **THEN** the existing `~/.claude/skills/radr/` directory and all its contents
  are deleted, and a fresh `~/.claude/skills/radr/` is written containing
  exactly `.claude-plugin/plugin.json` and `hooks/hooks.json` with the current
  eleven hook entries

#### Scenario: Install does not touch settings.json

- **WHEN** the user runs `rd install claude` and `~/.claude/settings.json`
  exists with unrelated user configuration or with radr hook entries from a
  previous `rd` version
- **THEN** `~/.claude/settings.json` is not read or modified in any way

#### Scenario: Install creates the skills directory when absent

- **WHEN** the user runs `rd install claude` and `~/.claude/skills/` does not
  exist
- **THEN** `~/.claude/skills/` is created before the `radr` plugin directory is
  written

### Requirement: rd uninstall claude removes the radr plugin directory

The `rd uninstall claude` subcommand SHALL remove the `~/.claude/skills/radr/`
directory and all its contents if it exists. It SHALL NOT read from, write to,
or otherwise inspect `~/.claude/settings.json`, and SHALL NOT attempt to remove
any radr hook entries a previous `rd` version may have registered there. The
command SHALL be idempotent: running it again after the directory has been
removed (or when it never existed) SHALL make no further filesystem changes and
SHALL exit 0.

#### Scenario: Uninstall removes an existing plugin directory

- **WHEN** the user runs `rd uninstall claude` and `~/.claude/skills/radr/`
  exists
- **THEN** `~/.claude/skills/radr/` and all its contents are deleted, and the
  command exits 0

#### Scenario: Uninstall is a no-op when the plugin is not installed

- **WHEN** the user runs `rd uninstall claude` and `~/.claude/skills/radr/` does
  not exist (whether or not `~/.claude/skills/` itself exists)
- **THEN** no filesystem changes occur and the command exits 0

#### Scenario: Uninstall does not touch settings.json

- **WHEN** the user runs `rd uninstall claude` and `~/.claude/settings.json`
  contains radr hook entries from a previous `rd` version
- **THEN** `~/.claude/settings.json` is not read or modified in any way

#### Scenario: Uninstall leaves the skills directory itself in place

- **WHEN** the user runs `rd uninstall claude` and `~/.claude/skills/` contains
  other, non-radr entries alongside `radr/`
- **THEN** only `~/.claude/skills/radr/` is removed; `~/.claude/skills/` and its
  other contents are left untouched
