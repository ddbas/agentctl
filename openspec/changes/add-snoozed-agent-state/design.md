## Context

`radr` tracks per-tmux-pane agent state in `src/state.rs` via the `AgentState`
enum (`Idle`, `Working`, `Done`, `Blocked`, `Unknown`), persisted to
`$XDG_DATA_HOME/radr/agents/<pane-id>/state.json` and rendered by `rd list`
sorted by a manually implemented `Ord` (priority:
`Blocked < Done < Working < Idle < Unknown`, most attention-needed first).

Claude Code's harness integration (`rd install claude`, documented in
`openspec/specs/claude-state-hooks/spec.md`) patches `~/.claude/settings.json`
with hook entries that call `rd agent status <state> --harness claude` at each
lifecycle point (`SessionStart`, `UserPromptSubmit`, `PostToolUse`,
`PreToolUse`, `Stop`, `StopFailure`, `Notification`, `SessionEnd`).

Claude Code's
[Monitor tool](https://code.claude.com/docs/en/tools-reference#monitor-tool)
lets Claude run a background command or open a WebSocket and receive
events/output back into the same conversation later, without the user needing to
send a new prompt. When a monitor is active, Claude's `Stop` hook still fires
(the visible turn ends) but the session is not truly settled — it will resume
automatically when the monitor reports something. Full detail is captured in the
handoff note this proposal is based on:
`~/Desktop/radr-new-agent-status-snoozed.md`.

## Goals / Non-Goals

**Goals:**

- Add `AgentState::Snoozed` as a first-class state with a well-defined sort
  priority relative to the existing five variants.
- Detect, at Claude hook time, when a Monitor-tool watch (or an equivalent
  Claude-native self-arming "resume later" mechanism) is active, and reflect
  that as `Snoozed` instead of `Done`/`Idle`.
- Ensure `Snoozed` reliably transitions to `Working`/`Blocked`/`Done` when the
  armed mechanism actually fires, so it never becomes a stuck terminal state.
- Keep the change additive and backward compatible with existing states,
  `rd list` sort behavior for pre-existing states, and the `pi` integration.

**Non-Goals:**

- Generalizing `Snoozed` detection to `pi` or any other harness in this change
  (`openspec/specs/pi-state-hooks/spec.md` is not touched).
- Building a general-purpose "list of active background watches" feature;
  `Snoozed` is a single boolean-ish state, not a registry of monitors.
- Detecting third-party/MCP tools that happen to run long-lived background
  processes outside of Claude's own Monitor tool and hook-visible signals.

## Decisions

### 1. Sort priority: `Blocked < Working < Snoozed < Done < Idle < Unknown`

`Snoozed` ranks below `Working`/`Blocked` (no immediate human attention needed —
nothing is broken and no input is requested) but above `Done`/`Idle` (there is
live, armed work that will resume without a new user prompt, which is "more
in-flight" than a fully settled or waiting agent). This matches the working
assumption in the handoff note and keeps `rd list`'s existing ordering among the
other four states unchanged — only `Snoozed`'s new slot is inserted between
`Working` and `Done`.

Alternative considered: placing `Snoozed` last (below `Unknown`), reasoning that
it needs no attention at all. Rejected — a snoozed agent is still doing
something on the user's behalf and surfacing it above `Done`/`Idle` avoids it
being visually indistinguishable from "nothing left to do here."

### 2. Detection mechanism: track "watch armed" via `PreToolUse`/`PostToolUse` on `Monitor`, resolve at `Stop`

Claude's hook payloads for `PreToolUse`/`PostToolUse` include `tool_name` and
`tool_input`. Detection has two conceptually separate parts:

- **Arming**: `PreToolUse` matcher `Monitor` — when Claude invokes the `Monitor`
  tool, a _background_ watch has been requested. `PostToolUse` for `Monitor`
  confirms the tool call completed (the watch is now running with a `command` or
  `ws` handle) unless the tool response indicates a synchronous,
  already-finished watch (e.g. `timeout_ms` elapsed before completion, or the
  call errored/was denied).
- **Resolving at `Stop`**: rather than trying to track "is the watch still live"
  as separate long-running state, the `Stop` hook (which already fires at the
  point today's code would set `Done`) is extended to check whether the most
  recent `Monitor` invocation in the transcript is still active. If so,
  `rd agent status snoozed --harness claude` is written instead of `done`. This
  keeps `Snoozed` detection localized to the two existing hook points
  (`PreToolUse`/`PostToolUse` for arming signal capture, `Stop`/`StopFailure`
  for the actual state decision) rather than requiring a new long-lived
  out-of-band watcher process.

Because hooks are stateless shell invocations, "is the watch still active" needs
a small piece of local memory. The design proposes writing a lightweight marker
file (e.g. alongside `state.json`, such as `monitor.json` in the same per-pane
directory) on `Monitor` `PreToolUse`, and clearing it under either of:

- a `PostToolUse` for `Monitor` whose `tool_response` indicates the watch
  already ended (timeout/error/cancelled), or
- a `PreToolUse` for `TaskStop`/an equivalent Claude-native cancel signal for
  the same watch, if such a signal is available in Claude's hook payload.

`Stop` and `StopFailure` then check for the marker's presence (rather than
unconditionally writing `done`) to decide between `snoozed` and `done`.

Any other Claude-native "resume later" mechanism identified in the future (none
is confirmed beyond Monitor as of this writing) should follow the same
arm/resolve pattern: a `PreToolUse` matcher for the arming tool, and a
`Stop`/`StopFailure` check for whether it's still armed.

Alternative considered: adding a new hook event solely for Monitor transitions.
Rejected — Claude's hook event set is fixed by the harness; `rd` can only react
to the existing eight events, so all logic must be expressible in terms of
`PreToolUse`, `PostToolUse`, `Stop`, and `StopFailure`.

Alternative considered: inferring "still armed" purely from process liveness
(e.g. checking if a background PID is still running). Rejected for this pass —
Claude Code manages the Monitor watch's lifecycle internally; `rd` hooks do not
have direct visibility into or control over that process, only the tool-call
payloads Claude's hook system exposes.

### 3. Transition out of `Snoozed`

When the monitored mechanism fires, Claude resumes processing in the same
session, which re-triggers the normal hook flow: `PostToolUse`/subsequent tool
calls set `working`, and the eventual `Stop` sets `done` (or `snoozed` again if
another watch is armed) — the same mechanism described in Decision 2 above
naturally handles the transition since `Snoozed` is just a computed outcome at
`Stop` time, not a stored "mode" that has to be explicitly exited. The marker
file guarantees that once a watch is known to have ended (via the `PostToolUse`
end-signal or a later `Stop` with no live marker), state returns to normal
(`Working` → `Done`) on the next hook firing.

## Risks / Trade-offs

- [Hook payload for `Monitor`'s `tool_response` may not reliably indicate "watch
  ended" vs. "watch still running in background"] → Fall back to a conservative
  default: if the response format is ambiguous, treat the watch as still armed
  (bias toward `Snoozed` over `Done`) since the cost of a stale `Snoozed` is
  lower than incorrectly signaling `Done` while the agent is about to resume
  unprompted.
- [Marker file adds a second piece of per-pane state alongside `state.json`,
  another place for the file to be deleted on `SessionEnd`/`rd agent clear`] →
  Reuse existing `agent_dir_in` helpers and add cleanup of the marker file to
  `delete_state`/`rd agent clear` so no new cleanup path is introduced.
- [Claude may add other self-arming mechanisms beyond Monitor over time] →
  Explicitly scoped as future work per the proposal's non-goals; the arm/resolve
  pattern in Decision 2 is designed to extend to new tools without changing the
  `AgentState` enum again.
- [Changing `AgentState`'s `Ord` insertion point could silently break an
  existing test that hardcodes numeric priorities] → Existing tests in
  `src/state.rs`/`src/commands/list.rs` assert pairwise ordering by variant, not
  raw priority numbers; add `Snoozed` cases following the same pattern rather
  than renumbering by hand.
