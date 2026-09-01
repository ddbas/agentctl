## ADDED Requirements

### Requirement: PreToolUse hook calls rd agent event TOOL_CALL_START for every tool call

The Claude `PreToolUse` hook SHALL additionally be registered without a tool
matcher (firing for every tool) to invoke a command that reads the hook's JSON
payload from stdin, extracts `tool_use_id`, and invokes
`rd agent event TOOL_CALL_START --id <tool_use_id> --harness claude` (async, no
`--type`), incrementing the pane's `active_tools` count for every tool call,
mirroring the pi extension's `tool_call` handler.

#### Scenario: Any tool call increments active_tools

- **WHEN** Claude fires the unmatched `PreToolUse` hook for any tool call and
  `$TMUX_PANE` is set
- **THEN** the state file's `active_tools` count is incremented by one

#### Scenario: Concurrent tool calls each get a distinct id

- **WHEN** Claude fires the unmatched `PreToolUse` hook for two tool calls
  running concurrently
- **THEN** each invocation uses its own `tool_use_id`, so both increments are
  recorded independently

### Requirement: PostToolUse hook calls rd agent event TOOL_CALL_RESULT for every tool call

The Claude `PostToolUse` hook SHALL additionally be registered without a tool
matcher (firing for every tool) to invoke a command that reads the hook's JSON
payload from stdin, extracts `tool_use_id`, and invokes
`rd agent event TOOL_CALL_RESULT --id <tool_use_id> --harness claude` (async, no
`--type`), decrementing the pane's `active_tools` count for every completed tool
call, mirroring the pi extension's `tool_result` handler. This entry is
independent of the existing unmatched `PostToolUse` →
`TOOL_CALL_RESULT --id "perm-<session_id>" --type elicitation` entry, which
continues to serve only permission-prompt block closure and is unaffected by
this requirement.

#### Scenario: Any tool call completion decrements active_tools

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

## MODIFIED Requirements

### Requirement: rd install claude registers handlers in user Claude settings

The `rd install claude` subcommand SHALL patch `~/.claude/settings.json` to
register eleven Claude hook entries using `rd agent event` and `rd agent clear`.
`SessionStart` and `SessionEnd` are registered without `async` so they complete
before Claude proceeds; all other hooks use `async: true`. The command SHALL be
idempotent and SHALL merge into any existing hook configuration without
overwriting it. If `~/.claude/settings.json` already contains hook entries
registered by an older `rd` binary using the pre-rename
`rd set-state`/`rd clear-state` commands, the subsequently renamed
`rd state set`/`rd state clear` commands, or the subsequently renamed
`rd agent status`/`rd agent clear` commands, those entries SHALL be recognized
as radr-managed, removed as superseded, and replaced with the equivalent
`rd agent event`/`rd agent clear` entries. If `~/.claude/settings.json` was
written by an `rd` binary that predates the two general tool-call-tracking
entries below, those two entries SHALL be added without disturbing any of the
nine entries already present.

The eleven entries are:

| Event              | Matcher                         | Command                                                                                                                           | Async |
| ------------------ | ------------------------------- | --------------------------------------------------------------------------------------------------------------------------------- | ----- |
| `SessionStart`     | `startup\|resume\|clear`        | reads stdin for `session_id`, then `rd agent event SESSION_STARTED --id <session_id> --harness claude`                            | no    |
| `UserPromptSubmit` | —                               | reads stdin for `session_id`, then `rd agent event RUN_STARTED --id <session_id> --harness claude`                                | yes   |
| `PreToolUse`       | — (unmatched)                   | reads stdin for `tool_use_id`, then `rd agent event TOOL_CALL_START --id <tool_use_id> --harness claude`                          | yes   |
| `PreToolUse`       | `AskUserQuestion\|ExitPlanMode` | reads stdin for `tool_use_id`, then `rd agent event TOOL_CALL_START --id <tool_use_id> --type elicitation --harness claude`       | yes   |
| `PostToolUse`      | — (unmatched)                   | reads stdin for `tool_use_id`, then `rd agent event TOOL_CALL_RESULT --id <tool_use_id> --harness claude`                         | yes   |
| `PostToolUse`      | `AskUserQuestion\|ExitPlanMode` | reads stdin for `tool_use_id`, then `rd agent event TOOL_CALL_RESULT --id <tool_use_id> --type elicitation --harness claude`      | yes   |
| `PostToolUse`      | — (unmatched)                   | reads stdin for `session_id`, then `rd agent event TOOL_CALL_RESULT --id "perm-<session_id>" --type elicitation --harness claude` | yes   |
| `Notification`     | `permission_prompt`             | reads stdin for `session_id`, then `rd agent event TOOL_CALL_START --id "perm-<session_id>" --type elicitation --harness claude`  | yes   |
| `Stop`             | —                               | reads stdin for `session_id`, then `rd agent event RUN_FINISHED --id <session_id> --harness claude`                               | yes   |
| `StopFailure`      | —                               | reads stdin for `session_id`, then `rd agent event RUN_FINISHED --id <session_id> --harness claude`                               | yes   |
| `SessionEnd`       | —                               | `rd agent clear`                                                                                                                  | no    |

Note: `PreToolUse` and `PostToolUse` each appear twice — once unmatched (every
tool, untyped, drives `active_tools`) and once matched (interactive tools only,
`--type elicitation`, drives `blocked` status) — plus `PostToolUse` appears a
third time unmatched with the `"perm-<session_id>"` id to close out
permission-prompt blocks, for three `PostToolUse` entries total.

#### Scenario: First-time install on a clean settings file

- **WHEN** the user runs `rd install claude` and `~/.claude/settings.json`
  contains no radr hook entries
- **THEN** the file is updated with all eleven entries listed above

#### Scenario: Idempotent re-install

- **WHEN** the user runs `rd install claude` a second time
- **THEN** no duplicate entries are added and the file content is identical to
  after the first run

#### Scenario: Install merges with existing hooks

- **WHEN** `~/.claude/settings.json` already contains non-radr hook entries
- **THEN** those existing entries are preserved and the radr entries are added
  alongside them

#### Scenario: Install migrates pre-rename hook commands

- **WHEN** `~/.claude/settings.json` contains radr hook entries using the old
  `rd set-state ...` / `rd clear-state` commands (from a `rd` binary installed
  before any of the renames)
- **THEN** running `rd install claude` removes the old entries and adds the
  equivalent new-form entries, with no duplicate or leftover old-style entries
  remaining

#### Scenario: Install migrates rd state to rd agent hook commands

- **WHEN** `~/.claude/settings.json` contains radr hook entries using the
  intermediate `rd state set ...` / `rd state clear` commands
- **THEN** running `rd install claude` removes the old entries and adds the
  equivalent new-form entries, with no duplicate or leftover old-style entries
  remaining

#### Scenario: Install migrates rd agent status to rd agent event hook commands

- **WHEN** `~/.claude/settings.json` contains radr hook entries using the
  `rd agent status ...` / `rd agent clear` commands (from a `rd` binary
  installed after the `state`→`agent` rename but before this `status`→`event`
  rename)
- **THEN** running `rd install claude` removes the old entries and adds the
  equivalent `rd agent event ...` / `rd agent clear` entries, with no duplicate
  or leftover old-style entries remaining

#### Scenario: Install adds the two tool-call-tracking entries to a nine-entry file

- **WHEN** `~/.claude/settings.json` contains the nine radr hook entries
  installed by an `rd` binary that predates the general tool-call-tracking
  entries, and none of the nine entries' commands, matchers, or async flags
  differ from the current table
- **THEN** running `rd install claude` adds the two new unmatched
  `PreToolUse`/`PostToolUse` entries, leaves the nine existing entries
  unchanged, and the file ends up with eleven radr entries total

### Requirement: rd uninstall claude removes only radr-managed hook entries

The `rd uninstall claude` subcommand SHALL remove radr-managed hook entries from
`~/.claude/settings.json`, recognizing the current-form commands
(`rd agent event ...` / `rd agent clear`) as well as all prior legacy forms
(`rd agent status ...` / `rd agent clear`, `rd state set ...` /
`rd state clear`, and the earlier `rd set-state ...` / `rd clear-state`) using
the same recognition logic as `rd install claude`'s migration path. All non-radr
content in the file, including non-radr hook entries under the same event and
any other top-level settings keys, SHALL be preserved unchanged. The command
SHALL be idempotent: running it again after all radr entries are removed SHALL
make no further changes.

#### Scenario: Uninstall removes all eleven radr hook entries

- **WHEN** the user runs `rd uninstall claude` and `~/.claude/settings.json`
  contains the eleven radr-managed hook entries installed by `rd install claude`
- **THEN** all eleven entries are removed and the command prints a summary
  listing each removed entry (prefixed with `-`)

#### Scenario: Uninstall preserves non-radr hooks under the same event

- **WHEN** `~/.claude/settings.json` contains a non-radr hook entry under
  `UserPromptSubmit` alongside the radr-managed entry for that event
- **THEN** `rd uninstall claude` removes only the radr-managed entry and leaves
  the non-radr entry in place

#### Scenario: Uninstall removes legacy pre-rename entries

- **WHEN** `~/.claude/settings.json` contains radr hook entries using any of the
  earlier `rd set-state ...` / `rd clear-state`, `rd state set ...` /
  `rd state clear`, or `rd agent status ...` / `rd agent clear` command strings
  (from an older `rd` binary, not yet migrated by `rd install claude`)
- **THEN** `rd uninstall claude` recognizes and removes those entries as
  radr-managed

#### Scenario: Uninstall prunes empty event arrays and the hooks key

- **WHEN** removing radr entries leaves an event's array empty
- **THEN** that event key is removed from `hooks`; if `hooks` becomes an empty
  object, the `hooks` key is removed from the settings file; the settings file
  itself is never deleted
