## MODIFIED Requirements

### Requirement: AgentState has a canonical sort ordering

The `AgentState` type SHALL implement `Ord` and `PartialOrd` using an explicit
priority mapping:
`Blocked(0) < Working(1) < Snoozed(2) < Done(3) < Idle(4) < Unknown(5)`. This
ordering is independent of enum variant declaration order. `Snoozed` SHALL rank
below `Blocked` and `Working` (no immediate human attention is required) and
above `Done` and `Idle` (a snoozed agent still has in-flight work that will
resume on its own, unlike a settled or waiting agent).

#### Scenario: Blocked is the lowest priority value

- **WHEN** `AgentState::Blocked` is compared to any other `AgentState` variant
- **THEN** `AgentState::Blocked` is less than all other variants

#### Scenario: Unknown is the highest priority value

- **WHEN** `AgentState::Unknown` is compared to any other `AgentState` variant
- **THEN** `AgentState::Unknown` is greater than all other variants

#### Scenario: Snoozed sorts below Working and Blocked

- **WHEN** `AgentState::Snoozed` is compared to `AgentState::Working` or
  `AgentState::Blocked`
- **THEN** `AgentState::Snoozed` is greater than both

#### Scenario: Snoozed sorts above Done and Idle

- **WHEN** `AgentState::Snoozed` is compared to `AgentState::Done` or
  `AgentState::Idle`
- **THEN** `AgentState::Snoozed` is less than both

#### Scenario: Full ordering is stable

- **WHEN** all six variants are sorted in ascending order
- **THEN** the result is `[Blocked, Working, Snoozed, Done, Idle, Unknown]`
