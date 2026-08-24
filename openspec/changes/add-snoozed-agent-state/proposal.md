## Why

The requirements and design decisions for this change were captured in a handoff
note at `~/Desktop/radr-new-agent-status-snoozed.md`; this proposal formalizes
that note into an OpenSpec change.

Some agent harnesses can go quiet — not streaming output, not actively "working"
— while a background mechanism (e.g. Claude Code's
[Monitor tool](https://code.claude.com/docs/en/tools-reference#monitor-tool),
which tails a command or holds open a WebSocket) is armed to wake the agent up
automatically later. None of the current `AgentState` variants (`Idle`,
`Working`, `Done`, `Blocked`, `Unknown`) describe this correctly: `Working`
overclaims active effort, `Done` is actively misleading (it implies the turn is
fully settled and could cause a user to context-switch away), and
`Idle`/`Unknown` don't capture "there's a live mechanism that will wake this
up." A new `Snoozed` state closes this gap.

## What Changes

- Add a new `AgentState::Snoozed` variant to the canonical state enum in
  `src/state.rs` (serde `snake_case` → `"snoozed"`, `Display` → `"snoozed"`).
- Define `Snoozed`'s sort priority: above `Done`/`Idle` (more "in-flight" than a
  settled or waiting agent) but below `Working`/`Blocked` (no immediate human
  attention needed). Final order:
  `Blocked < Working < Snoozed < Done < Idle < Unknown`.
- Update `rd agent status` (`src/commands/agent_status.rs`) to accept
  `"snoozed"` as a valid state string.
- Extend the Claude Code hook integration (`rd install claude`) to detect when
  Claude has armed a self-resuming background mechanism — starting with the
  Monitor tool, plus any other Claude-native "resume later" mechanism
  discoverable via hook payloads — and set `AgentState::Snoozed` at that point
  instead of the state that would otherwise be set (e.g. instead of
  transitioning to `Done`/`Idle` on `Stop`).
- Define the transition out of `Snoozed`: when the monitored mechanism actually
  fires and wakes Claude, state moves to `Working` (or `Blocked`/`Done` as
  appropriate) via the normal hook flow. `Snoozed` must not be a terminal or
  stuck state.
- Scope: Claude Code only for this change. `pi` and other harnesses
  (`openspec/specs/pi-state-hooks/spec.md`) are explicitly out of scope and are
  not touched.
- Add unit tests for the new variant (round-trip serde, `Display`, sort order in
  `src/state.rs` and `src/commands/list.rs`) and hook-level tests for
  Monitor-tool detection and the transition back out of `Snoozed`, following
  existing patterns in `openspec/specs/claude-state-hooks/spec.md`.

## Capabilities

### New Capabilities

(none — this change extends two existing capabilities)

### Modified Capabilities

- `agent-state`: adds the `Snoozed` variant to the `AgentState` enum.
- `list-sorting`: updates the canonical `AgentState` sort-priority ordering to
  insert `Snoozed` between `Working` and `Done`.
- `claude-state-hooks`: adds detection of Claude's self-arming "resume later"
  mechanisms (Monitor tool and equivalents) to set `Snoozed`, and defines the
  transition back out of `Snoozed` when the mechanism fires.

## Impact

- **Code**: `src/state.rs` (enum, `Display`, `Ord`/priority),
  `src/commands/agent_status.rs` (state string parsing), `src/commands/list.rs`
  (sort-order tests), `src/commands/hooks.rs` (`rd install claude` hook
  definitions/detection logic).
- **Specs**: `openspec/specs/agent-state/spec.md`,
  `openspec/specs/claude-state-hooks/spec.md`.
- **Docs**: any user-facing docs listing valid `rd agent status` values.
- **Compatibility**: additive — existing states, sort order among them, and the
  `pi` integration (`openspec/specs/pi-state-hooks/spec.md`) are unchanged.
