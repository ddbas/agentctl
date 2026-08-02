## Context

`rd` is the radr developer CLI built with Rust and `clap`. Currently it has only
a `help` subcommand. Developers frequently run AI agent harness processes
(`claude`, `pi`) inside tmux panes — in any session, with any session name — and
need a fast way to enumerate what is live. The tmux CLI (`tmux list-panes -a`)
is the canonical source of truth: it exposes `#{pane_current_command}`, the
basename of the foreground process in every pane across every session.

## Goals / Non-Goals

**Goals:**

- Add a `rd list` subcommand that prints one line per pane whose
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

### Filter by exact match on `pane_current_command`, with a process-argv fallback for `node`

**Decision**: A pane is included in the output if:

1. Its `pane_current_command` exactly equals `claude` (direct match), **or**
2. Its `pane_current_command` is `node` **and** the argv[0] basename of at least
   one direct child process of `#{pane_pid}` equals `pi`.

**Rationale**: `pi` is a Node.js application. Its launcher script
(`/opt/homebrew/bin/pi`) does `exec node …` with the pi script path as argv[1],
but argv[0] is preserved as `pi` via the shebang mechanism. tmux's
`pane_current_command` reflects the actual binary name (`node`), not argv[0].
Running `ps -A -o pid=,ppid=,args=` once and checking `args` (argv[0]) of direct
children of the pane's shell PID lets `rd` correctly identify those panes
without false-positives (an unrelated `node` process running something other
than `pi` would not match).

### Two-step pane resolution: `pane_current_command` then process-argv

**Decision**: When `pane_current_command == "node"`, issue one
`ps -A -o pid=,ppid=,args=` call (shared across all `node` panes in the current
query), build a ppid→argv0-basenames map, and check whether the pane shell's PID
has a direct child whose argv[0] basename is `pi`.

**Rationale**: A single `ps -A` call returns every process on the system in one
shot; building a PPID-keyed map makes the per-pane lookup O(1). This avoids one
subprocess per pane. The `ps` call is skipped entirely if no pane in the current
result reports `node`, keeping the fast path fast.

**Alternatives considered**:

- `pgrep -P <pane_pid>` per node pane — requires N subprocesses; ruled out.
- Read `/proc/<pid>/cmdline` — Linux-only; ruled out.
- Match `#{pane_title}` — user-controlled, fragile; ruled out.
- Have pi call `exec -a pi node …` — requires changing pi; ruled out for now.

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
