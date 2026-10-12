## Context

`dispatch_event` (`agentctl-core/src/app.rs`) computes two independent outputs
on every event: `status` (via `rust-fsm` in `status_fsm.rs`) and `active_tools`
(plain arithmetic). They are coupled at exactly one point: `dispatch_event`
passes the new count into `Event::RunFinished { active_tools }`, and the FSM's
wildcard arm returns `Some(Done)` only when it is `0`.

Claude's `Stop`/`StopFailure` hooks pass
`--active-tools <background_tasks.len()>` so the counter reflects background
work that outlives the launching tool call. Today that makes the pane stay
`working` for as long as background work exists.

## Goals / Non-Goals

**Goals:**

- `status` is a function of the event stream only, never of `active_tools`.
- `RUN_FINISHED` always means `done`.
- Elicitation tool calls still drive `blocked` / back to `working`.
- `activeTools` remains available as an independent, informational signal.

**Non-Goals:**

- Splitting `active_tools` into separate in-flight vs background counters.
- Fixing counter staleness when a background task ends silently (nothing
  decrements it until the next `Stop` reconciles).
- Making pi send `--active-tools 0` on `agent_settled` (counter self-heal).
  Worth a follow-up; status is no longer affected by a leaked pi counter.
- Any new status, rendering change, or a `TOOL_CALL_START`-from-`done`
  transition.

## Decisions

**1. Remove the payload from `Event::RunFinished` rather than ignore it.**
Making the type unable to carry the count enforces the decoupling at compile
time. Alternative (keep the field, ignore it) leaves a misleading API and
invites re-coupling. `dispatch_event` still computes `new_active_tools` for
persistence, it just no longer feeds the FSM.

**2. Keep `--active-tools` on `RUN_FINISHED` and the Claude hook commands.** It
still reconciles the counter to Claude's authoritative number, which is now
exactly the useful signal alongside `done` ("turn over, N background tasks").
Removing it would regress the count with no benefit.

**3. Accept that a wake-up turn is not detected at its start; recover on the
first tool call.** If a background task completion re-engages Claude without
`UserPromptSubmit`, the pane shows `done` at the start of that turn. To bound
the inaccuracy, `TOOL_CALL_START` moves `Done` → `Working` (or `Done` →
`Blocked` when typed `elicitation`): a tool starting means the model is acting,
so the first tool call of the wake-up turn corrects the status.
`TOOL_CALL_RESULT` is unchanged. This is a transition on the event stream only;
it does not read `active_tools`, so decision 1 still holds. Rejected
alternative: a dedicated wake-up hook (`PostToolBatch`, `MessageDisplay`) — none
cleanly marks a turn start; revisit only if the spike (tasks 4.2) shows
`UserPromptSubmit` does not fire on background-task wake-ups.

## Risks / Trade-offs

- [Wake-up turn shows `done` until its first tool call] → Unverified whether it
  occurs; verify in a testcontainer with a short background shell and inspect
  `events.jsonl`. A wake-up turn that uses no tools stays `done`.
- [Late `TOOL_CALL_START` after `RUN_FINISHED` flips a finished pane to
  `working`, and nothing flips it back until the next turn] → Hooks are async,
  but the last `PreToolUse` is separated from `Stop` by the tool's run time, its
  `PostToolUse`, and a final model response, so delivery would have to lag by
  seconds. pi is unaffected (its extension uses `execFileSync`, so events are
  ordered); only Claude's async hooks can race. Accepted (option A); if
  observed, follow up with hook-stamped event timestamps (`--at`) so the FSM can
  drop stale `TOOL_CALL_START`s, rather than a time-based grace window.
- [Background subagent tool calls after the parent's `Stop` revive the pane to
  `working`] → Intended: a subagent is an LLM doing real work. The pane stays
  `working` until the parent's next `Stop` (the subagent's report-back fires
  `UserPromptSubmit`). Background shells/`Monitor` fire no tool hooks and are
  unaffected.
- [`dispatch_event` read-modify-write is unlocked, so overlapping async hook
  processes can overwrite each other] → Pre-existing (affects the counter); out
  of scope.
- [`activeTools` can stay stale after a silent background completion] →
  Pre-existing; unchanged. Status is now honest regardless.
- [Users relying on `working` ≈ "anything running"] → Intentional behavior
  change; call out in README.

## Migration Plan

No data migration: `state.json` schema and hook commands are unchanged. Existing
panes pick up the new behavior at their next `RUN_FINISHED`. Rollback is
reverting the FSM change.

## Open Questions

- Does a Claude background-task wake-up turn fire `UserPromptSubmit`?
- Should `done` + nonzero `activeTools` get distinct rendering (dim, `bg`
  marker)?
