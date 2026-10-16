## Why

Each harness adapter embeds its hook files inside TypeScript source (a template
string for pi, a `CLAUDE_HOOKS` table plus JSON builders for Claude). The
installed files cannot be read or diffed as they actually appear. Moving them to
standalone asset files makes every hook change visible in isolation.

## What Changes

- Add standalone asset files, colocated with each adapter and mirroring the
  install destination:
  - `packages/adapters/src/harness-pi/assets/agentctl.ts`
  - `packages/adapters/src/harness-claude/assets/.claude-plugin/plugin.json`
  - `packages/adapters/src/harness-claude/assets/hooks/hooks.json`
- `agentctl install` copies these files verbatim to their destinations.
- Remove `harness-pi/extension.ts`, `CLAUDE_HOOKS`, and the Claude JSON
  builders.
- Assets are embedded into the single `cli.cjs` bundle at build time (Rspack
  `asset/source`, `?raw` imports; native in vitest).
- Claude's `hooks.json` is now hand-authored (event order, not key-sorted); the
  install summary is derived by parsing the copied file.

## Capabilities

### New Capabilities

- `harness-hook-assets`: hook files live as standalone assets and are copied
  verbatim on install.

### Modified Capabilities

- `pi-state-hooks`: install copies the asset file.
- `claude-state-hooks`: install copies the asset files.

## Impact

- `packages/adapters` (both harness adapters, `index.ts` exports, tests),
  `apps/agentctl/rspack.config.ts`, biome/tsc config (assets excluded from
  checks).
- No change to installed content for pi; Claude `hooks.json` bytes change (key
  order only, semantically identical).
