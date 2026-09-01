## Why

Panes sometimes stay stuck in `working` (or, in principle, could jump to `done`
too early) because the status FSM and the Claude hook set both have gaps in how
they reconcile "the run really is over" with "there's still in-flight work".
Concretely: (1) `RUN_FINISHED` only transitions `Working` → `Done`, so if a
pane's status has drifted to `Blocked`/`Idle`/`Done` (e.g. a permission-prompt
block that never got cleared) when `Stop`/`StopFailure` fires, the authoritative
"the harness says this turn is over" signal is silently dropped and the pane
never reaches `Done`. (2) A pane can also reach `Done` while a background task
is still running, because `active_tools` isn't consulted at all when
`RUN_FINISHED` fires. (3) The Claude hook set tracks permission-prompt blocking
with a single boolean toggle (any qualifying "tool finished" event clears
`Blocked`), so two concurrent permission prompts in the same session let the
first one to resolve incorrectly clear a still-pending second prompt — and,
separately, the synthetic `perm-<session_id>` bookkeeping events
double-decrement the shared `active_tools` counter on every tool completion
(once via the real untyped close, once via the `perm-` close), which would
defeat any `active_tools == 0` gate added for (2).

## What Changes

- Make the status FSM's `RUN_FINISHED` transition a wildcard: any current status
  (`Idle`, `Working`, `Blocked`, `Done`) SHALL transition to `Done` on
  `RUN_FINISHED`, not only `Working`, so `Stop`/`StopFailure` is authoritative
  regardless of whatever status a pane drifted into.
- Gate that transition on the reconciled `active_tools` count being `0`: when
  `RUN_FINISHED` fires and `active_tools` (after applying any `--active-tools`
  override) is greater than `0`, the pane's status SHALL NOT change (stays at
  its current status) — the false-`done`-while-still-working direction of the
  bug.
- Introduce a `permission-prompt` tool-call `--type`, distinct from
  `elicitation`, and a corresponding `blocked_prompts` counter in
  `state.json`/`AgentInfo` (alongside `active_tools`), incremented by
  `Notification`'s `permission_prompt` open and decremented by the unmatched
  `PostToolUse` close. `Blocked` → `Working` for this type SHALL only fire once
  `blocked_prompts` returns to `0`, so one resolved permission prompt cannot
  prematurely clear a different, still-pending one.
- `permission-prompt`-typed `TOOL_CALL_START`/`TOOL_CALL_RESULT` events SHALL
  NOT mutate `active_tools` (only `blocked_prompts`), closing the
  double-decrement hole that previously let the synthetic `perm-` close events
  corrupt the shared tool-call counter.
- Update the two Claude hook entries currently using
  `--id "perm-<session_id>" --type elicitation` (the `Notification`
  `permission_prompt` open and the unmatched `PostToolUse` close) to use
  `--type permission-prompt` instead, and append a per-invocation unique suffix
  (e.g. a random value) to the `perm-` id so distinct prompts are
  distinguishable in `events.jsonl` for debugging — this is a log-quality
  improvement only; Claude's `Notification` hook payload has no field that
  correlates a specific permission prompt with the tool call that will later
  resolve it (see rationale in `design.md`), so the actual concurrency fix is
  the `blocked_prompts` counter above, not id matching.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `agent-events`: `RUN_FINISHED` becomes a wildcard transition to `Done` from
  any status, gated on `active_tools == 0`; new `permission-prompt` tool-call
  type with counter-gated `Blocked` → `Working` transition.
- `agent-state`: new `blocked_prompts` field in the state file schema,
  incremented/decremented independently of `active_tools`, reset to `0` on
  `SESSION_STARTED`.
- `claude-state-hooks`: `Notification` (`permission_prompt`) and the unmatched
  `PostToolUse` permission-close entry switch from `--type elicitation` to
  `--type permission-prompt` and use a unique-per-invocation `perm-` id.

## Impact

- `src/status_fsm.rs`: transition table and `Event`/`ToolCallType` enums.
- `src/state.rs`: `AgentInfo` gains `blocked_prompts`; write/read helpers.
- `src/commands/agent_event.rs`: active-tools/blocked-prompts counter
  bookkeeping, and passing the reconciled counts into FSM transitions.
- `src/main.rs`: `ToolCallTypeArg` gains `PermissionPrompt`; `RUN_FINISHED`
  gating wiring.
- `src/commands/hooks.rs`: `RADR_CLAUDE_HOOKS` entries for `Notification` and
  the unmatched `PostToolUse` permission-close.
- `openspec/specs/agent-events/spec.md`, `openspec/specs/agent-state/spec.md`,
  `openspec/specs/claude-state-hooks/spec.md`.
- Existing unit/integration tests covering the FSM, `agent_event`, `state`, and
  `hooks` modules.
