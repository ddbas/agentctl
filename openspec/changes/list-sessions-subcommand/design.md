## Context

`dv` is the drovr developer CLI built with Rust and `clap`. Currently it has
only a `help` subcommand. Developers frequently run AI agent harness processes
(`claude`, `pi`) inside tmux panes — in any session, with any session name — and
need a fast way to enumerate what is live. The tmux CLI (`tmux list-panes -a`)
is the canonical source of truth: it exposes `#{pane_current_command}`, the
basename of the foreground process in every pane across every session.

## Goals / Non-Goals

**Goals:**

- Add a `dv list` subcommand that prints one line per pane whose
  `pane_current_command` exactly matches a known agent harness name (`claude` or
  `pi`).
- Each output line shows three columns: agent name, session name, and
  `window_index.pane_index` identifier — all left-aligned.
- Accept an optional `--server` / `-s` flag so callers can point at a specific
  tmux socket/server file (falls back to the default tmux server when omitted).

**Non-Goals:**

- Showing session status, pane titles, or richer metadata (deferred to a future
  iteration).
- Supporting harness types beyond `claude` and `pi` without a code change (the
  list is hard-coded for now).
- Interactive or TUI output.
- Starting, attaching to, or killing sessions.

## Decisions

### Shell-out to `tmux list-panes -a` rather than a tmux crate

**Decision**: Invoke
`tmux list-panes -a -F '#{pane_current_command}\t#{session_name}\t#{window_index}\t#{pane_index}'`
via `std::process::Command`.

**Rationale**: `list-panes -a` returns every pane across every session in a
single subprocess call and surfaces `pane_current_command` — the foreground
process basename — which is the authoritative signal for which agent is running.
This avoids any dependency on session naming conventions.

**Alternatives considered**:

- `tmux_interface` crate — more type-safe but heavy dependency; ruled out.
- `tmux list-sessions` + per-session `list-windows` — two-step, relies on
  session name heuristics; ruled out.
- `tmux list-windows -a` with `#{window_name}` — window name is often
  auto-renamed and unreliable as a process identifier; ruled out.

### Filter by exact match on `pane_current_command`

**Decision**: A pane is included in the output if its `pane_current_command`
exactly equals `claude` or `pi` (case-sensitive).

**Rationale**: `pane_current_command` is the basename of the foreground process
as reported by tmux — it is already the agent name, not a session label chosen
by the user. Exact match avoids false positives (e.g. a pane running `pilot`
would not match `pi`). If the agent binary is renamed, only the allow-list needs
updating.

### Module layout

**Decision**: Implement the command handler in `src/commands/list.rs` and expose
it via `src/commands/mod.rs`. CLI wiring lives in `src/main.rs`.

**Rationale**: Keeps `main.rs` thin (parse + dispatch only) and makes the list
logic independently testable.

### Output format

**Decision**: Print one pane per line with three space-padded columns:

```
claude  my-session    0.0
pi      work-session  1.0
pi      work-session  2.3
```

Column order: `agent`, `session`, `window_index.pane_index`. All three columns
are left-aligned; each is padded to the longest value in that column + 2 spaces.

**Rationale**: Putting agent name first makes it easiest to scan when multiple
harness types are interleaved. The `window.pane` notation follows standard tmux
convention and is directly usable in `tmux select-pane -t session:window.pane`.
Tab-separated would be simpler but looks ragged in terminals.

## Risks / Trade-offs

- **tmux not installed** → The `tmux` binary may not be present in all
  environments. Mitigation: surface a clear error message
  (`tmux not found — is tmux installed and on PATH?`) rather than a raw
  `No such file or directory` OS error.
- **No running tmux server** → `tmux list-panes -a` exits non-zero when no
  server is running. Mitigation: treat a non-zero exit as "no panes" and return
  an empty list rather than propagating an error.
- **Process name reliability** → `pane_current_command` reflects the current
  foreground process. If the agent spawns a subprocess (e.g. a shell) and that
  becomes the foreground, the pane may not be detected. Mitigation: acceptable
  limitation for the initial iteration; can be addressed with
  `pane_current_path` or `pane_title` heuristics later.
