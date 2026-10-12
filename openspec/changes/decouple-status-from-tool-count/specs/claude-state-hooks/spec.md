## MODIFIED Requirements

### Requirement: Stop hook calls agentctl agent event RUN_FINISHED

The Claude `Stop` hook SHALL invoke a command that reads the hook's JSON payload
from stdin, extracts `session_id`, and invokes
`agentctl agent event RUN_FINISHED --id <session_id> --harness claude` (async).
This is authoritative regardless of the pane's current status: it drives the
pane to `done` from any status (`idle`, `working`, `blocked`, or `done` itself),
independent of the pane's `active_tools` count (see the `agent-events` spec's
`RUN_FINISHED` transition). The hook's `--active-tools` value only reconciles
the displayed `active_tools` count; it does not affect `status`.

#### Scenario: Stop hook fires inside tmux

- **WHEN** Claude fires the `Stop` hook, the pane's `status` is `working` with
  `active_tools` `0`, and `$TMUX_PANE` is set
- **THEN** the agent's state file is updated to `"status": "done"`

#### Scenario: Stop hook clears a stale blocked status

- **WHEN** Claude fires the `Stop` hook, the pane's `status` is `blocked` (e.g.
  an elicitation block that was never explicitly cleared), and `$TMUX_PANE` is
  set
- **THEN** the agent's state file is updated to `"status": "done"`

#### Scenario: Stop hook forces done while a background task is still running

- **WHEN** Claude fires the `Stop` hook with a non-empty `background_tasks`
  array (so `--active-tools` reconciles the pane's `active_tools` to a value
  greater than `0`) and `$TMUX_PANE` is set
- **THEN** the agent's state file's `status` becomes `done` and `active_tools`
  equals the number of background tasks

### Requirement: StopFailure hook calls agentctl agent event RUN_FINISHED

The Claude `StopFailure` hook SHALL invoke a command that reads the hook's JSON
payload from stdin, extracts `session_id`, and invokes
`agentctl agent event RUN_FINISHED --id <session_id> --harness claude` (async)
so that API errors (rate-limit, server overload, authentication failure, etc.)
do not leave status permanently stuck at "working". This is authoritative
regardless of the pane's current status and of its `active_tools` count,
identically to the `Stop` hook.

#### Scenario: StopFailure fires inside tmux

- **WHEN** Claude's turn ends due to an API error, firing the `StopFailure`
  hook, and `$TMUX_PANE` is set
- **THEN** the agent's state file is updated to `"status": "done"`

#### Scenario: StopFailure clears a stale blocked status

- **WHEN** Claude's turn ends due to an API error while the pane's `status` is
  `blocked`, and `$TMUX_PANE` is set
- **THEN** the agent's state file is updated to `"status": "done"`

#### Scenario: StopFailure forces done while a background task is still running

- **WHEN** Claude's turn ends due to an API error with a non-empty
  `background_tasks` array and `$TMUX_PANE` is set
- **THEN** the agent's state file's `status` becomes `done` and `active_tools`
  equals the number of background tasks
