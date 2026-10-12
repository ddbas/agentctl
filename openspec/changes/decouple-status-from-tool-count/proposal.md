## Why

`RUN_FINISHED` only moves a pane to `done` when the reconciled `active_tools`
count is `0`. When the count is nonzero (e.g. Claude's `Stop` hook reports a
still-running background `Monitor`/shell), the pane stays `working` even though
the agent's turn is over. In `agentctl list`, that makes it impossible to tell
an agent that is actually doing work from one that merely has a process running
in the background. The same gate also lets a leaked counter (e.g. a missed pi
`tool_result`) pin a pane on `working` indefinitely.

## What Changes

- `RUN_FINISHED` SHALL always drive `status` to `done`, from any status,
  regardless of `active_tools`. It is no longer a no-op when `active_tools > 0`.
- The status FSM stops reading the tool counter entirely. The only tool-call
  events that affect status remain elicitation-typed ones
  (`TOOL_CALL_START --type elicitation` → `blocked`,
  `TOOL_CALL_RESULT --type elicitation` → `working`).
- `active_tools` becomes purely informational: it is still maintained (+1 / −1 /
  `--active-tools` override / reset on `SESSION_STARTED`) and shown in the
  `activeTools` column, but never influences `status`.
- Claude `Stop`/`StopFailure` hooks are unchanged on the wire (they still pass
  `--active-tools <n>`), but their documented behavior changes: the pane goes
  `done` even when `background_tasks` is non-empty, and `activeTools` shows the
  remaining background work.
- **BREAKING** (behavioral): a pane with background work after a turn ends now
  reads `done` + `N tools` instead of `working`.

## Capabilities

### New Capabilities

<!-- none -->

### Modified Capabilities

- `agent-events`: the "Events derive status via a finite state machine"
  requirement drops the `active_tools == 0` gate on `RUN_FINISHED`; the
  nonzero-count scenario is replaced.
- `claude-state-hooks`: the `Stop` and `StopFailure` requirements no longer gate
  `done` on the reconciled count; the "does not force done while a background
  task is still running" scenario is replaced.

## Impact

- Code: `crates/agentctl-state/src/status_fsm.rs` (`Event::RunFinished` loses
  its payload; wildcard arm always yields `Done`),
  `crates/agentctl-core/src/app.rs` (`dispatch_event` stops passing the count to
  the FSM), and their unit tests.
- Docs: `README.md` transition table and the "only refuses to fire…" paragraph
  (which also still describes the removed `permission-prompt`/`blocked_prompts`
  behavior — fix in passing).
- No change to hook commands, `state.json` schema, `events.jsonl`, list columns,
  or sorting.
- Known trade-off: a Claude "wake-up" turn (background task completion
  re-engaging the model) may show `done` if it emits no `UserPromptSubmit`.
  Unverified; see design.
