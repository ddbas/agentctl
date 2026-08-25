## 1. Dependencies

- [ ] 1.1 Add `tracing = "0.1"`,
      `tracing-subscriber = { version = "0.3", features = ["env-filter"] }`, and
      `tracing-appender = "0.2"` to `Cargo.toml` (matching bonsai's pinned
      versions).

## 2. Path helper visibility

- [ ] 2.1 Make `tilde_collapse` in `src/commands/hooks.rs` visible outside its
      module (e.g. `pub(crate)`) so it can be reused by `main.rs` and the `info`
      command.

## 3. Logging module

- [ ] 3.1 Create `src/logging.rs` ported from `bonsai::logging`, renaming the
      log file prefix from `bonsai.log` to `radr.log` and the log directory
      suffix from `bonsai/logs` to `radr/logs`.
- [ ] 3.2 Implement `LogLevel` enum (`ValueEnum`: trace/debug/info/warn/error,
      default `info`) with `to_level_filter()`.
- [ ] 3.3 Implement `log_dir()` — pure resolution via
      `dirs::state_dir().or_else(dirs::data_local_dir)` joined with `radr/logs`,
      no I/O.
- [ ] 3.4 Implement `current_log_file(dir: &Path)` — computes
      `<dir>/radr.log.YYYY-MM-DD` using local date, no I/O.
- [ ] 3.5 Implement `prune_logs(log_path, retention_count)` — deletes all but
      the `retention_count` most-recently-modified `radr.log.*` files, ignoring
      non-matching files.
- [ ] 3.6 Implement `init(level: LogLevel) -> Option<WorkerGuard>` — creates the
      log directory, prunes old logs (keep 7), sets up a daily rolling
      `tracing-appender::rolling::daily` non-blocking writer, builds a
      `tracing_subscriber::fmt` layer filtered by `level`, and initializes the
      global subscriber. Returns `None` on any failure (best-effort).
- [ ] 3.7 Register `mod logging;` in `src/main.rs`.

## 4. CLI wiring

- [ ] 4.1 Add a global `--log-level <LEVEL>` flag to the root `Cli` struct
      (default `"info"`, value type `logging::LogLevel`), documented as not
      affecting stdout/stderr.
- [ ] 4.2 At the top of `run()`, call `logging::init(cli.log_level)` and hold
      the returned `WorkerGuard` in a `let _guard = ...;` binding for the
      duration of `run()`.
- [ ] 4.3 Add a `tracing::debug!` log line recording the parsed CLI invocation
      args immediately after logging initialization.

## 5. `info` subcommand

- [ ] 5.1 Add an `Info` variant to the `Commands` enum with doc comments
      describing its output (version, log level, log directory, current log
      file; no filesystem writes).
- [ ] 5.2 Implement the `Commands::Info` match arm in `run()`: read
      `CARGO_PKG_VERSION`, map `cli.log_level` to its string form, resolve
      `logging::log_dir()` (erroring with a clear message if resolution fails),
      compute `logging::current_log_file(&log_dir)`, tilde-abbreviate both paths
      via `tilde_collapse`, and print the four `key: value` lines in order
      (`version`, `log level`, `log directory`, `current log file`).

## 6. Tests

- [ ] 6.1 Port bonsai's `logging` unit tests to `src/logging.rs`, adapted for
      `radr`/`radr.log.*` naming: log dir resolution returns `Some`,
      level-to-filter string mapping for all five levels, `prune_logs` keeps
      exactly the retention count, `prune_logs` ignores non-matching files,
      `current_log_file` returns a `radr.log.`-prefixed path with a `YYYY-MM-DD`
      suffix.
- [ ] 6.2 Add a smoke test (or manual verification step) confirming `init()`
      does not panic when called in a test environment.
- [ ] 6.3 Add/verify a test or manual check for `rd info` output format: exactly
      four `key: value` lines in the specified order, log level line reflecting
      `--log-level`, and no files created as a side effect.

## 7. Verification

- [ ] 7.1 Run `mise run` build/test/lint tasks (per project convention, not
      `cargo` directly) and confirm the full suite passes.
- [ ] 7.2 Manually run `rd --log-level debug info` and confirm a `radr/logs`
      directory is created under the resolved state directory with a
      `radr.log.YYYY-MM-DD` file, and that `rd info`'s output matches the
      resolved paths.
