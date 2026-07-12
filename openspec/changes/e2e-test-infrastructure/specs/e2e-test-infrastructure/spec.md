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

The project SHALL provide a `tests/common/mod.rs` module that exports
container-startup helpers and shared fixtures usable by all integration test
files.

#### Scenario: Common module is importable

- **WHEN** an integration test file declares `mod common;`
- **THEN** it can call helper functions from `tests/common/mod.rs` without
  compilation errors

### Requirement: Async container-backed tests with Tokio runtime

Every E2E test function that interacts with a testcontainer SHALL be annotated
with `#[tokio::test]`. Each test MUST start its own container instance and stop
it automatically when the test completes (via Drop or explicit cleanup).

#### Scenario: Container starts and is accessible

- **WHEN** a test function annotated with `#[tokio::test]` calls the container
  startup helper
- **THEN** a Docker container is running and the test can communicate with it

#### Scenario: Container stops after test ends

- **WHEN** a test function completes (success or failure)
- **THEN** the container is removed and no orphaned containers remain on the
  host

#### Scenario: Container startup failure surfaces clearly

- **WHEN** Docker is unavailable or the image cannot be pulled
- **THEN** the test fails with a descriptive panic message identifying the cause

### Requirement: Parallel test execution by default

Integration tests SHALL run in parallel. No test SHALL hold a process-wide lock
or use `#[serial]` unless a documented resource conflict requires it.

#### Scenario: Two independent tests run concurrently

- **WHEN** two E2E test functions are present in the suite
- **THEN** Cargo's test harness executes them in parallel threads without
  deadlocks or data races

### Requirement: Bootstrap example test

The scaffold SHALL include at least one working example E2E test that starts a
container, performs a trivial assertion, and stops the container. This validates
the entire infrastructure end-to-end and runs as part of `mise run test`.

#### Scenario: Example test passes with Docker available

- **WHEN** `mise run test` is invoked with Docker running
- **THEN** the example test starts a container, asserts the expected condition,
  and reports success

#### Scenario: Example test failure is descriptive when Docker is absent

- **WHEN** `mise run test` is invoked without Docker
- **THEN** the test fails with a message indicating Docker is unavailable, not a
  cryptic panic

### Requirement: DvEnv helper isolates dv binary execution

The `DvEnv` struct SHALL provide a method that returns a `std::process::Command`
pre-configured with `CARGO_BIN_EXE_dv` pointing at the compiled binary and with
host git environment variables (`GIT_DIR`, `GIT_INDEX_FILE`, `GIT_WORK_TREE`,
`GIT_PREFIX`, `GIT_COMMON_DIR`) cleared.

#### Scenario: dv binary is invocable from tests

- **WHEN** a test calls `env.dv()` and appends arguments
- **THEN** the returned `Command` runs the `dv` binary built by `cargo test`

#### Scenario: Host git env vars do not leak into dv invocations

- **WHEN** a test is executed inside a git-hook context where `GIT_DIR` is set
- **THEN** the `dv()` command builder strips `GIT_DIR` so `dv` operates on the
  correct repository

### Requirement: Help command integration tests

The project SHALL include integration tests that verify `dv help`, `dv --help`,
and `dv` (no arguments) produce expected output and exit successfully.

#### Scenario: dv help exits zero

- **WHEN** `dv help` is invoked
- **THEN** the process exits with status 0

#### Scenario: dv --help prints usage

- **WHEN** `dv --help` is invoked
- **THEN** stdout contains "Usage" (case-insensitive)

#### Scenario: dv with no args exits zero

- **WHEN** `dv` is invoked with no arguments
- **THEN** the process exits with status 0
