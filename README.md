# 📡 Radr

A tmux-based agent multiplexer. Spot and manage AI agent processes running
across your tmux sessions.

## Installation

```bash
cargo install --path .
```

## Usage

```bash
rd list                       # list all agent panes with their current status
rd agent event <NAME> ...     # dispatch a typed event from a hook
rd agent clear                # remove the state file for the current pane
rd install <harness>          # install harness hooks (pi, claude)
```

## Commands

### `rd list`

Queries every pane across all tmux sessions via `tmux list-panes -a` and prints
one line per pane whose foreground process is a known agent harness (`claude` or
`pi`). By default, output is tab-delimited plain text with five columns:

```
<agent-name>\t<session-name>\t<window>.<pane>\t<status>\t<activeTools>
```

Every row always has the same number of tab-separated fields as the number of
rendered columns, even when a trailing column's value is empty (e.g. an empty
`activeTools` or `updatedAt` cell) — this keeps the output reliably parseable by
tools like `cut -f` or `awk -F'\t'`.

The `status` column shows the agent's last derived status, or `unknown` if no
state file exists for that pane. The `activeTools` column shows the number of
in-flight tool calls (`N tool`/`N tools`), empty when zero, absent, or when
`status` is `unknown`.

Use `--columns`/`-c` to choose which columns are rendered and in what order, as
a comma-delimited list:

```bash
rd list --columns agent,status          # only agent and status
rd list --columns status,agent,updatedAt
```

Valid column names (identical vocabulary to `--sort-by`): `agent`, `session`,
`pane`, `status`, `updatedAt`, `activeTools`. `updatedAt` renders the agent's
last state-file update timestamp in RFC 3339 UTC form (e.g.
`2024-06-01T14:32:05+00:00`), empty when no state file exists.

Use `--sort-by` to control ordering (default: `updatedAt`, newest first).
`--sort-by activeTools` sorts numerically, busiest agents first by default. Pass
`--reverse` to flip the natural direction of either flag's key.

> **Breaking change**: the old `--sort-by state` and `--sort-by updated-at`
> values have been renamed to `status` and `updatedAt` respectively, to match
> the shared camelCase vocabulary used by `--columns`. Scripts using the old
> spellings must be updated.

As a side effect, `rd list` validates persisted state files and deletes stale
ones (see "Agent state tracking" below).

### `rd agent event <EVENT_NAME>`

Dispatch a typed event; the pane's `status` is derived from the stream of
dispatched events via a finite state machine (see "Agent state tracking" below),
rather than being set directly. Intended to be called from agent harness hooks.

Supported event names, each its own subcommand with its own required/optional
payload fields:

```bash
rd agent event SESSION_STARTED  --id <ID>                          [--harness <H>] [--pane <ID>]
rd agent event RUN_STARTED      --id <ID>                          [--harness <H>] [--pane <ID>]
rd agent event RUN_FINISHED     --id <ID> [--active-tools <N>]     [--harness <H>] [--pane <ID>]
rd agent event TOOL_CALL_START  --id <ID> [--type custom|elicitation|permission-prompt] [--harness <H>] [--pane <ID>]
rd agent event TOOL_CALL_RESULT --id <ID> [--type custom|elicitation|permission-prompt] [--harness <H>] [--pane <ID>]
```

```bash
rd agent event RUN_STARTED --id r1 --harness claude
rd agent event SESSION_STARTED --id s1 --harness pi --pane %12
```

Flags:

- `--id <id>` — required on all five events; uniquely identifies the event's
  subject (a session ID for `SESSION_STARTED`, a run ID for `RUN_*`, a tool-call
  ID for `TOOL_CALL_*`)
- `--type <custom|elicitation|permission-prompt>` — optional, and only accepted
  on `TOOL_CALL_START` / `TOOL_CALL_RESULT`; an `elicitation`-typed tool call
  drives the agent into (or out of) the `blocked` status. A
  `permission-prompt`-typed tool call also drives `blocked`/`working`, but via
  its own `blocked_prompts` counter (see "Agent state tracking" below) instead
  of `active_tools`, so that two concurrent permission prompts in the same
  session can't have the first one resolved prematurely clear the other.
- `--active-tools <n>` — optional, and only accepted on `RUN_FINISHED`;
  overrides the pane's `activeTools` counter to `n` instead of leaving it
  untouched. Reconciles radr's own increment/decrement counter against a
  harness's authoritative in-flight background-task count. This matters because
  some tools launch async background work and return (firing `PostToolUse`,
  which decrements the counter) well before that background work actually
  finishes — Claude Code's `Monitor` tool (event streaming from a background
  process) and backgrounded shells are the main examples. See "Claude lifecycle
  event → event mapping" below for how the `claude` harness populates this from
  the `background_tasks` array delivered to `Stop`/`StopFailure` hook payloads.
  `RUN_FINISHED` drives the pane to `done` from **any** current status
  (`idle`/`working`/`blocked`/`done`), provided this reconciled `active_tools`
  value is `0`; when it is greater than `0`, `RUN_FINISHED` leaves `status`
  unchanged
- `--harness <name>` — harness identifier written to the state file and event
  log (default: `"unknown"`); declared once as a global arg shared by every
  event subcommand
- `--pane <pane-id>` — override the pane ID instead of reading `$TMUX_PANE`;
  also declared once as a global arg shared by every event subcommand

`--harness` and `--pane` may appear either before or after the event name.

Exits 0 silently when `$TMUX_PANE` is not set and `--pane` is not supplied —
safe to call unconditionally from hooks that may run outside tmux.

Every dispatched event, including `SESSION_STARTED`, is also appended to an
append-only `events.jsonl` log next to `state.json` (see below).

### `rd agent clear`

Delete the state file (and its `events.jsonl` sibling) for the current pane
(`$TMUX_PANE`). Typically called from a session-end hook.

```bash
rd agent clear           # uses $TMUX_PANE
rd agent clear --pane %12
```

Exits 0 silently when `$TMUX_PANE` is not set, or when no state file exists.

### `rd install <harness>`

Installs hooks for the specified agent harness.

Supported harnesses: `claude`, `pi`.

#### `claude`

Installs a self-contained Claude Code plugin at `~/.claude/skills/radr/`,
consisting of a plugin manifest (`.claude-plugin/plugin.json`) and a hooks
configuration (`hooks/hooks.json`) registering eleven hook entries that call
`rd agent event` and `rd agent clear` at key Claude Code lifecycle events. Each
hook command reads the hook's JSON payload from stdin via `jq` and forwards the
extracted field (`session_id` or `tool_use_id`) as `--id` — **`jq` must be on
`PATH`** for these hooks to work.

The command never reads from or writes to `~/.claude/settings.json`. Install is
remove-then-write, not merge: any existing `~/.claude/skills/radr/` is deleted
in full before the plugin is written fresh, so it's simple and idempotent by
construction — re-running always produces the same on-disk result.

```bash
rd install claude
```

Example output:

```
Installed radr plugin for claude (11 hook entries):
  + ~/.claude/skills/radr/.claude-plugin/plugin.json
  + ~/.claude/skills/radr/hooks/hooks.json
    SessionStart (startup|resume|clear) → id=$(cat | jq -r '.session_id'); rd agent event SESSION_STARTED --id "$id" --harness claude
    UserPromptSubmit                    → id=$(cat | jq -r '.session_id'); rd agent event RUN_STARTED --id "$id" --harness claude [async]
    PreToolUse                          → id=$(cat | jq -r '.tool_use_id'); rd agent event TOOL_CALL_START --id "$id" --harness claude [async]
    PreToolUse (AskUserQuestion|ExitPlanMode) → id=$(cat | jq -r '.tool_use_id'); rd agent event TOOL_CALL_START --id "$id" --type elicitation --harness claude [async]
    PostToolUse                         → id=$(cat | jq -r '.tool_use_id'); rd agent event TOOL_CALL_RESULT --id "$id" --harness claude [async]
    PostToolUse (AskUserQuestion|ExitPlanMode) → id=$(cat | jq -r '.tool_use_id'); rd agent event TOOL_CALL_RESULT --id "$id" --type elicitation --harness claude [async]
    PostToolUse                         → id=$(cat | jq -r '.session_id'); rd agent event TOOL_CALL_RESULT --id "perm-$id-$RANDOM" --type permission-prompt --harness claude [async]
    Notification (permission_prompt)    → id=$(cat | jq -r '.session_id'); rd agent event TOOL_CALL_START --id "perm-$id-$RANDOM" --type permission-prompt --harness claude [async]
    Stop                                → payload=$(cat); id=$(echo "$payload" | jq -r '.session_id'); n=$(echo "$payload" | jq -r '(.background_tasks // []) | length'); rd agent event RUN_FINISHED --id "$id" --active-tools "$n" --harness claude [async]
    StopFailure                         → payload=$(cat); id=$(echo "$payload" | jq -r '.session_id'); n=$(echo "$payload" | jq -r '(.background_tasks // []) | length'); rd agent event RUN_FINISHED --id "$id" --active-tools "$n" --harness claude [async]
    SessionEnd                          → rd agent clear
(requires `jq` on PATH to parse hook payloads)
```

`rd uninstall claude` removes `~/.claude/skills/radr/` entirely (and is a no-op
if it doesn't exist); it likewise never touches `~/.claude/settings.json`.

**Claude lifecycle event → event mapping:**

| Claude hook event  | Matcher                         | Event dispatched                                                             | Async |
| ------------------ | ------------------------------- | ---------------------------------------------------------------------------- | ----- |
| `SessionStart`     | `startup\|resume\|clear`        | `SESSION_STARTED --id <session_id>`                                          | no    |
| `UserPromptSubmit` | _(none)_                        | `RUN_STARTED --id <session_id>`                                              | yes   |
| `PreToolUse`       | _(none — all tools)_            | `TOOL_CALL_START --id <tool_use_id>`                                         | yes   |
| `PreToolUse`       | `AskUserQuestion\|ExitPlanMode` | `TOOL_CALL_START --id <tool_use_id> --type elicitation`                      | yes   |
| `PostToolUse`      | _(none — all tools)_            | `TOOL_CALL_RESULT --id <tool_use_id>`                                        | yes   |
| `PostToolUse`      | `AskUserQuestion\|ExitPlanMode` | `TOOL_CALL_RESULT --id <tool_use_id> --type elicitation`                     | yes   |
| `PostToolUse`      | _(none — all tools)_            | `TOOL_CALL_RESULT --id "perm-<session_id>-<nonce>" --type permission-prompt` | yes   |
| `Notification`     | `permission_prompt`             | `TOOL_CALL_START --id "perm-<session_id>-<nonce>" --type permission-prompt`  | yes   |
| `Stop`             | _(none)_                        | `RUN_FINISHED --id <session_id> --active-tools <n>`                          | yes   |
| `StopFailure`      | _(none)_                        | `RUN_FINISHED --id <session_id> --active-tools <n>`                          | yes   |
| `SessionEnd`       | _(none)_                        | `rd agent clear`                                                             | no    |

`<n>` above is `(.background_tasks // []) | length` from the hook payload — the
number of tasks (of type `shell`, `subagent`, `monitor`, `workflow`, `teammate`,
`cloud session`, or `MCP task`) still in flight when the turn ends. This
reconciles `activeTools` against Claude's own authoritative accounting: the
unmatched `PreToolUse`/`PostToolUse` pair above only tracks the launching tool
call itself, which returns (firing `PostToolUse`, and decrementing the counter)
as soon as async background work is kicked off — not when that work actually
finishes. Without this reconciliation, a `Monitor` tool streaming a background
process's output, or a backgrounded shell command, would show `activeTools: 0`
in `rd list` well before the underlying work is done. `RUN_FINISHED` is
authoritative regardless of the pane's current status, provided this reconciled
`active_tools` value is `0` — see "Agent state tracking" below.

`<nonce>` above is a per-invocation unique value (e.g. `$RANDOM`) so each
permission-prompt open/close shows up as a distinct line in `events.jsonl` for
debugging; it plays no role in status derivation.

`SessionStart` and `SessionEnd` run synchronously so the state file is
established before any async hook fires and is cleaned up before the session
exits. All other hooks use `async: true` to avoid blocking Claude's operation.

The unmatched, untyped `PreToolUse`/`PostToolUse` entries fire for every tool
call (mirroring the pi extension's `tool_call`/`tool_result` handlers) and drive
the pane's `active_tools` count, independent of the `blocked`-status machinery.

The unmatched `PostToolUse` registration with the `perm-<session_id>-<nonce>` id
exists solely to close out a `permission_prompt`-triggered block, via the pane's
`blocked_prompts` counter (independent of `active_tools`): `blocked` is only
cleared once `blocked_prompts` returns to `0`, so a still-pending, concurrent
permission prompt opened separately isn't prematurely cleared by an unrelated
tool's completion.

#### `pi`

Writes a TypeScript extension to `~/.pi/agent/extensions/radr.ts` that
subscribes to pi lifecycle events and calls `rd agent event` / `rd agent clear`.
The command is idempotent: re-running when the file is already up to date
reports no change.

```bash
rd install pi
```

Example output (first install):

```
Installed radr extension for pi:
  + ~/.pi/agent/extensions/radr.ts
    session_start        → rd agent event SESSION_STARTED --harness pi
    before_agent_start   → rd agent event RUN_STARTED --harness pi
    agent_settled        → rd agent event RUN_FINISHED --harness pi
    session_shutdown     → rd agent clear
```

On a subsequent run (file already up to date):

```
Installed radr extension for pi:
  = ~/.pi/agent/extensions/radr.ts (already up to date)
```

The `~` prefix is used when the file existed but had outdated content and was
overwritten.

**Pi lifecycle event → event mapping:**

| Pi event             | Event dispatched                                                 |
| -------------------- | ---------------------------------------------------------------- |
| `session_start`      | `SESSION_STARTED --id <sessionId>` (new random id)               |
| `before_agent_start` | `RUN_STARTED --id <runId>` (new random id)                       |
| `agent_settled`      | `RUN_FINISHED --id <runId>` (same id as the run's `RUN_STARTED`) |
| `session_shutdown`   | `rd agent clear`                                                 |

Note: pi does not currently expose a hook for interactive/elicitation-style tool
prompts, so pi never dispatches `TOOL_CALL_START`/`TOOL_CALL_RESULT` and never
enters `blocked` today.

## Agent State Tracking

### Status model

`status` is derived from the event stream by a finite state machine (built on
the `rust-fsm` crate) rather than being set directly. Each agent pane can be in
one of five statuses:

| Status    | Meaning                             |
| --------- | ----------------------------------- |
| `idle`    | Waiting for user input              |
| `working` | Actively processing a task          |
| `done`    | Task complete, awaiting next prompt |
| `blocked` | Waiting for external input/action   |
| `unknown` | No state file, or state unreadable  |

**Transition table:**

| From      | Event                                                                                         | To        |
| --------- | --------------------------------------------------------------------------------------------- | --------- |
| `*` (any) | `SESSION_STARTED`                                                                             | `idle`    |
| `idle`    | `RUN_STARTED`                                                                                 | `working` |
| `done`    | `RUN_STARTED`                                                                                 | `working` |
| `*` (any) | `RUN_FINISHED`, provided reconciled `active_tools == 0`                                       | `done`    |
| `working` | `TOOL_CALL_START` with `type: elicitation`                                                    | `blocked` |
| `blocked` | `TOOL_CALL_RESULT` with `type: elicitation`                                                   | `working` |
| `working` | `TOOL_CALL_START` with `type: permission-prompt`                                              | `blocked` |
| `blocked` | `TOOL_CALL_RESULT` with `type: permission-prompt`, provided reconciled `blocked_prompts == 0` | `working` |

`RUN_FINISHED` is authoritative from any current status — including `idle` and
`blocked` — since `Stop`/`StopFailure` firing means the harness itself says the
turn is over; a pane that drifted to `blocked` (e.g. a permission prompt that
never got explicitly cleared) still reaches `done`. It only refuses to fire
(leaving `status` unchanged) when the reconciled `active_tools` count is greater
than `0` — i.e. there's still genuinely in-flight work.

`permission-prompt`-typed tool calls behave like `elicitation` for status, but
are gated on a separate `blocked_prompts` counter instead of `active_tools`, so
two concurrent permission prompts in the same session can't have the first one
resolved prematurely clear the other (see "File layout" below).

An event that doesn't match a defined transition for the pane's current status
is a no-op for `status` (it is still appended to `events.jsonl`).

### File layout

State is persisted as a JSON file at:

```
$XDG_DATA_HOME/radr/agents/<pane-id>/state.json
```

where `XDG_DATA_HOME` defaults to `~/.local/share` (standard XDG base dir spec).
`<pane-id>` is the tmux pane ID (e.g. `%12`), available as `$TMUX_PANE` inside
any process running in a tmux pane.

**Example state file** (`~/.local/share/radr/agents/%12/state.json`):

```json
{
  "pane_id": "%12",
  "harness": "claude",
  "status": "working",
  "updated_at": "2026-07-14T10:30:00Z",
  "active_tools": 0,
  "blocked_prompts": 0
}
```

Files are atomically overwritten (write to `.tmp` sibling, then rename) so
concurrent readers never see a partial write.

Every dispatched event is additionally appended, one JSON object per line, to an
`events.jsonl` file alongside `state.json`:

```
$XDG_DATA_HOME/radr/agents/<pane-id>/events.jsonl
```

**Example line:**

```json
{
  "event": "RUN_STARTED",
  "id": "r1",
  "harness": "claude",
  "pane_id": "%12",
  "timestamp": "2026-07-14T10:30:00Z"
}
```

This log is written for manual inspection and future replay/debugging; nothing
in `rd` reads it back today. Because `rd agent clear` deletes the whole per-pane
directory, `events.jsonl` is cleaned up automatically at session end.

### Stale file cleanup

When `rd list` runs, it validates all state files against the live tmux pane set
and deletes stale ones:

- **Orphaned** — `pane_id` not in the live pane set → deleted
- **Contradictory** — `pane_id` exists but `harness` (when not `"unknown"`)
  doesn't match `pane_current_command` → deleted
- **Unknown harness** — `harness` is `"unknown"` → only the orphan check applies

If the tmux query fails, validation is skipped entirely and no files are
deleted.

### Integrating from a harness hook

Call `rd agent event` from your harness lifecycle hooks. Example (shell):

```bash
# On session start
rd agent event SESSION_STARTED --id "$SESSION_ID" --harness myharness

# On task start
rd agent event RUN_STARTED --id "$RUN_ID" --harness myharness

# On task complete
rd agent event RUN_FINISHED --id "$RUN_ID" --harness myharness

# On session end
rd agent clear
```
