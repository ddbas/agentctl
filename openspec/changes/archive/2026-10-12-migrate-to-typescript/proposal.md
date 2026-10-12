## Why

`agentctl` is a ~5k-line CLI (plus ~2.8k lines of tests) that shells out to
`tmux`, reads and writes JSON state files, and renders text. None of that needs
Rust, and the Rust toolchain (cargo, rust-fsm, clap, testcontainers-rs, mise
task templates keyed to `crates/*`) is friction for a project whose surrounding
tooling and integrations (pi extensions, Claude hooks, openspec) are already
TypeScript/Node. Consolidating on `pnpm` + TypeScript + Turborepo gives one
language, one package manager, and one task runner.

## What Changes

- Replace the Cargo workspace (`crates/*`, `Cargo.toml`, `Cargo.lock`) with a
  pnpm + Turborepo workspace:
  - `apps/agentctl` — the CLI bin and composition root (commander wiring only).
  - `packages/core` — `App`, list rendering, status FSM, and the port interfaces
    (`AgentRepository`, tmux, `Harness`).
  - `packages/adapters` — tmux, state-fs, harness-pi, harness-claude, and
    logging adapters, each as a folder with an enforced one-way dependency on
    `core`.
- Toolchain: Node 26, TypeScript 7.x (`tsc --noEmit` typecheck only), Rspack
  bundling `apps/agentctl` to a single shebanged file, Biome for TS lint/format,
  prettier for markdown only, vitest for tests.
- Library swaps: clap → commander, serde → zod, rust-fsm → hand-rolled
  transition table, tracing → pino (file destination), owo-colors → picocolors,
  testcontainers-rs → testcontainers (npm).
- Rewrite the e2e harness so `TmuxEnv.agentctl()` runs the binary named by
  `AGENTCTL_BIN`. Port the existing e2e suite to vitest once and run it against
  both the Rust and TS binaries until parity, then delete Rust in a single
  cutover commit.
- Tooling: `mise.toml` only pins tools (node, pnpm, openspec, lefthook); tasks
  move to `package.json`/`turbo.json`; lefthook, CI, README, AGENTS.md, and
  `openspec/config.yaml` stop referencing Rust/cargo.
- Install story: private package with a `bin` entry, installed via
  `pnpm link --global` (replaces `cargo install --path .`).
- **BREAKING (tooling only):** `cargo build/install` and `target/` go away. CLI
  behavior is intentionally **not** changed (strict parity: identical stdout for
  `list`/`info`/`help`, unchanged state file format, no bug fixes during the
  port).

## Capabilities

### New Capabilities

None. This is a behavior-preserving implementation migration.

### Modified Capabilities

- `workspace-architecture`: Cargo workspace of nine crates becomes a pnpm
  workspace (`apps/agentctl`, `packages/core`, `packages/adapters`). The
  boundary rules (single FS access point for state, trait/interface-based
  harnesses, business logic isolated in core, thin composition root, logging via
  a shared interface) are preserved but restated in terms of packages, folders
  and module-boundary lint rules instead of crates.
- `e2e-test-infrastructure`: Rust `tests/` + testcontainers-rs + Tokio becomes
  vitest + testcontainers (npm); the binary runner is selected via
  `AGENTCTL_BIN`; the canary and help tests are retained.
- `agent-events`: wording only — status machine is no longer described as
  implemented with the `rust-fsm` crate or "nested clap subcommands". Transition
  table and behavior are unchanged.
- `info-command`: wording only — `version` is sourced from the package version
  rather than `CARGO_PKG_VERSION`; output is unchanged.
- `uninstall-command`: wording only — argument errors are described as a
  CLI-framework parse error rather than a "clap" error.

## Impact

- **Removed:** `crates/**`, `Cargo.toml`, `Cargo.lock`, `target/`, Rust entries
  in `mise.toml` (toolchain, `task_templates`, `[monorepo] config_roots`),
  `*.rs` globs in `lefthook.yml`.
- **Added:** `pnpm-workspace.yaml`, root `package.json`, `turbo.json`,
  `tsconfig` base, `biome.json`, `apps/agentctl` (incl. `rspack.config`),
  `packages/core`, `packages/adapters`, vitest config.
- **Changed:** `.github/workflows/ci.yml` (pnpm/turbo; e2e runs against both
  binaries until cutover), `README.md` install/usage, `AGENTS.md` task guidance,
  `openspec/config.yaml` tech stack, codegraph index.
- **Unchanged:** the on-disk state format under the XDG data dir, the hook files
  `install pi|claude` writes (except the invoked command if the distribution
  story requires it), and all behavioral specs.
- **Risks:**
  - Cold start on the hook path (`agent event` fires on every lifecycle event):
    Node vs Rust; pino and zod add to it. Measure early; fall back to a
    hand-rolled logger if needed.
  - Byte-exact output for `list`, `info`, `help`; commander's help and
    argument-error text differ from clap's and must be matched explicitly.
  - Rspack ESM output is still experimental; CJS bundle is the fallback.
  - Existing state files written by the Rust version must parse under the zod
    schemas (tested against Rust-written fixtures).
