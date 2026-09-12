## Why

`agentctl` currently has no logging at all — failures and internal behavior
(hook dispatch, FSM transitions, state file writes) are invisible once something
goes wrong in the field, and there's no way to inspect what `agentctl` did
without adding ad-hoc `eprintln!` calls. `bonsai` (the sibling project sharing
this workspace/tooling conventions) already solved this with a small,
well-tested `logging` module built on `tracing` + `tracing-appender`, a
`--log-level` global CLI flag, and an `info` subcommand that surfaces runtime
paths and metadata. Replicating that exact design in `agentctl` gives us proven,
tested logging infrastructure for free, keeps the two CLIs consistent for anyone
who uses both, and unblocks future diagnostic work (e.g. debugging hook
integration issues) without reinventing the approach.

## What Changes

- Add a `logging` module to `agentctl`, ported near-verbatim from
  `bonsai::logging`: `LogLevel` (trace/debug/info/warn/error) CLI enum,
  `log_dir()`, `current_log_file()`, `prune_logs()` (7-file retention), and
  `init()` wiring a daily-rolling, non-blocking `tracing-subscriber` file
  appender.
- Same output path convention: platform log directory (`dirs::state_dir()` →
  `dirs::data_local_dir()` fallback) joined with `agentctl/logs`, and log files
  named `agentctl.log.YYYY-MM-DD` (renamed from `bonsai.log.*` to match the
  `agentctl` binary/project name — this is the only intentional deviation from
  bonsai's implementation).
- Same CLI interface: a global `--log-level <LEVEL>` flag on the root `Cli`
  (default `info`), parsed via `clap`'s `ValueEnum`, affecting only the log
  file, not stdout/stderr.
- Wire `logging::init()` into `main.rs`'s `run()`, holding the returned
  `WorkerGuard` for the process lifetime, and add `tracing::debug!` logging of
  the parsed CLI invocation (mirroring bonsai).
- Add a new `agentctl info` subcommand that prints, as `key: value` lines:
  version (from `CARGO_PKG_VERSION`), effective log level, log directory, and
  current log file path (all paths tilde-abbreviated using agentctl's existing
  `tilde_collapse` helper).
- Add new Cargo dependencies to match bonsai: `tracing`, `tracing-subscriber`
  (with `env-filter`), `tracing-appender`.

## Capabilities

### New Capabilities

- `structured-logging`: File-based logging subsystem for `agentctl` — log
  directory/file resolution, daily rotation with retention pruning, CLI
  log-level control, and subscriber initialization.
- `info-command`: `agentctl info` subcommand reporting version, log level, log
  directory, and current log file.

### Modified Capabilities

(none — no existing capability's requirements change; this only adds new surface
area)

## Impact

- **Affected code**: `src/main.rs` (new `--log-level` global flag, new `Info`
  subcommand, `logging::init()` wiring), new `src/logging.rs` module,
  `Cargo.toml` (new dependencies).
- **Dependencies**: adds `tracing`, `tracing-subscriber`, `tracing-appender`
  (all already used and pinned in `bonsai`'s `Cargo.toml`, so versions are
  known-good).
- **Filesystem**: introduces a new on-disk log directory
  (`<state-dir>/agentctl/logs/`) with daily-rotated files and 7-file retention;
  no existing files or directories are touched.
- **CLI surface**: adds a global `--log-level` flag (non-breaking, default
  preserves current no-logging-visible-to-user behavior) and a new `info`
  subcommand (additive, no conflicts with existing subcommands).
- **Tests**: existing `commands/*` and `state.rs` tests are unaffected; new unit
  tests are added for the `logging` module mirroring bonsai's test suite (log
  dir resolution, level filter, pruning, current log file naming).
