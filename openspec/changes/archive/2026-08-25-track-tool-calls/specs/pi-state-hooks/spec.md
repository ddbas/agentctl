## MODIFIED Requirements

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

## ADDED Requirements

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
