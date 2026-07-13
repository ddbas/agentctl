## Why

The `dv` CLI currently has no awareness of what each agent is actually doing —
users have no way to tell at a glance whether an agent is waiting for input,
actively running, has finished, or has never been prompted. Adding first-class
state tracking gives operators immediate situational awareness across all agents
in a session, and lays the foundation for automated orchestration (e.g. routing
new work only to idle agents).

## What Changes

- Introduce a standard `AgentState` enum (`idle`, `working`, `done`, `blocked`,
  `unknown`) shared across the codebase.
- Define a file-based state persistence mechanism: each agent harness writes
  state-change events to a well-known path so that the CLI can read them at any
  time without needing to be running when the events occur.
- Implement Claude-specific hook handlers as internal `dv hook <event>`
  subcommands (e.g. `dv hook session-start`, `dv hook stop`). Each reads
  Claude's JSON event from stdin and writes the appropriate state file. Claude's
  hook system covers all five states with no gaps:
  - `idle` ← `SessionStart` (startup)
  - `working` ← `UserPromptSubmit` and `PreToolUse`
  - `done` ← `Stop`
  - `blocked` ← `Notification` (permission_prompt)
- Add a `dv hooks install` subcommand that registers the `dv hook <event>`
  commands in `~/.claude/settings.json`. Running it once per machine is all the
  setup required; the command is idempotent and safe to re-run. All registered
  hooks are `async` so they never block Claude's operation.
- Expose agent state in the `dv` CLI output (e.g. `dv list`).

## Capabilities

### New Capabilities

- `agent-state`: Standard `AgentState` enum, per-agent state file schema, and
  the read/write API used by the CLI and hook adapters.
- `claude-state-hooks`: Claude-specific hook handlers (`dv hook <event>`
  subcommands) and the `dv hooks install` command that registers them in
  `~/.claude/settings.json`.

### Modified Capabilities

- `list-sessions`: The session/agent listing output now includes the current
  state of each agent.

## Impact

- **New files**: state file under a per-agent directory (e.g.
  `~/.local/share/drovr/sessions/<session-id>/agents/<agent-id>/state.json`).
- **CLI binary (`dv`)**: reads state files when listing agents; exposes
  `dv hook <event>` internal handlers and `dv hooks install` for one-time setup.
- **Claude hooks**: `dv hooks install` patches `~/.claude/settings.json` to
  register async `dv hook <event>` handlers for `SessionStart`,
  `UserPromptSubmit`, `PreToolUse`, `Stop`, and `Notification`.
- **No breaking changes** to existing session or tmux APIs.
