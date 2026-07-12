## Why

The `dv` project has no integration or E2E test infrastructure. Without
container isolation, any test that shells out to `git` or touches the filesystem
can corrupt the developer's environment, be affected by host-level git
configuration and hooks, and produce non-deterministic results across machines.
We need a reproducible, host-safe test harness before adding meaningful CLI
coverage.

## What Changes

- Add a `tests/` directory at the repository root for Cargo integration test
  binaries.
- Add a `tests/common/mod.rs` shared helper module that exports a `DvEnv` struct
  backed by a `testcontainers` Docker container (`alpine/git`), providing fully
  isolated git environment and `dv` binary runner helpers.
- Add an example E2E test (`tests/e2e_example.rs`) that starts a `hello-world`
  container as a canary for the Docker / testcontainers plumbing.
- Add a first real integration test (`tests/help_command.rs`) exercising
  `dv help`, `dv --help`, and `dv` (no args) inside the isolated environment.
- Update `Cargo.toml` dev-dependencies to include `testcontainers`, `tokio`
  (with `macros`/`rt-multi-thread`/`time` features), and `tempfile`.

## Capabilities

### New Capabilities

- `e2e-test-infrastructure`: End-to-end/integration test scaffold — shared
  `DvEnv` helper, Docker-backed isolation via testcontainers, async Tokio test
  support, parallel execution, and a working example test validating the full
  container lifecycle.

### Modified Capabilities

<!-- none -->

## Impact

- `Cargo.toml`: dev-dependencies gain `testcontainers`, `tokio`, `tempfile`
  (already present, verify versions).
- New files under `tests/`: `common/mod.rs`, `e2e_example.rs`,
  `help_command.rs`.
- `mise run test` runs all integration tests; Docker must be available in CI.
- No changes to production `src/` code.
