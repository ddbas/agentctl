## Decisions

- **Colocated `assets/` per adapter**, mirroring the destination tree, so the
  mapping asset → install path is obvious.
- **Build-time embedding** via `?raw` imports: the CLI remains one `cli.cjs`
  with no runtime asset lookup. Rspack: `resourceQuery: /raw/` → `asset/source`,
  and the swc rule skips `?raw`. Vitest/Vite supports `?raw` natively. A `*?raw`
  module declaration provides types.
- **Assets excluded from tsc and biome**: `agentctl.ts` imports pi types that
  are not dependencies of this repo, and assets must stay byte-exact.
- **Claude summary from parsed `hooks.json`**: keeps the file the single source
  of truth; output format unchanged.
