## Context

The `dv` CLI launches agent harnesses (Claude, pi, etc.) in tmux panes. Today,
there is no mechanism to query what an agent is doing. Users must visually
inspect each pane to determine whether an agent is waiting, running, done, or
has never been prompted.

Agent harnesses such as Claude Code expose official lifecycle hooks that fire at
key moments (tool use start/end, session stop). These hooks are short-lived
scripts; the CLI is not running when they fire. We need a way to bridge the gap:
hooks write state, CLI reads state.

## Goals / Non-Goals

**Goals:**

- Define a harness-agnostic `AgentState` enum (`idle`, `working`, `done`,
  `blocked`, `unknown`) used everywhere in the codebase.
- Define a file-based state store so hooks can persist state updates without the
  CLI being present.
- Give every agent pane a stable unique identifier that both the spawner (CLI)
  and the hook scripts can agree on without coordination at hook time.
- Implement Claude-specific hook adapters that write state updates via the
  agreed schema.
- Surface the state in `dv list` output (no new subcommand required).

**Non-Goals:**

- Real-time push/streaming of state changes to the CLI.
- Support for harnesses other than Claude in this iteration.
- Detecting `blocked` state for any harness (no official hook exists for it).
- Centralised state server or IPC.

## Decisions

### D1 — Unique agent identifier: env var `DROVR_AGENT_ID`

**Decision**: When `dv` creates (or attaches to) a tmux pane for an agent, it
sets the environment variable `DROVR_AGENT_ID` to a freshly generated UUID v4
inside that pane via `tmux set-environment -t <pane>`. The hook scripts read
`$DROVR_AGENT_ID` to know which agent's state file to update.

**Why not tmux pane ID (`%N`)?** Pane IDs are numeric and local to the server;
they can be reused after a pane dies. UUIDs are globally unique and survive
server restarts.

**Why not process PID?** PIDs are recycled by the OS after a process exits. A
stale state file keyed on PID 4231 could be mistaken for a live agent if a
different process later gets the same PID. UUIDs are never recycled.

**Why not session+window+pane?** That composite key changes if the user moves a
pane and is awkward to pass through shell hook scripts.

**UUID stability across harness restarts**: Because the UUID is set on the tmux
_pane_ environment (not the harness process), it survives the user killing and
relaunching `claude` in the same pane. The state file tracks the _slot_, not the
process.

**Alternatives considered**: Writing the ID to a lock-file in the pane's working
directory — rejected because agents may `cd` freely.

---

### D2 — State file location: XDG data directory

**Decision**: State files are stored at:

```
$XDG_DATA_HOME/drovr/agents/<agent-id>/state.json
```

(`XDG_DATA_HOME` defaults to `~/.local/share`.)

Each agent gets its own directory. The CLI discovers all agents by globbing
`$XDG_DATA_HOME/drovr/agents/*/state.json`.

**Why XDG?** Follows the Linux standard, works on macOS with no extra config,
keeps runtime state out of the project repo, and is easy to wipe.

**Alternatives considered**: Writing next to the tmux socket — rejected because
the socket path is not always predictable. Writing to `/tmp` — rejected because
it is volatile across reboots.

---

### D3 — State file schema: minimal JSON

**Decision**:

```json
{
  "agent_id": "<uuid>",
  "harness": "claude",
  "state": "working",
  "updated_at": "2026-07-13T12:34:56Z"
}
```

The file is atomically overwritten (write to a `.tmp` sibling, then `mv`) on
every state change. The CLI reads it directly; no locking needed because a
single writer (the hook script for one pane) is active at a time.

**Why not append-only event log?** The CLI only needs current state. Keeping a
log would require parsing and is over-engineering for this iteration.

---

### D4 — Claude hook mapping

Claude Code fires these hooks (passed as JSON on stdin):

| Hook event    | Maps to state |
| ------------- | ------------- |
| `PreToolUse`  | `working`     |
| `PostToolUse` | `working`     |
| `Stop`        | `done`        |

`idle` is the initial state written when `DROVR_AGENT_ID` is first set (by the
CLI at spawn time). `unknown` is returned by the CLI when no state file exists
for a listed agent.

`blocked` is not mapped because Claude provides no official hook for it.

---

### D5 — Hook script delivery: shell scripts committed to the repo

**Decision**: Hook scripts live at `hooks/claude/<event>.sh` in the repo. Users
register them in their Claude Code config (`~/.claude/settings.json`) pointing
at the absolute path or via a `dv hooks install` helper (future work). For now,
the scripts are documented and must be manually registered.

**Why shell scripts?** They have zero build dependencies and can be installed
without compiling the Rust binary first.

---

### D6 — Harness abstraction in Rust

A `StateStore` trait (or simple free functions) in the `dv` codebase handles:

- `write_state(agent_id, harness, state)` — serialises and atomically writes
  `state.json`.
- `read_state(agent_id) -> AgentState` — reads `state.json`, returns `Unknown`
  on any error.
- `list_agents() -> Vec<AgentInfo>` — globs the data directory and returns all
  known agents.
- `cleanup_stale(live_ids: &[Uuid])` — deletes every state file whose `agent_id`
  is not present in `live_ids`. Called by `dv list` after collecting the set of
  `DROVR_AGENT_ID` values from all currently running harness panes. Only invoked
  when tmux is reachable; if the tmux query fails the cleanup step is skipped
  entirely to avoid wiping valid state due to a transient error.

The `list-sessions` command:

1. Queries tmux for all panes whose `pane_current_command` matches a known
   harness, reading `DROVR_AGENT_ID` for each via
   `tmux show-environment -t <pane> DROVR_AGENT_ID`.
2. Calls `cleanup_stale` with the collected live IDs.
3. Calls `read_state` for each live pane to populate the state column.

## Risks / Trade-offs

- **Race condition on fast successive hook fires** → mitigated by atomic rename
  (`mv`); concurrent writers for the same file are inherently serial on POSIX.
- **State goes stale** (agent process killed without firing Stop hook) → on
  every `dv list` run, `cleanup_stale` removes state files for agents whose pane
  no longer exists or whose `pane_current_command` no longer matches the
  harness. The worst-case window is the time between two `dv list` invocations;
  no background daemon is required.
- **DROVR_AGENT_ID not set** (user opens a pane manually without `dv`) → hook
  script detects empty var and exits 0 silently; CLI shows `unknown` for panes
  without a state file.
- **Multiple Claude processes in the same pane** → not a supported use-case;
  each `dv`-managed pane has exactly one harness process.

## Migration Plan

1. Add `AgentState` enum and `StateStore` to the Rust codebase.
2. Ship hook scripts under `hooks/claude/`.
3. Update `dv list` to read state.
4. Document manual hook registration in the README.
5. Add `dv hooks install` in a follow-up change.

No breaking changes to existing commands. State files are additive; absence of a
file is handled gracefully (`unknown`).

## Open Questions

- Should `dv list` show a state column only when state files exist for at least
  one agent, or always? → **Always show it** (display `unknown` when absent).
