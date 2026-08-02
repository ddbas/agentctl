## Why

The `agent-state-tracking` change establishes the foundational `rd set-state`
and `rd clear-state` commands and the `rd hooks install <harness>` placeholder,
but intentionally ships with no supported harnesses. This change adds the first
concrete harness integration: **Claude Code**.

Claude Code exposes an official hook system that fires at well-defined lifecycle
events (`SessionStart`, `UserPromptSubmit`, `PreToolUse`, `Stop`,
`Notification`, `SessionEnd`). Wiring these events to `rd set-state` gives
`rd list` accurate, real-time state for every Claude agent in a session —
without any polling or tmux-pane inspection.

## What Changes

- Implement the `claude` handler for `rd hooks install claude`. The command
  patches `~/.claude/settings.json` to register six hook entries that call
  `rd set-state <state> --harness claude` and `rd clear-state`. It is idempotent
  and merges safely with any pre-existing hook configuration.
- Map each Claude hook event to the appropriate canonical `AgentState`:
  - `SessionStart` → `rd set-state idle --harness claude`
  - `UserPromptSubmit` → `rd set-state working --harness claude` (async)
  - `PreToolUse` → `rd set-state working --harness claude` (async)
  - `Stop` → `rd set-state done --harness claude` (async)
  - `Notification` (matcher: `permission_prompt`) →
    `rd set-state blocked --harness claude` (async)
  - `SessionEnd` → `rd clear-state`

## Capabilities

### New Capabilities

- `claude-state-hooks`: Claude-specific hook registration and the
  `rd hooks install claude` command that patches `~/.claude/settings.json`.

### Dependencies

This change depends on `agent-state-tracking` and assumes the following
capabilities are already present in the codebase:

- `agent-state`: `AgentState` enum, `StateStore`, and the XDG file layout.
- `rd set-state <state>` and `rd clear-state` subcommands.
- The `rd hooks install <harness>` dispatch skeleton (currently exits 1 for all
  harnesses; this change fills in the `claude` branch).

## Impact

- **CLI binary (`rd`)**: adds the `claude` branch of `rd hooks install`,
  replacing the current "no harnesses supported" error with a working
  implementation.
- **Claude settings file** (`~/.claude/settings.json`): patched once by the user
  via `rd hooks install claude`; no automatic mutation.
- **No breaking changes** to any existing commands or data formats.
