## Purpose

`agentctl` writes structured, daily-rolling file logs (independent of
stdout/stderr command output) to support diagnostics, with pruning of old log
files and a CLI-controlled log level.

## Requirements

### Requirement: Log directory resolution

The system SHALL resolve a platform-appropriate log directory by trying
`dirs::state_dir()` first, falling back to `dirs::data_local_dir()`, and joining
the result with `agentctl/logs`. Resolution SHALL be pure (no I/O, no directory
creation) and SHALL return `None` if neither base directory can be determined.

#### Scenario: Resolve log directory on a supported platform

- **WHEN** the log directory is resolved on a platform where `dirs::state_dir()`
  or `dirs::data_local_dir()` returns a path
- **THEN** the system returns `Some(<base>/agentctl/logs)` without creating any
  directories or files

### Requirement: Current log file path computation

The system SHALL compute the expected current-day log file path as
`<log_dir>/agentctl.log.YYYY-MM-DD`, using the local date, without verifying the
file exists on disk and without performing any I/O.

#### Scenario: Compute today's log file path

- **WHEN** `current_log_file` is called with a log directory path
- **THEN** the system returns
  `<log_dir>/agentctl.log.<today's date in YYYY-MM-DD format>`

### Requirement: Daily log rotation

The system SHALL write log output to a daily-rolling file appender rooted at the
resolved log directory, using the file name prefix `agentctl.log`, producing one
file per calendar day named `agentctl.log.YYYY-MM-DD`.

#### Scenario: Logs roll over at day boundary

- **WHEN** the logging subsystem is initialized and log entries are written on
  two different calendar days
- **THEN** entries are written to two distinct files, each named
  `agentctl.log.<that day's date>`

### Requirement: Log retention pruning

Before initializing the file appender, the system SHALL prune old log files in
the log directory matching the `agentctl.log.*` prefix, keeping only the 7 most
recently modified files and deleting the rest. Files not matching the prefix
SHALL be left untouched.

#### Scenario: More than 7 log files exist

- **WHEN** the log directory contains 10 files matching `agentctl.log.*` and
  logging is initialized
- **THEN** only the 7 most recently modified `agentctl.log.*` files remain and
  the other 3 are deleted

#### Scenario: Non-log files are ignored by pruning

- **WHEN** the log directory contains files matching `agentctl.log.*` alongside
  files that do not match that prefix
- **THEN** pruning never deletes the non-matching files, regardless of their
  modification time or the retention count

### Requirement: CLI log level control

The system SHALL expose a global `--log-level <LEVEL>` CLI flag accepting
`trace`, `debug`, `info`, `warn`, or `error` (default: `info`), and SHALL use
the selected level as the file appender's filter level. This flag SHALL NOT
affect any stdout/stderr output produced by the CLI's normal command output.

#### Scenario: Default log level

- **WHEN** `agentctl` is invoked without `--log-level`
- **THEN** the effective log level for the log file is `info`

#### Scenario: Explicit log level

- **WHEN** `agentctl` is invoked with `--log-level debug`
- **THEN** log entries at `debug` level and above are written to the log file,
  and stdout/stderr output of the invoked command is unaffected

### Requirement: Best-effort logging initialization

The system SHALL initialize logging as a best-effort operation: if the log
directory cannot be resolved, created, or written to, initialization SHALL
return without an error and the CLI SHALL continue to execute the requested
command normally, without producing file logs.

#### Scenario: Logging initialization fails silently

- **WHEN** the log directory cannot be created (e.g. due to filesystem
  permissions)
- **THEN** the CLI command being invoked still completes successfully, and no
  error about logging is surfaced to the user
