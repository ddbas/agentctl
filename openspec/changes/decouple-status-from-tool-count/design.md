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

**3. Accept the wake-up-turn trade-off (option A) for now.** If a background
task completion re-engages Claude without `UserPromptSubmit`, that turn shows
`done`. Rejected alternative: `TOOL_CALL_START` from `done` → `working` — hooks
are async, so a late `PreToolUse` after `Stop` would wrongly flip `done` →
`working`, and it re-couples tools to status. Better future fix: find a Claude
hook that marks a wake-up and map it to `RUN_STARTED`.

## Risks / Trade-offs

- [Wake-up turn shows `done` while the model works] → Unverified whether it
  occurs; verify in a testcontainer with a short background shell and inspect
  `events.jsonl`. If real, follow up with a wake-up hook (preferred) rather than
  re-coupling.
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
