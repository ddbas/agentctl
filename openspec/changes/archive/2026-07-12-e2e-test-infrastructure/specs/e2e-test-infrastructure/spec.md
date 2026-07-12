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

### Requirement: TmuxEnv provides a container-backed isolated tmux server

`TmuxEnv` SHALL start a Docker container with tmux installed, start a tmux
server inside that container on a Unix socket within the bind-mounted workspace,
and expose the socket path so the host `dv` binary can be directed at it.

#### Scenario: Container starts with tmux server running

- **WHEN** `TmuxEnv::new().await` is called
- **THEN** a Docker container is running with a tmux server accessible at a
  known socket path within the bind-mounted workspace

#### Scenario: Each test gets an independent tmux server

- **WHEN** two `TmuxEnv` instances exist simultaneously
- **THEN** each container has its own tmux server on a distinct socket; sessions
  in one are invisible to the other

#### Scenario: Container and socket are removed after test ends

- **WHEN** a test function completes (success or failure)
- **THEN** the container is stopped, the bind-mounted `TempDir` is dropped, and
  the socket file is removed from the host

#### Scenario: Container startup failure surfaces clearly

- **WHEN** Docker is unavailable or the image cannot be pulled
- **THEN** the test panics with a descriptive message identifying the cause

### Requirement: dv binary runner targets the isolated tmux socket

`TmuxEnv::dv()` SHALL return a `std::process::Command` using `CARGO_BIN_EXE_dv`
with `TMUX` and `TMUX_PANE` removed and a socket-targeting env var
(`DV_TMUX_SOCKET`) set to the container's socket path so that `dv` subcommands
use the isolated server.

#### Scenario: dv binary is invocable from tests

- **WHEN** a test calls `env.dv()` and appends arguments
- **THEN** the returned `Command` runs the `dv` binary built by `cargo test`

#### Scenario: TMUX and TMUX_PANE are absent from dv invocations

- **WHEN** tests run inside an active tmux session on the developer's machine
- **THEN** `dv()` strips `TMUX` and `TMUX_PANE` so `dv` does not behave as if it
  is inside a session

#### Scenario: dv is directed at the container tmux socket

- **WHEN** `dv` shells out to `tmux`
- **THEN** `DV_TMUX_SOCKET` is set to the bind-mounted socket path so tmux
  connects to the container server, not the host server

### Requirement: Async container-backed tests with Tokio runtime

Every test that interacts with a `TmuxEnv` or a testcontainer SHALL be annotated
with `#[tokio::test]`.

#### Scenario: Async test runs successfully

- **WHEN** a `#[tokio::test]` function calls `TmuxEnv::new().await`
- **THEN** the container starts and the test can proceed without blocking the
  thread

### Requirement: Parallel test execution by default

Integration tests SHALL run in parallel. No test SHALL hold a process-wide lock
or use `#[serial]` unless a documented resource conflict requires it.

#### Scenario: Two independent TmuxEnv tests run concurrently

- **WHEN** two test functions each create a `TmuxEnv`
- **THEN** they execute in parallel without interfering with each other's tmux
  servers

### Requirement: Bootstrap canary test

The scaffold SHALL include a `hello-world` container test that validates the
Docker and testcontainers infrastructure independently of the tmux helper.

#### Scenario: Canary test passes with Docker available

- **WHEN** `mise run test` is invoked with Docker running
- **THEN** the canary starts a container, asserts a non-empty container ID, and
  reports success

#### Scenario: Canary test failure is descriptive when Docker is absent

- **WHEN** Docker is unavailable
- **THEN** the test panics with a message indicating Docker is unavailable

### Requirement: Help command integration tests

The project SHALL include integration tests that verify `dv help`, `dv --help`,
and `dv` (no arguments) produce expected output and exit successfully.

#### Scenario: dv help exits zero

- **WHEN** `dv help` is invoked via `TmuxEnv::dv()`
- **THEN** the process exits with status 0

#### Scenario: dv --help prints usage

- **WHEN** `dv --help` is invoked via `TmuxEnv::dv()`
- **THEN** stdout contains "usage" (case-insensitive)

#### Scenario: dv with no args exits zero

- **WHEN** `dv` is invoked with no arguments via `TmuxEnv::dv()`
- **THEN** the process exits with status 0
