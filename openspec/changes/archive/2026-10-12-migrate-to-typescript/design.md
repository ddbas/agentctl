## Context

`agentctl` is a Rust Cargo workspace of nine crates (~5.3k source lines, ~2.8k
test lines). It shells out to `tmux`, persists per-pane state as JSON/JSONL
under the XDG data dir, renders tab-delimited output, and installs hook
integrations for the pi and Claude harnesses. There is no async code, FFI, or
performance-critical algorithm. The strongest asset is the black-box e2e suite
(`crates/agentctl/tests/`): it spawns the binary against a containerized tmux
and asserts on stdout, exit codes and files.

Constraints:

- Strict behavior parity (see the proposal). The port must not change output,
  state format, or generated hook files.
- `agent event` runs on every pi/Claude lifecycle event, so process start time
  matters.
- Rust must stay buildable until parity is proven, so the Rust binary can serve
  as a reference oracle.

## Goals / Non-Goals

**Goals:**

- A pnpm + TypeScript + Turborepo workspace with no Rust or Cargo.
- Fastest safe path: reuse the existing e2e suite as the parity oracle; port
  leaf modules in parallel.
- Preserve the existing hexagonal boundaries (core knows no infrastructure) with
  fewer packages.

**Non-Goals:**

- Any behavior change, bug fix, or redesign during the port. Bugs found are
  logged as follow-up changes.
- Publishing to npm or shipping standalone binaries.
- Replacing tmux, changing the state format, or adding commands.

## Decisions

Decisions were made interactively; "rec" marks the option that was recommended.

### 1. Layout: 3 workspaces (rec)

```
apps/agentctl         bin + commander wiring + composition root
packages/core         App, list, status FSM, ports (AgentRepository,
                      Tmux, Harness, Logger), domain types
packages/adapters     src/{tmux, state-fs, harness-pi, harness-claude, logging}
```

`apps/` holds executables; `packages/` holds libraries only. Dependency
direction: `apps/agentctl → adapters → core` and `apps/agentctl → core`; `core`
imports nothing from `adapters`. Enforced by package.json dependencies plus a
Biome `noRestrictedImports` rule in `packages/core`. _Alternatives:_ 6
workspaces (real isolation, more boilerplate) and 1:1 with crates (9 workspaces;
mechanical but heavy for ~5k lines). Folder boundaries in `adapters` are cheap
to promote to packages later.

### 2. Runtime and build: Node 26, TypeScript 7, Rspack

- Node 26 (current release, not yet LTS; pinned exactly via mise).
- TypeScript 7.x (native compiler) runs `tsc --noEmit` as the `typecheck` task
  only; it does not emit.
- Rspack (`@rspack/core`, `@rspack/cli`) bundles `apps/agentctl` to one file,
  `target: 'node'`, with the built-in swc loader stripping types and workspace
  libs inlined. A `BannerPlugin({ raw: true })` adds the shebang. Libraries
  export TypeScript source directly and have no build step of their own.
- ESM output requires `experiments.outputModule`; if unstable, fall back to a
  CJS bundle (no consumer imports the bundle).

_Alternatives:_ tsdown (simpler, less config; initially recommended, replaced by
Rspack on request), Node native type stripping (no bundle, slower start with
many files, restricted syntax), Bun (fastest start and real single binary, but a
second runtime outside the pnpm/node brief).

### 3. Distribution: private package + `pnpm link --global`

`apps/agentctl/package.json` is private with a `bin` entry pointing at the
bundle. Replaces `cargo install --path .` in the README. pnpm 12 removed
`pnpm link --global` (and `pnpm add -g link:` does not link bins), so the root
script `pnpm install:local` builds the bundle (made executable by an Rspack
plugin) and symlinks it to `~/.local/bin/agentctl`. _Alternatives:_ npm publish
(needs release workflow, changesets), standalone binaries (heaviest CI).

### 4. CLI: commander

Closest to clap's model (subcommands, aliases such as `ls`, global options, env
fallbacks). Help and error text are customized to match the Rust output that
`help_command.rs` and the other e2e tests assert. _Alternatives:_ citty, cleye,
`util.parseArgs`.

### 5. Validation: zod

`zod` schemas live in the state-fs adapter and are the compatibility contract
for state files written by Rust. _Alternatives:_ valibot (smaller), arktype.

### 6. Status FSM: hand-rolled transition table; logging: pino

- FSM is a typed `Record<State, Partial<Record<EventKey, State>>>` of ~60 lines,
  with the same semantics as `rust-fsm` (`Unknown` accepted as a start state,
  never a target). xstate is heavy for four states and five events.
- Logging uses pino with a file destination behind the `Logger` port. Only the
  logging adapter imports pino. Log file location, rotation naming and line
  format follow the existing `structured-logging` spec. A hand-rolled logger was
  the recommended alternative (lower cold start); pino was chosen, so cold start
  is measured early (Risks).

### 7. Tests: vitest + testcontainers, dual-binary e2e (rec)

The e2e harness is ported once, with `TmuxEnv.agentctl()` selecting the
executable via `AGENTCTL_BIN`.

```
 vitest e2e suite ──► AGENTCTL_BIN=target/debug/agentctl  (Rust, reference)
        │
        └──────────► AGENTCTL_BIN=apps/agentctl/dist/cli  (TS, under test)
                            both ──► Alpine+tmux container (isolated socket, XDG)
```

Green on Rust first proves the ported tests are correct; then failures on the TS
bundle are implementation bugs. The suite can also be used to diff raw stdout of
both binaries on identical fixtures. The Rust leg is deleted at cutover without
touching test files. Unit tests port with each module (`vitest`, colocated
`*.test.ts`). _Alternatives:_ `node:test`; rewriting tests with the
implementation and no oracle.

### 8. Migration strategy: parity, then delete (rec)

Rust is untouched until the TS suite is green; one commit then removes
`crates/`, `Cargo.*`, and Rust tooling. _Alternatives:_ strangler (TS shells out
to Rust for unported commands; throwaway bridge) and big bang (no oracle).

### 9. Execution: scaffold, fan out, integrate (rec)

```
 Phase 0 (serial)   scaffold workspace, tooling, core ports/types, e2e harness
 Phase 1 (parallel) state(+FSM) · logging · state-fs · tmux · harness-pi · harness-claude
 Phase 2 (serial)   core list/app → CLI wiring → e2e parity on both binaries
 Phase 3 (serial)   cutover: delete Rust, tooling, docs, specs archive
```

Phase 1 modules depend only on interfaces fixed in Phase 0 and can be built by
separate agents in separate worktrees.

### 10. Tooling: Biome, prettier (markdown only), mise pins tools only

- Biome handles TS lint and format; prettier remains for `*.md`.
- `mise.toml` pins node, pnpm, openspec, lefthook; drops the Rust toolchain,
  `task_templates`, and `[monorepo] config_roots`. Tasks live in `turbo.json`
  and package scripts (`build`, `typecheck`, `lint`, `test`, `ci`).
- `lefthook.yml` swaps `*.rs` globs for `*.{ts,json}` and runs Biome and turbo.
- CI installs via mise, then `pnpm install --frozen-lockfile` and the turbo `ci`
  pipeline. `test` depends on `build` because e2e runs the bundle.
- The Docker image `am-tmux-test` and its build task are kept as-is.

### 11. Compatibility bar: strict

Byte-identical stdout for `list`, `info`, `help`; unchanged state JSON;
unchanged generated hook files except the invoked command if the install story
needs it. Time formats (`updatedAt`) must match Rust's output exactly.

Decision on parse errors: commander's default usage-error text differs from
clap's. Specs for argument errors (`uninstall-command`) were loosened to "usage
error, non-zero exit". Help text is matched exactly because e2e tests assert on
it.

### 12. OpenSpec packaging: one change

This change rewrites `workspace-architecture` and `e2e-test-infrastructure` and
makes wording-only edits in `agent-events`, `info-command`, and
`uninstall-command`. All other behavior specs are untouched.

## Risks / Trade-offs

- [Hook-path cold start: Node 26 + pino + zod vs Rust's ~2 ms] → Measure in
  Phase 0 with a trivial bundle, then again per phase; lazy-import `list`
  rendering; swap pino for a hand-rolled logger or zod for valibot if the budget
  is exceeded (budget to be agreed from measurements).
- [Byte-exact output drift in `list` (938 Rust lines), help, and `info`] →
  Dual-binary e2e; port the inline unit tests with each module; diff both
  binaries on shared fixtures.
- [State files written by Rust must parse] → Capture Rust-written fixtures
  (state.json, events.jsonl) in Phase 0 and test zod schemas against them,
  including corrupt/missing-file behavior.
- [Rspack ESM output experimental] → CJS fallback; the bundle is only executed,
  never imported.
- [Rspack does not typecheck; TS 7 ecosystem still maturing] → separate
  `typecheck` task gating CI; pin versions; verify Biome/Rspack with TS 7 in
  Phase 0.
- [Node 26 not yet LTS] → pin the exact version; revisit when LTS lands.
- [Generated hook files assume the invoked command] → Review `harness-pi` and
  `harness-claude` output in Phase 1; verify installed hooks call `agentctl` via
  `PATH`, which works with `pnpm link --global`.
- [Tooling assumptions about Rust (mise, lefthook, CI, codegraph, README)] →
  explicit cutover checklist in tasks.
- [Parallel agents conflict] → Phase 0 fixes all interfaces and file layout;
  each Phase 1 module owns a distinct folder.
- [Two e2e legs double CI time] → Temporary; Rust leg removed at cutover.

## Migration Plan

1. Land Phase 0 (scaffold + harness with `AGENTCTL_BIN`); Rust e2e leg green
   under vitest.
2. Land Phase 1 modules independently.
3. Land Phase 2; TS e2e leg green; compare cold start and output diffs.
4. Cutover commit: delete Rust, update tooling/docs, archive this change so the
   rewritten specs replace the Rust-era ones.

Rollback: before cutover, nothing user-facing changes (Rust remains the
installed binary). After cutover, revert the cutover commit to restore Rust.

## Phase 0 findings

- Rspack 2.x emits CJS; the bundle is `dist/cli.cjs` (package is `type: module`,
  so `.cjs` extension is required). No ESM experiment needed.
- pnpm 12 needs `allowBuilds` entries (ssh2, cpu-features, protobufjs set to
  `false`) in `pnpm-workspace.yaml`.
- Biome 2.5: `rules.preset: "recommended"`; `noRestrictedImports` lives under
  `style`. TypeScript 7.0.2, vitest 5, Biome and Rspack all work together.

## Phase 2 findings

- Cold start of `agent event` (20-run average, release Rust vs bundle): Rust ~5
  ms, TS bundle ~48 ms (bare `node -e 0` is ~29 ms). Within the agreed 100 ms
  budget, so no hand-rolled logger / valibot / lazy imports were needed.
- Stdout of `list` (all sort/columns variants, against a live host tmux with
  Rust-written state), `help`, `--help`, `-h`, `--version`, `info` is
  byte-identical between the Rust and TS binaries. Written state/events files
  and install/uninstall output are identical apart from timestamps; clap's
  usage-error text differs from commander's (accepted, exit code 2 preserved).
- `-h` short help and root `--help` are served from fixtures captured from the
  Rust binary; subcommand `--help` uses commander's default format.

## Open Questions

- ~~Cold-start budget for `agent event`~~: measured Phase 0 (hello-world bundle
  with pino + zod, Node 26.11): ~38 ms/run vs Rust debug ~26 ms/run (includes
  process spawn). Budget agreed: **≤ 100 ms** per `agent event` invocation.
- Exact `info` `version` source: `apps/agentctl/package.json` version (currently
  `0.1.0`, same as Cargo) injected at build time by Rspack `DefinePlugin`?
- Does anyone consume the log file format outside `agentctl info`? (Determines
  how strict pino output parity must be.)
- Should the e2e Docker image build move into a turbo task or stay a mise task?
