## Why

The `dv` CLI manages tmux sessions. Without container isolation, any test that
invokes `tmux` will interact with the developer's live sessions, be affected by
`TMUX`/`TMUX_PANE` env vars set when running inside an existing session, and
leave orphaned sessions on the host. We need a reproducible, host-safe test
harness that gives every test its own isolated tmux server before we add
meaningful CLI coverage.

## What Changes

- Add a `tests/` directory at the repository root for Cargo integration test
  binaries.
- Add a `tests/common/mod.rs` shared helper module that exports a `TmuxEnv`
  struct: it starts an `alpine`-based Docker container with tmux installed,
  starts an isolated tmux server inside that container on a bind-mounted socket
  path, and exposes a `dv` binary runner that points at that socket.
- Add an example E2E test (`tests/e2e_example.rs`) that starts a `hello-world`
  Docker container via testcontainers as a canary for the async runner
  infrastructure.
- Add a first real integration test (`tests/help_command.rs`) exercising
  `dv help`, `dv --help`, and `dv` (no args) inside an isolated `TmuxEnv`.
- Update `Cargo.toml` dev-dependencies to include `testcontainers`, `tokio`
  (with `macros`/`rt-multi-thread`/`time` features), and `tempfile`.

## Capabilities

### New Capabilities

- `e2e-test-infrastructure`: E2E/integration test scaffold — `TmuxEnv` helper
  with a container-backed tmux server, testcontainers async runner, parallel
  execution, canary test, and help command integration tests.

### Modified Capabilities

<!-- none -->

## Impact

- `Cargo.toml`: dev-dependencies gain `testcontainers`, `tokio`, `tempfile`
  (verify versions already present).
- New files under `tests/`: `common/mod.rs`, `e2e_example.rs`,
  `help_command.rs`.
- `mise run test` runs all integration tests; Docker must be available in CI.
- No changes to production `src/` code.
