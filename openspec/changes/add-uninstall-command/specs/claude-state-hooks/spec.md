## ADDED Requirements

### Requirement: rd uninstall claude removes only radr-managed hook entries

The `rd uninstall claude` subcommand SHALL remove radr-managed hook entries from
`~/.claude/settings.json`, recognizing both current-form commands
(`rd state set ...` / `rd state clear`) and pre-rename legacy commands
(`rd set-state ...` / `rd clear-state`) using the same recognition logic as
`rd install claude`'s migration path. All non-radr content in the file,
including non-radr hook entries under the same event and any other top-level
settings keys, SHALL be preserved unchanged. The command SHALL be idempotent:
running it again after all radr entries are removed SHALL make no further
changes.

#### Scenario: Uninstall removes all eight radr hook entries

- **WHEN** the user runs `rd uninstall claude` and `~/.claude/settings.json`
  contains the eight radr-managed hook entries installed by `rd install claude`
- **THEN** all eight entries are removed and the command prints a summary
  listing each removed entry (prefixed with `-`)

#### Scenario: Uninstall preserves non-radr hooks under the same event

- **WHEN** `~/.claude/settings.json` contains a non-radr hook entry under
  `UserPromptSubmit` alongside the radr-managed entry for that event
- **THEN** `rd uninstall claude` removes only the radr-managed entry and leaves
  the non-radr entry in place

#### Scenario: Uninstall removes legacy pre-rename entries

- **WHEN** `~/.claude/settings.json` contains radr hook entries using the
  pre-rename `rd set-state ...` / `rd clear-state` command strings (from an
  older `rd` binary, not yet migrated by `rd install claude`)
- **THEN** `rd uninstall claude` recognizes and removes those entries as
  radr-managed

#### Scenario: Uninstall prunes empty event arrays and the hooks key

- **WHEN** removing radr entries leaves an event's array empty
- **THEN** that event key is removed from `hooks`; if `hooks` becomes an empty
  object, the `hooks` key is removed from the settings file; the settings file
  itself is never deleted

#### Scenario: Uninstall is a no-op when no radr entries are present

- **WHEN** the user runs `rd uninstall claude` and `~/.claude/settings.json`
  contains no radr-managed hook entries (including when the file does not exist)
- **THEN** no filesystem write occurs, the command prints a summary indicating
  there was nothing to remove (prefixed with `=`), and exits 0

#### Scenario: Uninstall is idempotent

- **WHEN** the user runs `rd uninstall claude` twice in succession
- **THEN** the second run makes no further changes and reports nothing left to
  remove

#### Scenario: Uninstall does not touch multi-command hook entries

- **WHEN** a single hook entry's `hooks` array contains both a radr-managed
  command and a non-radr command mixed together
- **THEN** `rd uninstall claude` leaves that entry entirely untouched, since it
  cannot be safely attributed as solely radr-owned
