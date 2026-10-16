# agentctl

Implements the `agentctl` CLI executable .

## Rules

- It MUST not contain business logic. It is only a thin wrapper that wires
  together the core implementation (`@agentctl/core`) and the adapters.
