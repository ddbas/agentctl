## 1. Phase 0: Scaffold and measure (serial)

- [ ] 1.1 Add `pnpm-workspace.yaml`, root `package.json` (private, pins
      `packageManager`), and `.npmrc` as needed
- [ ] 1.2 Pin Node 26.x and pnpm in `mise.toml` (keep Rust until cutover); add
      `.node-version`
- [ ] 1.3 Add `turbo.json` with `build`, `typecheck`, `lint`, `test`, `ci` tasks
      (`test` depends on `build`)
- [ ] 1.4 Add shared `tsconfig.base.json` (strict, ESM, `noEmit`, TypeScript
      7.x) and per-workspace tsconfigs
- [ ] 1.5 Add `biome.json` (format + lint, `noRestrictedImports` for core
      boundary); keep prettier for markdown
- [ ] 1.6 Scaffold `apps/agentctl`, `packages/core`, `packages/adapters` with
      `package.json` and entry files
- [ ] 1.7 Add `apps/agentctl` Rspack config: node target, swc loader, shebang
      banner, single output file; verify a hello-world bundle runs
- [ ] 1.8 Verify tooling works with TypeScript 7 (tsc, Biome, Rspack, vitest);
      record any workarounds
- [ ] 1.9 Measure cold start of a hello-world bundle with pino + zod imported vs
      Rust `agentctl agent event`; record budget in design.md Open Questions
- [ ] 1.10 Capture Rust-written fixtures (state.json, events.jsonl,
      list/info/help stdout) from the Rust binary into
      `apps/agentctl/tests/fixtures/`

## 2. Phase 0: Core interfaces and e2e harness (serial)

- [ ] 2.1 Define domain types and port interfaces in `packages/core`
      (`AgentRepository`, `Tmux`, `Harness`, `Logger`, `AgentInfo`, `Event`)
- [ ] 2.2 Port the e2e support module to
      `apps/agentctl/tests/e2e/support/tmux-env.ts` with `testcontainers` and
      `AGENTCTL_BIN` selection
- [ ] 2.3 Port the canary and help e2e tests; run against the Rust binary
      (`AGENTCTL_BIN=target/debug/agentctl`) and confirm green
- [ ] 2.4 Port `agent_commands`, `hooks`, and `list_sessions` e2e tests; confirm
      green against Rust
- [ ] 2.5 Add a CI job leg that builds Rust and runs the vitest e2e suite with
      `AGENTCTL_BIN` set to the Rust binary

## 3. Phase 1: Leaf modules (parallel; one folder each)

- [ ] 3.1 Port `agentctl-state` (status FSM transition table, `AgentInfo`) with
      its unit tests into `packages/core`
- [ ] 3.2 Port `agentctl-logging` to `packages/adapters/src/logging` using pino;
      match `structured-logging` spec (file path, level, line format)
- [ ] 3.3 Port `agentctl-state-fs` to `packages/adapters/src/state-fs` with zod
      schemas; test against Rust-written fixtures
- [ ] 3.4 Port `agentctl-tmux` to `packages/adapters/src/tmux` using `execFile`;
      port its unit tests
- [ ] 3.5 Port `agentctl-harness-pi` to `packages/adapters/src/harness-pi`;
      output files identical to Rust's (diff test)
- [ ] 3.6 Port `agentctl-harness-claude` to
      `packages/adapters/src/harness-claude`; output files identical to Rust's
      (diff test)

## 4. Phase 2: Integration (serial)

- [ ] 4.1 Port `agentctl-core` `list` (sorting, columns, relative time, node→pi
      resolution) with its unit tests
- [ ] 4.2 Port `agentctl-core` `App` use cases (install, uninstall,
      dispatchEvent, clear, info)
- [ ] 4.3 Implement `apps/agentctl` commander wiring and composition root; match
      clap help text, aliases, global `--log-level`, and `AGENTCTL_TMUX_SOCKET`
      fallback
- [ ] 4.4 Make every e2e test pass against the TS bundle
- [ ] 4.5 Diff stdout of the Rust and TS binaries for `list`, `info`, `help` on
      shared fixtures; resolve all differences
- [ ] 4.6 Run `agent event` cold-start measurement against the agreed budget;
      adjust (hand-rolled logger / valibot / lazy imports) if exceeded
- [ ] 4.7 Verify `pnpm link --global` provides `agentctl` on PATH and installed
      pi/claude hooks work end to end

## 5. Phase 3: Cutover (serial)

- [ ] 5.1 Remove `crates/`, `Cargo.toml`, `Cargo.lock`, and `target/` ignore
      entry
- [ ] 5.2 Remove the Rust e2e CI leg; confirm no e2e test file changed
- [ ] 5.3 Strip Rust from `mise.toml` (toolchain, `task_templates`, `[monorepo]`
      roots, per-crate `mise.toml` files); keep tool pins
- [ ] 5.4 Update `lefthook.yml` (replace `*.rs` hooks with
      Biome/turbo/prettier-md) and `.github/workflows/ci.yml`
- [ ] 5.5 Update `README.md` install and usage, `AGENTS.md` (mise/turbo
      guidance, no cargo), and `openspec/config.yaml` tech stack
- [ ] 5.6 Re-index codegraph and confirm no references to removed crates remain
- [ ] 5.7 Run `pnpm turbo run ci` and `openspec validate --all --no-interactive`
- [ ] 5.8 Archive the change so delta specs update `openspec/specs/`
