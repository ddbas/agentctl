# uninstall-command

## Purpose

Specifies `rd uninstall <harness>`, the counterpart to `rd install <harness>`
that removes radr-managed integration files/entries for a given harness.

## Requirements

### Requirement: rd uninstall accepts the same harness values as rd install

The `rd uninstall <harness>` subcommand SHALL accept a `HARNESS` argument using
the same `Harness` value enum as `rd install`. Any harness value valid for
`rd install` SHALL be valid for `rd uninstall`, and any harness value invalid
for `rd install` (unrecognized string) SHALL be rejected by `rd uninstall` with
the same clap parse error.

#### Scenario: uninstall accepts a currently-supported harness

- **WHEN** the user runs `rd uninstall pi` or `rd uninstall claude`
- **THEN** the command parses successfully and dispatches to the corresponding
  uninstall handler

#### Scenario: uninstall rejects an unknown harness

- **WHEN** the user runs `rd uninstall notaharness`
- **THEN** the command exits non-zero with a clap argument-parsing error,
  identical in form to `rd install notaharness`

#### Scenario: a harness added to install is automatically available to uninstall

- **WHEN** a new variant is added to the shared `Harness` enum for a new harness
  and `rd install <new-harness>` is implemented
- **THEN** `rd uninstall <new-harness>` SHALL also be a valid invocation, since
  both subcommands share the same enum type

### Requirement: rd uninstall dispatches to a per-harness handler

`rd uninstall <harness>` SHALL dispatch to harness-specific removal logic
mirroring the dispatch structure of `rd install <harness>`.

#### Scenario: uninstall dispatches based on harness argument

- **WHEN** the user runs `rd uninstall pi`
- **THEN** the pi-specific uninstall handler runs and no other harness's files
  are touched

#### Scenario: uninstall prints a summary and exits 0 on success

- **WHEN** any supported harness's uninstall handler completes without error
- **THEN** the command prints a human-readable summary of what was removed (or
  that nothing needed removing) and exits with status 0
