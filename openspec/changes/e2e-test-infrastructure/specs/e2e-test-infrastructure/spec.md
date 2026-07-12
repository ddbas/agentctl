## ADDED Requirements

### Requirement: Tests directory exists with integration test support

The project SHALL have a `tests/` directory at the repository root. All E2E and
integration tests SHALL reside here so that Cargo compiles them as separate test
binaries isolated from production code.

#### Scenario: Integration test binary compiles

- **WHEN** `cargo test --test '*'` is invoked
- **THEN** all files under `tests/` compile successfully without errors

#### Scenario: Unit tests are unaffected

- **WHEN** `cargo test --lib` is invoked
- **THEN** only tests in `src/` run; no integration test binary is executed

### Requirement: Shared test helper module

The project SHALL provide a `tests/common/mod.rs` module that exports `TmuxEnv`
and a Docker canary helper usable by all integration test files.

#### Scenario: Common module is importable

- **WHEN** an integration test file declares `mod common;`
- **THEN** it can use `TmuxEnv` and `start_generic_container` without
  compilation errors

### Requirement: TmuxEnv isolates each test from the host tmux server

`TmuxEnv` SHALL point `TMUX_TMPDIR` at a per-test `TempDir` so that every `tmux`
invocation uses a fresh socket file completely separate from the developer's
running sessions.

#### Scenario: dv does not see the host tmux session

- **WHEN** a test runs inside an active tmux session (i.e. `TMUX` is set in the
  environment)
- **THEN** `TmuxEnv::dv()` clears `TMUX` and `TMUX_PANE` so `dv` behaves as if
  it is not inside tmux

#### Scenario: Isolated tmux socket is removed after test

- **WHEN** a test function completes (success or failure)
- **THEN** the `TempDir` holding `TMUX_TMPDIR` is dropped and all socket files
  are removed from the host

#### Scenario: Concurrent tests do not share sessions

- **WHEN** two `TmuxEnv` instances exist simultaneously
- **THEN** each uses a distinct `TMUX_TMPDIR` and tmux sockets do not overlap

### Requirement: dv binary runner strips tmux environment contamination

`TmuxEnv::dv()` SHALL return a `std::process::Command` using `CARGO_BIN_EXE_dv`
with `TMUX`, `TMUX_PANE` removed and `TMUX_TMPDIR` set to the isolated temp
directory.

#### Scenario: dv binary is invocable from tests

- **WHEN** a test calls `env.dv()` and appends arguments
- **THEN** the returned `Command` runs the `dv` binary built by `cargo test`

#### Scenario: TMUX_TMPDIR is set to the isolated directory

- **WHEN** `dv` shells out to `tmux`
- **THEN** tmux uses the per-test socket path, not the system default under
  `/tmp/tmux-<uid>/`

### Requirement: Async Docker canary test with Tokio runtime

The scaffold SHALL include a test that starts a Docker container via
testcontainers to validate the async runner infrastructure. The test MUST be
annotated with `#[tokio::test]` and the container MUST be removed automatically
on drop.

#### Scenario: Container starts and is accessible

- **WHEN** a `#[tokio::test]` calls `start_generic_container()`
- **THEN** a Docker container is running and the container ID is non-empty

#### Scenario: Container stops after test ends

- **WHEN** the test function completes
- **THEN** the container handle is dropped and no orphaned containers remain

#### Scenario: Container startup failure surfaces clearly

- **WHEN** Docker is unavailable
- **THEN** the test panics with a descriptive message, not a cryptic error

### Requirement: Parallel test execution by default

Integration tests SHALL run in parallel. No test SHALL hold a process-wide lock
or use `#[serial]` unless a documented resource conflict requires it.

#### Scenario: Two independent TmuxEnv tests run concurrently

- **WHEN** two test functions each create a `TmuxEnv`
- **THEN** they execute in parallel without interfering with each other's tmux
  sockets

### Requirement: Help command integration tests

The project SHALL include integration tests that verify `dv help`, `dv --help`,
and `dv` (no arguments) produce expected output and exit successfully.

#### Scenario: dv help exits zero

- **WHEN** `dv help` is invoked via `TmuxEnv::dv()`
- **THEN** the process exits with status 0

#### Scenario: dv --help prints usage

- **WHEN** `dv --help` is invoked via `TmuxEnv::dv()`
- **THEN** stdout contains "Usage" (case-insensitive)

#### Scenario: dv with no args exits zero

- **WHEN** `dv` is invoked with no arguments via `TmuxEnv::dv()`
- **THEN** the process exits with status 0
