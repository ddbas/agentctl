# pi-state-hooks

## Purpose

Specifies how agentctl integrates with the pi coding agent harness via a
TypeScript extension (`agentctl hooks install pi`) that tracks agent state in
real time by subscribing to pi lifecycle events and calling `agentctl set-state`
/ `agentctl clear-state`.

---

## Requirements

### Requirement: session_start event calls agentctl set-state idle

The pi extension SHALL invoke `agentctl set-state idle --harness pi` when the
`session_start` event fires, so that the agent's state file is written to
`$XDG_DATA_HOME/agentctl/agents/$TMUX_PANE/state.json` with `"state": "idle"`.
`agentctl set-state` exits 0 silently when `$TMUX_PANE` is not set.

#### Scenario: session_start fires inside tmux

- **WHEN** pi fires `session_start` (reason: `startup`, `new`, `resume`, or
  `fork`) and `$TMUX_PANE` is set
- **THEN** the state file at
  `$XDG_DATA_HOME/agentctl/agents/$TMUX_PANE/state.json` is written with
  `"state": "idle"` and the current `updated_at` timestamp

#### Scenario: session_start overwrites state from a prior session in the same pane

- **WHEN** a state file already exists for `$TMUX_PANE` (left by a previous
  session) and `session_start` fires again in that pane
- **THEN** the existing state file is overwritten with `"state": "idle"`

---

### Requirement: before_agent_start event calls agentctl set-state working

The pi extension SHALL invoke `agentctl set-state working --harness pi` when the
`before_agent_start` event fires, reflecting that the agent has started
processing the user's prompt.

#### Scenario: before_agent_start fires inside tmux

- **WHEN** pi fires `before_agent_start` after the user submits a prompt and
  `$TMUX_PANE` is set
- **THEN** the agent's state file is updated to `"state": "working"`

---

### Requirement: agent_settled event calls agentctl set-state done

The pi extension SHALL invoke `agentctl set-state done --harness pi` when the
`agent_settled` event fires. `agent_settled` fires only when pi will not
continue automatically (no pending retries, compaction, or follow-up messages),
making it the correct signal for the `done` state.

#### Scenario: agent_settled fires inside tmux after a normal turn

- **WHEN** pi fires `agent_settled` after completing a prompt and `$TMUX_PANE`
  is set
- **THEN** the agent's state file is updated to `"state": "done"`

#### Scenario: agent_settled fires after auto-compaction and retry

- **WHEN** pi auto-compacts and retries a turn, then fires `agent_settled` once
  the final run settles, and `$TMUX_PANE` is set
- **THEN** the agent's state file is updated to `"state": "done"` (and NOT
  before the final settlement, i.e. intermediate `agent_end` events do not set
  `done`)

---

### Requirement: session_shutdown event calls agentctl clear-state

The pi extension SHALL invoke `agentctl clear-state` when the `session_shutdown`
event fires, so the state file is removed when the session exits or is replaced.

#### Scenario: session_shutdown fires on quit inside tmux

- **WHEN** pi fires `session_shutdown` with reason `quit` and `$TMUX_PANE` is
  set
- **THEN** the state file at
  `$XDG_DATA_HOME/agentctl/agents/$TMUX_PANE/state.json` is deleted

#### Scenario: session_shutdown fires on session switch

- **WHEN** pi fires `session_shutdown` with reason `new`, `resume`, or `fork`
  and `$TMUX_PANE` is set
- **THEN** the state file is deleted before the new session's `session_start`
  event sets the state back to `idle`

#### Scenario: session_shutdown is a no-op when no state file exists

- **WHEN** pi fires `session_shutdown` but no state file exists for `$TMUX_PANE`
- **THEN** `agentctl clear-state` exits 0 without error

---

### Requirement: agentctl set-state and agentctl clear-state are no-ops outside tmux

Both `agentctl set-state <state>` and `agentctl clear-state` SHALL exit with
code 0 and write nothing when `$TMUX_PANE` is not set and `--pane` is not
supplied.

#### Scenario: Extension fires outside tmux

- **WHEN** any pi event fires and the extension calls `agentctl set-state` or
  `agentctl clear-state` with `$TMUX_PANE` unset and no `--pane` flag
- **THEN** the command exits 0 and makes no changes to the filesystem

---

### Requirement: agentctl hooks install pi generates the pi extension file

The `agentctl hooks install pi` subcommand SHALL write the agentctl TypeScript
extension to `~/.pi/agent/extensions/agentctl.ts`. The file SHALL subscribe to
`session_start`, `before_agent_start`, `agent_settled`, and `session_shutdown`
events and call the appropriate `agentctl` commands. The command SHALL be
idempotent: re-running when the file content is already identical SHALL make no
change and SHALL report the file as already up to date.

#### Scenario: First-time install creates the extension file

- **WHEN** the user runs `agentctl hooks install pi` and
  `~/.pi/agent/extensions/agentctl.ts` does not exist
- **THEN** the file is created with handlers for all four events and the command
  prints a summary prefixed with `+`

#### Scenario: Idempotent re-install reports no change

- **WHEN** the user runs `agentctl hooks install pi` a second time and the file
  content is already identical to what would be generated
- **THEN** no write is performed and the command prints the file as already up
  to date (prefixed with `=`)

#### Scenario: Install overwrites a stale extension file

- **WHEN** `~/.pi/agent/extensions/agentctl.ts` exists but its content differs
  from the current generated content (e.g. from an older version of `agentctl`)
- **THEN** the file is overwritten with the new content and the command prints
  the file as updated (prefixed with `~`)

#### Scenario: Install creates the extensions directory if absent

- **WHEN** `~/.pi/agent/extensions/` does not exist
- **THEN** `agentctl hooks install pi` creates the directory before writing
  `agentctl.ts`, and the command exits 0
