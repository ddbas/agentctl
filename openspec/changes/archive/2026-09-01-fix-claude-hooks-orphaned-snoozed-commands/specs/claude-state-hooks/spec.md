## MODIFIED Requirements

### Requirement: agentctl install claude registers handlers in user Claude settings

The `agentctl install claude` subcommand SHALL patch `~/.claude/settings.json`
to register nine Claude hook entries using `agentctl agent event` and
`agentctl agent clear`. `SessionStart` and `SessionEnd` are registered without
`async` so they complete before Claude proceeds; all other hooks use
`async: true`. The command SHALL be idempotent and SHALL merge into any existing
hook configuration without overwriting it. If `~/.claude/settings.json` already
contains hook entries registered by an older `agentctl` binary using the
pre-rename `agentctl set-state`/`agentctl clear-state` commands, the
subsequently renamed `agentctl state set`/`agentctl state clear` commands, or
the subsequently renamed `agentctl agent status`/`agentctl agent clear`
commands, those entries SHALL be recognized as agentctl-managed, removed as
superseded, and replaced with the equivalent
`agentctl agent event`/`agentctl agent clear` entries. If
`~/.claude/settings.json` contains hook entries using
`agentctl agent watch arm`, `agentctl agent watch resolve`, or
`agentctl agent stop` command strings (from a build of `agentctl` implementing a
feature never shipped on the mainline), those entries SHALL be recognized as
agentctl-managed and removed as superseded, with no replacement entries added
since no current hook set corresponds to them.

The nine entries are:

| Event              | Matcher                         | Command                                                                                                                                 | Async |
| ------------------ | ------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- | ----- |
| `SessionStart`     | `startup\|resume\|clear`        | reads stdin for `session_id`, then `agentctl agent event SESSION_STARTED --id <session_id> --harness claude`                            | no    |
| `UserPromptSubmit` | —                               | reads stdin for `session_id`, then `agentctl agent event RUN_STARTED --id <session_id> --harness claude`                                | yes   |
| `PreToolUse`       | `AskUserQuestion\|ExitPlanMode` | reads stdin for `tool_use_id`, then `agentctl agent event TOOL_CALL_START --id <tool_use_id> --type elicitation --harness claude`       | yes   |
| `PostToolUse`      | `AskUserQuestion\|ExitPlanMode` | reads stdin for `tool_use_id`, then `agentctl agent event TOOL_CALL_RESULT --id <tool_use_id> --type elicitation --harness claude`      | yes   |
| `PostToolUse`      | — (unmatched)                   | reads stdin for `session_id`, then `agentctl agent event TOOL_CALL_RESULT --id "perm-<session_id>" --type elicitation --harness claude` | yes   |
| `Notification`     | `permission_prompt`             | reads stdin for `session_id`, then `agentctl agent event TOOL_CALL_START --id "perm-<session_id>" --type elicitation --harness claude`  | yes   |
| `Stop`             | —                               | reads stdin for `session_id`, then `agentctl agent event RUN_FINISHED --id <session_id> --harness claude`                               | yes   |
| `StopFailure`      | —                               | reads stdin for `session_id`, then `agentctl agent event RUN_FINISHED --id <session_id> --harness claude`                               | yes   |
| `SessionEnd`       | —                               | `agentctl agent clear`                                                                                                                  | no    |

#### Scenario: First-time install on a clean settings file

- **WHEN** the user runs `agentctl install claude` and `~/.claude/settings.json`
  contains no agentctl hook entries
- **THEN** the file is updated with all nine entries listed above

#### Scenario: Idempotent re-install

- **WHEN** the user runs `agentctl install claude` a second time
- **THEN** no duplicate entries are added and the file content is identical to
  after the first run

#### Scenario: Install merges with existing hooks

- **WHEN** `~/.claude/settings.json` already contains non-agentctl hook entries
- **THEN** those existing entries are preserved and the agentctl entries are
  added alongside them

#### Scenario: Install migrates pre-rename hook commands

- **WHEN** `~/.claude/settings.json` contains agentctl hook entries using the
  old `agentctl set-state ...` / `agentctl clear-state` commands (from a
  `agentctl` binary installed before any of the renames)
- **THEN** running `agentctl install claude` removes the old entries and adds
  the equivalent new-form entries, with no duplicate or leftover old-style
  entries remaining

#### Scenario: Install migrates agentctl state to agentctl agent hook commands

- **WHEN** `~/.claude/settings.json` contains agentctl hook entries using the
  intermediate `agentctl state set ...` / `agentctl state clear` commands
- **THEN** running `agentctl install claude` removes the old entries and adds
  the equivalent new-form entries, with no duplicate or leftover old-style
  entries remaining

#### Scenario: Install migrates agentctl agent status to agentctl agent event hook commands

- **WHEN** `~/.claude/settings.json` contains agentctl hook entries using the
  `agentctl agent status ...` / `agentctl agent clear` commands (from a
  `agentctl` binary installed after the `state`→`agent` rename but before this
  `status`→`event` rename)
- **THEN** running `agentctl install claude` removes the old entries and adds
  the equivalent `agentctl agent event ...` / `agentctl agent clear` entries,
  with no duplicate or leftover old-style entries remaining

#### Scenario: Install removes orphaned Monitor-watch and stop entries with no replacement

- **WHEN** `~/.claude/settings.json` contains a `PreToolUse` entry (matcher
  `Monitor`) invoking `agentctl agent watch arm --harness claude`, a
  `PostToolUse` entry (matcher `Monitor`) invoking
  `agentctl agent watch resolve --harness claude`, and `Stop`/`StopFailure`
  entries invoking `agentctl agent stop --harness claude` (left over from a
  build of `agentctl` implementing a feature never shipped on the mainline)
- **THEN** running `agentctl install claude` removes all four entries, reports
  each in the summary's `[removed]` list, adds no replacement entries for them,
  and still adds/leaves-unchanged the nine current entries listed above

### Requirement: agentctl uninstall claude removes only agentctl-managed hook entries

The `agentctl uninstall claude` subcommand SHALL remove agentctl-managed hook
entries from `~/.claude/settings.json`, recognizing the current-form commands
(`agentctl agent event ...` / `agentctl agent clear`) as well as all prior
legacy forms (`agentctl agent status ...` / `agentctl agent clear`,
`agentctl state set ...` / `agentctl state clear`, the earlier
`agentctl set-state ...` / `agentctl clear-state`, and the never-shipped
`agentctl agent watch arm`/`agentctl agent watch resolve`/`agentctl agent stop`
forms) using the same recognition logic as `agentctl install claude`'s migration
path. All non-agentctl content in the file, including non-agentctl hook entries
under the same event and any other top-level settings keys, SHALL be preserved
unchanged. The command SHALL be idempotent: running it again after all agentctl
entries are removed SHALL make no further changes.

#### Scenario: Uninstall removes all nine agentctl hook entries

- **WHEN** the user runs `agentctl uninstall claude` and
  `~/.claude/settings.json` contains the nine agentctl-managed hook entries
  installed by `agentctl install claude`
- **THEN** all nine entries are removed and the command prints a summary listing
  each removed entry (prefixed with `-`)

#### Scenario: Uninstall preserves non-agentctl hooks under the same event

- **WHEN** `~/.claude/settings.json` contains a non-agentctl hook entry under
  `UserPromptSubmit` alongside the agentctl-managed entry for that event
- **THEN** `agentctl uninstall claude` removes only the agentctl-managed entry
  and leaves the non-agentctl entry in place

#### Scenario: Uninstall removes legacy pre-rename entries

- **WHEN** `~/.claude/settings.json` contains agentctl hook entries using any of
  the earlier `agentctl set-state ...` / `agentctl clear-state`,
  `agentctl state set ...` / `agentctl state clear`, or
  `agentctl agent status ...` / `agentctl agent clear` command strings (from an
  older `agentctl` binary, not yet migrated by `agentctl install claude`)
- **THEN** `agentctl uninstall claude` recognizes and removes those entries as
  agentctl-managed

#### Scenario: Uninstall removes orphaned Monitor-watch and stop entries

- **WHEN** `~/.claude/settings.json` contains hook entries invoking
  `agentctl agent watch arm --harness claude`,
  `agentctl agent watch resolve --harness claude`, or
  `agentctl agent stop --harness claude`
- **THEN** `agentctl uninstall claude` recognizes and removes those entries as
  agentctl-managed, alongside any other agentctl entries present

#### Scenario: Uninstall prunes empty event arrays and the hooks key

- **WHEN** removing agentctl entries leaves an event's array empty
- **THEN** that event key is removed from `hooks`; if `hooks` becomes an empty
  object, the `hooks` key is removed from the settings file; the settings file
  itself is never deleted
