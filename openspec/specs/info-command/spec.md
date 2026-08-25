## Purpose

The `rd info` subcommand surfaces runtime metadata (version, effective log
level, log directory, and current log file) for diagnostic purposes, without
performing any filesystem writes.

## Requirements

### Requirement: `rd info` subcommand

The system SHALL provide an `rd info` subcommand that prints runtime metadata as
`key: value` lines, with no other output, and SHALL perform no filesystem
writes. The printed keys, in order, SHALL be: `version`, `log level`,
`log directory`, `current log file`.

#### Scenario: Running `rd info`

- **WHEN** the user runs `rd info`
- **THEN** the command prints exactly four `key: value` lines, in order:
  `version: <CARGO_PKG_VERSION>`, `log level: <effective log level>`,
  `log directory: <tilde-abbreviated log directory path>`,
  `current log file: <tilde-abbreviated current log file path>`
- **AND** no files or directories are created or modified as a result

### Requirement: `rd info` reflects the effective `--log-level`

The `log level` line printed by `rd info` SHALL reflect the value passed to the
global `--log-level` flag (or its default, `info`, when omitted).

#### Scenario: `rd info` with explicit log level

- **WHEN** the user runs `rd --log-level warn info`
- **THEN** the printed `log level` line reads `log level: warn`

### Requirement: `rd info` paths are tilde-abbreviated

The `log directory` and `current log file` paths printed by `rd info` SHALL have
the user's home directory prefix replaced with `~`, using the same
tilde-abbreviation convention as the rest of `rd`'s output (e.g. the `hooks`
command's install-path messages).

#### Scenario: Home-relative path abbreviation

- **WHEN** the resolved log directory is a subdirectory of the current user's
  home directory
- **THEN** `rd info` prints it with the home-directory prefix replaced by `~`

#### Scenario: Path outside home directory

- **WHEN** the resolved log directory is not under the current user's home
  directory
- **THEN** `rd info` prints the path unmodified (no `~` substitution)
