## Context

`state.json` (`src/state.rs::AgentInfo`) currently holds `pane_id`, `harness`,
`status`, `updated_at`. The `agent-state` spec already flags a future per-pane
counter of active tool calls as an anticipated extension of this same file, so
this is additive rather than a schema redesign.

`agentctl agent event TOOL_CALL_START` / `TOOL_CALL_RESULT` already exist and
feed the `status_fsm` (`Working` ↔ `Blocked` transitions gated on
`--type elicitation`). Today they have **no other side effect** — the pi
extension (`src/commands/hooks.rs::PI_EXTENSION_CONTENT`) doesn't even call
them; only `session_start`, `before_agent_start`, `agent_settled`, and
`session_shutdown` are wired up. This change adds a second, independent side
effect to the same two events (mutating a counter) and wires the pi extension to
call them on every tool call, regardless of `--type`.

`agentctl list` (`src/commands/list.rs::format_panes`) renders four columns —
`agent`, `session`, `window.pane`, `state` — each separated by the max column
width plus two spaces. It needs a fifth column for the tool count, using the
same two-space delimiter convention as the existing four.

## Goals / Non-Goals

**Goals:**

- Track the number of in-flight tool calls per pane as `active_tools: u32` in
  `state.json`.
- Increment on every `TOOL_CALL_START`, decrement on every `TOOL_CALL_RESULT`,
  regardless of `--type` (elicitation or custom) — the counter tracks raw
  concurrency, not blocking state.
- Never go negative; a stray/duplicate `TOOL_CALL_RESULT` clamps to `0` rather
  than underflowing or erroring.
- Reset to `0` whenever `SESSION_STARTED` fires (fresh session ⇒ no in-flight
  calls), consistent with that event resetting `status` to `idle`.
- Wire the pi extension to call `TOOL_CALL_START` / `TOOL_CALL_RESULT` on pi's
  `tool_call` / `tool_result` lifecycle events, using `event.toolCallId` as the
  `--id`.
- Add a fifth `agentctl list` column, separated from the `state` column by the
  same two-space delimiter used between all other columns, showing `N tool` /
  `N tools` when `active_tools > 0` and empty (no trailing delimiter) when it is
  `0`.
- Deserialize old `state.json` files (written before this change) without
  `active_tools` as `0`, via `#[serde(default)]`.

**Non-Goals:**

- Do not change how `--type elicitation` drives the `Blocked`/`Working` status
  FSM transition — that logic is untouched and orthogonal to this counter.
- Do not attempt to reconcile `active_tools` against `events.jsonl` at read time
  (e.g. recomputing the count by replaying the log). The counter is maintained
  incrementally in `state.json` only, same as `status`.
- Do not persist per-tool-call identity/bookkeeping beyond the running count;
  `--id` is accepted (already required by the event schema) but not needed to
  correctly increment/decrement since calls are trusted to pair up 1:1.
- Not addressing already-in-flight tool calls started by an older pi extension
  version that never emitted `TOOL_CALL_START` — those simply won't be counted
  until the extension is reinstalled (`agentctl install pi`).

## Decisions

1. **New field vs. a separate file.** Add `active_tools: u32` directly to
   `AgentInfo`/`state.json` rather than a sibling file. It's small, always
   read/written alongside `status`, and the existing atomic-write path
   (`write_state_to`) already covers it for free. Alternative considered: a
   separate `tool-calls.json` — rejected as unnecessary indirection for a single
   integer.

2. **Increment/decrement on the existing two events, independent of `--type`.**
   `TOOL_CALL_START`/`TOOL_CALL_RESULT` are already the canonical events for
   tool-call boundaries; reusing them (rather than adding new event names) keeps
   `events.jsonl` and the FSM single-sourced. The counter mutation is
   implemented as a second, independent step in `agent_event::run` alongside
   (not inside) the FSM's `apply_event` call — the FSM only ever sees/affects
   `status`, keeping `status_fsm` free of counter concerns.

3. **Clamp decrement at zero via `saturating_sub`.** A `TOOL_CALL_RESULT`
   without a matching prior `TOOL_CALL_START` (e.g. extension reinstalled
   mid-run, or a state file deleted/reset between the two) must not underflow a
   `u32` or panic. `saturating_sub(1)` is the simplest correct clamp.

4. **Reset `active_tools` to `0` on `SESSION_STARTED`, exactly like `status`
   resets to `Idle`.** A new session has no in-flight tool calls by definition,
   and this avoids stale counts surviving a session restart in the same pane.
   Implemented by having `write_state` (or a thin wrapper) accept the tool count
   explicitly, with `agent_event::run` passing `0` on `SESSION_STARTED` and the
   incremented/decremented value on `TOOL_CALL_START`/`TOOL_CALL_RESULT`, and
   the current value unchanged on `RUN_STARTED`/`RUN_FINISHED`.

5. **A fifth column, not a suffix on the state word.** `agentctl list` already
   uses a consistent "pad to max width + two spaces" delimiter between its four
   columns; the tool count is added as a fifth column following that exact same
   convention (`1 tool` / `N tools`) rather than concatenated onto the state
   word with a single space. This keeps the delimiter uniform across every
   column boundary and keeps the state column's own width (and thus its
   alignment) independent of whether any given row has an active tool count.
   When `active_tools == 0` (or the pane's state is `unknown`), the fifth column
   is empty and the line is right-trimmed so no dangling delimiter is printed
   after `state` for that row.

6. **pi extension wiring uses `event.toolCallId` as `--id`.** This is already a
   required, stable per-call identifier available on both `tool_call` and
   `tool_result` events, so no extra ID generation/bookkeeping is needed in the
   extension (unlike `runId`, which pi doesn't provide and the extension must
   generate itself).

## Risks / Trade-offs

- **[Risk]** An older pi extension (pre-this-change) never emits
  `TOOL_CALL_START`/`TOOL_CALL_RESULT`, so `active_tools` stays `0` and the new
  display never appears until the user reruns `agentctl install pi`. →
  **Mitigation**: purely additive/backward-compatible; nothing breaks, the
  feature is simply inert until reinstalled. Document in the PR description that
  `agentctl install pi` must be rerun.
- **[Risk]** Crash mid-tool-call (agent process killed) leaves `active_tools`
  permanently incremented with no matching `TOOL_CALL_RESULT` ever arriving. →
  **Mitigation**: `SESSION_STARTED` unconditionally resets the counter to `0`,
  and `agentctl list`'s existing stale-state cleanup deletes the whole state
  file for dead panes, so the stuck count cannot outlive the pane or the next
  session in that pane.
- **[Risk]** Parallel tool execution (pi's default mode) can interleave
  `tool_call`/`tool_result` events across multiple concurrent calls; each call
  to `agentctl agent event` is a separate process invocation, so the
  read-modify-write on `state.json` is not atomic across calls. →
  **Mitigation**: accept this as a best-effort display counter (matching the
  existing best-effort nature of `status`, which has the same read-modify-write
  pattern today); a rare lost update only skews the displayed count by one and
  self-heals on the next `SESSION_STARTED`.

## Migration Plan

- No migration needed for `state.json`: `active_tools` deserializes via
  `#[serde(default)]`, so files written by older `agentctl` binaries continue to
  parse.
- Users must rerun `agentctl install pi` to pick up the updated extension
  content (same as any other extension-template change); `agentctl install pi`
  is already idempotent and safe to rerun.
- No CLI flags change; `agentctl agent event TOOL_CALL_START`/`TOOL_CALL_RESULT`
  keep their existing signatures.

## Open Questions

- None — the proposal and design above are sufficient to implement without
  further clarification.
