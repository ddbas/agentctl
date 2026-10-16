# 📡 Agentctl

A tmux-based agent multiplexer. Spot and manage AI agent processes running
across your tmux sessions.

## Installation

Requires [mise](https://mise.jdx.dev) (pins Node and pnpm) and `tmux`.

```bash
mise install
mise exec -- pnpm install --frozen-lockfile
mise run install             # builds the bundle and symlinks ~/.local/bin/agentctl
```

Set `AGENTCTL_INSTALL_DIR` to install somewhere other than `~/.local/bin`.

## Usage

```bash
agentctl list                       # list all agent panes with their current status
agentctl agent event <NAME> ...     # dispatch a typed event from a hook
agentctl agent clear                # remove the state file for the current pane
agentctl install <harness>          # install harness hooks (pi, claude)
```

## Commands

### `agentctl list`

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
agentctl list --columns agent,status          # only agent and status
agentctl list --columns status,agent,updatedAt
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

As a side effect, `agentctl list` validates persisted state files and deletes
stale ones (see "Agent state tracking" below).

### `agentctl agent event <EVENT_NAME>`

Dispatch a typed event; the pane's `status` is derived from the stream of
dispatched events via a finite state machine (see "Agent state tracking" below),
rather than being set directly. Intended to be called from agent harness hooks.

Supported event names, each its own subcommand with its own required/optional
payload fields:

```bash
agentctl agent event SESSION_STARTED  --id <ID>                          [--harness <H>] [--pane <ID>]
agentctl agent event RUN_STARTED      --id <ID>                          [--harness <H>] [--pane <ID>]
agentctl agent event RUN_FINISHED     --id <ID> [--active-tools <N>]     [--harness <H>] [--pane <ID>]
agentctl agent event TOOL_CALL_START  --id <ID> [--type custom|elicitation] [--harness <H>] [--pane <ID>]
agentctl agent event TOOL_CALL_RESULT --id <ID> [--type custom|elicitation] [--harness <H>] [--pane <ID>]
```

```bash
agentctl agent event RUN_STARTED --id r1 --harness claude
agentctl agent event SESSION_STARTED --id s1 --harness pi --pane %12
```

Flags:

- `--id <id>` — required on all five events; uniquely identifies the event's
  subject (a session ID for `SESSION_STARTED`, a run ID for `RUN_*`, a tool-call
  ID for `TOOL_CALL_*`)
- `--type <custom|elicitation>` — optional, and only accepted on
  `TOOL_CALL_START` / `TOOL_CALL_RESULT`; an `elicitation`-typed tool call
  drives the agent into (or out of) the `blocked` status.
- `--active-tools <n>` — optional, and only accepted on `RUN_FINISHED`;
  overrides the pane's `activeTools` counter to `n` instead of leaving it
  untouched. Reconciles agentctl's own increment/decrement counter against a
  harness's authoritative in-flight background-task count. This matters because
  some tools launch async background work and return (firing `PostToolUse`,
  which decrements the counter) well before that background work actually
  finishes — Claude Code's `Monitor` tool (event streaming from a background
  process) and backgrounded shells are the main examples. See "Claude lifecycle
  event → event mapping" below for how the `claude` harness populates this from
  the `background_tasks` array delivered to `Stop`/`StopFailure` hook payloads.
  `RUN_FINISHED` drives the pane to `done` from **any** current status
  (`idle`/`working`/`blocked`/`done`), regardless of this value; `activeTools`
  is informational only and never affects `status`
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

### `agentctl agent clear`

Delete the state file (and its `events.jsonl` sibling) for the current pane
(`$TMUX_PANE`). Typically called from a session-end hook.

```bash
agentctl agent clear           # uses $TMUX_PANE
agentctl agent clear --pane %12
```

Exits 0 silently when `$TMUX_PANE` is not set, or when no state file exists.

### `agentctl install <harness>`

Installs hooks for the specified agent harness.

Supported harnesses: `claude`, `pi`.

#### `claude`

Installs a self-contained Claude Code plugin at `~/.claude/skills/agentctl/`,
consisting of a plugin manifest (`.claude-plugin/plugin.json`) and a hooks
configuration (`hooks/hooks.json`) registering eleven hook entries that call
`agentctl agent event` and `agentctl agent clear` at key Claude Code lifecycle
events. Each hook command reads the hook's JSON payload from stdin via `jq` and
forwards the extracted field (`session_id` or `tool_use_id`) as `--id` — **`jq`
must be on `PATH`** for these hooks to work.

The command never reads from or writes to `~/.claude/settings.json`. Install is
remove-then-write, not merge: any existing `~/.claude/skills/agentctl/` is
deleted in full before the plugin is written fresh, so it's simple and
idempotent by construction — re-running always produces the same on-disk result.

```bash
agentctl install claude
```

Example output:

```
Installed agentctl plugin for claude (11 hook entries):
  + ~/.claude/skills/agentctl/.claude-plugin/plugin.json
  + ~/.claude/skills/agentctl/hooks/hooks.json
    SessionStart (startup|resume|clear) → id=$(cat | jq -r '.session_id'); agentctl agent event SESSION_STARTED --id "$id" --harness claude
    UserPromptSubmit                    → id=$(cat | jq -r '.session_id'); agentctl agent event RUN_STARTED --id "$id" --harness claude [async]
    PreToolUse                          → id=$(cat | jq -r '.tool_use_id'); agentctl agent event TOOL_CALL_START --id "$id" --harness claude [async]
    PreToolUse (AskUserQuestion|ExitPlanMode) → id=$(cat | jq -r '.tool_use_id'); agentctl agent event TOOL_CALL_START --id "$id" --type elicitation --harness claude [async]
    PostToolUse                         → id=$(cat | jq -r '.tool_use_id'); agentctl agent event TOOL_CALL_RESULT --id "$id" --harness claude [async]
    PostToolUse (AskUserQuestion|ExitPlanMode) → id=$(cat | jq -r '.tool_use_id'); agentctl agent event TOOL_CALL_RESULT --id "$id" --type elicitation --harness claude [async]
    Stop                                → payload=$(cat); id=$(echo "$payload" | jq -r '.session_id'); n=$(echo "$payload" | jq -r '(.background_tasks // []) | length'); agentctl agent event RUN_FINISHED --id "$id" --active-tools "$n" --harness claude [async]
    StopFailure                         → payload=$(cat); id=$(echo "$payload" | jq -r '.session_id'); n=$(echo "$payload" | jq -r '(.background_tasks // []) | length'); agentctl agent event RUN_FINISHED --id "$id" --active-tools "$n" --harness claude [async]
    SessionEnd                          → agentctl agent clear
(requires `jq` on PATH to parse hook payloads)
```

`agentctl uninstall claude` removes `~/.claude/skills/agentctl/` entirely (and
is a no-op if it doesn't exist); it likewise never touches
`~/.claude/settings.json`.

**Claude lifecycle event → event mapping:**

| Claude hook event  | Matcher                         | Event dispatched                                         | Async |
| ------------------ | ------------------------------- | -------------------------------------------------------- | ----- |
| `SessionStart`     | `startup\|resume\|clear`        | `SESSION_STARTED --id <session_id>`                      | no    |
| `UserPromptSubmit` | _(none)_                        | `RUN_STARTED --id <session_id>`                          | yes   |
| `PreToolUse`       | _(none — all tools)_            | `TOOL_CALL_START --id <tool_use_id>`                     | yes   |
| `PreToolUse`       | `AskUserQuestion\|ExitPlanMode` | `TOOL_CALL_START --id <tool_use_id> --type elicitation`  | yes   |
| `PostToolUse`      | _(none — all tools)_            | `TOOL_CALL_RESULT --id <tool_use_id>`                    | yes   |
| `PostToolUse`      | `AskUserQuestion\|ExitPlanMode` | `TOOL_CALL_RESULT --id <tool_use_id> --type elicitation` | yes   |
| `Stop`             | _(none)_                        | `RUN_FINISHED --id <session_id> --active-tools <n>`      | yes   |
| `StopFailure`      | _(none)_                        | `RUN_FINISHED --id <session_id> --active-tools <n>`      | yes   |
| `SessionEnd`       | _(none)_                        | `agentctl agent clear`                                   | no    |

`<n>` above is `(.background_tasks // []) | length` from the hook payload — the
number of tasks (of type `shell`, `subagent`, `monitor`, `workflow`, `teammate`,
`cloud session`, or `MCP task`) still in flight when the turn ends. This
reconciles `activeTools` against Claude's own authoritative accounting: the
unmatched `PreToolUse`/`PostToolUse` pair above only tracks the launching tool
call itself, which returns (firing `PostToolUse`, and decrementing the counter)
as soon as async background work is kicked off — not when that work actually
finishes. Without this reconciliation, a `Monitor` tool streaming a background
process's output, or a backgrounded shell command, would show `activeTools: 0`
in `agentctl list` well before the underlying work is done. `RUN_FINISHED` is
authoritative regardless of the pane's current status and of this value: the
pane goes `done` even with background work still running, and `activeTools`
shows the remaining background work — see "Agent state tracking" below.

`SessionStart` and `SessionEnd` run synchronously so the state file is
established before any async hook fires and is cleaned up before the session
exits. All other hooks use `async: true` to avoid blocking Claude's operation.

The unmatched, untyped `PreToolUse`/`PostToolUse` entries fire for every tool
call (mirroring the pi extension's `tool_call`/`tool_result` handlers) and drive
the pane's `active_tools` count, independent of the `blocked`-status machinery.

#### `pi`

Writes a TypeScript extension to `~/.pi/agent/extensions/agentctl.ts` that
subscribes to pi lifecycle events and calls `agentctl agent event` /
`agentctl agent clear`. The command is idempotent: re-running when the file is
already up to date reports no change.

```bash
agentctl install pi
```

Example output (first install):

```
Installed agentctl extension for pi:
  + ~/.pi/agent/extensions/agentctl.ts
    session_start        → agentctl agent event SESSION_STARTED --harness pi
    before_agent_start   → agentctl agent event RUN_STARTED --harness pi
    agent_settled        → agentctl agent event RUN_FINISHED --harness pi
    session_shutdown     → agentctl agent clear
```

On a subsequent run (file already up to date):

```
Installed agentctl extension for pi:
  = ~/.pi/agent/extensions/agentctl.ts (already up to date)
```

The `~` prefix is used when the file existed but had outdated content and was
overwritten.

**Pi lifecycle event → event mapping:**

| Pi event             | Event dispatched                                                 |
| -------------------- | ---------------------------------------------------------------- |
| `session_start`      | `SESSION_STARTED --id <sessionId>` (new random id)               |
| `before_agent_start` | `RUN_STARTED --id <runId>` (new random id)                       |
| `agent_settled`      | `RUN_FINISHED --id <runId>` (same id as the run's `RUN_STARTED`) |
| `session_shutdown`   | `agentctl agent clear`                                           |

Note: pi does not currently expose a hook for interactive/elicitation-style tool
prompts, so pi never dispatches `TOOL_CALL_START`/`TOOL_CALL_RESULT` and never
enters `blocked` today.

## Agent State Tracking

### Status model

`status` is derived from the event stream by a finite state machine (a small
transition table in `packages/core`) rather than being set directly. Each agent
pane can be in one of five statuses:

| Status    | Meaning                             |
| --------- | ----------------------------------- |
| `idle`    | Waiting for user input              |
| `working` | Actively processing a task          |
| `done`    | Task complete, awaiting next prompt |
| `blocked` | Waiting for external input/action   |
| `unknown` | No state file, or state unreadable  |

**Transition table:**

| From      | Event                                          | To        |
| --------- | ---------------------------------------------- | --------- |
| `*` (any) | `SESSION_STARTED`                              | `idle`    |
| `idle`    | `RUN_STARTED`                                  | `working` |
| `done`    | `RUN_STARTED`                                  | `working` |
| `*` (any) | `RUN_FINISHED` (regardless of `active_tools`)  | `done`    |
| `done`    | `TOOL_CALL_START` (no type, or `type: custom`) | `working` |
| `done`    | `TOOL_CALL_START` with `type: elicitation`     | `blocked` |
| `working` | `TOOL_CALL_START` with `type: elicitation`     | `blocked` |
| `blocked` | `TOOL_CALL_RESULT` with `type: elicitation`    | `working` |

`RUN_FINISHED` is authoritative from any current status — including `idle` and
`blocked` — since `Stop`/`StopFailure` firing means the harness itself says the
turn is over. It does not consult `active_tools`: a pane with background work
still running after its turn ends reads `done` with a nonzero `activeTools`. A
`TOOL_CALL_START` on a `done` pane moves it back to `working` (or `blocked`), so
a harness-initiated turn (e.g. a background-task wake-up) self-corrects at its
first tool call.

An event that doesn't match a defined transition for the pane's current status
is a no-op for `status` (it is still appended to `events.jsonl`).

### File layout

State is persisted as a JSON file at:

```
$XDG_DATA_HOME/agentctl/agents/<pane-id>/state.json
```

where `XDG_DATA_HOME` defaults to `~/.local/share` (standard XDG base dir spec).
`<pane-id>` is the tmux pane ID (e.g. `%12`), available as `$TMUX_PANE` inside
any process running in a tmux pane.

**Example state file** (`~/.local/share/agentctl/agents/%12/state.json`):

```json
{
  "pane_id": "%12",
  "harness": "claude",
  "status": "working",
  "updated_at": "2026-07-14T10:30:00Z",
  "active_tools": 0
}
```

Files are atomically overwritten (write to `.tmp` sibling, then rename) so
concurrent readers never see a partial write.

Every dispatched event is additionally appended, one JSON object per line, to an
`events.jsonl` file alongside `state.json`:

```
$XDG_DATA_HOME/agentctl/agents/<pane-id>/events.jsonl
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
in `agentctl` reads it back today. Because `agentctl agent clear` deletes the
whole per-pane directory, `events.jsonl` is cleaned up automatically at session
end.

### Stale file cleanup

When `agentctl list` runs, it validates all state files against the live tmux
pane set and deletes stale ones:

- **Orphaned** — `pane_id` not in the live pane set → deleted
- **Contradictory** — `pane_id` exists but `harness` (when not `"unknown"`)
  doesn't match `pane_current_command` → deleted
- **Unknown harness** — `harness` is `"unknown"` → only the orphan check applies

If the tmux query fails, validation is skipped entirely and no files are
deleted.

### Integrating from a harness hook

Call `agentctl agent event` from your harness lifecycle hooks. Example (shell):

```bash
# On session start
agentctl agent event SESSION_STARTED --id "$SESSION_ID" --harness myharness

# On task start
agentctl agent event RUN_STARTED --id "$RUN_ID" --harness myharness

# On task complete
agentctl agent event RUN_FINISHED --id "$RUN_ID" --harness myharness

# On session end
agentctl agent clear
```
