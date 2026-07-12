## 1. Dependencies

- [ ] 1.1 Verify `testcontainers = "0.23"`, `tokio` (with `macros`,
      `rt-multi-thread`, `time` features), and `tempfile = "3"` are present in
      `[dev-dependencies]` in `Cargo.toml`; add or update any that are missing

## 2. Shared Test Helper (`tests/common/mod.rs`)

- [ ] 2.1 Create `tests/common/mod.rs` with a module-level doc comment
      explaining the tmux isolation requirement and usage pattern
- [ ] 2.2 Implement `start_generic_container()` — starts `hello-world:latest`,
      waits for `WaitFor::message_on_stdout("Hello from Docker!")`, panics with
      a descriptive message if Docker is unavailable
- [ ] 2.3 Define `TmuxEnv` struct holding `_tmpdir: TempDir` and
      `tmux_tmpdir: PathBuf`
- [ ] 2.4 Implement `TmuxEnv::new()` — creates a `TempDir`, canonicalises its
      path into `tmux_tmpdir`
- [ ] 2.5 Implement `TmuxEnv::dv()` — returns
      `Command::new(env!("CARGO_BIN_EXE_dv"))` with `TMUX_TMPDIR` set to
      `self.tmux_tmpdir` and `TMUX`/`TMUX_PANE` removed from the environment

## 3. Canary E2E Test (`tests/e2e_example.rs`)

- [ ] 3.1 Create `tests/e2e_example.rs` with `mod common;`
- [ ] 3.2 Write `#[tokio::test] async fn test_container_starts_and_stops()` —
      calls `common::start_generic_container()`, asserts `container.id()` is
      non-empty, then drops the container

## 4. Help Command Integration Tests (`tests/help_command.rs`)

- [ ] 4.1 Create `tests/help_command.rs` with `mod common;`
- [ ] 4.2 Write `#[tokio::test] async fn help_subcommand_exits_zero()` — creates
      `TmuxEnv::new()`, calls `env.dv().arg("help").status()`, asserts success
- [ ] 4.3 Write `#[tokio::test] async fn help_subcommand_prints_program_name()`
      — assert stdout contains `"dv"`
- [ ] 4.4 Write `#[tokio::test] async fn help_subcommand_prints_about_text()` —
      assert stdout contains `"drovr"`
- [ ] 4.5 Write `#[tokio::test] async fn flag_help_exits_zero()` — `dv --help`
      exits 0
- [ ] 4.6 Write `#[tokio::test] async fn flag_help_prints_usage()` — stdout
      contains `"usage"` (case-insensitive)
- [ ] 4.7 Write `#[tokio::test] async fn no_args_exits_zero()` — `dv` with no
      args exits 0

## 5. Validation

- [ ] 5.1 Run `mise run build` and confirm the `dv` binary compiles without
      errors
- [ ] 5.2 Run `cargo test --lib` and confirm only unit tests run (no integration
      tests)
- [ ] 5.3 Run `mise run test` with Docker running and confirm all tests pass
- [ ] 5.4 Confirm no orphaned Docker containers remain after the run
      (`docker ps -a | grep hello-world`)
- [ ] 5.5 Confirm no tmux socket files remain under any test temp dirs after the
      run
