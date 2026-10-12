# agentctl

This project implements a **tmux-based agent multiplexer**.

## Required One-Time Setup for Pi Agents

If you are running inside pi, install the agentctl lifecycle hooks so your agent
state is tracked in real time:

```bash
agentctl install pi
```

This writes `~/.pi/agent/extensions/agentctl.ts`, a TypeScript extension that
subscribes to pi lifecycle events (`session_start`, `before_agent_start`,
`agent_settled`, `session_shutdown`) and calls `agentctl state set` /
`agentctl state clear` automatically. The command is idempotent — re-running it
is safe.

## Required One-Time Setup for Claude Agents

If you are running inside Claude Code, install the agentctl lifecycle hooks so
your agent state is tracked in real time:

```bash
agentctl install claude
```

This installs a Claude Code plugin at `~/.claude/skills/agentctl/` (containing
`.claude-plugin/plugin.json` and `hooks/hooks.json`) registering eleven hook
entries that call `agentctl agent event` / `agentctl agent clear` at key Claude
Code lifecycle events (`SessionStart`, `UserPromptSubmit`, `PreToolUse`,
`PostToolUse`, `Notification`, `Stop`, `StopFailure`, `SessionEnd`). The command
never reads or writes `~/.claude/settings.json`, and is idempotent by
construction — every run deletes and recreates `~/.claude/skills/agentctl/`.

## Layout

A pnpm + Turborepo TypeScript workspace:

- `apps/agentctl` — CLI bin and composition root (commander wiring only),
  bundled by Rspack to `dist/cli.cjs`.
- `packages/core` — `App` use cases, list rendering, status FSM, and the port
  interfaces (`AgentRepository`, `Tmux`, `Harness`, `Logger`). Must not import
  infrastructure (enforced by a Biome `noRestrictedImports` rule).
- `packages/adapters` — tmux, state-fs, harness-pi, harness-claude and logging
  adapters, one folder each.

## Running Tasks

Tools (node, pnpm, openspec, lefthook, prettier) are pinned by `mise`. Run
everything through `mise` so the pinned versions are used; tasks themselves live
in `package.json` / `turbo.json`.

```bash
mise exec -- pnpm install --frozen-lockfile
mise exec -- pnpm turbo run ci       # build, typecheck, lint, test
mise exec -- pnpm build              # bundle apps/agentctl/dist/cli.cjs
mise exec -- pnpm test               # unit + e2e (needs Docker)
mise exec -- pnpm format             # biome + prettier (markdown)
mise exec -- pnpm install:local      # build and symlink ~/.local/bin/agentctl
```

## Testing

All new features MUST include unit and/or integration tests (vitest).

All E2E and integration tests MUST use
[testcontainers](https://node.testcontainers.org/) so they are fully isolated
from the host environment (filesystem, git config, hooks, env vars, etc.). The
e2e suite lives in `apps/agentctl/tests/e2e` and runs the binary named by
`AGENTCTL_BIN` (default: the built bundle). See
`openspec/specs/e2e-test-infrastructure/spec.md` for details.

<!-- BEGIN:turborepo-agent-rules -->

# This is NOT the Turborepo you know

Turborepo configuration, task behavior, and CLI commands can vary between
installed versions and may differ from your training data. Resolve the `turbo`
package from this file's directory or relevant workspace; in monorepos, it may
not be visible from the repository root. For example, run
`node -p "require.resolve('turbo/package.json')"` from a workspace that depends
on `turbo`.

Read `docs/README.md` inside that installed package first, then read the
relevant pages from its `docs/` directory before changing Turborepo
configuration or commands. Heed deprecation notices. These bundled docs match
the installed package version and are available without network access.

This block is written and re-added by `turbo` before repository-scoped commands
when an AI agent is detected. In the Turborepo source repository, its template
is defined in `crates/turborepo-cli/src/cli/agent_guidance.rs`. Removing the
managed block while updates are enabled means a later qualifying invocation will
add it again. Set `"agentGuidance": false` in the root `turbo.json` or
`turbo.jsonc` to opt out; this does not remove an existing block. Keep the block
committed with your work to avoid an uncommitted change on the next agent
invocation.

<!-- END:turborepo-agent-rules -->
