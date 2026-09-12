## ADDED Requirements

### Requirement: agentctl uninstall pi removes the pi extension file

The `agentctl uninstall pi` subcommand SHALL delete
`~/.pi/agent/extensions/agentctl.ts` if it exists. If the file does not exist,
the command SHALL exit 0 without error and report that nothing needed removing.
The command SHALL NOT delete the `~/.pi/agent/extensions/` directory itself,
even if it becomes empty.

#### Scenario: Uninstall removes an existing extension file

- **WHEN** the user runs `agentctl uninstall pi` and
  `~/.pi/agent/extensions/agentctl.ts` exists
- **THEN** the file is deleted and the command prints a summary indicating it
  was removed (prefixed with `-`), and exits 0

#### Scenario: Uninstall is a no-op when the extension file is absent

- **WHEN** the user runs `agentctl uninstall pi` and
  `~/.pi/agent/extensions/agentctl.ts` does not exist
- **THEN** no filesystem changes are made, the command prints a summary
  indicating there was nothing to remove (prefixed with `=`), and exits 0

#### Scenario: Uninstall removes the file even if its content was hand-modified

- **WHEN** `~/.pi/agent/extensions/agentctl.ts` exists but its content no longer
  matches what `agentctl install pi` currently generates
- **THEN** `agentctl uninstall pi` still deletes the file, since the file is
  documented as agentctl-managed regardless of content drift

#### Scenario: Uninstall leaves the extensions directory in place

- **WHEN** `agentctl uninstall pi` deletes `agentctl.ts` and no other files
  remain in `~/.pi/agent/extensions/`
- **THEN** the `~/.pi/agent/extensions/` directory itself is left in place (not
  removed)
