## Why

`rd install claude` currently patches the user's `~/.claude/settings.json` in
place, which requires fragile merge/diff/migration logic (superseded-entry
detection, matcher-staleness checks, legacy command-string recognition across
four historical `rd` command renames) to avoid clobbering the user's own
settings. Claude Code now supports plugins, which let `rd` ship its hook
registrations as a self-contained, declarative bundle instead of mutating a
shared file. Since there is currently only one user of `rd`, now is the time to
replace the settings.json-patching approach with a plugin before more installs
accumulate history to migrate.

## What Changes

- **BREAKING**: `rd install claude` no longer reads or writes
  `~/.claude/settings.json`. It now installs a Claude Code plugin at
  `~/.claude/skills/radr/` containing `.claude-plugin/plugin.json` (plugin
  manifest) and `hooks/hooks.json` (the eleven radr hook entries).
- **BREAKING**: `rd uninstall claude` no longer removes entries from
  `~/.claude/settings.json`. It now removes the `~/.claude/skills/radr/`
  directory entirely.
- Install behavior is remove-then-write, not merge: `rd install claude` deletes
  any existing `~/.claude/skills/radr/` directory first, then writes the plugin
  fresh. No diffing, matcher-migration, or legacy-command-recognition logic is
  carried over.
- Remove all settings.json merge/cleanup logic for Claude: `ClaudeHookDef`'s
  settings.json-entry construction, `is_radr_command`, superseded-entry removal,
  stale-matcher updates, and the `HookStatus` (`New`/`Updated`/ `Unchanged`)
  tracking used only for that merge path.
- No migration or cleanup of any pre-existing `~/.claude/settings.json` hook
  entries from earlier `rd` versions is included in this change.

## Capabilities

### Modified Capabilities

- `claude-state-hooks`: the `rd install claude` and `rd uninstall claude`
  requirements change from settings.json merge/migrate semantics to plugin
  install/remove semantics (remove-then-write directory at
  `~/.claude/skills/radr/`, no settings.json involvement). The eleven individual
  hook-behavior requirements (`SessionStart` fires `SESSION_STARTED`, `Stop`
  fires `RUN_FINISHED`, etc.) are unchanged in substance, but now describe hooks
  sourced from the plugin's `hooks.json` instead of `settings.json`.

## Impact

- `src/commands/hooks.rs`: replace `install_claude`/`install_claude_to`,
  `uninstall_claude`/`uninstall_claude_to`, `RADR_CLAUDE_HOOKS`'s consumption
  path, `ClaudeHookDef`→settings.json entry construction,
  `is_radr_command`/`get_single_radr_command`, `HookStatus`, and the two
  `format_claude_*_summary` functions with plugin-file install/remove logic and
  new summary output.
- Existing unit tests covering settings.json merge/migration/cleanup behavior
  are replaced with tests covering plugin-directory install/remove.
- `openspec/specs/claude-state-hooks/spec.md`: update the `rd install claude`
  and `rd uninstall claude` requirements and scenarios.
- No changes to `rd install pi`/`rd uninstall pi`, `rd agent event`, or
  `rd agent clear`.
