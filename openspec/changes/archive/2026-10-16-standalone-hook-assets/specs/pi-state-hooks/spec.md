## MODIFIED Requirements

### Requirement: agentctl install pi generates the pi extension file

The `agentctl install pi` subcommand SHALL write the agentctl TypeScript
extension to `~/.pi/agent/extensions/agentctl.ts` by copying the standalone
asset `packages/adapters/src/harness-pi/assets/agentctl.ts` verbatim. The file
SHALL subscribe to `session_start`, `before_agent_start`, `agent_settled`,
`session_shutdown`, `tool_call`, and `tool_result` events and call the
appropriate `agentctl agent event` / `agentctl agent clear` commands. The
command SHALL be idempotent: re-running when the file content is already
identical SHALL make no change and SHALL report the file as already up to date.

#### Scenario: First-time install creates the extension file

- **WHEN** the user runs `agentctl install pi` and
  `~/.pi/agent/extensions/agentctl.ts` does not exist
- **THEN** the file is created as a copy of the asset with handlers for all six
  events and the command prints a summary prefixed with `+`

#### Scenario: Idempotent re-install reports no change

- **WHEN** the user runs `agentctl install pi` a second time and the file
  content is already identical to the asset
- **THEN** no write is performed and the command prints the file as already up
  to date (prefixed with `=`)

#### Scenario: Install overwrites a stale extension file

- **WHEN** `~/.pi/agent/extensions/agentctl.ts` exists but its content differs
  from the asset
- **THEN** the file is overwritten with the asset content and the command prints
  the file as updated (prefixed with `~`)

#### Scenario: Install creates the extensions directory if absent

- **WHEN** `~/.pi/agent/extensions/` does not exist
- **THEN** `agentctl install pi` creates the directory before writing
  `agentctl.ts`, and the command exits 0
