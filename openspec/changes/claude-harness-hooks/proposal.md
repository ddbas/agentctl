## Why

The `agent-state-tracking` change establishes the foundational `dv set-state`
and `dv clear-state` commands and the `dv hooks install <harness>` placeholder,
but intentionally ships with no supported harnesses. This change adds the first
concrete harness integration: **Claude Code**.

Claude Code exposes an official hook system that fires at well-defined lifecycle
events (`SessionStart`, `UserPromptSubmit`, `PreToolUse`, `Stop`,
`Notification`, `SessionEnd`). Wiring these events to `dv set-state` gives
`dv list` accurate, real-time state for every Claude agent in a session —
without any polling or tmux-pane inspection.

## What Changes

- Implement the `claude` handler for `dv hooks install claude`. The command
  patches `~/.claude/settings.json` to register six hook entries that call
  `dv set-state <state> --harness claude` and `dv clear-state`. It is idempotent
  and merges safely with any pre-existing hook configuration.
- Map each Claude hook event to the appropriate canonical `AgentState`:
  - `SessionStart` → `dv set-state idle --harness claude`
  - `UserPromptSubmit` → `dv set-state working --harness claude` (async)
  - `PreToolUse` → `dv set-state working --harness claude` (async)
  - `Stop` → `dv set-state done --harness claude` (async)
  - `Notification` (matcher: `permission_prompt`) →
    `dv set-state blocked --harness claude` (async)
  - `SessionEnd` → `dv clear-state`

## Capabilities

### New Capabilities

- `claude-state-hooks`: Claude-specific hook registration and the
  `dv hooks install claude` command that patches `~/.claude/settings.json`.

### Dependencies

This change depends on `agent-state-tracking` and assumes the following
capabilities are already present in the codebase:

- `agent-state`: `AgentState` enum, `StateStore`, and the XDG file layout.
- `dv set-state <state>` and `dv clear-state` subcommands.
- The `dv hooks install <harness>` dispatch skeleton (currently exits 1 for all
  harnesses; this change fills in the `claude` branch).

## Impact

- **CLI binary (`dv`)**: adds the `claude` branch of `dv hooks install`,
  replacing the current "no harnesses supported" error with a working
  implementation.
- **Claude settings file** (`~/.claude/settings.json`): patched once by the user
  via `dv hooks install claude`; no automatic mutation.
- **No breaking changes** to any existing commands or data formats.
