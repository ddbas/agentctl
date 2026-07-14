## Context

The `dv` CLI monitors agent harnesses (Claude, pi, etc.) running in tmux panes.
Today, there is no mechanism to query what an agent is doing. Users must
visually inspect each pane to determine whether an agent is waiting, running,
done, or has never been prompted.

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
- Expose `dv set-state <state>` and `dv clear-state` for any harness to call
  from its hooks.
- Provide a `dv hooks install <harness>` skeleton for harness-specific
  registration; no concrete harness is supported in this iteration.
- Surface the state in `dv list` output (no new subcommand required).

**Non-Goals:**

- Real-time push/streaming of state changes to the CLI.
- Concrete harness support in this iteration (Claude integration lives in the
  `claude-harness-hooks` change that depends on this one).
- Centralised state server or IPC.

## Decisions

### D1 — Agent identifier: `$TMUX_PANE`

**Decision**: The tmux pane ID — available as the `$TMUX_PANE` environment
variable (e.g. `%12`) inside any process running in a tmux pane — is the agent
identifier. It is used as the state file directory name. Every hook subprocess
inherits it from the shell; `dv list` reads it via `#{pane_id}` in tmux format
strings. No injection, no coordination, no UUID generation required.

The state file for each agent is stored at:

```
$XDG_DATA_HOME/drovr/agents/<pane-id>/state.json
```

(e.g. `~/.local/share/drovr/agents/%12/state.json`)

**Pane ID reuse is acceptable.** A pane ID can technically be reused after a
pane exits and a new one is created. This is fine because:

1. At most one agent harness runs in a given pane at a time — that is an
   explicit assumption of this design.
2. `SessionStart` overwrites any existing state file when a new session begins
   in the same pane, so stale data from a prior session is replaced immediately.
3. `dv list` validates state files as a last resort (see D7).

**Why not a harness-specific ID (e.g. Claude's `session_id`)?** Harness IDs are
not a guaranteed concept across all agent harnesses. `$TMUX_PANE` is a
harness-agnostic, universally available identifier for any process running in
tmux — making the state file schema and the `dv` CLI fully harness-agnostic.

**Why not process PID?** PIDs are recycled frequently by the OS and carry no
harness-agnostic meaning.

---

### D2 — State file location: XDG data directory

**Decision**: State files are stored at:

```
$XDG_DATA_HOME/drovr/agents/<pane-id>/state.json
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
  "pane_id": "%12",
  "harness": "claude",
  "state": "working",
  "updated_at": "2026-07-13T12:34:56Z"
}
```

`pane_id` is read from `$TMUX_PANE` and doubles as both the directory name and
the field used by `dv list` for correlation and stale validation.

The file is atomically overwritten (write to a `.tmp` sibling, then `mv`) on
every state change, and deleted by the `SessionEnd` hook when the session exits
normally.

**Why not append-only event log?** The CLI only needs current state.

**What if `$TMUX_PANE` is not set?** The hook is running outside of tmux. It
exits 0 silently and writes nothing — `dv` is tmux-oriented and has no pane to
correlate the state to.

---

### D4 — Hook handlers: single `dv set-state <state>` subcommand

**Decision**: State-writing logic is exposed through a single
`dv set-state <state>` subcommand that any harness can call from its hooks.
Accepted states: `idle`, `working`, `done`, `blocked`. The command reads
`$TMUX_PANE` from the environment, resolves the XDG data directory, and
atomically overwrites `state.json`. It exits 0 silently when `$TMUX_PANE` is not
set.

Optional flags:

- `--harness <harness_id>` — overrides the `harness` field written to
  `state.json`. Defaults to `"unknown"` when omitted.
- `--pane <pane_id>` — overrides the pane ID instead of reading `$TMUX_PANE`.
  Useful for scripted or testing scenarios.

For session cleanup a companion `dv clear-state` subcommand removes the state
file and its parent directory for the current pane (`$TMUX_PANE`). It is a no-op
when the file does not exist and exits 0 silently outside tmux.

**Why a single set-state command over per-event handlers?** Per-event
subcommands (`dv hook session-start`, `dv hook user-prompt-submit`, …) lock the
state-writing logic to Claude Code's specific event vocabulary. Adding a second
harness (e.g. pi, aider) would require implementing matching per-event
subcommands — effectively re-coupling the CLI to each harness's lifecycle
events.

`dv set-state` inverts the dependency: harness authors install hooks that emit
one of the four canonical states directly. `dv` has no knowledge of
harness-specific event names. XDG resolution and atomic file-write logic are
centralised in one place, and any new harness gains first-class support by
installing hooks that call `dv set-state` — nothing else required.

**Why not a separate binary?** `dv` is already on PATH; a second binary adds
complexity with no benefit.

---

### D5 — Hook registration: `dv hooks install <harness>` placeholder

**Decision**: A new `dv hooks install <harness>` subcommand is introduced as a
dispatch skeleton. In this iteration **no harness values are supported**; the
command exits 1 with a clear error message (e.g.
`"unknown harness '<harness>'; no harnesses are currently supported"`) for any
input, including `claude`.

Concrete harness support (starting with `claude`) is delivered by the
`claude-harness-hooks` change, which depends on this one and fills in the
`claude` branch of the dispatcher.

**Why introduce the command now with no implementation?** Establishing the
subcommand name and argument shape in the CLI allows the `claude-harness-hooks`
change to extend it without touching the CLI argument parser, and signals the
intended extension point to future harness authors.

---

### D6 — Harness abstraction in Rust

A `StateStore` trait (or simple free functions) in the `dv` codebase handles:

- `write_state(pane_id, harness, state)` — serialises and atomically writes
  `state.json` to `$XDG_DATA_HOME/drovr/agents/<pane-id>/`.
- `delete_state(pane_id)` — removes the state file and its parent directory.
  Called by `dv clear-state`.
- `read_state(pane_id) -> AgentState` — reads `state.json`, returns `Unknown` on
  any error.
- `list_agents() -> Vec<AgentInfo>` — globs the data directory and returns all
  known agents.

The `list-sessions` command:

1. Queries tmux for all panes whose `pane_current_command` matches a known
   harness, collecting `#{pane_id}` and `#{pane_current_command}` for each.
2. Reads all state files from the data directory.
3. For each state file, validates it as a last resort:
   - If the state file's `pane_id` is not in the live pane set → delete it (pane
     no longer exists).
   - If the pane exists **and** the state file's `harness` field is not
     `"unknown"`, and `pane_current_command` does not match `harness` → delete
     it (different process running in that pane now). When `harness` is
     `"unknown"` this check is skipped — we cannot validate what we do not know.
   - This validation only runs when tmux is reachable; skip entirely on tmux
     errors to avoid deleting valid state.
4. For each live pane, calls `read_state` to populate the state column. Displays
   `unknown` when no state file exists for that pane.

## Risks / Trade-offs

- **Race condition on fast successive hook fires** → mitigated by atomic rename
  (`mv`); concurrent writers for the same file are inherently serial on POSIX.
- **Unclean shutdown** (agent process killed without firing `SessionEnd`) →
  `dv list` validates each state file as a last resort: deletes it if the pane
  no longer exists or if `pane_current_command` no longer matches the `harness`
  field. The worst-case window is the time between two `dv list` invocations.
- **Pane ID reuse** (new session starts in a pane whose old state file was not
  cleaned up) → the new `SessionStart` hook unconditionally overwrites the state
  file, so stale data is replaced immediately.
- **Hook fires outside tmux** (`$TMUX_PANE` unset) → hook exits 0 silently; no
  state file is written. `dv` is tmux-oriented and has no pane to correlate
  state to.

## Migration Plan

1. Add `AgentState` enum and `StateStore` to the Rust codebase.
2. Implement `dv set-state <state>` and `dv clear-state` subcommands.
3. Implement `dv hooks install <harness>` skeleton (exits 1 for all harnesses).
4. Update `dv list` to read state and run stale cleanup.
5. Document the state model and XDG file layout in the README.

No breaking changes to existing commands. State files are additive; absence of a
file is handled gracefully (`unknown`).

## Open Questions

- Should `dv list` show a state column only when state files exist for at least
  one agent, or always? → **Always show it** (display `unknown` when absent).
