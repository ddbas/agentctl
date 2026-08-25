## Context

`rd agent status <state>` is called directly by two harness integrations:

- **pi** (`~/.pi/agent/extensions/radr.ts`, a long-lived in-process TS
  extension): `session_start` → idle, `before_agent_start` → working,
  `agent_settled` → done, `session_shutdown` → `rd agent clear`. pi never
  triggers `blocked` today.
- **Claude Code** (`~/.claude/settings.json` hook entries, each a _fresh,
  stateless `rd` process_ spawned per hook firing, no shared memory between
  invocations): eight hooks map to idle/working/blocked/done, per
  `openspec/specs/claude-state-hooks/spec.md`. Notably, `PreToolUse`/
  `PostToolUse` matched to `AskUserQuestion|ExitPlanMode` set/clear `blocked`,
  `Notification` matched to `permission_prompt` also sets `blocked`, and a
  matcher-less `PostToolUse` (any tool) is the mechanism that clears `blocked`
  back to `working` after a permission prompt is resolved (today it does this by
  unconditionally re-asserting `working`, since it can't tell whether the pane
  was blocked).

This change replaces the direct "set this status" call in both integrations with
an event-dispatching interface (`rd agent event <NAME> ...`), and adds a
`rust-fsm` state machine inside `rd` that derives `status` entirely from the
event stream — including session bootstrap ("idle"), which is modeled as its own
`SESSION_STARTED` event rather than a separate non-event command, so every
status transition flows through the same dispatch/FSM/log pipeline. It must
reproduce today's observable status transitions exactly for both harnesses —
this is a refactor, not a behavior change — while establishing a vocabulary
(`SESSION_STARTED`/`RUN_STARTED`/`RUN_FINISHED`/`TOOL_CALL_START`/
`TOOL_CALL_RESULT`) that later changes can extend (e.g. counting active tool
calls).

## Goals / Non-Goals

**Goals:**

- Replace `rd agent status` with `rd agent event <EVENT_NAME> ...`, an
  extensible, per-event-validated dispatch subcommand, with no non-event escape
  hatch for status changes other than `rd agent clear` (deletion).
- Derive `status` (idle/working/done/blocked) from a `rust-fsm`-based state
  machine driven by the five events, with no observable regression versus
  today's status transitions for either the `pi` or `claude` integration.
- Rename `state.json`'s `state` field to `status`.
- Persist every dispatched event, including `SESSION_STARTED`, to an append-only
  `events-log.jsonl` alongside `state.json`.
- Define concretely how `pi` and `claude` produce the required `id` (and, where
  relevant, `type`) fields for each event, since each harness has very different
  constraints (in-process extension vs. stateless per-hook processes).

**Non-Goals:**

- Tracking or exposing `active_tool_calls` (deferred to a future change).
- Changing `rd list`, `rd agent clear`, or stale-state validation behavior.
- Replaying or otherwise programmatically consuming `events-log.jsonl` (write
  path only, for now).
- Reworking Claude's hook matcher set beyond what's needed to preserve current
  behavior (e.g. not instrumenting every tool call).

## Decisions

### 1. `rd agent event <EVENT_NAME>` as nested clap subcommands

Each event name (`SESSION_STARTED`, `RUN_STARTED`, `RUN_FINISHED`,
`TOOL_CALL_START`, `TOOL_CALL_RESULT`) is its own clap subcommand nested under
`event`, each with its own typed args:

```
rd agent event SESSION_STARTED   --id <ID> [--harness <H>] [--pane <ID>]
rd agent event RUN_STARTED       --id <ID> [--harness <H>] [--pane <ID>]
rd agent event RUN_FINISHED      --id <ID> [--harness <H>] [--pane <ID>]
rd agent event TOOL_CALL_START   --id <ID> [--type custom|elicitation] [--harness <H>] [--pane <ID>]
rd agent event TOOL_CALL_RESULT  --id <ID> [--type custom|elicitation] [--harness <H>] [--pane <ID>]
```

`--id` is required on all five (clap `required = true`); `--type` is an optional
`ValueEnum` on the two `TOOL_CALL_*` variants only. This gives us free per-event
validation and `--help` text via clap, and adding a sixth event later is
additive (new subcommand + new FSM transition), matching the brief's "each event
name is really just a subcommand of the event dispatching subcommand."

**Alternative considered**: a single `rd agent event <NAME> --id ... --type ...`
with `NAME` as a free-form `ValueEnum` and one flat arg struct shared by all
events. Rejected because it can't express "‑‑type is only valid for
`TOOL_CALL_*`" or "`--id` required for all five but nothing else required for
`RUN_*`/`SESSION_STARTED`" without runtime checks duplicating what clap already
does per subcommand.

### 2. Session bootstrap is the `SESSION_STARTED` event, not a separate command

`idle` no longer has a bespoke non-event command. `SESSION_STARTED` is added as
a fifth event so that "a harness session began" is expressed the same way as
every other fact radr tracks: dispatched via `rd agent event`, validated by
clap, run through the FSM, and appended to `events-log.jsonl` like any other
event. This replaces the `rd agent init` idea from an earlier draft of this
proposal — everything now goes through one event-dispatching interface, with
`rd agent clear` (deletion, not a status value) remaining the sole exception,
since it removes the pane's tracked state entirely rather than reporting a fact
about it.

`SESSION_STARTED --id <ID>` requires an `id` for consistency with every other
event and so `events-log.jsonl` entries are uniformly correlatable (e.g. to a
harness's session id), even though `SESSION_STARTED` has no terminating
counterpart to reconcile against.

### 3. FSM shape (`rust-fsm`)

States: `Idle | Working | Blocked | Done`. `Idle` is the FSM's declared start
state; `Done` is its declared end state (reached at the close of each run).
Transitions:

| From      | Event                                    | To        |
| --------- | ---------------------------------------- | --------- |
| `*` (any) | `SESSION_STARTED`                        | `Idle`    |
| `Idle`    | `RUN_STARTED`                            | `Working` |
| `Done`    | `RUN_STARTED`                            | `Working` |
| `Working` | `RUN_FINISHED`                           | `Done`    |
| `Working` | `TOOL_CALL_START { type: elicitation }`  | `Blocked` |
| `Blocked` | `TOOL_CALL_RESULT { type: elicitation }` | `Working` |

`SESSION_STARTED` is a wildcard reset: it transitions the pane back to `Idle`
regardless of its current status (including from a stale `Working`/`Blocked`
left over from a crashed prior session), matching the old `rd agent status idle`
behavior at `session_start`/`SessionStart`.

`TOOL_CALL_START`/`TOOL_CALL_RESULT` with `type: custom` or no `type` are
accepted (written to `events-log.jsonl`) but do not drive any status transition
in this change — they exist for future use (e.g. counting active tool calls) and
are no-ops for `status` today. An event that doesn't match a defined transition
for the current state (e.g. a stray `TOOL_CALL_RESULT` when not `Blocked`) is a
no-op for `status` (logged, not rejected), since hook delivery can race or
duplicate and the FSM must stay lenient rather than error out and break a
harness's hook chain.

`id` is recorded on every event but, in this change, is not used to strictly
reject mismatched start/end pairs — see Open Questions. It's threaded end-to-end
(CLI → FSM input → `events-log.jsonl`) so a later change can add strict
reconciliation (e.g. rejecting a `TOOL_CALL_RESULT` whose `id` doesn't match the
currently-open `TOOL_CALL_START`) without changing the CLI surface.

**Alternative considered**: hand-rolled `match` on an enum instead of
`rust-fsm`. Rejected per the brief's explicit request to bring in `rust-fsm`,
and because the declarative transition-table style scales better as more
events/states are added later.

### 4. `state.json` and `events-log.jsonl` layout

`state.json` keeps its existing fields except `state` → `status`:

```json
{
  "pane_id": "%12",
  "harness": "claude",
  "status": "working",
  "updated_at": "2024-01-01T00:00:00Z"
}
```

`events-log.jsonl` lives at
`$XDG_DATA_HOME/radr/agents/<pane-id>/events-log.jsonl`, one JSON object per
line, e.g.:

```json
{"event":"SESSION_STARTED","id":"...","harness":"claude","pane_id":"%12","timestamp":"..."}
{"event":"RUN_STARTED","id":"...","harness":"claude","pane_id":"%12","timestamp":"..."}
{"event":"TOOL_CALL_START","id":"toolu_01...","type":"elicitation","harness":"claude","pane_id":"%12","timestamp":"..."}
```

Appends are simple `OpenOptions::append(true).create(true)` writes (one line per
event; no rotation/truncation in this change). Because `rd agent clear` already
`remove_dir_all`s the whole per-pane directory, the log is cleaned up for free
when a session ends — no new cleanup code needed.

### 5. How `pi` produces event payloads

pi's extension is a single long-lived process per pi session, so it can hold
in-memory state across hook callbacks:

- `session_start` → generate (or reuse pi's own session id if exposed) an id and
  call `rd agent event SESSION_STARTED --id <sessionId> --harness pi` (was:
  `status idle`).
- `before_agent_start` → generate a run id with `crypto.randomUUID()`, store it
  in a module-scoped variable, call
  `rd agent event RUN_STARTED --id <runId> --harness pi`.
- `agent_settled` → `rd agent event RUN_FINISHED --id <runId> --harness pi`
  using the same stored id from `before_agent_start`.
- `session_shutdown` → `rd agent clear` (unchanged).

pi does not currently expose a hook for interactive/elicitation-style tool
prompts, so pi never enters `blocked` today, and this change does not add that
(no regression: `blocked` was already unreachable via pi). Wiring pi's tool-call
lifecycle into `TOOL_CALL_START`/`TOOL_CALL_RESULT` is future work.

### 6. How Claude Code produces event payloads

Claude hook commands are spawned fresh per firing with no shared process state,
and Claude passes the hook's JSON payload on the command's stdin (not as CLI
args). Following Claude's own documented pattern for stdin-parsing hooks (`jq`),
each hook command becomes a small shell snippet that extracts the field it needs
from stdin and forwards it as `--id`:

| Event              | Matcher                         | Command                                                                                                                 | Async |
| ------------------ | ------------------------------- | ----------------------------------------------------------------------------------------------------------------------- | ----- |
| `SessionStart`     | `startup\|resume\|clear`        | `id=$(cat \| jq -r '.session_id'); rd agent event SESSION_STARTED --id "$id" --harness claude`                          | no    |
| `UserPromptSubmit` | —                               | `id=$(cat \| jq -r '.session_id'); rd agent event RUN_STARTED --id "$id" --harness claude`                              | yes   |
| `PreToolUse`       | `AskUserQuestion\|ExitPlanMode` | `id=$(cat \| jq -r '.tool_use_id'); rd agent event TOOL_CALL_START --id "$id" --type elicitation --harness claude`      | yes   |
| `PostToolUse`      | `AskUserQuestion\|ExitPlanMode` | `id=$(cat \| jq -r '.tool_use_id'); rd agent event TOOL_CALL_RESULT --id "$id" --type elicitation --harness claude`     | yes   |
| `Notification`     | `permission_prompt`             | `id=$(cat \| jq -r '.session_id'); rd agent event TOOL_CALL_START --id "perm-$id" --type elicitation --harness claude`  | yes   |
| `PostToolUse`      | — (unmatched, all tools)        | `id=$(cat \| jq -r '.session_id'); rd agent event TOOL_CALL_RESULT --id "perm-$id" --type elicitation --harness claude` | yes   |
| `Stop`             | —                               | `id=$(cat \| jq -r '.session_id'); rd agent event RUN_FINISHED --id "$id" --harness claude`                             | yes   |
| `StopFailure`      | —                               | `id=$(cat \| jq -r '.session_id'); rd agent event RUN_FINISHED --id "$id" --harness claude`                             | yes   |
| `SessionEnd`       | —                               | `rd agent clear`                                                                                                        | no    |

Notes:

- `SessionStart` now reads `session_id` from stdin via `jq`, same as the other
  hooks, instead of calling a fixed no-payload command — this keeps every hook
  entry using the same "extract from stdin, dispatch an event" shape.
- `PostToolUse` is registered **twice** (once matched to
  `AskUserQuestion|ExitPlanMode`, once unmatched for all tools) instead of once
  unconditionally. The unmatched registration's only job is to close out an
  outstanding `permission_prompt`-triggered block using the same synthetic
  `perm-<session_id>` id used to open it; since the FSM treats an unmatched
  `TOOL_CALL_RESULT` (nothing currently `Blocked`, or a mismatched id — see Open
  Questions) as a no-op, this is safe to fire on every tool completion without
  unconditionally forcing `working` the way the old "any `PostToolUse` →
  working" hook did. This is what lets us drop the old always-on refresh while
  still clearing the `permission_prompt` block.
- `UserPromptSubmit`/`Stop`/`StopFailure` reuse Claude's `session_id` as the run
  `id`. `session_id` is stable across an entire session (not unique per turn),
  which is an acceptable trade-off in this change since Claude only ever has one
  active run per session at a time (see Open Questions for a future
  improvement).
- `rd install claude` / `rd uninstall claude` recognition logic (which currently
  pattern-matches literal `rd agent status ...` / `rd agent clear` command
  strings) is extended to recognize these new shell-snippet forms as
  radr-managed, and to migrate the old literal `rd agent status ...` forms to
  the new ones, mirroring the existing `set-state` → `state set` →
  `agent status` migration chain.

### 7. `jq` as an implicit runtime dependency for the Claude integration

The Claude hook commands above require `jq` to be on `PATH`. This is already
true of Claude's own documented hook examples (block-rm.sh etc.), so we don't
consider this a new burden specific to radr, but `rd install claude` should note
this requirement in its help/summary output.

**Alternative considered**: have `rd agent event` itself accept the raw Claude
hook JSON on stdin and extract fields internally (no `jq`, no shell snippet),
via e.g. `--from-claude-hook`. Rejected for this change to keep the CLI's input
model uniform (`--id`/`--type` flags, always) and avoid baking a
harness-specific parsing mode into the core dispatcher; revisit if `jq` proves
to be a real installation obstacle in practice.

## Risks / Trade-offs

- **[Risk]** `session_id`-based `id` for Claude's `RUN_STARTED`/`RUN_FINISHED`
  doesn't disambiguate individual turns within one session → **Mitigation**:
  acceptable today because Claude runs turns sequentially (never two concurrent
  runs in one pane); flagged as an Open Question for a future, more precise
  per-turn id.
- **[Risk]** `jq` must be installed for the Claude integration to work →
  **Mitigation**: already an implicit requirement of Claude's own hook examples;
  document it in `rd install claude` output.
- **[Risk]** `SESSION_STARTED`'s wildcard "reset to idle from any state"
  transition means a stray/duplicate `SESSION_STARTED` could clobber a
  legitimately `working`/`blocked` pane → **Mitigation**: this exactly mirrors
  today's behavior (`rd agent status idle` was already unconditional), and
  `SESSION_STARTED` is only ever dispatched from `session_start`/`SessionStart`,
  which Claude/pi fire once per session start/resume/clear, not mid-run.
- **[Risk]** Leniency in the FSM (unknown/mismatched transitions are
  logged-but-ignored no-ops rather than errors) could mask a genuinely broken
  harness integration → **Mitigation**: every event is still written to
  `events-log.jsonl` regardless of whether it drove a transition, so mismatches
  are inspectable after the fact.
- **[Trade-off]** Splitting `PostToolUse` into two hook registrations (matched +
  unmatched) instead of Claude's previous single unmatched registration adds one
  more entry to `rd install claude`'s generated settings. This is judged an
  acceptable, well-contained increase in exchange for dropping the unconditional
  "every tool call forces working" refresh.

## Migration Plan

1. Land `rust-fsm` dependency, FSM module, and rename `state.json`'s `state`
   field to `status` (with existing read/write helpers and their tests updated
   in place — no separate migration of on-disk files is needed since
   `rd agent clear` already deletes state on session end and `rd list`'s
   stale-state validation prunes anything else).
2. Add `events-log.jsonl` writing, wired through the same write path as
   `state.json` so every event append and status write happen together.
3. Add `rd agent event <NAME>` (including `SESSION_STARTED`); remove
   `rd agent status`.
4. Update `rd install pi` / `rd install claude` templates and their
   idempotent-upgrade detection to the new command forms; add a migration path
   recognizing the old `rd agent status ...` forms (extending the existing
   `set-state` → `state set` → `agent status` chain).
5. Update all tests/fixtures across `tests/agent_commands.rs`,
   `tests/common/mod.rs`, and `src/commands/hooks.rs` unit tests that assert on
   old command strings.

Rollback: since this is a pre-release internal tool with no persisted data
migration concerns (state files are ephemeral/regenerated per session), a
rollback is simply reverting the commit(s) and re-running
`rd install pi`/`rd install claude` to restore the old hook commands.

## Open Questions

- Should Claude's `RUN_STARTED`/`RUN_FINISHED` `id` be made per-turn (not just
  per-session) once Claude Code exposes a stable per-turn identifier in hook
  payloads (today only `session_id` is documented as stable/available)?
- Should the FSM eventually reject (non-zero exit) a `TOOL_CALL_RESULT` whose
  `id` doesn't match the currently-open `TOOL_CALL_START`, instead of silently
  no-op'ing, once harness integrations are proven reliable enough that a
  mismatch signals a real bug worth surfacing?
- Should `rd agent event` read the harness's native hook JSON from stdin
  directly (dropping the `jq` shell-snippet requirement for Claude) in a future
  change?
