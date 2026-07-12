## Why

The `dv` CLI manages tmux sessions. Without test isolation, any test that shells
out to `tmux` will attach to or interfere with the developer's own running tmux
sessions, be affected by the `TMUX`/`TMUX_PANE` env vars injected when running
inside an existing session, and leave orphaned sessions on the host. We need a
lightweight, reproducible test harness that gives every test its own isolated
tmux socket before we add meaningful CLI coverage.

## What Changes

- Add a `tests/` directory at the repository root for Cargo integration test
  binaries.
- Add a `tests/common/mod.rs` shared helper module that exports a `TmuxEnv`
  struct providing fully isolated tmux sessions via a per-test `TempDir` used as
  `TMUX_TMPDIR`, plus a `dv` binary runner that strips tmux-injected env vars.
- Add an example E2E test (`tests/e2e_example.rs`) that starts a `hello-world`
  Docker container via testcontainers as a canary for the async runner
  infrastructure.
- Add a first real integration test (`tests/help_command.rs`) exercising
  `dv help`, `dv --help`, and `dv` (no args) inside an isolated `TmuxEnv`.
- Update `Cargo.toml` dev-dependencies to include `testcontainers`, `tokio`
  (with `macros`/`rt-multi-thread`/`time` features), and `tempfile`.

## Capabilities

### New Capabilities

- `e2e-test-infrastructure`: E2E/integration test scaffold — shared `TmuxEnv`
  helper providing tmux socket isolation, testcontainers-backed Docker canary,
  async Tokio test support, parallel execution, and working example tests.

### Modified Capabilities

<!-- none -->

## Impact

- `Cargo.toml`: dev-dependencies gain `testcontainers`, `tokio`, `tempfile`
  (verify versions already present).
- New files under `tests/`: `common/mod.rs`, `e2e_example.rs`,
  `help_command.rs`.
- `mise run test` runs all integration tests; Docker must be available in CI for
  the canary test.
- No changes to production `src/` code.
