## ADDED Requirements

### Requirement: agentctl uninstall claude removes only agentctl-managed hook entries

The `agentctl uninstall claude` subcommand SHALL remove agentctl-managed hook
entries from `~/.claude/settings.json`, recognizing both current-form commands
(`agentctl state set ...` / `agentctl state clear`) and pre-rename legacy
commands (`agentctl set-state ...` / `agentctl clear-state`) using the same
recognition logic as `agentctl install claude`'s migration path. All
non-agentctl content in the file, including non-agentctl hook entries under the
same event and any other top-level settings keys, SHALL be preserved unchanged.
The command SHALL be idempotent: running it again after all agentctl entries are
removed SHALL make no further changes.

#### Scenario: Uninstall removes all eight agentctl hook entries

- **WHEN** the user runs `agentctl uninstall claude` and
  `~/.claude/settings.json` contains the eight agentctl-managed hook entries
  installed by `agentctl install claude`
- **THEN** all eight entries are removed and the command prints a summary
  listing each removed entry (prefixed with `-`)

#### Scenario: Uninstall preserves non-agentctl hooks under the same event

- **WHEN** `~/.claude/settings.json` contains a non-agentctl hook entry under
  `UserPromptSubmit` alongside the agentctl-managed entry for that event
- **THEN** `agentctl uninstall claude` removes only the agentctl-managed entry
  and leaves the non-agentctl entry in place

#### Scenario: Uninstall removes legacy pre-rename entries

- **WHEN** `~/.claude/settings.json` contains agentctl hook entries using the
  pre-rename `agentctl set-state ...` / `agentctl clear-state` command strings
  (from an older `agentctl` binary, not yet migrated by
  `agentctl install claude`)
- **THEN** `agentctl uninstall claude` recognizes and removes those entries as
  agentctl-managed

#### Scenario: Uninstall prunes empty event arrays and the hooks key

- **WHEN** removing agentctl entries leaves an event's array empty
- **THEN** that event key is removed from `hooks`; if `hooks` becomes an empty
  object, the `hooks` key is removed from the settings file; the settings file
  itself is never deleted

#### Scenario: Uninstall is a no-op when no agentctl entries are present

- **WHEN** the user runs `agentctl uninstall claude` and
  `~/.claude/settings.json` contains no agentctl-managed hook entries (including
  when the file does not exist)
- **THEN** no filesystem write occurs, the command prints a summary indicating
  there was nothing to remove (prefixed with `=`), and exits 0

#### Scenario: Uninstall is idempotent

- **WHEN** the user runs `agentctl uninstall claude` twice in succession
- **THEN** the second run makes no further changes and reports nothing left to
  remove

#### Scenario: Uninstall does not touch multi-command hook entries

- **WHEN** a single hook entry's `hooks` array contains both a agentctl-managed
  command and a non-agentctl command mixed together
- **THEN** `agentctl uninstall claude` leaves that entry entirely untouched,
  since it cannot be safely attributed as solely agentctl-owned
