## Context

`dv` is the drovr developer CLI built with Rust and `clap`. Currently it has
only a `help` subcommand. Developers frequently run multiple AI agent harness
sessions (`claude`, `pi`) inside tmux and need a fast way to enumerate what is
live. The tmux CLI (`tmux list-sessions`) is the canonical source of truth for
running sessions on a given server.

## Goals / Non-Goals

**Goals:**

- Add a `dv list` subcommand that prints all windows belonging to tmux sessions
  whose names match known agent harnesses (`claude`, `pi`).
- Each output line shows: session name, window index, and window name — in a
  readable columnar format.
- Accept an optional `--server` / `-s` flag so callers can point at a specific
  tmux socket/server file (falls back to the default tmux server when omitted).

**Non-Goals:**

- Showing session status, pane-level detail, or richer metadata (deferred to a
  future iteration).
- Supporting harness types beyond `claude` and `pi` without a code change (the
  list is hard-coded for now).
- Interactive or TUI output.
- Starting, attaching to, or killing sessions.

## Decisions

### Shell-out to `tmux list-windows -a` rather than a tmux crate

**Decision**: Invoke
`tmux list-windows -a -F '#{session_name}\t#{window_index}\t#{window_name}'` via
`std::process::Command`.

**Rationale**: The `tmux` binary is already a hard runtime dependency of any
workflow that creates tmux sessions, so it will always be present. A pure-Rust
tmux library (e.g., `tmux_interface`) adds a compile-time dependency and
maintenance surface for no practical benefit at this stage. Shell-out keeps the
code small and directly testable against real tmux behaviour.

`list-windows -a` is preferred over two-step `list-sessions` + per-session
`list-windows` because it returns all windows in a single subprocess call and
the output is easily tab-split into (session, index, name) tuples.

**Alternatives considered**:

- `tmux_interface` crate — more type-safe but heavy dependency for a simple
  listing operation; ruled out.
- `tmux list-sessions` + per-session `list-windows` — two-step, higher overhead,
  ruled out.
- Parse `/tmp/tmux-*/` socket directory directly — too fragile and
  platform-specific; ruled out.

### Filter by prefix match

**Decision**: A session is included in the output if its name starts with
`claude` or `pi` (case-sensitive).

**Rationale**: Agent harness sessions are typically named `claude`, `claude-1`,
`pi`, `pi-work`, etc. A prefix match captures all variants without requiring an
exact name list. If the tmux session naming convention changes, only the filter
list needs updating.

### Module layout

**Decision**: Implement the command handler in `src/commands/list.rs` and expose
it via `src/commands/mod.rs`. CLI wiring lives in `src/main.rs`.

**Rationale**: Keeps `main.rs` thin (parse + dispatch only) and makes the list
logic independently testable.

### Output format

**Decision**: Print one window per line, with columns left-aligned and
space-padded:

```
claude          0  zsh
claude          1  nvim
pi-work         0  bash
```

Column widths: session name padded to the longest session name + 2 spaces;
window index right-aligned in a fixed-width field (e.g. 2 chars); window name
unpadded.

**Rationale**: Tab-separated would be simpler but looks ragged in terminals.
Fixed-width column padding is easy to implement with Rust's `format!` width
specifiers and makes the output scannable at a glance. A future `--json` flag
can be added when richer metadata is introduced.

## Risks / Trade-offs

- **tmux not installed** → The `tmux` binary may not be present in all
  environments. Mitigation: surface a clear error message
  (`tmux not found — is tmux installed and on PATH?`) rather than a raw
  `No such file or directory` OS error.
- **No running tmux server** → `tmux list-sessions` exits non-zero and prints an
  error when no server is running. Mitigation: treat a non-zero exit from
  `tmux list-sessions` as "no sessions" and return an empty list (with an
  optional debug message) rather than propagating an error.
- **Prefix false-positives** → A non-harness session named `pilot` would match
  the `pi` prefix. Mitigation: acceptable trade-off for now; can be tightened to
  exact-match or configurable patterns in a later change.
