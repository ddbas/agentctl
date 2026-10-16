## 1. Assets

- [x] 1.1 Add `harness-pi/assets/agentctl.ts`
- [x] 1.2 Add `harness-claude/assets/.claude-plugin/plugin.json` and
      `hooks/hooks.json`

## 2. Code

- [x] 2.1 Rspack `?raw` rule; `*?raw` type declaration; exclude assets from tsc
      and biome
- [x] 2.2 Rewrite pi and Claude adapters to copy assets; delete `extension.ts`,
      `CLAUDE_HOOKS`, JSON builders; update `index.ts` exports
- [x] 2.3 Update tests

## 3. Verify

- [x] 3.1 Run full CI via mise; `openspec validate standalone-hook-assets`
