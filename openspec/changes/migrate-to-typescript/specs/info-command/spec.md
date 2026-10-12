## MODIFIED Requirements

### Requirement: `agentctl info` subcommand

The system SHALL provide an `agentctl info` subcommand that prints runtime
metadata as `key: value` lines, with no other output, and SHALL perform no
filesystem writes. The printed keys, in order, SHALL be: `version`, `log level`,
`log directory`, `current log file`.

#### Scenario: Running `agentctl info`

- **WHEN** the user runs `agentctl info`
- **THEN** the command prints exactly four `key: value` lines, in order:
  `version: <package version of apps/agentctl>`,
  `log level: <effective log level>`,
  `log directory: <tilde-abbreviated log directory path>`,
  `current log file: <tilde-abbreviated current log file path>`
- **AND** no files or directories are created or modified as a result
