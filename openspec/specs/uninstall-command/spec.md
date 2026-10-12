# uninstall-command

## Purpose

Specifies `agentctl uninstall <harness>`, the counterpart to
`agentctl install <harness>` that removes agentctl-managed integration
files/entries for a given harness.

## Requirements

### Requirement: agentctl uninstall accepts the same harness values as agentctl install

The `agentctl uninstall <harness>` subcommand SHALL accept a `HARNESS` argument
using the same `Harness` value set as `agentctl install`. Any harness value
valid for `agentctl install` SHALL be valid for `agentctl uninstall`, and any
harness value invalid for `agentctl install` (unrecognized string) SHALL be
rejected by `agentctl uninstall` with the same usage error as
`agentctl install`.

#### Scenario: uninstall accepts a currently-supported harness

- **WHEN** the user runs `agentctl uninstall pi` or `agentctl uninstall claude`
- **THEN** the command parses successfully and dispatches to the corresponding
  uninstall handler

#### Scenario: uninstall rejects an unknown harness

- **WHEN** the user runs `agentctl uninstall notaharness`
- **THEN** the command exits non-zero with a usage error on stderr, identical in
  form to `agentctl install notaharness`

#### Scenario: a harness added to install is automatically available to uninstall

- **WHEN** a new variant is added to the shared `Harness` value set for a new
  harness and `agentctl install <new-harness>` is implemented
- **THEN** `agentctl uninstall <new-harness>` SHALL also be a valid invocation,
  since both subcommands share the same value set

### Requirement: agentctl uninstall dispatches to a per-harness handler

`agentctl uninstall <harness>` SHALL dispatch to harness-specific removal logic
mirroring the dispatch structure of `agentctl install <harness>`.

#### Scenario: uninstall dispatches based on harness argument

- **WHEN** the user runs `agentctl uninstall pi`
- **THEN** the pi-specific uninstall handler runs and no other harness's files
  are touched

#### Scenario: uninstall prints a summary and exits 0 on success

- **WHEN** any supported harness's uninstall handler completes without error
- **THEN** the command prints a human-readable summary of what was removed (or
  that nothing needed removing) and exits with status 0
