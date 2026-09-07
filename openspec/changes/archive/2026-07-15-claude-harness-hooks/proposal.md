## Why

The `agent-state-tracking` change establishes the foundational
`agentctl set-state` and `agentctl clear-state` commands and the
`agentctl hooks install <harness>` placeholder, but intentionally ships with no
supported harnesses. This change adds the first concrete harness integration:
**Claude Code**.

Claude Code exposes an official hook system that fires at well-defined lifecycle
events (`SessionStart`, `UserPromptSubmit`, `PreToolUse`, `Stop`,
`Notification`, `SessionEnd`). Wiring these events to `agentctl set-state` gives
`agentctl list` accurate, real-time state for every Claude agent in a session —
without any polling or tmux-pane inspection.

## What Changes

- Implement the `claude` handler for `agentctl hooks install claude`. The
  command patches `~/.claude/settings.json` to register six hook entries that
  call `agentctl set-state <state> --harness claude` and `agentctl clear-state`.
  It is idempotent and merges safely with any pre-existing hook configuration.
- Map each Claude hook event to the appropriate canonical `AgentState`:
  - `SessionStart` → `agentctl set-state idle --harness claude`
  - `UserPromptSubmit` → `agentctl set-state working --harness claude` (async)
  - `PreToolUse` → `agentctl set-state working --harness claude` (async)
  - `Stop` → `agentctl set-state done --harness claude` (async)
  - `Notification` (matcher: `permission_prompt`) →
    `agentctl set-state blocked --harness claude` (async)
  - `SessionEnd` → `agentctl clear-state`

## Capabilities

### New Capabilities

- `claude-state-hooks`: Claude-specific hook registration and the
  `agentctl hooks install claude` command that patches
  `~/.claude/settings.json`.

### Dependencies

This change depends on `agent-state-tracking` and assumes the following
capabilities are already present in the codebase:

- `agent-state`: `AgentState` enum, `StateStore`, and the XDG file layout.
- `agentctl set-state <state>` and `agentctl clear-state` subcommands.
- The `agentctl hooks install <harness>` dispatch skeleton (currently exits 1
  for all harnesses; this change fills in the `claude` branch).

## Impact

- **CLI binary (`agentctl`)**: adds the `claude` branch of
  `agentctl hooks install`, replacing the current "no harnesses supported" error
  with a working implementation.
- **Claude settings file** (`~/.claude/settings.json`): patched once by the user
  via `agentctl hooks install claude`; no automatic mutation.
- **No breaking changes** to any existing commands or data formats.
