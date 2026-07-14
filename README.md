# 🐂 Drovr

A tmux-based agent multiplexer. Spot and manage AI agent processes running
across your tmux sessions.

## Installation

```bash
cargo install --path .
```

## Usage

```bash
dv list          # list all agent panes with their current state
dv set-state <state>   # write agent state from a hook
dv clear-state         # remove the state file for the current pane
dv hooks install <harness>   # install harness hooks (placeholder)
```

## Commands

### `dv list`

Queries every pane across all tmux sessions via `tmux list-panes -a` and prints
one line per pane whose foreground process is a known agent harness (`claude` or
`pi`). Output is four left-aligned columns:

```
<agent-name>  <session-name>  <window>.<pane>  <state>
```

The `state` column shows the agent's last reported state, or `unknown` if no
state file exists for that pane.

As a side effect, `dv list` validates persisted state files and deletes stale
ones (see "Agent state tracking" below).

### `dv set-state <state>`

Write the current agent state to a file. Intended to be called from agent
harness hooks. Accepted values: `idle`, `working`, `done`, `blocked`.

```bash
dv set-state working --harness claude
dv set-state idle    --harness pi --pane %12
```

Flags:

- `--harness <name>` — harness identifier written to the state file (default:
  `"unknown"`)
- `--pane <pane-id>` — override the pane ID instead of reading `$TMUX_PANE`

Exits 0 silently when `$TMUX_PANE` is not set and `--pane` is not supplied —
safe to call unconditionally from hooks that may run outside tmux.

### `dv clear-state`

Delete the state file for the current pane (`$TMUX_PANE`). Typically called from
a session-end hook.

```bash
dv clear-state           # uses $TMUX_PANE
dv clear-state --pane %12
```

Exits 0 silently when `$TMUX_PANE` is not set, or when no state file exists.

### `dv hooks install <harness>`

Placeholder for harness-specific hook registration. No harnesses are supported
in the current release; the command exits 1 with a clear message for all inputs.
Concrete harness support (e.g. `claude`) will be added in a subsequent release.

## Agent State Tracking

### State model

Each agent pane can be in one of five states:

| State     | Meaning                             |
| --------- | ----------------------------------- |
| `idle`    | Waiting for user input              |
| `working` | Actively processing a task          |
| `done`    | Task complete, awaiting next prompt |
| `blocked` | Waiting for external input/action   |
| `unknown` | No state file, or state unreadable  |

### File layout

State is persisted as a JSON file at:

```
$XDG_DATA_HOME/drovr/agents/<pane-id>/state.json
```

where `XDG_DATA_HOME` defaults to `~/.local/share` (standard XDG base dir spec).
`<pane-id>` is the tmux pane ID (e.g. `%12`), available as `$TMUX_PANE` inside
any process running in a tmux pane.

**Example state file** (`~/.local/share/drovr/agents/%12/state.json`):

```json
{
  "pane_id": "%12",
  "harness": "claude",
  "state": "working",
  "updated_at": "2026-07-14T10:30:00Z"
}
```

Files are atomically overwritten (write to `.tmp` sibling, then rename) so
concurrent readers never see a partial write.

### Stale file cleanup

When `dv list` runs, it validates all state files against the live tmux pane set
and deletes stale ones:

- **Orphaned** — `pane_id` not in the live pane set → deleted
- **Contradictory** — `pane_id` exists but `harness` (when not `"unknown"`)
  doesn't match `pane_current_command` → deleted
- **Unknown harness** — `harness` is `"unknown"` → only the orphan check applies

If the tmux query fails, validation is skipped entirely and no files are
deleted.

### Integrating from a harness hook

Call `dv set-state` from your harness lifecycle hooks. Example (shell):

```bash
# On task start
dv set-state working --harness myharness

# On task complete
dv set-state idle --harness myharness

# On session end
dv clear-state
```
