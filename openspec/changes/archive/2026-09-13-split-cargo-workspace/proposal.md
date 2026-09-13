## Why

`agentctl` is currently a single crate (~6.9k lines across `src/main.rs`,
`state.rs`, `status_fsm.rs`, `events_log.rs`, `logging.rs`, and
`commands/*.rs`). The module boundaries that already exist in that layout aren't
enforced by the compiler, and all business logic (sorting, rendering, harness
install/uninstall dispatch, event FSM orchestration) is only reachable by
reading source files directly.

The project is developed 100% by coding agents, and the maintainer wants to
govern it by reading crate public APIs (exported types and function signatures)
rather than diffing implementation source. Splitting the project into a Cargo
workspace — with dependency-inversion boundaries (trait-based ports, a single
filesystem chokepoint for agent-state persistence, and one crate per harness
integration) — turns each crate's public surface into the thing that gets
reviewed, and makes the core business logic reusable by a future non-CLI binary
(e.g. a web server) without any rewrite.

## What Changes

- Convert the single `agentctl` crate into a Cargo workspace with one binary
  crate and eight library crates (see `design.md` for full rationale and API
  surfaces):
  - `agentctl` (bin): clap argument parsing and the composition root only — no
    business logic.
  - `agentctl-core`: the application/use-case layer (List, Install/Uninstall,
    AgentEvent dispatch, Clear, Info) — all current business logic, expressed
    purely against trait objects (`AgentRepository`, `Harness`).
  - `agentctl-state`: domain types (`AgentState`, `AgentInfo`, `Event`,
    `ToolCallType`), the status FSM, and the new `AgentRepository` trait. No
    filesystem access.
  - `agentctl-state-fs`: `FileAgentRepository`, the **sole** crate that reads or
    writes `state.json` / `events.jsonl`. Implements `AgentRepository`.
  - `agentctl-tmux`: tmux/`ps`-based pane discovery and harness resolution,
    parameterized over `&dyn AgentRepository`.
  - `agentctl-harness`: the `Harness` trait — the contract a harness integration
    must implement. No I/O of its own.
  - `agentctl-harness-pi`: `PiHarness`, implementing `Harness` for the pi coding
    agent extension.
  - `agentctl-harness-claude`: `ClaudeHarness`, implementing `Harness` for the
    Claude Code plugin/hooks integration.
  - `agentctl-logging`: shared logging interface (`LogLevel`, `init`, structured
    logging helpers) used by every other crate; only the bin calls `init()`.
- Move all existing unit tests to the crate that now owns the code they test.
  Integration tests (`tests/*.rs`) stay in the `agentctl` bin crate.
- No user-visible CLI behavior changes as part of this restructuring — this is a
  structural refactor, not a feature or requirement change.

## Capabilities

### New Capabilities

- `workspace-architecture`: the crate boundary invariants introduced by this
  change — the workspace layout, which crate owns which responsibility, and the
  dependency-inversion rules (trait ports vs. concrete implementations, the
  single filesystem chokepoint for agent-state, the composition root) that keep
  those boundaries meaningful going forward. This is an internal architecture
  capability, not an end-user-facing one, but it is written as testable
  requirements so future changes can be checked against it.

### Modified Capabilities

_None — no spec-level (user-facing behavior) requirements change. `agentctl`'s
CLI surface, output, and documented behavior across `agent-events`,
`agent-state`, `claude-state-hooks`, `pi-state-hooks`, `list-columns`,
`list-sessions`, `list-sorting`, `info-command`, `uninstall-command`, and
`structured-logging` remain exactly as specified today; this change only
relocates their implementations across crate boundaries._

## Impact

- **Affected code**: the entire `src/` tree is reorganized into
  `crates/agentctl/`, `crates/agentctl-core/`, `crates/agentctl-state/`,
  `crates/agentctl-state-fs/`, `crates/agentctl-tmux/`,
  `crates/agentctl-harness/`, `crates/agentctl-harness-pi/`,
  `crates/agentctl-harness-claude/`, and `crates/agentctl-logging/`.
- **Build/CI**: root `Cargo.toml` becomes a `[workspace]` manifest with
  `[workspace.dependencies]`; CI commands (`mise run <task>`) need to build/test
  the whole workspace instead of a single crate.
- **Dependencies**: no new external dependencies required; `dirs`, `chrono`,
  `serde`, `serde_json`, `rust-fsm`, `tracing`, `clap`, `owo-colors`, `anyhow`
  are redistributed across the new crates per their responsibilities.
- **Testing**: `testcontainers`-based e2e/integration tests continue to live
  with the bin crate per `openspec/specs/e2e-test-infrastructure/spec.md`.
- **Governance**: opens the door to adding `cargo-semver-checks` (or similar) to
  CI in a later change, once the workspace boundaries are stable.
