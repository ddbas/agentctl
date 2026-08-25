# pi-state-hooks

## Purpose

Specifies how radr integrates with the pi coding agent harness via a TypeScript
extension (`rd install pi`) that tracks agent state in real time by subscribing
to pi lifecycle events and calling `rd agent event` / `rd agent clear`.

---

## Requirements

### Requirement: session_start event calls rd agent event SESSION_STARTED

The pi extension SHALL invoke
`rd agent event SESSION_STARTED --id <sessionId> --harness pi` when the
`session_start` event fires, so that the agent's state file is written to
`$XDG_DATA_HOME/radr/agents/$TMUX_PANE/state.json` with `"status": "idle"`.
`rd agent event` exits 0 silently when `$TMUX_PANE` is not set.

#### Scenario: session_start fires inside tmux

- **WHEN** pi fires `session_start` (reason: `startup`, `new`, `resume`, or
  `fork`) and `$TMUX_PANE` is set
- **THEN** the state file at `$XDG_DATA_HOME/radr/agents/$TMUX_PANE/state.json`
  is written with `"status": "idle"` and the current `updated_at` timestamp

#### Scenario: session_start overwrites state from a prior session in the same pane

- **WHEN** a state file already exists for `$TMUX_PANE` (left by a previous
  session) and `session_start` fires again in that pane
- **THEN** the existing state file is overwritten with `"status": "idle"`

### Requirement: before_agent_start event calls rd agent event RUN_STARTED

The pi extension SHALL, when the `before_agent_start` event fires, generate a
unique run ID (e.g. via `crypto.randomUUID()`), retain it in memory for the
duration of the run, and invoke
`rd agent event RUN_STARTED --id <runId> --harness pi`, reflecting that the
agent has started processing the user's prompt.

#### Scenario: before_agent_start fires inside tmux

- **WHEN** pi fires `before_agent_start` after the user submits a prompt and
  `$TMUX_PANE` is set
- **THEN** a run ID is generated and
  `rd agent event RUN_STARTED --id <runId> --harness pi` is invoked, updating
  the agent's state file to `"status": "working"`

### Requirement: agent_settled event calls rd agent event RUN_FINISHED

The pi extension SHALL invoke
`rd agent event RUN_FINISHED --id <runId> --harness pi` when the `agent_settled`
event fires, reusing the run ID generated at the most recent
`before_agent_start`. `agent_settled` fires only when pi will not continue
automatically (no pending retries, compaction, or follow-up messages), making it
the correct signal for the `done` status.

#### Scenario: agent_settled fires inside tmux after a normal turn

- **WHEN** pi fires `agent_settled` after completing a prompt and `$TMUX_PANE`
  is set
- **THEN** `rd agent event RUN_FINISHED --id <runId> --harness pi` is invoked
  with the run ID from the corresponding `before_agent_start`, updating the
  agent's state file to `"status": "done"`

#### Scenario: agent_settled fires after auto-compaction and retry

- **WHEN** pi auto-compacts and retries a turn, then fires `agent_settled` once
  the final run settles, and `$TMUX_PANE` is set
- **THEN** the agent's state file is updated to `"status": "done"` (and NOT
  before the final settlement, i.e. intermediate `agent_end` events do not
  trigger `RUN_FINISHED`)

### Requirement: session_shutdown event calls rd agent clear

The pi extension SHALL invoke `rd agent clear` when the `session_shutdown` event
fires, so the state file is removed when the session exits or is replaced.

#### Scenario: session_shutdown fires on quit inside tmux

- **WHEN** pi fires `session_shutdown` with reason `quit` and `$TMUX_PANE` is
  set
- **THEN** the state file at `$XDG_DATA_HOME/radr/agents/$TMUX_PANE/state.json`
  is deleted

#### Scenario: session_shutdown fires on session switch

- **WHEN** pi fires `session_shutdown` with reason `new`, `resume`, or `fork`
  and `$TMUX_PANE` is set
- **THEN** the state file is deleted before the new session's `session_start`
  event sets the state back to `idle`

#### Scenario: session_shutdown is a no-op when no state file exists

- **WHEN** pi fires `session_shutdown` but no state file exists for `$TMUX_PANE`
- **THEN** `rd agent clear` exits 0 without error

### Requirement: rd agent event and rd agent clear are no-ops outside tmux

Every `rd agent event <NAME>` subcommand and `rd agent clear` SHALL exit with
code 0 and write nothing when `$TMUX_PANE` is not set and `--pane` is not
supplied.

#### Scenario: Extension fires outside tmux

- **WHEN** any pi event fires and the extension calls `rd agent event <NAME>` or
  `rd agent clear` with `$TMUX_PANE` unset and no `--pane` flag
- **THEN** the command exits 0 and makes no changes to the filesystem

### Requirement: rd install pi generates the pi extension file

The `rd install pi` subcommand SHALL write the radr TypeScript extension to
`~/.pi/agent/extensions/radr.ts`. The file SHALL subscribe to `session_start`,
`before_agent_start`, `agent_settled`, `session_shutdown`, `tool_call`, and
`tool_result` events and call the appropriate `rd agent event` /
`rd agent clear` commands. The command SHALL be idempotent: re-running when the
file content is already identical SHALL make no change and SHALL report the file
as already up to date.

#### Scenario: First-time install creates the extension file

- **WHEN** the user runs `rd install pi` and `~/.pi/agent/extensions/radr.ts`
  does not exist
- **THEN** the file is created with handlers for all six events and the command
  prints a summary prefixed with `+`

#### Scenario: Idempotent re-install reports no change

- **WHEN** the user runs `rd install pi` a second time and the file content is
  already identical to what would be generated
- **THEN** no write is performed and the command prints the file as already up
  to date (prefixed with `=`)

#### Scenario: Install overwrites a stale extension file

- **WHEN** `~/.pi/agent/extensions/radr.ts` exists but its content differs from
  the current generated content (e.g. from an older version of `rd`, including
  one that still generates `rd set-state`/`rd clear-state`,
  `rd state set`/`rd state clear`, `rd agent status`/`rd agent init`/
  `rd agent clear` calls, or one that predates the `tool_call`/`tool_result`
  handlers)
- **THEN** the file is overwritten with the new content (using `rd agent event`
  / `rd agent clear`) and the command prints the file as updated (prefixed with
  `~`)

#### Scenario: Install creates the extensions directory if absent

- **WHEN** `~/.pi/agent/extensions/` does not exist
- **THEN** `rd install pi` creates the directory before writing `radr.ts`, and
  the command exits 0

### Requirement: tool_call event calls rd agent event TOOL_CALL_START

The pi extension SHALL invoke
`rd agent event TOOL_CALL_START --id <toolCallId> --harness pi` when pi's
`tool_call` event fires, using `event.toolCallId` as `<toolCallId>`, so that the
pane's `active_tools` count in `state.json` is incremented.

#### Scenario: tool_call fires inside tmux

- **WHEN** pi fires `tool_call` for a tool invocation and `$TMUX_PANE` is set
- **THEN** `rd agent event TOOL_CALL_START --id <toolCallId> --harness pi` is
  invoked with the tool call's `toolCallId`, incrementing `active_tools` in the
  pane's state file

### Requirement: tool_result event calls rd agent event TOOL_CALL_RESULT

The pi extension SHALL invoke
`rd agent event TOOL_CALL_RESULT --id <toolCallId> --harness pi` when pi's
`tool_result` event fires, using the same `event.toolCallId` value emitted on
the corresponding `tool_call`, so that the pane's `active_tools` count is
decremented.

#### Scenario: tool_result fires inside tmux

- **WHEN** pi fires `tool_result` for a completed tool invocation and
  `$TMUX_PANE` is set
- **THEN** `rd agent event TOOL_CALL_RESULT --id <toolCallId> --harness pi` is
  invoked with the matching `toolCallId`, decrementing `active_tools` in the
  pane's state file

#### Scenario: Parallel tool calls each pair their own start and result

- **WHEN** pi runs several tool calls concurrently and fires `tool_call` for
  each followed later by `tool_result` for each, in any interleaving order
- **THEN** each `tool_call` increments and each corresponding `tool_result`
  decrements `active_tools` by one, using each call's own `toolCallId`

### Requirement: rd uninstall pi removes the pi extension file

The `rd uninstall pi` subcommand SHALL delete `~/.pi/agent/extensions/radr.ts`
if it exists. If the file does not exist, the command SHALL exit 0 without error
and report that nothing needed removing. The command SHALL NOT delete the
`~/.pi/agent/extensions/` directory itself, even if it becomes empty.

#### Scenario: Uninstall removes an existing extension file

- **WHEN** the user runs `rd uninstall pi` and `~/.pi/agent/extensions/radr.ts`
  exists
- **THEN** the file is deleted and the command prints a summary indicating it
  was removed (prefixed with `-`), and exits 0

#### Scenario: Uninstall is a no-op when the extension file is absent

- **WHEN** the user runs `rd uninstall pi` and `~/.pi/agent/extensions/radr.ts`
  does not exist
- **THEN** no filesystem changes are made, the command prints a summary
  indicating there was nothing to remove (prefixed with `=`), and exits 0

#### Scenario: Uninstall removes the file even if its content was hand-modified

- **WHEN** `~/.pi/agent/extensions/radr.ts` exists but its content no longer
  matches what `rd install pi` currently generates
- **THEN** `rd uninstall pi` still deletes the file, since the file is
  documented as radr-managed regardless of content drift

#### Scenario: Uninstall leaves the extensions directory in place

- **WHEN** `rd uninstall pi` deletes `radr.ts` and no other files remain in
  `~/.pi/agent/extensions/`
- **THEN** the `~/.pi/agent/extensions/` directory itself is left in place (not
  removed)
