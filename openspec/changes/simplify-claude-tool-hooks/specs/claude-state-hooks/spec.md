## ADDED Requirements

### Requirement: PostToolUseFailure hook calls rd agent event TOOL_CALL_RESULT for every failed tool call

The Claude `PostToolUseFailure` hook SHALL be registered without a tool matcher
(firing for every tool whose call fails) to invoke a command that reads the
hook's JSON payload from stdin, extracts `tool_use_id`, and invokes
`rd agent event TOOL_CALL_RESULT --id <tool_use_id> --harness claude` (async, no
`--type`), decrementing the pane's `active_tools` count for every failed tool
call, exactly mirroring the untyped `PostToolUse` registration. This closes out
the `TOOL_CALL_START` opened by the unmatched `PreToolUse` registration for tool
calls that fail rather than succeed, since Claude Code fires exactly one of
`PostToolUse`/`PostToolUseFailure` per tool call, never both.

#### Scenario: A failed tool call still decrements active_tools

- **WHEN** Claude fires the unmatched `PostToolUseFailure` hook for a tool call
  that errored (e.g. a nonzero Bash exit or an MCP tool error) and `$TMUX_PANE`
  is set
- **THEN** the state file's `active_tools` count is decremented by one (never
  below zero)

#### Scenario: Success and failure never both fire for the same tool call

- **WHEN** a single tool call completes
- **THEN** Claude Code fires exactly one of the unmatched `PostToolUse` or
  unmatched `PostToolUseFailure` hooks for that `tool_use_id`, never both, so
  `active_tools` is decremented exactly once per completed tool call regardless
  of outcome

### Requirement: PostToolUseFailure hook calls rd agent event TOOL_CALL_RESULT for failed interactive tools

The Claude `PostToolUseFailure` hook SHALL invoke a command that reads the
hook's JSON payload from stdin, extracts `tool_use_id`, and invokes
`rd agent event TOOL_CALL_RESULT --id <tool_use_id> --type elicitation --harness claude`
(async) when registered with `matcher: "AskUserQuestion|ExitPlanMode"`, closing
out the `blocked` status opened by the matching `PreToolUse` registration for
the same tool call, for the (rare) case where an interactive tool call itself
fails rather than completing normally.

#### Scenario: AskUserQuestion tool call fails inside tmux

- **WHEN** Claude's `AskUserQuestion` tool call fails and Claude fires the
  matched `PostToolUseFailure` hook, and `$TMUX_PANE` is set
- **THEN** the agent's state file is updated to `"status": "working"`

## MODIFIED Requirements

### Requirement: PostToolUse hook calls rd agent event TOOL_CALL_RESULT for every tool call

The Claude `PostToolUse` hook SHALL additionally be registered without a tool
matcher (firing for every tool) to invoke a command that reads the hook's JSON
payload from stdin, extracts `tool_use_id`, and invokes
`rd agent event TOOL_CALL_RESULT --id <tool_use_id> --harness claude` (async, no
`--type`), decrementing the pane's `active_tools` count for every completed tool
call, mirroring the pi extension's `tool_result` handler. This entry only fires
for tool calls that succeed; failed tool calls are covered by the matching
unmatched `PostToolUseFailure` registration instead, since Claude Code fires
exactly one of the two per tool call.

#### Scenario: Any successful tool call completion decrements active_tools

- **WHEN** Claude fires the unmatched `PostToolUse` hook for any tool call and
  `$TMUX_PANE` is set
- **THEN** the state file's `active_tools` count is decremented by one (never
  below zero)

#### Scenario: Interactive tool calls still track active_tools alongside blocked status

- **WHEN** Claude fires `PreToolUse`/`PostToolUse` for `AskUserQuestion` or
  `ExitPlanMode`
- **THEN** both the elicitation-typed entries (driving `blocked` status) and the
  general untyped entries (driving `active_tools`) fire for that same tool call,
  each independently

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
proceeds); `UserPromptSubmit`, both `PreToolUse` entries, both `PostToolUse`
entries, both `PostToolUseFailure` entries, `Stop`, and `StopFailure` with
`async: true`. The eleven entries are: `SessionStart`, `UserPromptSubmit`,
`PreToolUse` (unmatched and matched), `PostToolUse` (unmatched and matched),
`PostToolUseFailure` (unmatched and matched), `Stop`, `StopFailure`, and
`SessionEnd`. There is no `Notification` registration.

This command SHALL NOT read from, write to, or otherwise inspect
`~/.claude/settings.json`. It SHALL NOT attempt to detect, migrate, or clean up
any radr hook entries a previous `rd` version may have registered in
`~/.claude/settings.json`.

#### Scenario: First-time install with no prior plugin present

- **WHEN** the user runs `rd install claude` and neither `~/.claude/skills/` nor
  `~/.claude/skills/radr/` exists
- **THEN** `~/.claude/skills/` is created, `~/.claude/skills/radr/` is created
  containing `.claude-plugin/plugin.json` and `hooks/hooks.json`, and
  `hooks/hooks.json` registers all eleven radr hook entries, including both
  `PostToolUseFailure` entries and no `Notification` entry

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

## REMOVED Requirements

### Requirement: Unmatched PostToolUse hook closes out permission-prompt blocks

**Reason**: The `Notification`/`PostToolUse` id pairing this requirement
depended on was broken by construction — each side independently evaluated its
own `$RANDOM`, so the opening id (from `Notification`) and the closing id (from
this `PostToolUse` registration) could never match. `PostToolUseFailure` now
closes every `TOOL_CALL_START` that `PostToolUse` doesn't (i.e. on tool call
failure), which was the actual gap in coverage; the permission-prompt mechanism
itself is removed rather than repaired.

**Migration**: None required for users; `rd install claude` regenerates
`hooks/hooks.json` without this entry on next run. No consumer of `events.jsonl`
should have depended on `permission-prompt`-typed events, since they were never
correctly correlated.

### Requirement: Notification hook calls rd agent event TOOL_CALL_START on permission prompts

**Reason**: Same root cause as above — the id this hook generated
(`"perm-<session_id>-<nonce>"`) could never be matched by the corresponding
`PostToolUse` "close" entry, since each was computed independently via `$RANDOM`
in a separate shell subprocess. Removing the hook removes a source of
permanently-unresolved `blocked` state and orphaned `events.jsonl` entries
rather than attempting to fix the id correlation.

**Migration**: None required for users; `rd install claude` regenerates
`hooks/hooks.json` without this entry on next run. Any pane currently stuck in
`blocked` status due to an orphaned permission-prompt open should be cleared by
its next `SessionStart`, `Stop`, or `StopFailure` event as usual.
