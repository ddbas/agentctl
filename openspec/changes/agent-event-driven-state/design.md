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
an event-dispatching interface (`rd agent event <NAME> ...`) plus a small
non-event bootstrap command (`rd agent init`), and adds a `rust-fsm` state
machine inside `rd` that derives `status` from the event stream. It must
reproduce today's observable status transitions exactly for both harnesses —
this is a refactor, not a behavior change — while establishing a vocabulary
(`RUN_STARTED`/`RUN_FINISHED`/`TOOL_CALL_START`/`TOOL_CALL_RESULT`) that later
changes can extend (e.g. counting active tool calls).

## Goals / Non-Goals

**Goals:**

- Replace `rd agent status` with `rd agent event <EVENT_NAME> ...`, an
  extensible, per-event-validated dispatch subcommand.
- Derive `status` (idle/working/done/blocked) from a `rust-fsm`-based state
  machine driven by the four events, with no observable regression versus
  today's status transitions for either the `pi` or `claude` integration.
- Rename `state.json`'s `state` field to `status`.
- Persist every dispatched event to an append-only `events-log.jsonl` alongside
  `state.json`.
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

Each event name (`RUN_STARTED`, `RUN_FINISHED`, `TOOL_CALL_START`,
`TOOL_CALL_RESULT`) is its own clap subcommand nested under `event`, each with
its own typed args:

```
rd agent event RUN_STARTED       --id <ID> [--harness <H>] [--pane <ID>]
rd agent event RUN_FINISHED      --id <ID> [--harness <H>] [--pane <ID>]
rd agent event TOOL_CALL_START   --id <ID> [--type custom|elicitation] [--harness <H>] [--pane <ID>]
rd agent event TOOL_CALL_RESULT  --id <ID> [--type custom|elicitation] [--harness <H>] [--pane <ID>]
```

`--id` is required on all four (clap `required = true`); `--type` is an optional
`ValueEnum` on the two `TOOL_CALL_*` variants only. This gives us free per-event
validation and `--help` text via clap, and adding a fifth event later is
additive (new subcommand + new FSM transition), matching the brief's "each event
name is really just a subcommand of the event dispatching subcommand."

**Alternative considered**: a single `rd agent event <NAME> --id ... --type ...`
with `NAME` as a free-form `ValueEnum` and one flat arg struct shared by all
events. Rejected because it can't express "‑‑type is only valid for
`TOOL_CALL_*`" or "`--id` required for all four but nothing else required for
`RUN_*`" without runtime checks duplicating what clap already does per
subcommand.

### 2. `rd agent init` stays outside the event/FSM pipeline

`idle` is not one of the four supported events — it's a session bootstrap
concern, not a fact about a run. `rd agent init [--harness] [--pane]` is added
as a sibling to the existing `rd agent clear`, both bypassing the FSM/event log
and writing status directly (`clear` deletes the file; `init` writes
`status: idle`). This preserves the pi/Claude semantics of "at session start,
force the pane back to a known idle state" without inventing a fifth
`SESSION_STARTED`-style event that doesn't fit the run/tool-call vocabulary the
brief defines. `rd agent init` is not written to `events-log.jsonl`, consistent
with `rd agent clear` today not being logged either.

### 3. FSM shape (`rust-fsm`)

States: `Idle | Working | Blocked | Done`. `Idle` is the FSM's declared start
state; `Done` is its declared end state (reached at the close of each run).
Transitions:

| From      | Event                                    | To        |
| --------- | ---------------------------------------- | --------- |
| `Idle`    | `RUN_STARTED`                            | `Working` |
| `Done`    | `RUN_STARTED`                            | `Working` |
| `Working` | `RUN_FINISHED`                           | `Done`    |
| `Working` | `TOOL_CALL_START { type: elicitation }`  | `Blocked` |
| `Blocked` | `TOOL_CALL_RESULT { type: elicitation }` | `Working` |

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

- `session_start` → `rd agent init --harness pi` (was: `status idle`).
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

| Event              | Matcher                         | Command                                                                                                             | Async                                                      |
| ------------------ | ------------------------------- | ------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------- | --- |
| `SessionStart`     | `startup\|resume\|clear`        | `rd agent init --harness claude`                                                                                    | no                                                         |
| `UserPromptSubmit` | —                               | `id=$(cat \| jq -r '.session_id'); rd agent event RUN_STARTED --id "$id" --harness claude`                          | yes                                                        |
| `PreToolUse`       | `AskUserQuestion\|ExitPlanMode` | `id=$(cat \| jq -r '.tool_use_id'); rd agent event TOOL_CALL_START --id "$id" --type elicitation --harness claude`  | yes                                                        |
| `PostToolUse`      | `AskUserQuestion\|ExitPlanMode` | `id=$(cat \| jq -r '.tool_use_id'); rd agent event TOOL_CALL_RESULT --id "$id" --type elicitation --harness claude` | yes                                                        |
| `Notification`     | `permission_prompt`             | `rd agent event TOOL_CALL_START --id "perm-$(cat                                                                    | jq -r '.session_id')" --type elicitation --harness claude` | yes |
| `PostToolUse`      | — (unmatched, all tools)        | `rd agent event TOOL_CALL_RESULT --id "perm-$(cat                                                                   | jq -r '.session_id')" --type elicitation --harness claude` | yes |
| `Stop`             | —                               | `id=$(cat \| jq -r '.session_id'); rd agent event RUN_FINISHED --id "$id" --harness claude`                         | yes                                                        |
| `StopFailure`      | —                               | `id=$(cat \| jq -r '.session_id'); rd agent event RUN_FINISHED --id "$id" --harness claude`                         | yes                                                        |
| `SessionEnd`       | —                               | `rd agent clear`                                                                                                    | no                                                         |

Notes:

- `PostToolUse` is now registered **twice** (once matched to
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
3. Add `rd agent event <NAME>` and `rd agent init`; remove `rd agent status`.
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
- Should `rd agent event`/`rd agent init` read the harness's native hook JSON
  from stdin directly (dropping the `jq` shell-snippet requirement for Claude)
  in a future change?
