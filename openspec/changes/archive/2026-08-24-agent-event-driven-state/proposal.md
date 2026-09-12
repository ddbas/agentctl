## Why

Today `agentctl agent status <state>` is the only way harnesses report agent
state, and it only tracks a single dimension: the coarse
`idle/working/done/blocked` status. We are about to add more kinds of state to
track over time (starting, in a future change, with active tool calls — useful
for long-running or indefinite tools like Claude Code's `Monitor`), and a
one-off imperative subcommand per state doesn't scale. We need an event-driven
interface where harness plugins report discrete, timestamped facts ("a run
started", "a tool call started") and `agentctl` derives whatever state it needs
from that stream — now and in the future — instead of harnesses pushing
pre-computed state values directly. This change lays that foundation: it
replaces the imperative status setter with an event dispatcher and an FSM that
derives status from events, and it starts persisting the raw event stream to
disk. It intentionally does NOT yet add any new tracked state (e.g. active tool
call counts) — that is future work once the event pipeline exists.

## What Changes

- **BREAKING**: Remove `agentctl agent status <state> [--harness] [--pane]`.
- Add `agentctl agent event <EVENT_NAME> ...` as the single event-dispatching
  subcommand. Each event name is its own clap subcommand under `event` with its
  own required/optional payload fields, validated by the CLI. `--harness` and
  `--pane` are declared once as global args on `agentctl agent event` (not
  repeated per event name), so they are guaranteed to be available, with
  identical behavior, on every current and future event subcommand.
- Support five events for now: `SESSION_STARTED`, `RUN_STARTED`, `RUN_FINISHED`,
  `TOOL_CALL_START`, `TOOL_CALL_RESULT`. All five require a unique `id` field (a
  session ID for `SESSION_STARTED`, a run ID for `RUN_*`, a tool-call ID for
  `TOOL_CALL_*`) so terminating events can be reconciled with their starting
  event where applicable. The two `TOOL_CALL_*` events additionally accept an
  optional `type` field (`custom` | `elicitation`); an `elicitation`-typed tool
  call is what drives the agent into the `blocked` status.
- `SESSION_STARTED` replaces the "idle" case of the old `agentctl agent status`:
  everything is now handled through the event dispatcher, with no separate
  non-event bootstrap command. Dispatching `SESSION_STARTED` (re)sets a pane's
  status to `idle` from any prior status (including a stale `working`/`blocked`
  left over from a crashed session) and is recorded in `events.jsonl` like every
  other event.
- Introduce a finite state machine (using the `rust-fsm` crate) that consumes
  the five events and derives the existing
  `idle → working → blocked ⇄ working → done → working → …` status lifecycle,
  replacing the current "harness passes the status directly" model with
  "agentctl computes the status from events." `idle` is the FSM's start state,
  `done` is its end state, reached at the end of each run (a subsequent
  `RUN_STARTED` moves a `done` pane back to `working` for the next run, and
  `SESSION_STARTED` can reset any state back to `idle`).
- **BREAKING**: Rename the `state` field in `state.json` to `status` so it reads
  unambiguously now that the file will need to accommodate other,
  independently-tracked state in future changes (starting with active tool call
  counts). This change does not yet add any such additional field.
- Add an append-only `events.jsonl` file alongside `state.json`
  (`$XDG_DATA_HOME/agentctl/agents/<pane-id>/events.jsonl`) that records every
  dispatched event as one JSON object per line, for inspection and future
  replay/debugging. This is the first consumer of an events-log abstraction
  designed so more event-driven state can be added later without changing the
  logging mechanism.
- Update the `pi` and `claude` hook installers (`agentctl install pi`,
  `agentctl install claude`) to emit the new events instead of calling
  `agentctl agent status`, preserving today's observable status behavior exactly
  (including when `blocked` is entered/exited for Claude's interactive tools and
  permission prompts).
- Update all existing tests and fixtures that assert on
  `agentctl agent status ...` command strings to assert on the new
  `agentctl agent event ...` invocations.

### Explicitly out of scope (future work)

- Tracking or persisting an `active_tool_calls` counter, or any other new
  tracked state derived from `TOOL_CALL_START`/`TOOL_CALL_RESULT` beyond
  deriving `blocked`/`working`.
- Surfacing anything new in `agentctl list` output.
- Replaying or otherwise consuming `events.jsonl` programmatically (it is
  written for manual inspection only in this change).

## Capabilities

### New Capabilities

- `agent-events`: Event-dispatching CLI
  (`agentctl agent event <EVENT_NAME> ...`), the payload schema for
  `SESSION_STARTED`, `RUN_STARTED`, `RUN_FINISHED`, `TOOL_CALL_START`,
  `TOOL_CALL_RESULT`, the FSM that derives `status` from the event stream, and
  the `events.jsonl` append-only log.

### Modified Capabilities

- `agent-state`: `state.json`'s `state` field is renamed to `status`; status is
  now written as a side-effect of event processing (and of
  `agentctl agent clear`) rather than directly by a "set status" command.
- `pi-state-hooks`: The pi extension's lifecycle hooks emit
  `agentctl agent event ...` instead of `agentctl agent status ...`.
- `claude-state-hooks`: The Claude Code hook commands emit
  `agentctl agent event ...` instead of `agentctl agent status ...`.

## Impact

- Affected code: `src/state.rs` (schema rename), `src/commands/agent_status.rs`
  (removed), new `src/commands/agent_event.rs` (event dispatcher, one match arm
  per event, including `SESSION_STARTED`), a new FSM module (e.g.
  `src/status_fsm.rs`) built on `rust-fsm`, a new events-log module (e.g.
  `src/events_log.rs`), `src/commands/hooks.rs` (installer templates +
  matching/upgrade logic for `agentctl agent status` command strings),
  `src/main.rs` (CLI wiring), `tests/agent_commands.rs`, `tests/common/mod.rs`.
- New dependency: `rust-fsm`.
- New on-disk artifact: `events.jsonl` next to `state.json` in each agent's
  state directory.
- No change to `agentctl list`, `agentctl agent clear`, or the stale-state
  validation logic beyond reading the renamed `status` field.
- Users who have already run `agentctl install pi`/`agentctl install claude`
  need to re-run install to pick up the new hook commands (existing
  upgrade-detection logic in `hooks.rs` should be extended to recognize the old
  `agentctl agent status` forms and rewrite them).
