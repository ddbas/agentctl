## 1. Dependencies

- [x] 1.1 Verify `testcontainers = "0.23"`, `tokio` (with `macros`,
      `rt-multi-thread`, `time` features), and `tempfile = "3"` are present in
      `[dev-dependencies]` in `Cargo.toml`; add or update any that are missing

## 2. Shared Test Helper (`tests/common/mod.rs`)

- [x] 2.1 Create `tests/common/mod.rs` with a module-level doc comment
      explaining the tmux container isolation requirement and usage pattern
- [x] 2.2 Implement `start_generic_container()` — starts `hello-world:latest`,
      waits for `WaitFor::message_on_stdout("Hello from Docker!")`, panics with
      a descriptive message if Docker is unavailable
- [x] 2.3 Define `TmuxEnv` struct holding
      `_container: ContainerAsync<GenericImage>`, `_workspace: TempDir`,
      `workspace_path: PathBuf`, and `socket_path: PathBuf`
- [x] 2.4 Implement `TmuxEnv::new()` — creates a canonicalized `TempDir`, starts
      an `alpine:latest` container with entrypoint `sh -c "tail -f /dev/null"`
      and the workspace bind-mounted to `/workspace`
- [x] 2.5 In `TmuxEnv::new()`, exec `apk add --no-cache tmux` inside the
      container using `CmdWaitFor::exit_code(0)`, capturing stderr for panic
      messages on failure
- [x] 2.6 In `TmuxEnv::new()`, exec
      `tmux -S /workspace/tmux.sock start-server -d` inside the container to
      start the isolated tmux server; set `socket_path` to
      `workspace_path.join("tmux.sock")`
- [x] 2.7 Implement `TmuxEnv::agentctl()` — returns
      `Command::new(env!("CARGO_BIN_EXE_agentctl"))` with `TMUX` and `TMUX_PANE`
      removed, and `AGENTCTL_TMUX_SOCKET` set to `self.socket_path`
- [x] 2.8 Implement a `TmuxEnv::exec()` helper that runs arbitrary commands
      inside the container via `ExecCommand` with `CmdWaitFor::exit_code(0)`,
      for use in future tests that need to inspect tmux state

## 3. Canary E2E Test (`tests/e2e_example.rs`)

- [x] 3.1 Create `tests/e2e_example.rs` with `mod common;`
- [x] 3.2 Write `#[tokio::test] async fn test_container_starts_and_stops()` —
      calls `common::start_generic_container()`, asserts `container.id()` is
      non-empty, then drops the container

## 4. Help Command Integration Tests (`tests/help_command.rs`)

- [x] 4.1 Create `tests/help_command.rs` with `mod common;`
- [x] 4.2 Write `#[tokio::test] async fn help_subcommand_exits_zero()` —
      `TmuxEnv::new().await`, `env.agentctl().arg("help").status()`, assert
      success
- [x] 4.3 Write `#[tokio::test] async fn help_subcommand_prints_program_name()`
      — assert stdout contains `"agentctl"`
- [x] 4.4 Write `#[tokio::test] async fn help_subcommand_prints_about_text()` —
      assert stdout contains `"agentctl"`
- [x] 4.5 Write `#[tokio::test] async fn flag_help_exits_zero()` —
      `agentctl --help` exits 0
- [x] 4.6 Write `#[tokio::test] async fn flag_help_prints_usage()` — stdout
      contains `"usage"` (case-insensitive)
- [x] 4.7 Write `#[tokio::test] async fn no_args_exits_zero()` — `agentctl` with
      no args exits 0

## 5. Validation

- [x] 5.1 Run `mise run build` and confirm the `agentctl` binary compiles
      without errors
- [x] 5.2 Run `cargo test --lib` and confirm only unit tests run (no integration
      tests)
- [x] 5.3 Run `mise run test` with Docker running and confirm all tests pass
- [x] 5.4 Confirm no orphaned Docker containers remain after the run
      (`docker ps -a | grep -E 'alpine|hello-world'`)
- [x] 5.5 Confirm the tmux socket file is removed from the host after each test
      (`ls /tmp` and the test TempDir paths)
