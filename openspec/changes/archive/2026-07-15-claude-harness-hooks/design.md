## Context

Claude Code ships a hook system: short-lived subprocesses that are spawned at
key moments in the agent lifecycle. Each hook receives a JSON payload on stdin
and is expected to exit quickly. Because the hooks are subprocesses (not
long-running daemons), they must communicate state through a side-channel —
exactly the file-based `StateStore` that `agent-state-tracking` provides.

`rd hooks install claude` is the one-time setup command that wires these hooks
to `rd set-state` and `rd clear-state`. Once installed, every Claude session
started inside tmux automatically writes accurate state to the XDG data
directory.

## Goals / Non-Goals

**Goals:**

- Map all relevant Claude hook events to the five canonical `AgentState`
  transitions.
- Provide an idempotent, merge-safe `rd hooks install claude` command that
  patches `~/.claude/settings.json`.
- Ensure `SessionStart` and `SessionEnd` run synchronously so the state file
  exists before any other hook fires and is cleaned up before the session exits.

**Non-Goals:**

- Parsing or inspecting Claude's JSON stdin payload inside `rd` — Claude's
  `matcher` field handles event filtering so `rd set-state` receives only the
  canonical state as a CLI argument.
- Modifying the core `StateStore` or `AgentState` types (those belong to
  `agent-state-tracking`).

## Decisions

### D1 — Claude hook event → state mapping

**Decision**: Six Claude hook events are mapped to `rd` commands as follows.

| Hook event         | Matcher             | Command                                 | Async |
| ------------------ | ------------------- | --------------------------------------- | ----- |
| `SessionStart`     | `startup`           | `rd set-state idle --harness claude`    | no    |
| `UserPromptSubmit` | _(none)_            | `rd set-state working --harness claude` | yes   |
| `PreToolUse`       | `*`                 | `rd set-state working --harness claude` | yes   |
| `Stop`             | _(none)_            | `rd set-state done --harness claude`    | yes   |
| `Notification`     | `permission_prompt` | `rd set-state blocked --harness claude` | yes   |
| `SessionEnd`       | _(none)_            | `rd clear-state`                        | no    |

`SessionStart` and `SessionEnd` are synchronous so the state file is established
before any async hook fires and is removed before the session exits.

`PreToolUse` is retained alongside `UserPromptSubmit` as a belt-and-suspenders
signal: it fires on every tool call and keeps the state as `working` throughout
a multi-tool turn. Because it fires very frequently, it must be `async: true` so
it never blocks Claude's operation.

`unknown` is the implicit fallback returned by `rd list` when no state file
exists for a listed pane; it does not need a hook entry.

---

### D2 — Hook registration scope: user (`~/.claude/settings.json`)

**Decision**: `rd hooks install claude` patches `~/.claude/settings.json`
(user-scope settings), not a per-project `.claude/settings.json`.

**Why user scope?** The hooks must fire for all projects the user opens in
Claude, not just the radr repo. User-scope settings apply globally.

**Why not auto-install on first `rd` run?** Modifying the user's Claude config
without explicit consent is surprising and hard to debug. An explicit install
command is transparent and easily reversible (`rd hooks uninstall claude` is a
future concern, but the user can also edit the JSON directly).

---

### D3 — Idempotency and merge strategy

**Decision**: `rd hooks install claude` reads the current
`~/.claude/settings.json`, inspects the `hooks` object for existing radr
entries, adds only the missing ones, and writes the file back atomically.

The identity of a "radr entry" is determined by the `command` string (e.g.
`"rd set-state idle --harness claude"`). If that exact command already appears
in the hook list for the corresponding event, it is not duplicated.

All existing non-radr hook entries are preserved.

---

### D4 — JSON structure written to `~/.claude/settings.json`

The full set of entries written (or verified already present) by
`rd hooks install claude`:

```json
{
  "hooks": {
    "SessionStart": [
      {
        "matcher": "startup",
        "hooks": [
          { "type": "command", "command": "rd set-state idle --harness claude" }
        ]
      }
    ],
    "UserPromptSubmit": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "rd set-state working --harness claude",
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
            "command": "rd set-state working --harness claude",
            "async": true
          }
        ]
      }
    ],
    "Stop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "rd set-state done --harness claude",
            "async": true
          }
        ]
      }
    ],
    "Notification": [
      {
        "matcher": "permission_prompt",
        "hooks": [
          {
            "type": "command",
            "command": "rd set-state blocked --harness claude",
            "async": true
          }
        ]
      }
    ],
    "SessionEnd": [
      {
        "hooks": [{ "type": "command", "command": "rd clear-state" }]
      }
    ]
  }
}
```

---

### D5 — Output of `rd hooks install claude`

The command prints a human-readable summary of what changed, e.g.:

```
Installed radr hooks for claude:
  + SessionStart (startup)     → rd set-state idle --harness claude
  + UserPromptSubmit           → rd set-state working --harness claude [async]
  + PreToolUse                 → rd set-state working --harness claude [async]
  + Stop                       → rd set-state done --harness claude [async]
  + Notification (permission_prompt) → rd set-state blocked --harness claude [async]
  + SessionEnd                 → rd clear-state
```

On an idempotent re-run, lines that were already present are prefixed with `=`
instead of `+`.

## Risks / Trade-offs

- **Claude settings format changes**: If Anthropic changes
  `~/.claude/settings.json`'s structure, `rd hooks install claude` may produce
  invalid JSON. Mitigation: unit-test the merge logic against the known schema
  and add a schema version check if Claude ever introduces one.
- **Hook fires outside tmux** (`$TMUX_PANE` unset): `rd set-state` and
  `rd clear-state` exit 0 silently when outside tmux. This is handled by the
  foundational `agent-state-tracking` implementation and requires no
  Claude-specific logic.

## Migration Plan

1. Add the `claude` branch to the `rd hooks install <harness>` dispatcher,
   replacing the current "no harnesses supported" exit-1 for `claude`.
2. Implement the merge-safe JSON patching logic.
3. Write unit and integration tests.
4. Update the README and `AGENTS.md` to document `rd hooks install claude` as
   the required one-time setup step.
