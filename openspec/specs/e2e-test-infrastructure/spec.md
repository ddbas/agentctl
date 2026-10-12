# e2e-test-infrastructure

## Purpose

Provide a reproducible, host-safe E2E and integration test infrastructure using
ephemeral Docker containers via testcontainers. Each test gets a fully isolated
tmux server running inside a container, with automatic cleanup on completion.

## Requirements

### Requirement: TmuxEnv provides a container-backed isolated tmux server

`TmuxEnv` SHALL start a Docker container with tmux installed (via the
`testcontainers` npm package), start a tmux server inside that container on a
Unix socket within the bind-mounted workspace, and expose the socket path so the
host `agentctl` process can be directed at it.

#### Scenario: Container starts with tmux server running

- **WHEN** `await TmuxEnv.create()` is called
- **THEN** a Docker container is running with a tmux server accessible at a
  known socket path within the bind-mounted workspace

#### Scenario: Each test gets an independent tmux server

- **WHEN** two `TmuxEnv` instances exist simultaneously
- **THEN** each container has its own tmux server on a distinct socket; sessions
  in one are invisible to the other

#### Scenario: Container and socket are removed after test ends

- **WHEN** a test completes (success or failure) and `env.dispose()` runs in
  `afterEach`/`using` cleanup
- **THEN** the container is stopped, the temp workspace directory is removed,
  and the socket file is removed from the host

#### Scenario: Container startup failure surfaces clearly

- **WHEN** Docker is unavailable or the image cannot be pulled
- **THEN** the test fails with a descriptive message identifying the cause

### Requirement: Parallel test execution by default

Integration tests SHALL run in parallel (vitest default file and test
concurrency). No test SHALL use a global lock or `--no-file-parallelism` unless
a documented resource conflict requires it.

#### Scenario: Two independent TmuxEnv tests run concurrently

- **WHEN** two tests each create a `TmuxEnv`
- **THEN** they execute in parallel without interfering with each other's tmux
  servers

### Requirement: Bootstrap canary test

The suite SHALL include a `hello-world` container test that validates the Docker
and testcontainers infrastructure independently of the tmux helper.

#### Scenario: Canary test passes with Docker available

- **WHEN** `pnpm turbo run test` is invoked with Docker running
- **THEN** the canary starts a container, asserts a non-empty container ID, and
  reports success

#### Scenario: Canary test failure is descriptive when Docker is absent

- **WHEN** Docker is unavailable
- **THEN** the test fails with a message indicating Docker is unavailable

### Requirement: Help command integration tests

The project SHALL include integration tests that verify `agentctl help`,
`agentctl --help`, and `agentctl` (no arguments) produce expected output and
exit successfully.

#### Scenario: agentctl help exits zero

- **WHEN** `agentctl help` is invoked via `TmuxEnv::agentctl()`
- **THEN** the process exits with status 0

#### Scenario: agentctl --help prints usage

- **WHEN** `agentctl --help` is invoked via `TmuxEnv::agentctl()`
- **THEN** stdout contains "usage" (case-insensitive)

#### Scenario: agentctl with no args exits zero

- **WHEN** `agentctl` is invoked with no arguments via `TmuxEnv::agentctl()`
- **THEN** the process exits with status 0

### Requirement: E2E test workspace exists with vitest

E2E and integration tests SHALL live in `apps/agentctl/tests/e2e/` and run with
vitest, separately from unit tests colocated with `packages/*` sources.

#### Scenario: E2E tests are discovered

- **WHEN** `pnpm turbo run test` is invoked
- **THEN** vitest discovers and runs every `*.e2e.test.ts` file under
  `apps/agentctl/tests/e2e/`

#### Scenario: Unit tests are unaffected

- **WHEN** `pnpm --filter @agentctl/core test` is invoked
- **THEN** only unit tests in that package run; no e2e test is executed

### Requirement: Shared e2e helper module

The project SHALL provide `apps/agentctl/tests/e2e/support/tmux-env.ts` that
exports `TmuxEnv` and a Docker canary helper usable by all e2e test files.

#### Scenario: Support module is importable

- **WHEN** an e2e test file imports from `./support/tmux-env`
- **THEN** it can use `TmuxEnv` and `startGenericContainer` and typechecks

### Requirement: agentctl runner targets the isolated tmux socket and a selectable binary

`TmuxEnv.agentctl(args)` SHALL spawn the executable named by the `AGENTCTL_BIN`
environment variable (defaulting to the built TypeScript bundle, which is run
under `node`) with `TMUX` and `TMUX_PANE` removed, `XDG_DATA_HOME` set to the
test's isolated directory, and `AGENTCTL_TMUX_SOCKET` set to the container's
socket path so that `agentctl` subcommands use the isolated server.

#### Scenario: agentctl is invocable from tests

- **WHEN** a test calls `env.agentctl(["help"])`
- **THEN** the selected binary runs and the result exposes `status`, `stdout`,
  and `stderr`

#### Scenario: A native binary is executed directly

- **WHEN** `AGENTCTL_BIN` points at a non-JavaScript executable (anything other
  than `.js`, `.cjs`, or `.mjs`)
- **THEN** `env.agentctl()` runs that binary directly (not under `node`) and the
  same tests execute unchanged

#### Scenario: TMUX and TMUX_PANE are absent from agentctl invocations

- **WHEN** tests run inside an active tmux session on the developer's machine
- **THEN** `agentctl()` strips `TMUX` and `TMUX_PANE` so `agentctl` does not
  behave as if it is inside a session

#### Scenario: agentctl is directed at the container tmux socket

- **WHEN** `agentctl` shells out to `tmux`
- **THEN** `AGENTCTL_TMUX_SOCKET` is set to the bind-mounted socket path so tmux
  connects to the container server, not the host server

### Requirement: Async container-backed tests

Every test that uses `TmuxEnv` or a testcontainer SHALL be an `async` vitest
test with a timeout sized for container startup.

#### Scenario: Async test runs successfully

- **WHEN** an async test awaits `TmuxEnv.create()`
- **THEN** the container starts and the test proceeds without blocking the event
  loop

### Requirement: Dual-implementation run during migration

Until the Rust implementation is removed, CI SHALL run the e2e suite twice: once
with `AGENTCTL_BIN` set to the Rust binary and once against the TypeScript
bundle. Removing the Rust implementation SHALL remove the first run without
modifying any test.

#### Scenario: Both legs must pass

- **WHEN** CI runs during the migration window
- **THEN** a failure in either leg fails the pipeline

#### Scenario: Cutover leaves tests untouched

- **WHEN** the Rust implementation and the Rust CI leg are deleted
- **THEN** no e2e test file changes in that commit
