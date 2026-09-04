## MODIFIED Requirements

### Requirement: rd install claude registers handlers via a Claude Code plugin

The `rd install claude` subcommand SHALL install radr's eleven hook entries as a
Claude Code plugin under `~/.claude/skills/radr/`, consisting of a plugin
manifest at `~/.claude/skills/radr/.claude-plugin/plugin.json` and a hooks
configuration at `~/.claude/skills/radr/hooks/hooks.json`. The command SHALL:

1. Create `~/.claude/skills/` if it does not already exist.
2. Delete `~/.claude/skills/radr/` and its full contents if it already exists
   (`fs::remove_dir_all` or equivalent) — no diffing or in-place patching of an
   existing plugin installation.
3. Write `.claude-plugin/plugin.json` and `hooks/hooks.json` fresh, containing
   the same eleven hook entries the prior `~/.claude/settings.json`-based
   implementation registered.

`hooks/hooks.json` SHALL use the plugin hooks wrapper format
(`{ "hooks": { "<Event>": [ { "matcher"?: ..., "hooks": [ { "type": "command", "command": ..., "async"?: true } ] } ] } }`)
and SHALL register the same event/matcher/command/async combinations as before:
`SessionStart` and `SessionEnd` without `async` (so they complete before Claude
proceeds); `UserPromptSubmit`, both `PreToolUse` entries, all three
`PostToolUse` entries, `Notification`, `Stop`, and `StopFailure` with
`async: true`.

This command SHALL NOT read from, write to, or otherwise inspect
`~/.claude/settings.json`. It SHALL NOT attempt to detect, migrate, or clean up
any radr hook entries a previous `rd` version may have registered in
`~/.claude/settings.json`.

#### Scenario: First-time install with no prior plugin present

- **WHEN** the user runs `rd install claude` and neither `~/.claude/skills/` nor
  `~/.claude/skills/radr/` exists
- **THEN** `~/.claude/skills/` is created, `~/.claude/skills/radr/` is created
  containing `.claude-plugin/plugin.json` and `hooks/hooks.json`, and
  `hooks/hooks.json` registers all eleven radr hook entries

#### Scenario: Re-install replaces the existing plugin directory

- **WHEN** the user runs `rd install claude` and `~/.claude/skills/radr/`
  already exists (from a prior install, with any contents, including files not
  written by `rd`)
- **THEN** the existing `~/.claude/skills/radr/` directory and all its contents
  are deleted, and a fresh `~/.claude/skills/radr/` is written containing
  exactly `.claude-plugin/plugin.json` and `hooks/hooks.json` with the current
  eleven hook entries

#### Scenario: Install does not touch settings.json

- **WHEN** the user runs `rd install claude` and `~/.claude/settings.json`
  exists with unrelated user configuration or with radr hook entries from a
  previous `rd` version
- **THEN** `~/.claude/settings.json` is not read or modified in any way

#### Scenario: Install creates the skills directory when absent

- **WHEN** the user runs `rd install claude` and `~/.claude/skills/` does not
  exist
- **THEN** `~/.claude/skills/` is created before the `radr` plugin directory is
  written

### Requirement: rd uninstall claude removes the radr plugin directory

The `rd uninstall claude` subcommand SHALL remove the `~/.claude/skills/radr/`
directory and all its contents if it exists. It SHALL NOT read from, write to,
or otherwise inspect `~/.claude/settings.json`, and SHALL NOT attempt to remove
any radr hook entries a previous `rd` version may have registered there. The
command SHALL be idempotent: running it again after the directory has been
removed (or when it never existed) SHALL make no further filesystem changes and
SHALL exit 0.

#### Scenario: Uninstall removes an existing plugin directory

- **WHEN** the user runs `rd uninstall claude` and `~/.claude/skills/radr/`
  exists
- **THEN** `~/.claude/skills/radr/` and all its contents are deleted, and the
  command exits 0

#### Scenario: Uninstall is a no-op when the plugin is not installed

- **WHEN** the user runs `rd uninstall claude` and `~/.claude/skills/radr/` does
  not exist (whether or not `~/.claude/skills/` itself exists)
- **THEN** no filesystem changes occur and the command exits 0

#### Scenario: Uninstall does not touch settings.json

- **WHEN** the user runs `rd uninstall claude` and `~/.claude/settings.json`
  contains radr hook entries from a previous `rd` version
- **THEN** `~/.claude/settings.json` is not read or modified in any way

#### Scenario: Uninstall leaves the skills directory itself in place

- **WHEN** the user runs `rd uninstall claude` and `~/.claude/skills/` contains
  other, non-radr entries alongside `radr/`
- **THEN** only `~/.claude/skills/radr/` is removed; `~/.claude/skills/` and its
  other contents are left untouched
