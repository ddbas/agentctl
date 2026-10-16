## MODIFIED Requirements

### Requirement: agentctl install claude registers handlers via a Claude Code plugin

The `agentctl install claude` subcommand SHALL install agentctl's eleven hook
entries as a Claude Code plugin under `~/.claude/skills/agentctl/`, by copying
the standalone assets in `packages/adapters/src/harness-claude/assets/`
verbatim: `.claude-plugin/plugin.json` and `hooks/hooks.json`. The command
SHALL:

1. Create `~/.claude/skills/` if it does not already exist.
2. Delete `~/.claude/skills/agentctl/` and its full contents if it already
   exists — no diffing or in-place patching of an existing plugin installation.
3. Copy the asset files into `~/.claude/skills/agentctl/`.

`hooks/hooks.json` SHALL use the plugin hooks wrapper format
(`{ "hooks": { "<Event>": [ { "matcher"?: ..., "hooks": [ { "type": "command", "command": ..., "async"?: true } ] } ] } }`)
and SHALL register the same event/matcher/command/async combinations as before:
`SessionStart` and `SessionEnd` without `async`; `UserPromptSubmit`, both
`PreToolUse` entries, both `PostToolUse` entries, both `PostToolUseFailure`
entries, `Stop`, and `StopFailure` with `async: true`. The eleven entries are:
`SessionStart`, `UserPromptSubmit`, `PreToolUse` (unmatched and matched),
`PostToolUse` (unmatched and matched), `PostToolUseFailure` (unmatched and
matched), `Stop`, `StopFailure`, and `SessionEnd`. There is no `Notification`
registration. The install summary SHALL be derived from the copied `hooks.json`.

This command SHALL NOT read from, write to, or otherwise inspect
`~/.claude/settings.json`. It SHALL NOT attempt to detect, migrate, or clean up
any agentctl hook entries a previous `agentctl` version may have registered in
`~/.claude/settings.json`.

#### Scenario: First-time install with no prior plugin present

- **WHEN** the user runs `agentctl install claude` and neither
  `~/.claude/skills/` nor `~/.claude/skills/agentctl/` exists
- **THEN** `~/.claude/skills/` is created, `~/.claude/skills/agentctl/` is
  created containing copies of `.claude-plugin/plugin.json` and
  `hooks/hooks.json`, and `hooks/hooks.json` registers all eleven agentctl hook
  entries, including both `PostToolUseFailure` entries and no `Notification`
  entry

#### Scenario: Re-install replaces the existing plugin directory

- **WHEN** the user runs `agentctl install claude` and
  `~/.claude/skills/agentctl/` already exists (with any contents, including
  files not written by `agentctl`)
- **THEN** the existing directory and all its contents are deleted, and a fresh
  one is written containing exactly the two asset files

#### Scenario: Install does not touch settings.json

- **WHEN** the user runs `agentctl install claude` and `~/.claude/settings.json`
  exists with unrelated user configuration or with agentctl hook entries from a
  previous `agentctl` version
- **THEN** `~/.claude/settings.json` is not read or modified in any way

#### Scenario: Install creates the skills directory when absent

- **WHEN** the user runs `agentctl install claude` and `~/.claude/skills/` does
  not exist
- **THEN** `~/.claude/skills/` is created before the `agentctl` plugin directory
  is written
