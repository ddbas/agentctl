## Context

The `agentctl` CLI manages tmux sessions. Currently it only has a `help`
command, but future subcommands will shell out to `tmux` to create, list, attach
to, and kill sessions. Without test isolation, tests that invoke `tmux` will
interact with the developer's live sessions, be affected by `TMUX`/`TMUX_PANE`
env vars set when running inside an existing session, and leave orphaned
sessions on the host.

The bonsai project (`~/repos/bonsai`) solves an analogous problem for git using
a `GitEnv` backed by an `alpine/git` container. We apply the same pattern here:
a `TmuxEnv` backed by a container that has tmux installed and runs an isolated
tmux server. The host `agentctl` binary communicates with the container's tmux
server via a Unix socket on a bind-mounted `TempDir`.

## Goals / Non-Goals

**Goals:**

- Establish a `tests/` directory with Cargo integration test support.
- Provide a `tests/common/mod.rs` module with:
  - `TmuxEnv`: starts a Docker container with tmux, starts an isolated tmux
    server inside the container on a socket within a bind-mounted `TempDir`, and
    exposes a `agentctl()` command builder pre-configured with the socket path
    and with `TMUX`/`TMUX_PANE` cleared.
  - `start_generic_container()`: canary helper that starts `hello-world:latest`.
- Ship `tests/e2e_example.rs` — Docker canary test.
- Ship `tests/help_command.rs` — integration tests for `agentctl help`,
  `agentctl --help`, and `agentctl` (no args).
- All tests async with `#[tokio::test]`; no process-wide serial locks.
- `mise run test` runs all integration tests.

**Non-Goals:**

- Implementing `AGENTCTL_TMUX_SOCKET` support in `agentctl` itself in this
  change — the env var is wired into the `agentctl()` command builder now;
  `agentctl` will read it when the first tmux-calling subcommand is added.
- CI/CD pipeline configuration — separate concern.
- Pinning the container image to a digest — deferred.

## Decisions

### D1: Run tmux inside a Docker container, share socket via bind-mount

**Decision**: `TmuxEnv` starts an `alpine`-based container with `tmux`
installed, starts `tmux -S /workspace/tmux.sock start-server -d` inside it, and
bind-mounts the host `TempDir` to `/workspace`. The socket is accessible on the
host at `workspace_path/tmux.sock`.

**Rationale**: This mirrors the bonsai `GitEnv` pattern exactly — the container
provides a clean, version-pinned environment; the bind-mount bridges host and
container. Every test gets a completely independent tmux server with no shared
state. When `agentctl` gains tmux subcommands, pointing them at this socket is
the single change needed to make all existing tests exercise real tmux
behaviour.

**Alternatives considered**:

- _`TMUX_TMPDIR` isolation on host (no container)_: Simpler and avoids Docker
  but uses the host's tmux installation. Different developers may have different
  tmux versions; CI may have none. A container gives a consistent, pinned
  runtime.
- _Shared tmux server with session namespacing_: Fragile — session names could
  still collide, and cleanup after a panicking test is unreliable.

### D2: Container image — alpine with tmux installed at startup

**Decision**: Use `alpine:latest` as the base image. After the container starts
(entrypoint: `sh -c "tail -f /dev/null"`), exec `apk add --no-cache tmux` once
during `TmuxEnv::new()`, then start the tmux server.

**Rationale**: `alpine` is tiny and pulls fast. There is no widely-used official
`alpine+tmux` image worth pinning to, so installing at test startup is
acceptable. The install is fast (~2s) and is cached by Docker layer caching in
CI after the first pull.

**Alternatives considered**:

- _Custom Docker image with tmux pre-installed_: Cleaner at runtime but requires
  maintaining a `Dockerfile` and a registry. Deferred.
- _`ubuntu` base_: Much larger image; slower pull. No benefit for a single
  package.

### D3: Socket path — explicit `-S /workspace/tmux.sock`

**Decision**: Both the container tmux server and the host `agentctl()`
invocations use an explicit `-S /workspace/tmux.sock` socket path rather than
relying on `TMUX_TMPDIR`.

**Rationale**: `TMUX_TMPDIR` creates sockets under `TMPDIR/tmux-<uid>/`, where
`<uid>` is the process UID. The container runs as root (uid 0); the host user
has a different UID. Using `-S` with an absolute path sidesteps the UID mismatch
entirely and is unambiguous.

### D4: AGENTCTL_TMUX_SOCKET env var as the test injection point

**Decision**: `TmuxEnv::agentctl()` sets `AGENTCTL_TMUX_SOCKET` to the host path
of the container's socket. `agentctl` will read this env var and pass
`-S <path>` to every tmux invocation when it is set.

**Rationale**: This is a clean, zero-magic injection point. Tests set the env
var; production runs without it use the default tmux socket. Implementing
`AGENTCTL_TMUX_SOCKET` support is deferred to the first subcommand that calls
tmux; the infrastructure is wired now.

### D5: Help command tests use TmuxEnv despite not needing tmux yet

**Decision**: `help_command.rs` tests create a `TmuxEnv` even though
`agentctl help` doesn't call tmux.

**Rationale**: Establishes the pattern all future tests will follow. Ensures the
helper compiles and works end-to-end. Overhead is minimal — the container starts
in ~2s and runs in parallel with other tests.

## Risks / Trade-offs

- **Docker unavailability** → All tests that use `TmuxEnv` fail with a
  descriptive panic. Mitigation: document Docker as a test dependency in the
  README.
- **Unix socket bind-mounts on macOS Docker Desktop** → Newer Docker Desktop
  versions support Unix socket bind-mounts. If a version is encountered that
  does not, `TMUX_TMPDIR` isolation is a straightforward fallback. Mitigation:
  note minimum Docker Desktop version requirement once confirmed during
  implementation.
- **apk install latency** → ~2s per test container on first run; subsequent runs
  benefit from Docker layer caching. Mitigation: acceptable; a custom pre-built
  image can replace this later if it becomes a bottleneck.
- **Root UID in container vs. host UID** → Handled by D3 (explicit `-S` socket
  path). No `chmod` dance needed since socket ownership is irrelevant when using
  `-S` directly.
- **tmux server not yet used by agentctl** → `TmuxEnv` starts a server that
  `agentctl help` never touches. This is intentional scaffolding; the server is
  ready for the first real tmux subcommand.

## Migration Plan

1. Verify / add `testcontainers`, `tokio` (features: `macros`,
   `rt-multi-thread`, `time`), `tempfile` to `[dev-dependencies]` in
   `Cargo.toml`.
2. Create `tests/common/mod.rs` with `TmuxEnv` and `start_generic_container`.
3. Create `tests/e2e_example.rs` Docker canary.
4. Create `tests/help_command.rs` integration tests.
5. Run `mise run test` locally with Docker running to confirm all tests pass.

No rollback needed — all changes are additive.

## Open Questions

- Should we build and publish a custom `agentctl-test:latest` image with tmux
  pre-installed to eliminate the `apk add` step? Deferred — do it if `apk add`
  latency becomes a problem.
- Should `AGENTCTL_TMUX_SOCKET` be the env var name, or something like
  `TMUX_SOCKET_PATH`? Decided at implementation time when the first tmux
  subcommand is added.
