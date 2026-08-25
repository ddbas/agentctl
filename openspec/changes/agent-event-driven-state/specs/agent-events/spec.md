## ADDED Requirements

### Requirement: rd agent event dispatches typed events

The system SHALL provide
`rd agent event <EVENT_NAME> [--id <ID>] [--type <TYPE>] [--harness <HARNESS>] [--pane <PANE_ID>]`
as nested clap subcommands, one per supported event name. Supported event names
are `RUN_STARTED`, `RUN_FINISHED`, `TOOL_CALL_START`, and `TOOL_CALL_RESULT`.

#### Scenario: Unknown event name is rejected

- **WHEN** `rd agent event` is invoked with a name that is not one of
  `RUN_STARTED`, `RUN_FINISHED`, `TOOL_CALL_START`, `TOOL_CALL_RESULT`
- **THEN** the command exits non-zero with an error identifying the accepted
  event names

### Requirement: id is required on all four events

Every event dispatched via `rd agent event` SHALL require a non-empty `--id`
value. `id` uniquely identifies the event's subject (a run ID for `RUN_*`
events, a tool-call ID for `TOOL_CALL_*` events) so that a terminating event can
later be reconciled with its corresponding starting event.

#### Scenario: Missing id is rejected

- **WHEN** any of the four `rd agent event` subcommands is invoked without
  `--id`
- **THEN** the command exits non-zero with a usage error, and no state or log
  changes are made

### Requirement: type is optional and only accepted on TOOL*CALL*\* events

`TOOL_CALL_START` and `TOOL_CALL_RESULT` SHALL accept an optional `--type` flag
with accepted values `custom` and `elicitation`. `RUN_STARTED` and
`RUN_FINISHED` SHALL NOT accept a `--type` flag.

#### Scenario: TOOL_CALL_START accepts type elicitation

- **WHEN** `rd agent event TOOL_CALL_START --id <id> --type elicitation` is
  invoked
- **THEN** the command succeeds and the event is recorded with
  `type: "elicitation"`

#### Scenario: TOOL_CALL_START omits type

- **WHEN** `rd agent event TOOL_CALL_START --id <id>` is invoked without
  `--type`
- **THEN** the command succeeds and the event is recorded with no `type` field

#### Scenario: type on RUN_STARTED is rejected

- **WHEN** `rd agent event RUN_STARTED --id <id> --type custom` is invoked
- **THEN** the command exits non-zero with a usage error, since `RUN_STARTED`
  does not accept `--type`

#### Scenario: Invalid type value is rejected

- **WHEN** `rd agent event TOOL_CALL_START --id <id> --type bogus` is invoked
- **THEN** the command exits non-zero with an error identifying the accepted
  values (`custom`, `elicitation`)

### Requirement: Events derive status via a finite state machine

The system SHALL derive the pane's `status` field from the sequence of
dispatched events using a finite state machine (implemented with the `rust-fsm`
crate) with states `Idle`, `Working`, `Blocked`, `Done`. `Idle` SHALL be the
machine's declared start state and `Done` SHALL be its declared end state. The
machine SHALL apply the following transitions:

- `Idle` on `RUN_STARTED` → `Working`
- `Done` on `RUN_STARTED` → `Working`
- `Working` on `RUN_FINISHED` → `Done`
- `Working` on `TOOL_CALL_START` with `type: elicitation` → `Blocked`
- `Blocked` on `TOOL_CALL_RESULT` with `type: elicitation` → `Working`

An event that does not match a defined transition for the pane's current status
SHALL NOT change `status`, but SHALL still be appended to `events-log.jsonl`.

#### Scenario: RUN_STARTED moves idle pane to working

- **WHEN** a pane's `status` is `idle` and `RUN_STARTED --id r1` is dispatched
- **THEN** the pane's `status` becomes `working`

#### Scenario: RUN_FINISHED moves working pane to done

- **WHEN** a pane's `status` is `working` and `RUN_FINISHED --id r1` is
  dispatched
- **THEN** the pane's `status` becomes `done`

#### Scenario: RUN_STARTED moves done pane back to working

- **WHEN** a pane's `status` is `done` and `RUN_STARTED --id r2` is dispatched
- **THEN** the pane's `status` becomes `working`

#### Scenario: elicitation TOOL_CALL_START moves working pane to blocked

- **WHEN** a pane's `status` is `working` and
  `TOOL_CALL_START --id t1 --type elicitation` is dispatched
- **THEN** the pane's `status` becomes `blocked`

#### Scenario: elicitation TOOL_CALL_RESULT moves blocked pane back to working

- **WHEN** a pane's `status` is `blocked` and
  `TOOL_CALL_RESULT --id t1 --type elicitation` is dispatched
- **THEN** the pane's `status` becomes `working`

#### Scenario: custom-typed tool call events do not change status

- **WHEN** a pane's `status` is `working` and
  `TOOL_CALL_START --id t2 --type custom` (or
  `TOOL_CALL_RESULT --id t2 --type custom`) is dispatched
- **THEN** the pane's `status` remains `working`

#### Scenario: tool call events without a type do not change status

- **WHEN** a pane's `status` is `working` and `TOOL_CALL_START --id t3` (no
  `--type`) is dispatched
- **THEN** the pane's `status` remains `working`

#### Scenario: unmatched event for current status is a no-op for status

- **WHEN** a pane's `status` is `idle` and
  `TOOL_CALL_RESULT --id t4 --type elicitation` is dispatched (no corresponding
  `Blocked` status)
- **THEN** the pane's `status` remains `idle` and the event is still appended to
  `events-log.jsonl`

### Requirement: Every dispatched event is appended to events-log.jsonl

The system SHALL append every event dispatched via `rd agent event` as one JSON
object per line to `$XDG_DATA_HOME/radr/agents/<pane-id>/events-log.jsonl`,
creating the file and its parent directory if absent. Each logged line SHALL
include at least the event name, `id`, `harness`, `pane_id`, and a timestamp;
`TOOL_CALL_*` events SHALL include `type` when supplied.

#### Scenario: First event creates the log file

- **WHEN** `rd agent event RUN_STARTED --id r1 --harness claude --pane %12` is
  dispatched and no `events-log.jsonl` exists yet for `%12`
- **THEN** `$XDG_DATA_HOME/radr/agents/%12/events-log.jsonl` is created
  containing exactly one line describing the `RUN_STARTED` event

#### Scenario: Subsequent events are appended, not overwritten

- **WHEN** a second event is dispatched for a pane that already has an
  `events-log.jsonl` with prior lines
- **THEN** the new event is appended as an additional line and all prior lines
  remain unchanged

#### Scenario: Events that do not change status are still logged

- **WHEN** an event is dispatched that does not match any FSM transition for the
  pane's current status
- **THEN** the event is still appended to `events-log.jsonl`

### Requirement: rd agent init sets status to idle outside the event pipeline

The system SHALL provide
`rd agent init [--harness <HARNESS>] [--pane <PANE_ID>]`, which writes
`status: idle` to the pane's `state.json` directly, independent of the FSM and
without appending to `events-log.jsonl`. Pane ID resolution SHALL follow the
same order as `rd agent clear`: `--pane` flag, then `$TMUX_PANE`, then silent
exit 0 when neither is available.

#### Scenario: rd agent init writes idle status

- **WHEN** `rd agent init --harness pi --pane %12` is invoked
- **THEN** `state.json` for `%12` is written with `"status": "idle"` and the
  current `updated_at` timestamp, and no line is appended to `events-log.jsonl`

#### Scenario: rd agent init overwrites any prior status

- **WHEN** a pane's `state.json` currently has `"status": "blocked"` (e.g. left
  over from a crashed session) and `rd agent init` is invoked for that pane
- **THEN** `state.json` is overwritten with `"status": "idle"`

#### Scenario: rd agent init is a no-op outside tmux

- **WHEN** `rd agent init` is invoked with `$TMUX_PANE` unset and no `--pane`
  flag
- **THEN** the command exits 0 and makes no changes to the filesystem

### Requirement: rd agent status is removed

The system SHALL NOT provide an `rd agent status` subcommand.

#### Scenario: rd agent status no longer exists

- **WHEN** `rd agent status idle` (or any other value) is invoked
- **THEN** the command exits non-zero as an unrecognized subcommand
