## Why

The `rd` CLI currently has no awareness of what each agent is actually doing —
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
- Add a `rd hooks install <harness>` subcommand as a placeholder for
  harness-specific hook registration. No harnesses are supported in this
  iteration; the command exits 1 with a clear message. Concrete harness support
  (e.g. `claude`) is added by dependent changes.
- Expose agent state in the `rd` CLI output (e.g. `rd list`).

## Capabilities

### New Capabilities

- `agent-state`: Standard `AgentState` enum, per-agent state file schema, and
  the read/write API used by the CLI and hook adapters.

### Modified Capabilities

- `list-sessions`: The session/agent listing output now includes the current
  state of each agent.

## Impact

- **New files**: state file under a per-agent directory (e.g.
  `~/.local/share/radr/sessions/<session-id>/agents/<agent-id>/state.json`).
- **CLI binary (`rd`)**: reads state files when listing agents; exposes
  `rd set-state`, `rd clear-state`, and `rd hooks install <harness>` (no-op
  placeholder in this iteration).
- **No breaking changes** to existing session or tmux APIs.
