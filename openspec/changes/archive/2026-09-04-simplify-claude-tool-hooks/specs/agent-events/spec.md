## MODIFIED Requirements

### Requirement: type is optional and only accepted on TOOL*CALL*\* events

`TOOL_CALL_START` and `TOOL_CALL_RESULT` SHALL accept an optional `--type` flag
with accepted values `custom` and `elicitation`. `SESSION_STARTED`,
`RUN_STARTED`, and `RUN_FINISHED` SHALL NOT accept a `--type` flag.
`permission-prompt` is no longer an accepted value.

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

#### Scenario: permission-prompt is no longer an accepted type value

- **WHEN** `rd agent event TOOL_CALL_START --id <id> --type permission-prompt`
  is invoked
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
- Any status on `RUN_FINISHED` → `Done`, **provided** the pane's reconciled
  `active_tools` count is `0` at the time `RUN_FINISHED` is dispatched. When
  `active_tools` is greater than `0`, `RUN_FINISHED` SHALL NOT change `status`
  (regardless of current status).
- `Working` on `TOOL_CALL_START` with `type: elicitation` → `Blocked`
- `Blocked` on `TOOL_CALL_RESULT` with `type: elicitation` → `Working`

An event that does not match a defined transition for the pane's current status
SHALL NOT change `status`, but SHALL still be appended to `events.jsonl`. There
is no `permission-prompt` type or transition.

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

#### Scenario: RUN_FINISHED moves working pane to done when active_tools is zero

- **WHEN** a pane's `status` is `working`, `active_tools` is `0`, and
  `RUN_FINISHED --id r1` is dispatched
- **THEN** the pane's `status` becomes `done`

#### Scenario: RUN_FINISHED moves a blocked pane to done when active_tools is zero

- **WHEN** a pane's `status` is `blocked` (e.g. an elicitation block that was
  never cleared), `active_tools` is `0`, and `RUN_FINISHED --id r1` is
  dispatched
- **THEN** the pane's `status` becomes `done`, since `Stop`/`StopFailure` are
  authoritative regardless of the pane's prior status

#### Scenario: RUN_FINISHED moves an idle pane to done when active_tools is zero

- **WHEN** a pane's `status` is `idle`, `active_tools` is `0`, and
  `RUN_FINISHED --id r1` is dispatched
- **THEN** the pane's `status` becomes `done`

#### Scenario: RUN_FINISHED does not change status when active_tools is nonzero

- **WHEN** a pane's `status` is `working`, `active_tools` is `1` (reconciled
  from an `--active-tools` override or the pane's own counter), and
  `RUN_FINISHED --id r1` is dispatched
- **THEN** the pane's `status` remains `working` and the event is still appended
  to `events.jsonl`

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

### Requirement: TOOL_CALL_START and TOOL_CALL_RESULT mutate active_tools independently of status

`TOOL_CALL_START` and `TOOL_CALL_RESULT` SHALL always update the pane's
`active_tools` counter in `state.json`, in addition to the status-FSM transition
already defined for these events (elicitation-typed calls toggling `Working` ↔
`Blocked`): `TOOL_CALL_START` increments `active_tools` by one,
`TOOL_CALL_RESULT` decrements it by one (clamped at a minimum of `0`). This
mutation SHALL occur regardless of whether the event drove a status transition,
for every accepted `--type` value (`custom`, `elicitation`, or omitted).

#### Scenario: Elicitation TOOL_CALL_START both blocks status and increments active_tools

- **WHEN** a pane's `status` is `working` and `active_tools` is `0`, and
  `rd agent event TOOL_CALL_START --id t1 --type elicitation` is dispatched
- **THEN** the pane's `status` becomes `blocked` and `active_tools` becomes `1`

#### Scenario: Custom TOOL_CALL_START increments active_tools without changing status

- **WHEN** a pane's `status` is `working` and `active_tools` is `0`, and
  `rd agent event TOOL_CALL_START --id t2 --type custom` is dispatched
- **THEN** the pane's `status` remains `working` and `active_tools` becomes `1`

#### Scenario: TOOL_CALL_RESULT decrements active_tools even when it is a status no-op

- **WHEN** a pane's `status` is `idle`, `active_tools` is `1`, and
  `rd agent event TOOL_CALL_RESULT --id t3` is dispatched (no `--type`, so no
  matching FSM transition exists from `idle`)
- **THEN** the pane's `status` remains `idle` and `active_tools` becomes `0`

## REMOVED Requirements

### Requirement: blocked_prompts tracks in-flight permission-prompt blocks independently of active_tools

**Reason**: `blocked_prompts` existed solely to prevent one `permission-prompt`
close from prematurely clearing a `blocked` status opened for a second,
concurrent permission prompt. The `permission-prompt` type and its
`Notification`-driven open were never actually correlated correctly (the open
and close ids were independently random), so this counter never protected
anything in practice. With `permission-prompt` removed, there is no remaining
writer or reader for `blocked_prompts`.

**Migration**: None required. `elicitation`'s `Blocked` ↔ `Working` transition
remains unconditional (no counter) exactly as it already behaved. Old
`state.json` files containing a `blocked_prompts` key continue to deserialize
successfully (the field is simply dropped from newly written files).

### Requirement: SESSION_STARTED resets blocked_prompts to zero

**Reason**: Removed alongside `blocked_prompts` itself; see above.

**Migration**: None required.
