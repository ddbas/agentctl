## 1. Dependencies

- [ ] 1.1 Verify `testcontainers = "0.23"`, `tokio` (with `macros`,
      `rt-multi-thread`, `time` features), and `tempfile = "3"` are present in
      `[dev-dependencies]` in `Cargo.toml`; add or update any that are missing

## 2. Shared Test Helper (`tests/common/mod.rs`)

- [ ] 2.1 Create `tests/common/mod.rs` with module-level doc comment explaining
      the isolation requirement and usage pattern
- [ ] 2.2 Implement `start_generic_container()` — starts `hello-world:latest`,
      waits for `WaitFor::message_on_stdout("Hello from Docker!")`, panics with
      a descriptive message if Docker is unavailable
- [ ] 2.3 Define `DvEnv` struct holding
      `_container: ContainerAsync<GenericImage>`, `_repo: TempDir`, and
      `repo_path: PathBuf`
- [ ] 2.4 Implement `DvEnv::new()` — creates a canonicalized `TempDir`, starts
      an `alpine/git:latest` container with `sh -c "tail -f /dev/null"`
      entrypoint, bind-mounts the temp dir to `/workspace`
- [ ] 2.5 Implement `DvEnv::git()` — execs `git -C /workspace <args>` inside the
      container using `CmdWaitFor::exit_code(0)`, captures stderr, and panics
      with full context on non-zero exit
- [ ] 2.6 In `DvEnv::new()`, run git initialisation via container exec:
      `safe.directory`, `init`, `user.email`, `user.name`,
      `commit.gpgsign false`, write `README.md` from host, `git add .`,
      `git commit -m "init"`, then `chmod -R a+rwX /workspace`
- [ ] 2.7 Implement `DvEnv::dv()` — returns
      `Command::new(env!("CARGO_BIN_EXE_dv"))` with
      `current_dir(&self.repo_path)` and all hook-injected git env vars removed
      (`GIT_DIR`, `GIT_INDEX_FILE`, `GIT_WORK_TREE`, `GIT_PREFIX`,
      `GIT_COMMON_DIR`)
- [ ] 2.8 Add `fix_permissions()` helper on `DvEnv` that runs
      `chmod -R a+rwX /workspace` via container exec (called after init and
      after each commit)

## 3. Canary E2E Test (`tests/e2e_example.rs`)

- [ ] 3.1 Create `tests/e2e_example.rs` with `mod common;`
- [ ] 3.2 Write `#[tokio::test] async fn test_container_starts_and_stops()` that
      calls `common::start_generic_container()`, asserts `container.id()` is
      non-empty, then drops the container

## 4. Help Command Integration Tests (`tests/help_command.rs`)

- [ ] 4.1 Create `tests/help_command.rs` with `mod common;`
- [ ] 4.2 Write `#[tokio::test] async fn help_subcommand_exits_zero()` —
      `DvEnv::new().await`, `env.dv().arg("help").status()`, assert success
- [ ] 4.3 Write `#[tokio::test] async fn help_subcommand_prints_program_name()`
      — assert stdout contains `"dv"`
- [ ] 4.4 Write `#[tokio::test] async fn help_subcommand_prints_about_text()` —
      assert stdout contains `"drovr"` (matches the about string in
      `src/main.rs`)
- [ ] 4.5 Write `#[tokio::test] async fn flag_help_exits_zero()` — `dv --help`
      exits 0
- [ ] 4.6 Write `#[tokio::test] async fn flag_help_prints_usage()` — stdout
      contains `"Usage"` (case-insensitive)
- [ ] 4.7 Write `#[tokio::test] async fn no_args_exits_zero()` — `dv` with no
      args exits 0

## 5. Validation

- [ ] 5.1 Run `mise run build` and confirm the `dv` binary compiles without
      errors
- [ ] 5.2 Run `cargo test --lib` and confirm only unit tests run (no integration
      tests)
- [ ] 5.3 Run `mise run test` with Docker running and confirm all tests pass
      (canary + help integration tests)
- [ ] 5.4 Confirm no orphaned Docker containers remain after the test run
      (`docker ps -a | grep -E 'hello-world|alpine'`)
