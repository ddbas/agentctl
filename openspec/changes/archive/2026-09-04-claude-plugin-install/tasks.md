## 1. Plugin content constants

- [x] 1.1 Add `AGENTCTL_CLAUDE_PLUGIN_NAME`/path helpers for
      `~/.claude/skills/agentctl/.claude-plugin/plugin.json` and
      `~/.claude/skills/agentctl/hooks/hooks.json`, mirroring
      `default_pi_extension_path`'s pattern of a resolver function so tests can
      supply a custom root.
- [x] 1.2 Add a `plugin.json` builder/constant (`name`, `description`) per
      design.md.
- [x] 1.3 Rewrite `build_claude_hook_entry`/hook-entry construction to target
      the `hooks.json` wrapper shape (`{ "hooks": { <event>: [...] }     }`)
      instead of a settings.json fragment, reusing the existing
      `AGENTCTL_CLAUDE_HOOKS` table (event, matcher, command, async) unchanged.

## 2. Replace install_claude

- [x] 2.1 Replace `install_claude_to`'s body: create `~/.claude/skills` if
      absent, `fs::remove_dir_all` `~/.claude/skills/agentctl` if it exists,
      then write `.claude-plugin/plugin.json` and `hooks/hooks.json` fresh via
      `fs::create_dir_all` + `fs::write`.
- [x] 2.2 Remove all settings.json read/parse logic from `install_claude_to` (no
      more reading `~/.claude/settings.json`, no JSON root merging).
- [x] 2.3 Update/replace the install summary output (new function replacing
      `format_claude_summary`) to report the eleven hooks as freshly written,
      dropping the `Updated`/`Unchanged`/`[removed]` states.
- [x] 2.4 Delete `HookStatus`, `is_agentctl_command`,
      `get_single_agentctl_command`, and the superseded-entry/stale-matcher
      cleanup passes — no longer reachable.

## 3. Replace uninstall_claude

- [x] 3.1 Replace `uninstall_claude_to`'s body: check whether
      `~/.claude/skills/agentctl` exists; if so, `fs::remove_dir_all` it; if
      not, no-op.
- [x] 3.2 Remove all settings.json read/parse/rewrite logic from
      `uninstall_claude_to`.
- [x] 3.3 Update/replace the uninstall summary output (new function replacing
      `format_claude_uninstall_summary`) to report whether the plugin directory
      was removed or was already absent.

## 4. Path resolution and module docs

- [x] 4.1 Replace `default_claude_settings_path` with a
      `default_claude_plugin_dir` (or equivalent) resolving
      `~/.claude/skills/agentctl`.
- [x] 4.2 Update the module-level doc comment at the top of
      `src/commands/hooks.rs` describing the `claude` harness to reflect the
      plugin-install mechanism instead of settings.json merging.

## 5. Tests

- [x] 5.1 Remove/replace unit tests covering settings.json merge, matcher
      staleness, superseded-entry removal, and legacy command-string migration
      (pre-rename
      `agentctl set-state`/`agentctl state set`/`agentctl agent status`/
      `agentctl agent watch` forms) — no longer applicable.
- [x] 5.2 Add unit tests for `install_claude_to`/equivalent against a
      `TempDir`-based plugin root: first-time install creates both files with
      correct content; re-install when the directory already exists (including
      with unrelated pre-existing files inside it) deletes and recreates it;
      `~/.claude/skills` is created when absent.
- [x] 5.3 Add unit tests asserting `hooks.json`'s written content contains all
      eleven event/matcher/command/async combinations and that `plugin.json` is
      valid JSON with the expected fields.
- [x] 5.4 Add unit tests for `uninstall_claude_to`/equivalent: removes an
      existing plugin directory; no-op (and exit 0) when absent; leaves sibling
      entries under `~/.claude/skills/` untouched.
- [x] 5.5 Run the full test suite via `mise run test` (or the project's
      configured test task) and confirm no regressions.

## 6. Spec sync

- [x] 6.1 Verify the delta spec at
      `openspec/changes/claude-plugin-install/specs/claude-state-hooks/spec.md`
      matches the implemented behavior (paths, remove-then-write semantics, no
      settings.json involvement) before archiving.
