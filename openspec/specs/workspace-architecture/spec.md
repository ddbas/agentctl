## Purpose

Define the pnpm and Turborepo workspace structure for `agentctl`: the set of
workspaces, their dependency boundaries, and the architectural constraints
(single filesystem access point for agent-state, interface-based harness
integrations, business logic isolated in `packages/core`, and a thin app
composition root) that keep the codebase modular and testable.

## Requirements

### Requirement: This change preserves existing CLI behavior

Migrating `agentctl` from Rust to TypeScript SHALL NOT change any previously
specified CLI behavior (arguments, output format, exit codes, state file format,
generated hook files, or side effects) documented in the existing specs
(`agent-events`, `agent-state`, `claude-state-hooks`, `pi-state-hooks`,
`list-columns`, `list-sessions`, `list-sorting`, `info-command`,
`uninstall-command`, `structured-logging`). State files written by the Rust
implementation SHALL be readable by the TypeScript implementation.

#### Scenario: Full e2e suite passes against both implementations

- **WHEN** the ported e2e suite is run with `AGENTCTL_BIN` pointing at the Rust
  binary and again pointing at the TypeScript bundle
- **THEN** every test passes in both runs with identical assertions

#### Scenario: Rust-written state is readable

- **WHEN** the TypeScript `agentctl list` and `agentctl info` run against a
  state directory populated by the Rust implementation
- **THEN** they succeed and print the same output the Rust implementation
  printed for the same state

### Requirement: agentctl is organized as a pnpm and Turborepo workspace

The project SHALL be organized as a pnpm workspace orchestrated by Turborepo
with one application workspace and a set of library workspaces: `apps/agentctl`
(the CLI executable and composition root), `packages/core` (domain types, the
status state machine, use cases, and port interfaces) and one adapter package
each for `packages/tmux`, `packages/state-fs`, `packages/logging`,
`packages/harness-pi` and `packages/harness-claude`. Applications SHALL live
under `apps/`; only libraries SHALL live under `packages/`. No Cargo manifest,
Rust source, or `crates/` directory SHALL remain in the repository.

#### Scenario: Workspace builds as a whole

- **WHEN** `pnpm turbo run build` is run from the repository root
- **THEN** `apps/agentctl` builds successfully to a single bundled executable
  file

#### Scenario: Whole workspace typechecks and lints

- **WHEN** `pnpm turbo run typecheck lint` is run from the repository root
- **THEN** every workspace passes with no errors

#### Scenario: Rust is gone

- **WHEN** the repository is searched for `Cargo.toml`, `Cargo.lock`, `*.rs`,
  and a `crates/` directory
- **THEN** none are found

### Requirement: The state-fs adapter is the sole filesystem access point for agent-state persistence

No module other than `packages/state-fs/src` SHALL perform filesystem reads or
writes against agent-state files (`state.json`, `events.jsonl`, or their
containing directories). The state-fs adapter SHALL implement the
`AgentRepository` interface defined in `packages/core`.

#### Scenario: Only state-fs touches agent-state paths

- **WHEN** the `core` package and the tmux and harness adapters are inspected
- **THEN** none of them call `node:fs` functions against agent-state paths; all
  agent-state reads and writes go through an injected `AgentRepository`

#### Scenario: FileAgentRepository is not a global singleton

- **WHEN** the state-fs adapter is inspected
- **THEN** it exports a `FileAgentRepository` class with explicit constructors
  or factories (e.g. one taking a base directory, one resolving the XDG data
  directory) and does not expose a process-wide singleton instance

### Requirement: Harness integrations are defined by a shared interface and one module per harness

`packages/core` SHALL define a `Harness` interface describing the install and
uninstall contract for a harness integration, with no harness-specific code and
no filesystem or process I/O of its own. Each supported harness SHALL be
implemented in its own package (`packages/harness-pi`,
`packages/harness-claude`, ...) that implements `Harness`.

#### Scenario: Adding a new harness does not modify existing harness modules

- **WHEN** a new harness integration is added as `harness-<name>` implementing
  `Harness`
- **THEN** no changes are required to the `Harness` interface, `harness-pi`, or
  `harness-claude`

#### Scenario: The Harness interface has no I/O

- **WHEN** the module declaring `Harness` in `packages/core` is inspected
- **THEN** it performs no filesystem or process I/O itself; it only declares
  types and the interface

### Requirement: The core package contains all business logic and depends only on interface ports

`packages/core` SHALL implement every current CLI use case (list, install,
uninstall, agent event dispatch, clear, info) and SHALL NOT depend on the
adapter packages, `commander`, `pino`, or `zod`. All infrastructure it needs
(agent repository, tmux, harnesses, logger) SHALL be expressed as interfaces
declared in `packages/core` and injected by the caller.

#### Scenario: core has no concrete infrastructure dependencies

- **WHEN** `packages/core/package.json` and its source imports are inspected
- **THEN** it lists no dependency on, and no source file imports from, any
  `@agentctl/*` adapter package, `commander`, `pino`, or `zod`

#### Scenario: core exposes one entry point per CLI command

- **WHEN** the public API of `packages/core` is inspected
- **THEN** it exposes an `App` class with one method per current subcommand
  (list, install, uninstall, dispatchEvent, clear, info), each returning a plain
  value or throwing a typed error, with no CLI-framework or terminal-specific
  types in its signature

#### Scenario: Boundary is enforced by tooling

- **WHEN** a file in `packages/core` adds an import from an adapter package such
  as `@agentctl/tmux`
- **THEN** `pnpm turbo run lint` fails

### Requirement: The agentctl app is a thin composition root

`apps/agentctl` SHALL contain only the `commander` command definitions,
conversions from parsed CLI values to `packages/core` domain types, and the
composition root that constructs concrete adapters (`FileAgentRepository`, the
tmux adapter, harness implementations, the logger) and passes them into `App`.
It SHALL NOT contain sorting, rendering, or command-dispatch business logic.

#### Scenario: App delegates every command to core

- **WHEN** the command handlers in `apps/agentctl` are inspected
- **THEN** every subcommand handler calls exactly one `App` method and prints or
  propagates its result, with no additional business logic in between

### Requirement: Every library module logs through the core Logger port

`packages/core` SHALL declare a `Logger` interface. Every library module that
logs SHALL do so through an injected `Logger` rather than importing the logging
backend directly. The logging adapter (pino writing to a log file) SHALL be the
only module that imports `pino`, and only `apps/agentctl` SHALL construct and
configure the concrete logger.

#### Scenario: Only the app configures the log sink

- **WHEN** the repository is searched for construction of the concrete logger
  (the logging adapter's factory function)
- **THEN** exactly one call site exists, in the `apps/agentctl` composition root

#### Scenario: Only the logging adapter imports pino

- **WHEN** the repository is searched for imports of `pino`
- **THEN** every match is inside `packages/logging/src`

### Requirement: Tool versions and tasks are pinned and defined in the workspace

The repository SHALL pin Node (26.x), pnpm, and TypeScript (7.x) and SHALL
define `build`, `typecheck`, `lint`, `test`, and `ci` tasks through Turborepo
and package scripts. `mise.toml` SHALL only pin developer tools (node, pnpm,
openspec, lefthook, and similar) and SHALL NOT define Rust toolchains,
`task_templates`, or per-crate monorepo roots.

#### Scenario: CI runs through turbo

- **WHEN** the CI workflow runs
- **THEN** it installs pinned tools, runs `pnpm install --frozen-lockfile`, and
  runs the turbo `ci` pipeline (typecheck, lint, build, test)

#### Scenario: Cold-start cost is measured

- **WHEN** the TypeScript bundle is run with `agent event` against an empty
  state directory
- **THEN** wall-clock time per invocation is recorded in the CI log for tracking
  against the Rust baseline
