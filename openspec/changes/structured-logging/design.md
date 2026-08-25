## Context

`radr` (binary `rd`) has no logging today. `bonsai` (binary `bs`), built with
the same toolchain and conventions, already ships a `logging` module
(`src/logging.rs`) that is small, unit-tested, and battle-tested in production
use: it resolves a platform log directory, rolls log files daily, prunes old
files, and exposes a `LogLevel` CLI enum wired to a global `--log-level` flag.
`bonsai` also has an `Info` subcommand that prints
version/log-level/log-dir/current-log-file/managed-root as `key: value` lines
using a `tilde_path` helper.

`radr` already has an equivalent tilde-abbreviation helper (`tilde_collapse` in
`src/commands/hooks.rs`), and a `main.rs` structured similarly to `bonsai`'s
(clap `Parser`/`Subcommand`, a `run() -> Result<()>` entry point,
`Cli::command().print_long_help()` for `Help`). This design ports
`bonsai::logging` into `radr` with minimal, mechanical changes (crate name
only), and adds a new `rd info` subcommand modeled on `bonsai`'s `Info` command
but scoped to radr's actual state (no `managed_root` concept in radr, so that
line is omitted).

## Goals / Non-Goals

**Goals:**

- Port `bonsai::logging` to `radr` as close to verbatim as possible: same public
  API shape (`LogLevel`, `log_dir()`, `current_log_file()`, `init()`), same
  rotation/retention behavior (daily rolling, 7-file retention), same
  non-blocking `tracing-appender` writer.
- Same CLI ergonomics: global `--log-level <LEVEL>` flag (default `info`),
  `ValueEnum`-derived, documented identically ("Does not affect stdout/stderr
  output").
- Add `rd info` mirroring `bs info`'s output shape and semantics, adapted to
  radr's domain (version, log level, log directory, current log file — no
  `managed_root` line, since radr has no equivalent concept).
- Preserve bonsai's test coverage shape: unit tests for log dir resolution,
  level-to-filter mapping, pruning behavior, and current-log-file naming.

**Non-Goals:**

- No change to radr's existing subcommands' behavior or output.
- No structured/JSON log format — plain `tracing_subscriber::fmt` layer, same as
  bonsai.
- No log shipping, remote aggregation, or log level reconfiguration at runtime
  (no `RUST_LOG` env-filter wiring beyond what bonsai already does — bonsai
  depends on `tracing-subscriber`'s `env-filter` feature but does not actually
  use `EnvFilter` in `logging.rs`; we replicate that as-is rather than "fixing"
  it, since exact parity is the explicit goal).
- No cross-invocation log level persistence — the flag is per-invocation only,
  same as bonsai.

## Decisions

1. **Copy `bonsai::logging` near-verbatim into `src/logging.rs`, renaming only
   the file-name prefix.** Log files become `radr.log.YYYY-MM-DD` (was
   `bonsai.log.YYYY-MM-DD`) and the log directory becomes
   `<state-dir>/radr/logs` (was `<state-dir>/bonsai/logs`), so the two tools'
   logs never collide on disk even though they share the same state-dir base
   resolution logic (`dirs::state_dir().or_else(dirs::data_local_dir)`).
   _Alternative considered_: extracting a shared `radr-logging`/`bonsai-logging`
   crate. Rejected — the two projects are not currently set up as a Cargo
   workspace sharing crates, and the proposal explicitly asks for the exact same
   module ported in-place, not a shared dependency, to keep this change
   self-contained and low-risk.

2. **Reuse radr's existing `tilde_collapse` (in `commands/hooks.rs`) for the
   `info` command's path formatting instead of introducing a `tilde_path`
   duplicate.** Behaviorally equivalent to bonsai's `tilde_path` (replace
   home-dir prefix with `~`). Avoids duplicating logic already present in the
   codebase; `tilde_collapse` will need to be exposed (`pub(crate)` or `pub`)
   from `commands::hooks` (or moved to a shared location) if it isn't already
   visible to `main.rs`. _Alternative considered_: copy bonsai's `tilde_path`
   into `logging.rs` or a new `paths` module. Rejected for now to avoid two
   near-identical helpers in the same binary; revisit if `tilde_collapse`'s
   current home in `commands::hooks` proves awkward to import from `main.rs`.

3. **`rd info` omits the `managed_root` line bonsai's `Info` prints.** `radr`
   has no equivalent "managed root" concept (that's a `bonsai`-only
   worktree-pool idea). The `rd info` output is: `version`, `log level`,
   `log directory`, `current log file` — a strict subset of bonsai's fields, in
   the same order and `key: value` format.

4. **Wire `logging::init(cli.log_level)` at the very top of `run()`, holding the
   `WorkerGuard` for the process lifetime, exactly as bonsai does.** Add a
   `tracing::debug!` line logging the parsed CLI invocation args, matching
   bonsai's behavior, to get baseline diagnostic value immediately.

5. **New dependencies match bonsai's pinned versions exactly**
   (`tracing = "0.1"`,
   `tracing-subscriber = { version = "0.3", features = ["env-filter"] }`,
   `tracing-appender = "0.2"`) to minimize the chance of divergent behavior or
   duplicate transitive dependency versions if the two crates are ever built in
   the same workspace/cache.

## Risks / Trade-offs

- [Risk] Two independent copies of near-identical logic (`bonsai::logging` and
  `radr::logging`) will drift over time as each project evolves independently. →
  Mitigation: none needed for this change — explicit scope is exact replication
  now; a future change can extract a shared crate if drift becomes a real
  maintenance burden.
- [Risk] `tilde_collapse` in `commands/hooks.rs` may not currently be `pub` and
  importable from `main.rs`. → Mitigation: tasks include making it accessible
  (visibility change only, no behavior change) as a prerequisite step.
- [Risk] Best-effort logging (`init()` returns `None` silently on failure, per
  bonsai's design) means log directory permission issues fail silently with no
  logs and no error surfaced to the user. → Mitigation: this is intentional
  parity with bonsai's documented behavior; `rd info` still reports the resolved
  (possibly non-existent/non-writable) log directory path so a user can diagnose
  manually.
- [Risk] Adding `tracing`/`tracing-subscriber`/`tracing-appender` increases
  `radr`'s dependency tree and binary size. → Mitigation: acceptable trade-off
  given the explicit ask for parity with bonsai, which already carries this
  cost.

## Migration Plan

Purely additive — no existing state, config, or CLI behavior changes for current
users. New log directory and files are created on first run when logging is
initialized; no rollback steps beyond reverting the change are needed (deleting
`<state-dir>/radr/logs` if desired is optional cleanup).

## Open Questions

- None — scope is a direct, faithful port of an existing, working
  implementation.
