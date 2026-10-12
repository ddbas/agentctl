## MODIFIED Requirements

### Requirement: Events derive status via a finite state machine

The system SHALL derive the pane's `status` field from the sequence of
dispatched events using a finite state machine (implemented with the `rust-fsm`
crate) with states `Idle`, `Working`, `Blocked`, `Done`. `Idle` SHALL be the
machine's declared start state and `Done` SHALL be its declared end state. The
machine SHALL apply the following transitions:

- Any status on `SESSION_STARTED` → `Idle`
- `Idle` on `RUN_STARTED` → `Working`
- `Done` on `RUN_STARTED` → `Working`
- Any status on `RUN_FINISHED` → `Done`, unconditionally. The pane's
  `active_tools` count SHALL NOT influence this transition: `RUN_FINISHED`
  drives `status` to `Done` whether `active_tools` is `0` or greater than `0`.
- `Working` on `TOOL_CALL_START` with `type: elicitation` → `Blocked`
- `Done` on `TOOL_CALL_START` (no `type`, or `type: custom`) → `Working`
- `Done` on `TOOL_CALL_START` with `type: elicitation` → `Blocked`
- `Blocked` on `TOOL_CALL_RESULT` with `type: elicitation` → `Working`

The status machine SHALL NOT read `active_tools` for any transition. An event
that does not match a defined transition for the pane's current status SHALL NOT
change `status`, but SHALL still be appended to `events.jsonl`. There is no
`permission-prompt` type or transition.

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

- **WHEN** a pane's `status` is `working`, `active_tools` is `0`, and
  `RUN_FINISHED --id r1` is dispatched
- **THEN** the pane's `status` becomes `done`

#### Scenario: RUN_FINISHED moves a blocked pane to done

- **WHEN** a pane's `status` is `blocked` (e.g. an elicitation block that was
  never cleared) and `RUN_FINISHED --id r1` is dispatched
- **THEN** the pane's `status` becomes `done`, since `Stop`/`StopFailure` are
  authoritative regardless of the pane's prior status

#### Scenario: RUN_FINISHED moves an idle pane to done

- **WHEN** a pane's `status` is `idle` and `RUN_FINISHED --id r1` is dispatched
- **THEN** the pane's `status` becomes `done`

#### Scenario: RUN_FINISHED moves working pane to done even when active_tools is nonzero

- **WHEN** a pane's `status` is `working`, `active_tools` is `1`, and
  `RUN_FINISHED --id r1` is dispatched
- **THEN** the pane's `status` becomes `done`, and `active_tools` is left
  unchanged at `1`

#### Scenario: RUN_FINISHED with an --active-tools override still reaches done

- **WHEN** a pane's `status` is `working` and
  `RUN_FINISHED --id r1 --active-tools 2` is dispatched
- **THEN** the pane's `status` becomes `done` and `active_tools` becomes `2`

#### Scenario: RUN_FINISHED moves a blocked pane to done even when active_tools is nonzero

- **WHEN** a pane's `status` is `blocked`, `active_tools` is `1` (the pending
  elicitation tool call), and `RUN_FINISHED --id r1` is dispatched
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

#### Scenario: TOOL_CALL_START moves a done pane to working

- **WHEN** a pane's `status` is `done` and `TOOL_CALL_START --id t5` (no
  `--type`, or `--type custom`) is dispatched
- **THEN** the pane's `status` becomes `working` and `active_tools` is
  incremented

#### Scenario: elicitation TOOL_CALL_START moves a done pane to blocked

- **WHEN** a pane's `status` is `done` and
  `TOOL_CALL_START --id t6 --type elicitation` is dispatched
- **THEN** the pane's `status` becomes `blocked`

#### Scenario: TOOL_CALL_RESULT does not move a done pane

- **WHEN** a pane's `status` is `done` and `TOOL_CALL_RESULT --id t7` is
  dispatched
- **THEN** the pane's `status` remains `done` and `active_tools` is decremented
  (never below `0`)

#### Scenario: TOOL_CALL_START does not move an idle pane

- **WHEN** a pane's `status` is `idle` and `TOOL_CALL_START --id t8` is
  dispatched
- **THEN** the pane's `status` remains `idle`

#### Scenario: custom-typed tool call events do not change a working pane's status

- **WHEN** a pane's `status` is `working` and
  `TOOL_CALL_START --id t2 --type custom` (or
  `TOOL_CALL_RESULT --id t2 --type custom`) is dispatched
- **THEN** the pane's `status` remains `working`

#### Scenario: tool call events without a type do not change a working pane's status

- **WHEN** a pane's `status` is `working` and `TOOL_CALL_START --id t3` (no
  `--type`) is dispatched
- **THEN** the pane's `status` remains `working`

#### Scenario: unmatched event for current status is a no-op for status

- **WHEN** a pane's `status` is `idle` and
  `TOOL_CALL_RESULT --id t4 --type elicitation` is dispatched (no corresponding
  `Blocked` status)
- **THEN** the pane's `status` remains `idle` and the event is still appended to
  `events.jsonl`
