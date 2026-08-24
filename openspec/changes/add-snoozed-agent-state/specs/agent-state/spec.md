## MODIFIED Requirements

### Requirement: AgentState enum

The system SHALL define a standard `AgentState` enum with the following
variants: `Idle`, `Working`, `Done`, `Blocked`, `Snoozed`, `Unknown`. This enum
SHALL be the canonical representation of agent state throughout the codebase.
`Snoozed` represents an agent that is not currently producing visible output but
has an armed, self-resuming mechanism (e.g. a background watch or listener) that
will wake it up automatically without new human input.

#### Scenario: All variants are defined

- **WHEN** the `AgentState` type is used anywhere in the codebase
- **THEN** it exposes exactly the variants `Idle`, `Working`, `Done`, `Blocked`,
  `Snoozed`, and `Unknown` with no additional variants

#### Scenario: Snoozed serializes and displays as "snoozed"

- **WHEN** `AgentState::Snoozed` is serialized to JSON or formatted via
  `Display`
- **THEN** it renders as the string `"snoozed"` in both cases
