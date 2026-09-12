## ADDED Requirements

### Requirement: TOOL_CALL_START and TOOL_CALL_RESULT mutate active_tools independently of status

`TOOL_CALL_START` and `TOOL_CALL_RESULT` SHALL always update the pane's
`active_tools` counter in `state.json`, in addition to the status-FSM transition
already defined for these events (elicitation-typed calls toggling `Working` ↔
`Blocked`): `TOOL_CALL_START` increments `active_tools` by one,
`TOOL_CALL_RESULT` decrements it by one (clamped at a minimum of `0`). This
mutation SHALL occur regardless of the event's `--type` value and regardless of
whether the event drove a status transition.

#### Scenario: Elicitation TOOL_CALL_START both blocks status and increments active_tools

- **WHEN** a pane's `status` is `working` and `active_tools` is `0`, and
  `agentctl agent event TOOL_CALL_START --id t1 --type elicitation` is
  dispatched
- **THEN** the pane's `status` becomes `blocked` and `active_tools` becomes `1`

#### Scenario: Custom TOOL_CALL_START increments active_tools without changing status

- **WHEN** a pane's `status` is `working` and `active_tools` is `0`, and
  `agentctl agent event TOOL_CALL_START --id t2 --type custom` is dispatched
- **THEN** the pane's `status` remains `working` and `active_tools` becomes `1`

#### Scenario: TOOL_CALL_RESULT decrements active_tools even when it is a status no-op

- **WHEN** a pane's `status` is `idle`, `active_tools` is `1`, and
  `agentctl agent event TOOL_CALL_RESULT --id t3` is dispatched (no `--type`, so
  no matching FSM transition exists from `idle`)
- **THEN** the pane's `status` remains `idle` and `active_tools` becomes `0`
