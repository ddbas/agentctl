## 1. Workspace scaffolding

- [x] 1.1 Convert root `Cargo.toml` into a `[workspace]` manifest with
      `members = ["crates/*"]` and a `[workspace.dependencies]` table for shared
      deps (`anyhow`, `dirs`, `chrono`, `clap`, `owo-colors`, `serde`,
      `serde_json`, `rust-fsm`, `tracing`, `tracing-subscriber`,
      `tracing-appender`, `tempfile`, `testcontainers`, `tokio`).
- [x] 1.2 Create `crates/agentctl-logging`, `crates/agentctl-state`,
      `crates/agentctl-state-fs`, `crates/agentctl-tmux`,
      `crates/agentctl-harness`, `crates/agentctl-harness-pi`,
      `crates/agentctl-harness-claude`, `crates/agentctl-core`, and move
      `agentctl`'s existing bin sources into `crates/agentctl/`.
- [x] 1.3 Give each new crate a minimal `Cargo.toml` (name, edition, deps per
      the dependency graph in `design.md`) and an empty `src/lib.rs` (or
      `src/main.rs` for `agentctl`) so `cargo build --workspace` succeeds before
      any logic moves.

## 2. agentctl-logging

- [x] 2.1 Move `src/logging.rs` into `crates/agentctl-logging/src/lib.rs`,
      generalizing the hardcoded `"agentctl"` app name into a parameter on
      `init`, `log_dir`, and `current_log_file`.
- [x] 2.2 Add a `mod ops` with structured logging helper functions that other
      crates will call instead of raw `tracing` macros.
- [x] 2.3 Move `logging.rs`'s existing unit tests into this crate, updating them
      for the new `app_name` parameter.
- [x] 2.4 Curate `lib.rs` exports: `LogLevel`, `init`, `log_dir`,
      `current_log_file`, `ops::*` — nothing else `pub`.

## 3. agentctl-state

- [x] 3.1 Move `AgentState`, `AgentInfo` (and XDG path helpers used only for
      documentation/tests, not runtime I/O) from `src/state.rs`, and
      `Event`/`ToolCallType`/`apply_event`/`StatusFsm` from `src/status_fsm.rs`,
      into `crates/agentctl-state/src/`.
- [x] 3.2 Move `EventRecord` from `src/events_log.rs` into this crate.
- [x] 3.3 Define the `AgentRepository` trait (`read_agent_info`,
      `write_state_with_tools`, `delete_state`, `list_agent_infos`,
      `append_event`) derived from the existing free-function signatures in
      `state.rs`/`events_log.rs`.
- [x] 3.4 Move the FSM and `AgentState` ordering/display unit tests into this
      crate; confirm no `std::fs` calls remain here.
- [x] 3.5 Curate `lib.rs` exports per `design.md` Decision 2 and the
      `workspace-architecture` spec.

## 4. agentctl-state-fs

- [x] 4.1 Move the file-I/O bodies (atomic write, XDG resolution, JSON
      (de)serialization) from `src/state.rs` and `src/events_log.rs` into
      `crates/agentctl-state-fs/src/lib.rs`.
- [x] 4.2 Define `FileAgentRepository { base: PathBuf }` with
      `new(base:     PathBuf)` and `xdg()` constructors; implement
      `AgentRepository` for it.
- [x] 4.3 Move the corresponding file-I/O unit tests (round-trip, corrupt file,
      stale cleanup, active-tools, legacy-field-compat, event log
      append/ordering) into this crate.
- [x] 4.4 Confirm (e.g. via `cargo tree` / a workspace-wide `grep` for `std::fs`
      on agent-state paths) that no other crate performs this I/O directly, per
      the `workspace-architecture` spec.

## 5. agentctl-tmux

- [x] 5.1 Move `PaneEntry`, `tmux_panes`, `build_process_table`,
      `resolve_from_process_table`, `resolve_node_agents`, `filter_agent_panes`,
      `KNOWN_AGENTS`, `load_state_map[_from]`, and `validate_state_files[_in]`
      from `commands/list.rs` into `crates/agentctl-tmux/src/lib.rs`.
- [x] 5.2 Change `load_state_map`/`validate_state_files` to take
      `&dyn AgentRepository` instead of reading `XDG_DATA_HOME`/calling
      `agentctl-state`'s old free functions directly.
- [x] 5.3 Move the corresponding unit tests (pane filtering, node resolution,
      stale-state validation) into this crate, adapting them to construct an
      in-memory or temp-dir-backed `AgentRepository` implementation for tests
      (e.g. reusing `FileAgentRepository::new(tempdir)` from `agentctl-state-fs`
      as a dev-dependency).
- [x] 5.4 Curate `lib.rs` exports; confirm `Field`, `sort_panes`,
      `format_panes`, `colorize_state` are **not** here (they move to
      `agentctl-core`).

## 6. agentctl-harness, agentctl-harness-pi, agentctl-harness-claude

- [x] 6.1 Define the `Harness` trait (`name`, `install`, `uninstall`) and
      `InstallSummary`/`UninstallSummary` types in
      `crates/agentctl-harness/src/lib.rs`, with no I/O and no pi/Claude
      references.
- [x] 6.2 Move `PI_EXTENSION_CONTENT`, `default_pi_extension_path`,
      `install_pi[_to]`, `uninstall_pi[_to]`, and related pi-only helpers from
      `commands/hooks.rs` into `crates/agentctl-harness-pi/src/lib.rs` as a
      `PiHarness` struct implementing `Harness`.
- [x] 6.3 Move the Claude plugin manifest/hooks-json generation,
      `default_claude_plugin_dir`, `install_claude[_to]`,
      `uninstall_claude[_to]`, and `AGENTCTL_CLAUDE_HOOKS` from
      `commands/hooks.rs` into `crates/agentctl-harness-claude/src/lib.rs` as a
      `ClaudeHarness` struct implementing `Harness`.
- [x] 6.4 Move `tilde_collapse` to a small shared location both harness crates
      (and `agentctl-core`'s `info` use case) can reach — e.g. as a free
      function in `agentctl-harness` itself, since both display-summary
      generation paths need it.
- [x] 6.5 Move each harness's existing unit tests (install/uninstall
      idempotency, stale-file overwrite, hooks.json shape, summary text) into
      its respective new crate.
- [x] 6.6 Confirm neither harness crate depends on `agentctl-state` or
      `agentctl-core` (per the `workspace-architecture` spec).

## 7. agentctl-core

- [x] 7.1 Define
      `App<'a> { repo: &'a dyn AgentRepository, harnesses:     Vec<Box<dyn Harness>> }`
      with a `new` constructor in `crates/agentctl-core/src/lib.rs`.
- [x] 7.2 Move `Field`, `sort_panes`, `format_panes`, `colorize_state`,
      `cell_value`, `DEFAULT_COLUMNS[_STR]` from `commands/list.rs` into this
      crate; implement `App::list(...)` combining `agentctl-tmux` pane discovery
      with this rendering logic.
- [x] 7.3 Move the dispatch logic from `commands/agent_event.rs` into
      `App::dispatch_event(...)`, operating against `self.repo`.
- [x] 7.4 Move the dispatch logic from `commands/agent_clear.rs` into
      `App::clear(...)`.
- [x] 7.5 Move the `run_install`/`run_uninstall` dispatch from
      `commands/hooks.rs` into `App::install(...)`/`App::uninstall(...)`,
      iterating `self.harnesses` and matching by `Harness::name()` instead of a
      hardcoded enum match.
- [x] 7.6 Move `commands/info.rs` into `App::info(...)`.
- [x] 7.7 Move all corresponding unit tests (sorting, formatting/coloring,
      column selection, event dispatch state transitions, install/uninstall
      dispatch) into this crate, using a temp-dir-backed `FileAgentRepository`
      and stub `Harness` implementations as needed for tests.
- [x] 7.8 Confirm `agentctl-core`'s `Cargo.toml` has no dependency on
      `agentctl-state-fs`, `agentctl-harness-pi`, `agentctl-harness-claude`, or
      `clap` (per the `workspace-architecture` spec).

## 8. agentctl (binary)

- [x] 8.1 Reduce `crates/agentctl/src/main.rs` to: `Cli`/`Commands`/
      `AgentSubcommand` clap definitions, CLI-local mirror enums (`HarnessArg`,
      `FieldArg`, `LogLevelArg`, `ToolCallTypeArg`) with `Into<>` conversions to
      `agentctl-core`/domain types, the composition root (constructing
      `FileAgentRepository`, `PiHarness`, `ClaudeHarness`, calling
      `agentctl_logging::init`, building `agentctl_core::App`), and one
      `app.*()` call per subcommand that prints/propagates the result.
- [x] 8.2 Remove all moved business logic from `main.rs`/`commands/*.rs`; delete
      the now-empty `commands/` module tree.
- [x] 8.3 Keep `tests/*.rs` (help, hooks, list_sessions, agent_commands,
      e2e_example) and `tests/common/mod.rs` in `crates/agentctl/`, updating
      only what's needed to compile against the new crate layout (test
      behavior/assertions should not change).

## 9. Validation

- [x] 9.1 Run `cargo build --workspace` and `cargo build -p <crate>` for each
      library crate individually; confirm all succeed.
- [x] 9.2 Run the full test suite (`mise run <test task>`) and confirm every
      previously-passing test still passes with no assertion changes.
- [x] 9.3 Manually smoke-test every subcommand (`list`, `install`, `uninstall`,
      `agent event ...`, `agent clear`, `info`, `help`) against a real tmux
      session to confirm zero behavior change.
- [x] 9.4 Verify (manually or via a `cargo tree`/`grep` check) each requirement
      in `specs/workspace-architecture/spec.md`: sole filesystem access via
      `agentctl-state-fs`, no I/O in `agentctl-harness`, `agentctl-core`'s
      dependency restrictions, single `init()` call site, and the binary's thin
      composition-root shape.
- [x] 9.5 Update `AGENTS.md`/README references (if any) that describe the old
      single-crate layout to reflect the new workspace structure.
