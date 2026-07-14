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
- Implement Claude-specific hook adapters that write state updates via the
  agreed schema.
- Surface the state in `dv list` output (no new subcommand required).

**Non-Goals:**

- Real-time push/streaming of state changes to the CLI.
- Support for harnesses other than Claude in this iteration.
- Centralised state server or IPC.

## Decisions

### D1 — Agent identifier: Claude's `session_id`

**Decision**: Each hook handler reads the `session_id` field from the JSON
payload that Claude passes on stdin. This value is the agent identifier used as
the state file directory name. No UUID generation, no env var injection, and no
coordination with `dv` is required — the identifier is already there in every
hook invocation.

The state file for each agent is stored at:

```
$XDG_DATA_HOME/drovr/agents/<session-id>/state.json
```

**Why `session_id` over a self-generated UUID?** `session_id` is already a
stable, unique identifier provided by Claude for free on every hook event.
Generating our own UUID would require persisting it somewhere (an env var or a
temp file) so subsequent hooks can reuse it — unnecessary complexity.

**Why not `$TMUX_PANE`?** Pane IDs (e.g. `%12`) can be reused after a pane exits
and the tmux server is long-running. `session_id` is tied to a specific Claude
session and is never reused.

**Why not process PID?** PIDs are recycled by the OS. A stale state file keyed
on a PID could be mistaken for a live agent if a different process later gets
the same PID.

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
  "session_id": "<claude-session-id>",
  "pane_id": "%12",
  "harness": "claude",
  "state": "working",
  "updated_at": "2026-07-13T12:34:56Z"
}
```

The `session_id` is read from the hook's stdin JSON and used as the directory
name. The `pane_id` is read from the `$TMUX_PANE` environment variable, which
tmux sets automatically in every pane — no injection by `dv` required. It is
stored so that `dv list` can correlate each state file back to a live tmux pane
without querying Claude.

The file is atomically overwritten (write to a `.tmp` sibling, then `mv`) on
every state change.

**Why not append-only event log?** The CLI only needs current state. Keeping a
log would require parsing and is over-engineering for this iteration.

**What if `$TMUX_PANE` is not set?** The hook is running outside of tmux.
`pane_id` is omitted from the file and the agent will not appear in `dv list`
output (which is tmux-based), but the state file is still written for potential
future consumers.

---

### D4 — Claude hook mapping

Claude Code fires these hooks (passed as JSON on stdin). All handlers are
registered as `async: true` so they never block Claude's operation.

| Hook event         | Matcher             | Maps to state |
| ------------------ | ------------------- | ------------- |
| `SessionStart`     | `startup`           | `idle`        |
| `UserPromptSubmit` | _(no matcher)_      | `working`     |
| `PreToolUse`       | `*`                 | `working`     |
| `Stop`             | _(no matcher)_      | `done`        |
| `Notification`     | `permission_prompt` | `blocked`     |

This covers all five `AgentState` variants. `unknown` is the implicit fallback
returned by the CLI when no state file exists for a listed pane.

`PreToolUse` is retained alongside `UserPromptSubmit` as a belt-and-suspenders
signal: it fires on every tool call and keeps the state as `working` throughout
a multi-tool turn. `PostToolUse` is not used — it is redundant with `PreToolUse`
and fires at a point where Claude is still working.

---

### D5 — Hook handlers: internal `dv hook <event>` subcommands

**Decision**: Hook logic is implemented as internal subcommands of the `dv`
binary. Each handler (`dv hook session-start`, `dv hook user-prompt-submit`,
`dv hook pre-tool-use`, `dv hook stop`, `dv hook notification`) reads Claude's
JSON event payload from stdin, extracts `session_id` and `$TMUX_PANE`, and calls
`write_state`.

**Why internal subcommands over shell scripts?** Hook logic (UUID lookup, JSON
deserialisation, atomic file write) maps naturally to Rust and benefits from the
same error handling and XDG resolution already in the binary. Shell scripts
would duplicate that logic and introduce a second artefact to distribute.

**Why not a separate binary?** `dv` is already on PATH; a second binary adds
complexity with no benefit.

---

### D6 — Hook registration: `dv hooks install`

**Decision**: A new `dv hooks install` subcommand patches
`~/.claude/settings.json` to register the five hook handlers. The command merges
its entries into any existing hook configuration and is **idempotent**. Example
of what it writes:

```json
{
  "hooks": {
    "SessionStart": [
      {
        "matcher": "startup",
        "hooks": [
          {
            "type": "command",
            "command": "dv hook session-start",
            "async": true
          }
        ]
      }
    ],
    "UserPromptSubmit": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "dv hook user-prompt-submit",
            "async": true
          }
        ]
      }
    ],
    "PreToolUse": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "dv hook pre-tool-use",
            "async": true
          }
        ]
      }
    ],
    "Stop": [
      {
        "hooks": [
          { "type": "command", "command": "dv hook stop", "async": true }
        ]
      }
    ],
    "Notification": [
      {
        "matcher": "permission_prompt",
        "hooks": [
          {
            "type": "command",
            "command": "dv hook notification",
            "async": true
          }
        ]
      }
    ]
  }
}
```

**Why `~/.claude/settings.json` (user scope)?** The hooks must fire for all
projects, not just the drovr repo. User-scope settings apply globally.

**Why not auto-install on first run?** Modifying the user's Claude config
without explicit consent is surprising. An explicit install command is
transparent and reversible.

---

### D7 — Harness abstraction in Rust

A `StateStore` trait (or simple free functions) in the `dv` codebase handles:

- `write_state(session_id, pane_id, harness, state)` — serialises and atomically
  writes `state.json`.
- `read_state(session_id) -> AgentState` — reads `state.json`, returns `Unknown`
  on any error.
- `list_agents() -> Vec<AgentInfo>` — globs the data directory and returns all
  known agents.
- `cleanup_stale(live_pane_ids: &[&str])` — deletes every state file whose
  `pane_id` is not present in `live_pane_ids`. Called by `dv list` after
  collecting the set of `#{pane_id}` values from all currently running harness
  panes. Only invoked when tmux is reachable; if the tmux query fails the
  cleanup step is skipped entirely to avoid wiping valid state due to a
  transient error.

The `list-sessions` command:

1. Queries tmux for all panes whose `pane_current_command` matches a known
   harness, collecting `#{pane_id}` for each.
2. Calls `cleanup_stale` with the collected live pane IDs.
3. For each live pane, finds the state file whose `pane_id` matches and calls
   `read_state` to populate the state column. Displays `unknown` when no
   matching state file exists.

## Risks / Trade-offs

- **Race condition on fast successive hook fires** → mitigated by atomic rename
  (`mv`); concurrent writers for the same file are inherently serial on POSIX.
- **State goes stale** (agent process killed without firing Stop hook) → on
  every `dv list` run, `cleanup_stale` removes state files for agents whose
  `pane_id` is no longer in the live pane set. The worst-case window is the time
  between two `dv list` invocations; no background daemon is required.
- **Hook fires outside tmux** (e.g. Claude running in a plain terminal) →
  `$TMUX_PANE` is unset; the hook writes the state file without a `pane_id` and
  the agent does not appear in `dv list` output. This is acceptable: `dv` is
  tmux-oriented.
- **Multiple Claude sessions in the same pane** → not a supported use-case;
  `SessionStart` with `startup` fires once per new session, so the state file is
  correctly initialised each time.

## Migration Plan

1. Add `AgentState` enum and `StateStore` to the Rust codebase.
2. Implement `dv hook <event>` internal subcommands.
3. Implement `dv hooks install` to register them in `~/.claude/settings.json`.
4. Update `dv list` to read state and run stale cleanup.
5. Document `dv hooks install` in the README as the required one-time setup
   step.

No breaking changes to existing commands. State files are additive; absence of a
file is handled gracefully (`unknown`).

## Open Questions

- Should `dv list` show a state column only when state files exist for at least
  one agent, or always? → **Always show it** (display `unknown` when absent).
