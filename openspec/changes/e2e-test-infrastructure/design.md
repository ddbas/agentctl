## Context

The `dv` CLI is a Rust/Cargo project with no existing integration or E2E tests.
The only tests today are unit tests in `src/`. Without a container-backed test
harness, any future integration test that calls `git` or writes to the
filesystem can corrupt the developer's home directory, be influenced by the
host's `~/.gitconfig`, commit hooks, or leaked `GIT_*` env vars — particularly
dangerous when running inside a pre-commit hook (which sets `GIT_DIR`,
`GIT_INDEX_FILE`, etc.).

The bonsai project (`~/repos/bonsai`) has already solved this with a
`testcontainers`-backed `GitEnv` helper. We adapt that pattern here, naming the
helper `DvEnv` and tailoring it to the `dv` binary.

## Goals / Non-Goals

**Goals:**

- Establish a `tests/` directory with Cargo integration test support.
- Provide a `tests/common/mod.rs` module with a `DvEnv` helper that starts an
  `alpine/git` Docker container per test, bind-mounts a host `TempDir` as
  `/workspace`, and exposes helpers to run `git` commands inside the container
  and the `dv` binary on the host.
- Ship a canary `e2e_example.rs` test that starts a `hello-world` container to
  prove the testcontainers plumbing works.
- Ship a `help_command.rs` integration test covering `dv help`, `dv --help`, and
  `dv` (no args).
- Keep all tests async with `#[tokio::test]`; no process-wide serial locks.
- Ensure `mise run test` runs all integration tests.

**Non-Goals:**

- CI/CD pipeline configuration (GitHub Actions) — that is a separate concern.
- Testing any `dv` subcommand beyond `help` — this change lays the scaffold;
  feature tests follow.
- Pinning the `alpine/git` image to a digest in this change — pinning is
  recommended for CI but deferred to avoid scope creep.

## Decisions

### D1: Use testcontainers (not bare Docker API or fake git)

**Decision**: Use the `testcontainers` crate with the `AsyncRunner` and
`GenericImage`.

**Rationale**: `testcontainers` handles container lifecycle (start, wait, exec,
stop/drop) automatically. The async runner integrates cleanly with Tokio.
Alternatives:

- _Bare `bollard`_: More control but significant boilerplate; lifecycle
  management is manual.
- _Fake git stubs_: Does not catch env-var contamination or real path issues;
  brittle.

### D2: DvEnv struct owns container and temp dirs

**Decision**: `DvEnv` holds `_container: ContainerAsync<GenericImage>`,
`_repo: TempDir`, and the resolved path `repo_path: PathBuf`. It drops all of
them automatically at end of test.

**Rationale**: RAII cleanup is idiomatic Rust. No explicit teardown call needed;
test failures still clean up. This mirrors the bonsai `GitEnv` design.

### D3: git commands run inside the container; dv runs on the host

**Decision**: `DvEnv::git()` execs commands inside the container. `DvEnv::dv()`
returns a `std::process::Command` using `env!("CARGO_BIN_EXE_dv")` and runs on
the host with the repo bind-mount path as `current_dir`.

**Rationale**: The `dv` binary is a host binary; it cannot run inside the
container without a build step. The container provides an isolated `git`
environment. The shared bind-mount makes both the container's git and the host's
`dv` see the same files. This is exactly the pattern used in bonsai.

### D4: Strip hook-injected GIT\_\* env vars from dv commands

**Decision**: `DvEnv::dv()` clears `GIT_DIR`, `GIT_INDEX_FILE`, `GIT_WORK_TREE`,
`GIT_PREFIX`, `GIT_COMMON_DIR`.

**Rationale**: `mise run test` (and therefore `cargo test`) is often invoked
from a pre-commit hook context where these vars are set by git itself. Without
clearing them, `dv` would see an incorrect `GIT_DIR` pointing at `.git` rather
than the temp repo, causing spurious failures.

### D5: Canary test uses hello-world image

**Decision**: `e2e_example.rs` uses `hello-world:latest` which exits after
printing a message, validated with `WaitFor::message_on_stdout`.

**Rationale**: Lightweight; requires no git setup. Proves Docker +
testcontainers async runner work end-to-end without coupling to the git helper.

## Risks / Trade-offs

- **Docker unavailability in CI** → Tests panic with a descriptive message. CI
  must have Docker available. Mitigation: document in README; ensure GitHub
  Actions runner has Docker.
- **macOS bind-mount path resolution** → Docker Desktop's VM maps `/var/folders`
  → `/private/var/folders`. We call `.canonicalize()` on temp dirs before
  passing to `Mount::bind_mount` to get the resolved path. Mitigation: same
  approach as bonsai; tested on macOS.
- **Root ownership on Linux CI** → Container runs as root (uid 0); files created
  in bind-mount are owned by root on the host. The host `dv` binary (uid 1001 on
  GHA) gets EACCES. Mitigation: run `chmod -R a+rwX /workspace` after git setup,
  mirroring bonsai's `fix_permissions` helper.
- **Image pull latency** → First run pulls `alpine/git:latest` and
  `hello-world:latest`. Mitigation: layer caching in CI; acceptable for local
  dev.

## Migration Plan

1. Add `testcontainers`, `tokio` (with required features), `tempfile` to
   `[dev-dependencies]` in `Cargo.toml` (some already present — verify
   versions).
2. Create `tests/common/mod.rs` with `DvEnv` and `start_generic_container`
   helpers.
3. Create `tests/e2e_example.rs` canary test.
4. Create `tests/help_command.rs` integration tests.
5. Run `mise run test` locally with Docker running to confirm all tests pass.

No rollback needed — all changes are additive (new files, dev-dependency
additions).

## Open Questions

- Should we pin `alpine/git` to a specific digest for reproducibility?
  (Recommended for CI, deferred from this change.)
- Should `mise run test` skip integration tests when Docker is unavailable (via
  `#[cfg]` or `--test-threads`)? For now, tests fail descriptively; a skip
  mechanism is deferred.
