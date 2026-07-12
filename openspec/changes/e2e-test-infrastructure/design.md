## Context

The `dv` CLI manages tmux sessions. Currently it only has a `help` command, but
future subcommands will shell out to `tmux` to create, list, attach to, and kill
sessions. Without test isolation, tests that invoke `tmux` will interact with
the developer's live sessions, be affected by `TMUX`/`TMUX_PANE` env vars set
when running inside an existing tmux session, and leave orphaned sessions on the
host.

The bonsai project (`~/repos/bonsai`) solved an analogous problem for git using
a `GitEnv` helper backed by a Docker container. The core pattern transfers here
— a per-test `TmuxEnv` helper — but the isolation mechanism is simpler: tmux
isolation is achieved by pointing `TMUX_TMPDIR` at a throwaway `TempDir`, no
container required for most tests.

## Goals / Non-Goals

**Goals:**

- Establish a `tests/` directory with Cargo integration test support.
- Provide a `tests/common/mod.rs` module with:
  - `TmuxEnv`: a per-test isolation helper that sets `TMUX_TMPDIR` to a fresh
    `TempDir`, clears `TMUX` and `TMUX_PANE`, and exposes a `dv()` command
    builder.
  - `start_generic_container()`: a testcontainers canary helper that starts
    `hello-world:latest` to validate Docker + async runner infrastructure.
- Ship `tests/e2e_example.rs` — Docker canary test.
- Ship `tests/help_command.rs` — `dv help`, `dv --help`, `dv` (no args) tests
  using `TmuxEnv`.
- All tests async with `#[tokio::test]`; no process-wide serial locks.
- `mise run test` runs all integration tests.

**Non-Goals:**

- Starting a tmux server inside a Docker container — tmux socket isolation via
  `TMUX_TMPDIR` is sufficient and avoids cross-boundary complexity (the `dv`
  host binary cannot exec into a container).
- Testing any `dv` subcommand beyond `help` in this change — this lays the
  scaffold; feature tests follow.
- CI/CD pipeline configuration — separate concern.
- Pinning the Docker image to a digest — deferred.

## Decisions

### D1: tmux isolation via TMUX_TMPDIR, not a container

**Decision**: `TmuxEnv` sets `TMUX_TMPDIR` to a `TempDir` path rather than
running tmux inside a Docker container.

**Rationale**: tmux uses a per-user socket directory (`/tmp/tmux-<uid>/` by
default). Setting `TMUX_TMPDIR` redirects the socket to an isolated path — this
is the official tmux mechanism for exactly this purpose. Since `dv` is a host
binary that shells out to the host `tmux`, running tmux inside a container would
require socket forwarding or a separate tmux socket mount, adding significant
complexity for no benefit. Each `TmuxEnv` gets its own `TempDir` so concurrent
tests never share sockets.

**Alternatives considered**:

- _alpine/git-style container with tmux_: Requires bind-mounting the tmux socket
  into the container or running `dv` inside the container — neither is practical
  for a host binary.
- _Global test mutex + shared socket_: Serialises tests and doesn't clean up
  properly on failure.

### D2: Keep testcontainers for a Docker canary only

**Decision**: testcontainers is used solely in `e2e_example.rs` to prove the
Docker + async runner plumbing works. It is not used for every test.

**Rationale**: Requiring Docker for every test would slow down the suite and
block contributors without Docker installed from running basic tests. The tmux
isolation story doesn't need a container. The canary test is a lightweight
gatekeeper: if Docker is available, it passes; if not, it fails with a clear
message.

### D3: TmuxEnv is a plain struct, not async

**Decision**: `TmuxEnv::new()` is a synchronous constructor (no `async`).

**Rationale**: Creating a `TempDir` is synchronous. No container needs to be
started or waited on. Test functions still use `#[tokio::test]` for consistency
and forward-compatibility (future tests may need async when `dv` subcommands are
async), but `TmuxEnv` itself has no async surface.

### D4: Strip TMUX and TMUX_PANE from dv invocations

**Decision**: `TmuxEnv::dv()` removes `TMUX` and `TMUX_PANE` from the child
process environment.

**Rationale**: When `mise run test` (or `cargo test`) is run from inside an
active tmux session, these vars are set. Future `dv` subcommands may branch on
whether they're inside tmux. Tests must run in a clean, non-tmux context to be
deterministic.

### D5: Canary uses hello-world image

**Decision**: `e2e_example.rs` uses `hello-world:latest`.

**Rationale**: Minimal image — no tmux, no git, no OS. Validates Docker +
testcontainers async runner in isolation from the rest of the test suite.

## Risks / Trade-offs

- **tmux not installed on host** → Tests that shell out to `tmux` will fail if
  tmux is absent. Mitigation: tmux is a declared runtime dependency of `dv`;
  document it in README. This is no different from any CLI that depends on an
  external binary.
- **Docker unavailability** → Canary test panics with a descriptive message.
  Non-Docker tests are unaffected. Mitigation: note Docker requirement in README
  and CI config.
- **TMUX_TMPDIR not honoured by all tmux builds** → Rare but possible with very
  old tmux versions. Mitigation: target tmux ≥ 2.x (universally available);
  document minimum version requirement.
- **TempDir cleanup on test panic** → Rust drops `TempDir` even on panics
  (unless the process aborts). Orphaned socket files are unlikely but possible
  if the OS kills the process mid-test. Mitigation: accepted; temp dirs under
  `/tmp` are cleaned by the OS on reboot.

## Migration Plan

1. Verify / add `testcontainers`, `tokio` (features: `macros`,
   `rt-multi-thread`, `time`), `tempfile` to `[dev-dependencies]` in
   `Cargo.toml`.
2. Create `tests/common/mod.rs` with `TmuxEnv` and `start_generic_container`.
3. Create `tests/e2e_example.rs` Docker canary.
4. Create `tests/help_command.rs` integration tests.
5. Run `mise run test` locally to confirm all tests pass.

No rollback needed — all changes are additive.

## Open Questions

- Should `TmuxEnv` also start a background `tmux` server in the isolated socket
  dir (via `tmux -L <socket> start-server`) so tests that need a running server
  don't have to do it themselves? Deferred until a subcommand requires it.
- Should the canary test be skipped (not failed) when Docker is unavailable?
  Deferred — fail-loud is safer for CI.
