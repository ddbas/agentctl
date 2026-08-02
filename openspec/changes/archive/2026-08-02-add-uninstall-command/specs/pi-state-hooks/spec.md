## ADDED Requirements

### Requirement: rd uninstall pi removes the pi extension file

The `rd uninstall pi` subcommand SHALL delete `~/.pi/agent/extensions/radr.ts`
if it exists. If the file does not exist, the command SHALL exit 0 without error
and report that nothing needed removing. The command SHALL NOT delete the
`~/.pi/agent/extensions/` directory itself, even if it becomes empty.

#### Scenario: Uninstall removes an existing extension file

- **WHEN** the user runs `rd uninstall pi` and `~/.pi/agent/extensions/radr.ts`
  exists
- **THEN** the file is deleted and the command prints a summary indicating it
  was removed (prefixed with `-`), and exits 0

#### Scenario: Uninstall is a no-op when the extension file is absent

- **WHEN** the user runs `rd uninstall pi` and `~/.pi/agent/extensions/radr.ts`
  does not exist
- **THEN** no filesystem changes are made, the command prints a summary
  indicating there was nothing to remove (prefixed with `=`), and exits 0

#### Scenario: Uninstall removes the file even if its content was hand-modified

- **WHEN** `~/.pi/agent/extensions/radr.ts` exists but its content no longer
  matches what `rd install pi` currently generates
- **THEN** `rd uninstall pi` still deletes the file, since the file is
  documented as radr-managed regardless of content drift

#### Scenario: Uninstall leaves the extensions directory in place

- **WHEN** `rd uninstall pi` deletes `radr.ts` and no other files remain in
  `~/.pi/agent/extensions/`
- **THEN** the `~/.pi/agent/extensions/` directory itself is left in place (not
  removed)
