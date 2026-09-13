## Purpose

Define the Cargo workspace structure for `agentctl`: the set of crates, their
dependency boundaries, and the architectural constraints (single filesystem
access point for agent-state, trait-based harness integrations, business logic
isolated in `agentctl-core`, and a thin binary composition root) that keep the
codebase modular and testable.

## Requirements

### Requirement: agentctl is organized as a Cargo workspace

The project SHALL be organized as a Cargo workspace with one binary crate
(`agentctl`) and eight library crates (`agentctl-core`, `agentctl-state`,
`agentctl-state-fs`, `agentctl-tmux`, `agentctl-harness`, `agentctl-harness-pi`,
`agentctl-harness-claude`, `agentctl-logging`), each under `crates/<name>/`.

#### Scenario: Workspace builds as a whole

- **WHEN** `cargo build --workspace` is run from the repository root
- **THEN** all nine crates compile successfully

#### Scenario: Each crate builds independently

- **WHEN** `cargo build -p <crate-name>` is run for any single library crate
- **THEN** that crate compiles using only its declared dependencies, without
  requiring the `agentctl` binary crate

### Requirement: agentctl-state-fs is the sole filesystem access point for agent-state persistence

No crate other than `agentctl-state-fs` SHALL perform filesystem reads or writes
against agent-state files (`state.json`, `events.jsonl`, or their containing
directories). `agentctl-state-fs` SHALL implement the `AgentRepository` trait
defined in `agentctl-state`.

#### Scenario: Only agentctl-state-fs depends on filesystem I/O for state

- **WHEN** the dependency graph of `agentctl-state`, `agentctl-tmux`,
  `agentctl-core`, and `agentctl-harness*` crates is inspected
- **THEN** none of them directly call `std::fs` functions against agent-state
  paths; all agent-state reads/writes go through an injected
  `&dyn AgentRepository`

#### Scenario: FileAgentRepository is not a global singleton

- **WHEN** `agentctl-state-fs` is inspected
- **THEN** it exports a plain `FileAgentRepository` struct with explicit
  constructors (e.g. `new(base: PathBuf)`, `xdg()`) and does not expose a
  process-wide static/singleton instance

### Requirement: Harness integrations are defined by a shared trait and one crate per harness

`agentctl-harness` SHALL define a `Harness` trait describing the install/
uninstall contract for a harness integration, with no harness-specific code and
no filesystem I/O of its own. Each supported harness SHALL be implemented in its
own crate (`agentctl-harness-pi`, `agentctl-harness-claude`, ...) that depends
on `agentctl-harness` and implements `Harness`.

#### Scenario: Adding a new harness does not modify existing harness crates

- **WHEN** a new harness integration is added as `agentctl-harness-<name>`
  implementing `Harness`
- **THEN** no changes are required to `agentctl-harness`, `agentctl-harness-pi`,
  or `agentctl-harness-claude`

#### Scenario: agentctl-harness has no I/O

- **WHEN** `agentctl-harness`'s dependencies are inspected
- **THEN** it performs no filesystem or process I/O itself; it only declares
  types and the `Harness` trait

### Requirement: agentctl-core contains all business logic and depends only on trait ports

`agentctl-core` SHALL implement every current CLI use case (list, install,
uninstall, agent event dispatch, clear, info) and SHALL depend only on
`agentctl-state` (for `AgentRepository` and domain types), `agentctl-tmux`,
`agentctl-harness` (for the `Harness` trait), and `agentctl-logging`.
`agentctl-core` SHALL NOT depend on `agentctl-state-fs`, `agentctl-harness-pi`,
`agentctl-harness-claude`, or `clap`.

#### Scenario: agentctl-core has no concrete infrastructure dependencies

- **WHEN** `agentctl-core`'s `Cargo.toml` is inspected
- **THEN** it lists no dependency on `agentctl-state-fs`, `agentctl-harness-pi`,
  `agentctl-harness-claude`, or `clap`

#### Scenario: agentctl-core exposes one entry point per CLI command

- **WHEN** `agentctl-core`'s public API is inspected
- **THEN** it exposes an `App` type with one method per current subcommand
  (list, install, uninstall, dispatch_event, clear, info), each returning a
  plain `Result` value with no `clap` or terminal-specific types in its
  signature

### Requirement: agentctl binary is a thin composition root

The `agentctl` binary crate SHALL contain only `clap` argument definitions,
conversions from CLI-derived enums to `agentctl-core`/domain types, and the
composition root that constructs concrete infrastructure (`FileAgentRepository`,
harness implementations, the logging sink) and passes them into
`agentctl_core::App`. It SHALL NOT contain sorting, rendering, or
command-dispatch business logic.

#### Scenario: Binary crate delegates every command to agentctl-core

- **WHEN** the `agentctl` binary's command dispatch is inspected
- **THEN** every subcommand handler calls exactly one `agentctl_core::App`
  method and prints or propagates its result, with no additional business logic
  in between

### Requirement: Every library crate depends on agentctl-logging for its logging interface

Every workspace crate other than `agentctl-logging` itself SHALL depend on
`agentctl-logging` and use its structured logging helpers rather than calling
the underlying logging backend directly. Only the `agentctl` binary crate SHALL
call `agentctl_logging::init(...)` to configure the actual log sink.

#### Scenario: Only the binary configures the log sink

- **WHEN** the workspace is searched for calls to `agentctl_logging::init`
- **THEN** exactly one call site exists, in the `agentctl` binary crate's
  composition root

### Requirement: This change preserves existing CLI behavior

Splitting `agentctl` into a workspace SHALL NOT change any previously specified
CLI behavior (arguments, output format, exit codes, or side effects) documented
in the existing specs (`agent-events`, `agent-state`, `claude-state-hooks`,
`pi-state-hooks`, `list-columns`, `list-sessions`, `list-sorting`,
`info-command`, `uninstall-command`, `structured-logging`).

#### Scenario: Full existing test suite passes unchanged

- **WHEN** the existing integration and unit test suite is run against the
  workspace-based crate layout
- **THEN** every test that passed before the restructuring still passes, with no
  changes to test assertions required
