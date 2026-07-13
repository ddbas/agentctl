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
- Implement a Claude-specific hook adapter that translates Claude lifecycle
  events into state-change writes.
- Expose agent state in the `dv` CLI output (e.g. `dv ls`, `dv status`).

## Capabilities

### New Capabilities

- `agent-state`: Standard `AgentState` enum, per-agent state file schema, and
  the read/write API used by the CLI and hook adapters.
- `claude-state-hooks`: Claude-specific hook adapter that listens to Claude's
  official lifecycle hooks and persists state updates to the agent state files.

### Modified Capabilities

- `list-sessions`: The session/agent listing output now includes the current
  state of each agent.

## Impact

- **New files**: state file under a per-agent directory (e.g.
  `~/.local/share/drovr/sessions/<session-id>/agents/<agent-id>/state.json`).
- **CLI binary (`dv`)**: reads state files when listing or inspecting agents.
- **Claude hooks**: new hook scripts (shell or binary) registered via Claude's
  hook mechanism; they write state updates using the agent's unique ID.
- **No breaking changes** to existing session or tmux APIs.
