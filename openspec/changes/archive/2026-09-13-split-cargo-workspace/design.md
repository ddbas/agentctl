## Context

`agentctl` (~6.9k lines) currently lives in a single crate: `src/main.rs` (clap
CLI

- dispatch), `src/state.rs` (agent status types + file-backed state store),
  `src/status_fsm.rs` (event-driven status FSM), `src/events_log.rs`
  (append-only `events.jsonl`), `src/logging.rs` (rolling file logging), and
  `src/commands/{list,hooks,agent_event,agent_clear,info}.rs`.

The maintainer does 100% agent-driven development and wants to govern the
project by reading crate public APIs, never source diffs. They also want some
libraries reusable by a possible future non-CLI binary (e.g. a web server),
while starting coarse and splitting further only when a concrete need arises.

This design was produced interactively (via a reviewed visual artifact) and
iterated through several rounds of feedback before converging on the shape
below.

## Goals / Non-Goals

**Goals:**

- Convert `agentctl` into a Cargo workspace with narrow, independently
  reviewable crate boundaries.
- Push all current business logic out of the binary into a single
  `agentctl-core` application-layer crate that depends only on trait interfaces,
  never on concrete implementations.
- Centralize all filesystem access for agent-state persistence (`state.json` /
  `events.jsonl`) behind one crate implementing a shared trait, so there is
  exactly one place to audit that specific class of disk I/O.
- Make harness integrations (pi, Claude Code) independently extensible: a trait
  crate defining the contract, plus one crate per harness implementation, so
  adding a third harness never touches the first two.
- Make the logging setup reusable and give every crate a consistent structured
  logging interface, while only the binary configures the actual sink.
- Preserve 100% of current CLI behavior — this is a structural refactor only.

**Non-Goals:**

- No new user-facing CLI capabilities, flags, or output changes.
- No second binary (e.g. `agentctl-web`) in this change — the design leaves room
  for one, but building it is out of scope.
- No CI wiring for `cargo-semver-checks`/`cargo public-api` in this change
  (tracked as a follow-up; see Open Questions).
- Not attempting one universal filesystem chokepoint for _all_ disk I/O in the
  project (see Decision 5) — only agent-state persistence is centralized this
  way.

## Decisions

### Decision 1: Nine crates — one binary, one application layer, seven domain/infra libraries

```
agentctl/                       (workspace root)
├── Cargo.toml                  [workspace] members + [workspace.dependencies]
└── crates/
    ├── agentctl/               bin — clap parsing + composition root only
    ├── agentctl-core/          lib — ALL business logic (List/Install/AgentEvent/Clear/Info)
    ├── agentctl-state/         lib — domain types, FSM, AgentRepository trait
    ├── agentctl-state-fs/      lib — FileAgentRepository, sole fs access for agent state
    ├── agentctl-tmux/          lib — pane discovery (depends on agentctl-state)
    ├── agentctl-harness/       lib — Harness trait (no I/O)
    ├── agentctl-harness-pi/    lib — PiHarness
    ├── agentctl-harness-claude/ lib — ClaudeHarness
    └── agentctl-logging/       lib — shared logging interface
```

Dependency graph (top to bottom = depends on):

- `agentctl` (bin) → `agentctl-core`, and directly constructs
  `agentctl-state-fs::FileAgentRepository`, `agentctl-harness-pi::PiHarness`,
  `agentctl-harness-claude::ClaudeHarness`, plus calls
  `agentctl-logging::init(...)`.
- `agentctl-core` → `agentctl-state` (types + `AgentRepository` trait),
  `agentctl-tmux`, `agentctl-harness` (trait only), `agentctl-logging`.
- `agentctl-state-fs` → `agentctl-state` (implements `AgentRepository`),
  `agentctl-logging`.
- `agentctl-tmux` → `agentctl-state`, `agentctl-logging`.
- `agentctl-harness-pi` / `agentctl-harness-claude` → `agentctl-harness`
  (implement `Harness`), `agentctl-logging`.
- `agentctl-logging` → no workspace dependencies.

This is deliberately more granular than a first "one thin CLI + a handful of
libraries" pass, because it optimizes for two axes the maintainer explicitly
called out: a single filesystem chokepoint for agent-state, and independent
per-harness extensibility. Every other axis (tmux discovery, logging) stays at
the coarser granularity from the first draft.

**Alternatives considered:**

- _Five crates (bin + agent-state + agent-tmux + agent-hooks + app-logging)_ —
  the initial proposal. Rejected because it left business logic in the bin (not
  reusable by a future non-CLI adapter) and mixed filesystem-writing harness
  code with a single generic `Harness` enum, making per-harness extension
  require editing a shared file.
- _One `agentctl-fs` crate as the sole filesystem access point for the entire
  project_ (state **and** harness installs) — considered and explicitly
  deferred; see Decision 5.

### Decision 2: Dependency inversion via `AgentRepository` and `Harness` traits

`agentctl-state` defines:

```rust
trait AgentRepository {
    fn read_agent_info(&self, pane_id: &str) -> Option<AgentInfo>;
    fn write_state_with_tools(&self, pane_id: &str, harness: &str, status: AgentState, active_tools: u32) -> Result<()>;
    fn delete_state(&self, pane_id: &str) -> Result<()>;
    fn list_agent_infos(&self) -> Vec<AgentInfo>;
    fn append_event(&self, pane_id: &str, record: &EventRecord) -> Result<()>;
}
```

`agentctl-state-fs` exports `FileAgentRepository` (a plain struct with
`new(base: PathBuf)` and `xdg()` constructors) implementing this trait — **not**
a lazy-initialized singleton. `agentctl-harness` defines:

```rust
trait Harness {
    fn name(&self) -> &'static str;
    fn install(&self) -> Result<InstallSummary>;
    fn uninstall(&self) -> Result<UninstallSummary>;
}
```

`agentctl-harness-pi::PiHarness` and `agentctl-harness-claude::ClaudeHarness`
implement it. `agentctl-core::App` is generic/dynamic over both traits:

```rust
struct App<'a> {
    repo: &'a dyn AgentRepository,
    harnesses: Vec<Box<dyn Harness>>,
}
```

The binary is the **composition root** — the only place that chooses concrete
implementations:

```rust
let repo = FileAgentRepository::xdg();
let harnesses: Vec<Box<dyn Harness>> = vec![Box::new(PiHarness), Box::new(ClaudeHarness)];
let _guard = agentctl_logging::init("agentctl", cli.log_level.into());
let app = agentctl_core::App::new(&repo, harnesses);
```

Every `clap`-derived enum in the bin (`HarnessArg`, `FieldArg`, `LogLevelArg`,
`ToolCallTypeArg`) converts `Into<>` the corresponding plain domain/core type —
the same pattern the codebase already uses today for
`ToolCallTypeArg → status_fsm::ToolCallType`. No library crate ever depends on
`clap`.

**Alternatives considered:**

- _Generic `App<R: AgentRepository, H: Harness>`_ instead of trait objects —
  rejected because a heterogeneous `Vec` of harnesses (pi + Claude + future
  ones) needs dynamic dispatch (`Vec<Box<dyn Harness>>`) regardless, so a
  generic `App` would still need `dyn Harness` at the harness level; keeping
  `App` itself non-generic (using `&dyn AgentRepository`) is simpler to read and
  test.

### Decision 3: `agentctl-core` owns all business logic, including rendering

`agentctl-core::App` gains one method per current CLI command: `list`,
`install`, `uninstall`, `dispatch_event`, `clear`, `info` — each returning a
`Result<String>` (or `Result<()>`) that the bin prints directly. This includes
what used to be presentation logic in `commands/list.rs` (`Field`, `sort_panes`,
`format_panes`, `colorize_state`): sorting rules ("newest first by default",
"blocked sorts before done") and text/color rendering are business rules, not
CLI plumbing, so they belong where a future non-CLI adapter (e.g. a web server
producing its own JSON) could still reach the sorting logic even if it swaps out
the text renderer.

**Alternatives considered:**

- _Leave `Field`/sorting/formatting in the bin_ — rejected per explicit
  maintainer feedback; the bin should contain no logic beyond CLI parsing and
  the composition root.

### Decision 4: One crate per harness, contract crate has no I/O

`agentctl-harness` intentionally contains zero pi-specific or Claude-specific
code — only the `Harness` trait and its supporting types (`InstallSummary`,
`UninstallSummary`). Adding a third harness later means adding a third crate
(`agentctl-harness-<name>`) implementing the trait; `agentctl-harness`,
`agentctl-harness-pi`, and `agentctl-harness-claude` never need to change.

### Decision 5: Filesystem chokepoint is scoped to agent-state, not harness installs

"Only one workspace ever accesses the filesystem" is scoped to **agent-state
persistence** (`agentctl-state-fs`) — the mutable data (`state.json`,
`events.jsonl`) a future web adapter would also need to read/write, so
centralizing it behind `AgentRepository` has clear reuse value.
`agentctl-harness-pi` and `agentctl-harness-claude` still write their own files
directly (the pi extension file, the Claude plugin directory) — this is a
different class of filesystem write (installing static integration artifacts,
not tracking mutable agent state), and folding it through the same
`AgentRepository` abstraction would force an awkward shared interface between
two unrelated concerns.

**Alternative considered (deferred, not rejected):** a lower-level `agentctl-fs`
crate (atomic write, directory creation, tilde-collapse-for- display helpers)
that both `agentctl-state-fs` and the two harness crates depend on, giving one
audit point for literally all disk writes in the project. Left as an explicit
open question below rather than decided unilaterally.

### Decision 6: Shared logging interface, single sink owner

`agentctl-logging` exports `LogLevel`, `init(app_name, level)` (called only by
the bin), directory/rotation helpers, and a `mod ops` of structured logging
helper functions. Every other crate — including `agentctl-core` — depends on
`agentctl-logging` and calls its `ops` helpers instead of the raw `tracing`
crate directly, so logging is a first-class interface dependency, not an
incidental one, even though only the bin ever configures where logs go.

## Risks / Trade-offs

- **[Risk] Nine crates is more workspace overhead than a single crate for a
  ~7k-line project** → Mitigation: crate boundaries map almost 1:1 onto module
  boundaries that already exist today, so the migration is close to a pure file
  move plus introducing a handful of traits; very little new code is written.
  The "when to split further" guidance in the proposal review explicitly avoids
  adding _more_ crates until a concrete signal appears.
- **[Risk] Dependency injection (trait objects passed through constructors) adds
  indirection compared to today's free functions reading `$XDG_DATA_HOME`
  directly** → Mitigation: this indirection is exactly what makes
  `agentctl-core` and `agentctl-tmux` reusable by a future non-CLI binary and
  testable without mutating global environment variables (several existing tests
  already work around global env races with a `Mutex`; DI removes that need).
- **[Risk] Scoping the filesystem chokepoint to agent-state only (Decision 5)
  may not match the maintainer's full intent** → Mitigation: flagged as an open
  question; easy to introduce a lower-level `agentctl-fs` crate later without
  disturbing the trait boundaries above it.
- **[Risk] Splitting harnesses into separate crates before more than two exist
  may be premature** → Mitigation: explicit maintainer request; the trait crate
  (`agentctl-harness`) keeps the shared contract centralized so the actual
  duplication risk (three near-identical `Cargo.toml`s) is low cost.

## Migration Plan

1. Introduce the workspace `Cargo.toml` and `crates/` layout with all nine
   crates as empty stubs compiling against each other per the dependency graph
   above.
2. Move `agentctl-state`'s types + FSM verbatim from `src/state.rs` /
   `src/status_fsm.rs`; add the `AgentRepository` trait derived from
   `state.rs`/`events_log.rs`'s existing function signatures.
3. Move `agentctl-state-fs`'s file I/O bodies from `src/state.rs` /
   `src/events_log.rs`; implement `AgentRepository` for `FileAgentRepository`.
4. Move `agentctl-tmux` from the data half of `commands/list.rs`; thread
   `&dyn AgentRepository` through `load_state_map`/`validate_state_files`.
5. Move `agentctl-harness`'s trait definition out of the implicit shape of
   `run_install`/`run_uninstall`'s exhaustive match in `commands/hooks.rs`;
   split pi/Claude bodies into `agentctl-harness-pi`/`agentctl-harness-claude`.
6. Move `agentctl-logging` from `src/logging.rs`, generalizing the hardcoded
   `"agentctl"` app name to a parameter; add the `ops` helper module.
7. Build `agentctl-core::App` from the remaining orchestration in
   `commands/list.rs` (rendering), `commands/agent_event.rs`,
   `commands/agent_clear.rs`, `commands/hooks.rs` (dispatch), and
   `commands/info.rs`.
8. Reduce `src/main.rs` (now `crates/agentctl/src/main.rs`) to clap definitions,
   the composition root, and one `app.*()` call per subcommand.
9. Move each moved module's unit tests alongside its new crate; leave
   `tests/*.rs` integration tests in `crates/agentctl`.
10. Run the full test suite (`mise run <task>`) and manually smoke-test every
    subcommand to confirm zero behavior change before merging.

No rollback complexity beyond reverting the commit(s) — there is no persisted
data format change, so a rollback is safe at any point.

## Open Questions

- Should `App::dispatch_event` internally call the FSM, repository write, and
  event-log append as one bundled step (as sketched above), or should the bin
  sequence those as explicit calls? This is now purely a question of how many
  public methods `App` exposes for one logical action, since orchestration lives
  in `agentctl-core` either way.
- Does the filesystem-chokepoint scoping in Decision 5 (agent-state only, not
  harness installs) match the intended governance model, or should a shared
  `agentctl-fs` crate be introduced for both?
- Should `cargo-semver-checks` (or `cargo public-api`) be wired into CI as part
  of this change, or as an immediate follow-up once the workspace boundaries are
  stable?
