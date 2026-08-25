## ADDED Requirements

### Requirement: rd agent event dispatches typed events

The system SHALL provide
`rd agent event <EVENT_NAME> [--id <ID>] [--type <TYPE>] [--harness <HARNESS>] [--pane <PANE_ID>]`
as nested clap subcommands, one per supported event name. Supported event names
are `SESSION_STARTED`, `RUN_STARTED`, `RUN_FINISHED`, `TOOL_CALL_START`, and
`TOOL_CALL_RESULT`.

#### Scenario: Unknown event name is rejected

- **WHEN** `rd agent event` is invoked with a name that is not one of
  `SESSION_STARTED`, `RUN_STARTED`, `RUN_FINISHED`, `TOOL_CALL_START`,
  `TOOL_CALL_RESULT`
- **THEN** the command exits non-zero with an error identifying the accepted
  event names

### Requirement: --harness and --pane are global args shared by every event subcommand

`--harness` and `--pane` SHALL be declared once, as global arguments on the
`rd agent event` subcommand, rather than being redefined separately on each of
the five event subcommands. Every current and future event subcommand SHALL
accept `--harness` and `--pane` with identical behavior as a result, without
needing to redeclare them.

#### Scenario: --harness is accepted by every event subcommand

- **WHEN** `--harness <H>` is passed to any of `rd agent event SESSION_STARTED`,
  `RUN_STARTED`, `RUN_FINISHED`, `TOOL_CALL_START`, or `TOOL_CALL_RESULT`
- **THEN** the command accepts the flag and records `<H>` as the event's
  `harness`

#### Scenario: --pane is accepted by every event subcommand

- **WHEN** `--pane <ID>` is passed to any of `rd agent event SESSION_STARTED`,
  `RUN_STARTED`, `RUN_FINISHED`, `TOOL_CALL_START`, or `TOOL_CALL_RESULT`
- **THEN** the command resolves the pane using `<ID>` instead of `$TMUX_PANE`,
  exactly as `--pane` already behaves for `rd agent clear`

#### Scenario: --harness and --pane may be placed before or after the event name

- **WHEN** `rd agent event --harness claude --pane %12 RUN_STARTED --id r1` or
  `rd agent event RUN_STARTED --id r1 --harness claude --pane %12` is invoked
- **THEN** both invocations are accepted and behave identically

### Requirement: id is required on all five events

Every event dispatched via `rd agent event` SHALL require a non-empty `--id`
value. `id` uniquely identifies the event's subject (a session ID for
`SESSION_STARTED`, a run ID for `RUN_*` events, a tool-call ID for `TOOL_CALL_*`
events) so that a terminating event can later be reconciled with its
corresponding starting event where one exists.

#### Scenario: Missing id is rejected

- **WHEN** any of the five `rd agent event` subcommands is invoked without
  `--id`
- **THEN** the command exits non-zero with a usage error, and no state or log
  changes are made

### Requirement: type is optional and only accepted on TOOL*CALL*\* events

`TOOL_CALL_START` and `TOOL_CALL_RESULT` SHALL accept an optional `--type` flag
with accepted values `custom` and `elicitation`. `SESSION_STARTED`,
`RUN_STARTED`, and `RUN_FINISHED` SHALL NOT accept a `--type` flag.

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

#### Scenario: type on SESSION_STARTED is rejected

- **WHEN** `rd agent event SESSION_STARTED --id <id> --type custom` is invoked
- **THEN** the command exits non-zero with a usage error, since
  `SESSION_STARTED` does not accept `--type`

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

- Any status on `SESSION_STARTED` → `Idle`
- `Idle` on `RUN_STARTED` → `Working`
- `Done` on `RUN_STARTED` → `Working`
- `Working` on `RUN_FINISHED` → `Done`
- `Working` on `TOOL_CALL_START` with `type: elicitation` → `Blocked`
- `Blocked` on `TOOL_CALL_RESULT` with `type: elicitation` → `Working`

An event that does not match a defined transition for the pane's current status
SHALL NOT change `status`, but SHALL still be appended to `events.jsonl`.

#### Scenario: SESSION_STARTED resets an idle pane to idle

- **WHEN** a pane's `status` is `idle` and `SESSION_STARTED --id s1` is
  dispatched
- **THEN** the pane's `status` remains `idle`

#### Scenario: SESSION_STARTED resets a working pane to idle

- **WHEN** a pane's `status` is `working` and `SESSION_STARTED --id s2` is
  dispatched
- **THEN** the pane's `status` becomes `idle`

#### Scenario: SESSION_STARTED resets a blocked pane to idle

- **WHEN** a pane's `status` is `blocked` and `SESSION_STARTED --id s3` is
  dispatched
- **THEN** the pane's `status` becomes `idle`

#### Scenario: SESSION_STARTED resets a done pane to idle

- **WHEN** a pane's `status` is `done` and `SESSION_STARTED --id s4` is
  dispatched
- **THEN** the pane's `status` becomes `idle`

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
  `events.jsonl`

### Requirement: Every dispatched event is appended to events.jsonl

The system SHALL append every event dispatched via `rd agent event`, including
`SESSION_STARTED`, as one JSON object per line to
`$XDG_DATA_HOME/radr/agents/<pane-id>/events.jsonl`, creating the file and its
parent directory if absent. Each logged line SHALL include at least the event
name, `id`, `harness`, `pane_id`, and a timestamp; `TOOL_CALL_*` events SHALL
include `type` when supplied.

#### Scenario: First event creates the log file

- **WHEN** `rd agent event SESSION_STARTED --id s1 --harness claude --pane %12`
  is dispatched and no `events.jsonl` exists yet for `%12`
- **THEN** `$XDG_DATA_HOME/radr/agents/%12/events.jsonl` is created containing
  exactly one line describing the `SESSION_STARTED` event

#### Scenario: Subsequent events are appended, not overwritten

- **WHEN** a second event is dispatched for a pane that already has an
  `events.jsonl` with prior lines
- **THEN** the new event is appended as an additional line and all prior lines
  remain unchanged

#### Scenario: Events that do not change status are still logged

- **WHEN** an event is dispatched that does not match any FSM transition for the
  pane's current status
- **THEN** the event is still appended to `events.jsonl`

### Requirement: rd agent status is removed

The system SHALL NOT provide an `rd agent status` subcommand, and SHALL NOT
provide a separate non-event `rd agent init` command; session bootstrap to
`idle` SHALL be performed exclusively via `rd agent event SESSION_STARTED`.

#### Scenario: rd agent status no longer exists

- **WHEN** `rd agent status idle` (or any other value) is invoked
- **THEN** the command exits non-zero as an unrecognized subcommand

#### Scenario: rd agent init no longer exists

- **WHEN** `rd agent init` is invoked
- **THEN** the command exits non-zero as an unrecognized subcommand
