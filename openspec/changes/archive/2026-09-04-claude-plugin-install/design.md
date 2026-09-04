## Context

`rd install claude` / `rd uninstall claude` currently read-modify-write
`~/.claude/settings.json`, a file also used for arbitrary user configuration.
Because the file is shared and free-form, the current implementation carries
substantial defensive logic: a `radr_lookup` table to detect "superseded"
entries, per-entry matcher-staleness diffing, an `is_radr_command` classifier
that recognizes five historical command-string forms across renames
(`rd set-state`/`rd clear-state` → `rd state set`/`rd state clear` →
`rd agent status`/`rd agent clear` → `rd agent event`/`rd agent clear`, plus an
orphaned never-shipped `rd agent watch`/`rd agent stop` form), and a three-state
`HookStatus` (`New`/`Updated`/`Unchanged`) purely to report what the merge did.

Claude Code now supports plugins (`https://code.claude.com/docs/en/plugins`): a
plugin is a directory containing a `.claude-plugin/plugin.json` manifest and a
`hooks/hooks.json` file with the wrapper shape
`{ "hooks": { <event>: [...] } }`. Per this project's `AGENTS.md`, there is
currently exactly one user of `rd`, so this change treats the plugin-based
install as a first implementation rather than a migration: no logic is added to
detect or clean up hook entries left in `~/.claude/settings.json` by prior `rd`
versions. The user will remove those manually.

## Goals / Non-Goals

**Goals:**

- `rd install claude` writes a self-contained Claude Code plugin at
  `~/.claude/skills/radr/` (per the user's explicit path choice) containing
  `.claude-plugin/plugin.json` and `hooks/hooks.json`, registering the same
  eleven hook entries the current implementation registers in `settings.json`.
- `rd uninstall claude` removes `~/.claude/skills/radr/` entirely.
- Both commands are simple and idempotent by construction: install always
  deletes-then-writes, uninstall always removes-if-present.
- Delete all now-dead settings.json merge/diff/migration code and its unit
  tests.

**Non-Goals:**

- Migrating or cleaning up any radr hook entries an older `rd` binary left in
  `~/.claude/settings.json`. Out of scope per the proposal; the sole current
  user will clean this up by hand.
- Registering the plugin with Claude Code's plugin marketplace/config (e.g. any
  enablement list) beyond writing the files. The proposal's explicit, literal
  three-step install procedure is what's being implemented, and Claude Code
  auto-discovers plugins present under the directory it scans (see Open
  Questions).
- Changing any hook's event, matcher, command body, or async flag — the eleven
  entries' content is carried over unchanged, only its storage location and
  installer mechanism change.

## Decisions

### Decision: Remove-then-write instead of merge

`install_claude_to` will (1) create `~/.claude/skills` if absent, (2) delete
`~/.claude/skills/radr` if it already exists (`fs::remove_dir_all`), (3) write
`.claude-plugin/plugin.json` and `hooks/hooks.json` fresh via
`fs::create_dir_all` + `fs::write`.

**Alternative considered**: keep merge semantics (read existing `hooks.json`,
diff, patch) to preserve any user hand-edits to the plugin. Rejected — the
plugin directory is entirely radr-owned (nothing else should write there), so
there is nothing to merge with, and the proposal explicitly asks for
remove-and-recreate to avoid carrying over diffing logic.

### Decision: Hook definitions move from settings.json-entry builders to hooks.json builders

`RADR_CLAUDE_HOOKS: &[ClaudeHookDef]` (event, optional matcher, command, async
flag) is retained as the single source of truth for the eleven hook entries, but
`build_claude_hook_entry` now emits into a `hooks.json`-shaped
`{ "hooks": { <event>: [ { "matcher"?, "hooks": [ { "type": "command", "command", "async"? } ] } ] } }`
document instead of contributing to a settings.json `hooks` map merged with
existing content. Multiple `ClaudeHookDef` entries sharing an event (e.g. the
three `PostToolUse` entries) still each become a separate array entry under that
event key, exactly as today.

### Decision: Delete `is_radr_command`, `HookStatus`, and superseded-entry cleanup

These existed solely to make settings.json merging idempotent and
migration-safe. With a dedicated, radr-owned plugin directory, "idempotent"
reduces to "delete then rewrite the same bytes," so none of that classification
logic is needed. Deleted entirely, not deprecated in place.

### Decision: `plugin.json` is a minimal manifest

```json
{
  "name": "radr",
  "description": "radr agent-state lifecycle hooks for Claude Code"
}
```

No `version`, `author`, or other optional fields — nothing in the proposal
requires them, and per Claude Code's plugin manifest reference the manifest is
otherwise optional/best-effort metadata. `hooks/hooks.json` is the default hooks
path, so `plugin.json` does not need an explicit `hooks` field pointing at it.

### Decision: Summary output changes shape, keeping the `+`/`-`/`=` prefix convention

`rd install claude` prints a summary listing the eleven hooks as newly written
(no more `~` "updated matcher" state, since there is no partial update path).
`rd uninstall claude` prints whether the plugin directory was removed or absent.
Exact wording is an implementation detail, not specified further here.

## Risks / Trade-offs

- [Deleting `~/.claude/skills/radr` on every install destroys any local edits
  the user made directly to the installed plugin files] → Acceptable: the
  directory is documented as fully `rd`-managed and regenerated on every
  install, matching the existing `rd install pi` contract for
  `~/.pi/agent/extensions/radr.ts`.
- [Stale hook entries left in `~/.claude/settings.json` by a previous `rd`
  version continue to fire after this change ships, potentially double-firing
  events alongside the new plugin] → Out of scope per the proposal; the sole
  current user will manually remove them. Documented as a non-goal above.
- [`~/.claude/skills/` is not documented as a Claude Code plugin-discovery root
  in all Claude Code versions] → Tracked as an open question; this design
  implements the proposal's literal, explicitly-requested path.

## Open Questions

- Does Claude Code auto-discover plugins placed directly under
  `~/.claude/skills/<name>/`, or does it require separate registration (e.g. a
  marketplace config or an enabled-plugins list) for the plugin's hooks to
  actually take effect? The proposal specifies only writing the three files; if
  discovery requires an additional registration step, that would need a
  follow-up change once confirmed.
