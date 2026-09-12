## Why

`agentctl install claude` currently patches the user's `~/.claude/settings.json`
in place, which requires fragile merge/diff/migration logic (superseded-entry
detection, matcher-staleness checks, legacy command-string recognition across
four historical `agentctl` command renames) to avoid clobbering the user's own
settings. Claude Code now supports plugins, which let `agentctl` ship its hook
registrations as a self-contained, declarative bundle instead of mutating a
shared file. Since there is currently only one user of `agentctl`, now is the
time to replace the settings.json-patching approach with a plugin before more
installs accumulate history to migrate.

## What Changes

- **BREAKING**: `agentctl install claude` no longer reads or writes
  `~/.claude/settings.json`. It now installs a Claude Code plugin at
  `~/.claude/skills/agentctl/` containing `.claude-plugin/plugin.json` (plugin
  manifest) and `hooks/hooks.json` (the eleven agentctl hook entries).
- **BREAKING**: `agentctl uninstall claude` no longer removes entries from
  `~/.claude/settings.json`. It now removes the `~/.claude/skills/agentctl/`
  directory entirely.
- Install behavior is remove-then-write, not merge: `agentctl install claude`
  deletes any existing `~/.claude/skills/agentctl/` directory first, then writes
  the plugin fresh. No diffing, matcher-migration, or legacy-command-recognition
  logic is carried over.
- Remove all settings.json merge/cleanup logic for Claude: `ClaudeHookDef`'s
  settings.json-entry construction, `is_agentctl_command`, superseded-entry
  removal, stale-matcher updates, and the `HookStatus` (`New`/`Updated`/
  `Unchanged`) tracking used only for that merge path.
- No migration or cleanup of any pre-existing `~/.claude/settings.json` hook
  entries from earlier `agentctl` versions is included in this change.

## Capabilities

### Modified Capabilities

- `claude-state-hooks`: the `agentctl install claude` and
  `agentctl uninstall claude` requirements change from settings.json
  merge/migrate semantics to plugin install/remove semantics (remove-then-write
  directory at `~/.claude/skills/agentctl/`, no settings.json involvement). The
  eleven individual hook-behavior requirements (`SessionStart` fires
  `SESSION_STARTED`, `Stop` fires `RUN_FINISHED`, etc.) are unchanged in
  substance, but now describe hooks sourced from the plugin's `hooks.json`
  instead of `settings.json`.

## Impact

- `src/commands/hooks.rs`: replace `install_claude`/`install_claude_to`,
  `uninstall_claude`/`uninstall_claude_to`, `AGENTCTL_CLAUDE_HOOKS`'s
  consumption path, `ClaudeHookDef`→settings.json entry construction,
  `is_agentctl_command`/`get_single_agentctl_command`, `HookStatus`, and the two
  `format_claude_*_summary` functions with plugin-file install/remove logic and
  new summary output.
- Existing unit tests covering settings.json merge/migration/cleanup behavior
  are replaced with tests covering plugin-directory install/remove.
- `openspec/specs/claude-state-hooks/spec.md`: update the
  `agentctl install claude` and `agentctl uninstall claude` requirements and
  scenarios.
- No changes to `agentctl install pi`/`agentctl uninstall pi`,
  `agentctl agent event`, or `agentctl agent clear`.
