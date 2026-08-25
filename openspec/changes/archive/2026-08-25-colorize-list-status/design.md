## Context

`rd list` (`src/commands/list.rs`) renders one line per tmux pane with four
left-aligned, padded columns: agent, session, `window.pane`, and state. The
state column currently prints the plain `AgentState` string (`idle`, `working`,
`blocked`, `done`, or `unknown`). Column widths are computed from
`unicode-width`'s display width of each cell's plain text, then padded.

The `owo-colors` crate is already a declared dependency (unused so far), so no
new crate needs to be added for coloring. Rust's standard library exposes
`std::io::IsTerminal` (stable since 1.70) for TTY detection without adding a
dependency.

## Goals / Non-Goals

**Goals:**

- Color only the state column's text using a foreground color keyed to the
  `AgentState` value: `idle` grey, `working` yellow, `blocked` red, `done`
  green.
- Preserve exact column alignment behavior — padding must be computed from the
  _visible_ width of the state text, never the byte length of the ANSI-wrapped
  string.
- Automatically disable color when stdout is not a terminal (piped output), so
  existing scripts/tests that scan plain text keep working unless they
  explicitly opt into forcing color.
- Keep `unknown` state uncolored (default terminal foreground) since it's a
  reconciliation artifact, not a real agent status.

**Non-Goals:**

- No new CLI flag (e.g. `--color=always|never|auto`) in this change; color is
  purely automatic based on TTY detection. A future change can add explicit
  override flags if needed.
- No color changes to the agent, session, or pane columns.
- No change to sort behavior, state reconciliation, or any other `list`
  semantics — this is purely a rendering change.

## Decisions

### Compute width before coloring, wrap only for the final string

`format_panes` already computes `max_agent`, `max_session`, `max_wp`, and
implicitly the state column's own width isn't currently padded (it's the last
column, so no trailing padding is needed). The state color wrap SHALL be applied
last, after the plain state string has already been used for any width
calculations. Since state is the last column in each row, no padding is computed
from it today, so no width regression is introduced — but if a future column is
added after state, this ordering (measure plain, wrap for display) becomes the
standing pattern.

### Use `owo-colors`'s conditional coloring support

`owo-colors` provides `if_supports_color`/`OwoColorize` combinators driven by a
`Stream` (Stdout/Stderr) and an internal `supports-colors` check, or a simple
`.color(...)` call combined with a manual toggle. To keep control explicit and
testable, wrap state text with a small helper:

```rust
fn colorize_state(state: AgentState, text: &str, use_color: bool) -> String {
    if !use_color {
        return text.to_string();
    }
    match state {
        AgentState::Idle => text.fg::<owo_colors::colors::BrightBlack>().to_string(),
        AgentState::Working => text.fg::<owo_colors::colors::Yellow>().to_string(),
        AgentState::Blocked => text.fg::<owo_colors::colors::Red>().to_string(),
        AgentState::Done => text.fg::<owo_colors::colors::Green>().to_string(),
        AgentState::Unknown => text.to_string(),
    }
}
```

`use_color` is threaded in as an explicit `bool` parameter to `format_panes` (or
a new sibling function) rather than computed internally, so unit tests can force
it on/off deterministically without depending on the test runner's TTY state.

### TTY detection at the call site, not inside `format_panes`

`run()` computes `use_color = std::io::stdout().is_terminal()` once and passes
it down. This keeps `format_panes` a pure, easily-testable function and matches
existing patterns in the codebase where `run()` handles I/O concerns and helper
functions stay pure.

### Grey mapping

"Grey" has no ANSI standard 3-bit code; the closest portable choice is bright
black (`BrightBlack`, ANSI code 90), which most terminals render as a visible
grey rather than invisible black. This avoids needing 256-color or truecolor
support for a legible grey.

## Risks / Trade-offs

- [Risk] Some terminals/themes render `BrightBlack` as too dark or
  indistinguishable from background → Mitigation: no override flag lands in this
  change, but the color mapping is centralized in one function, making a
  follow-up "grey" tweak or config option cheap to add later.
- [Risk] ANSI escape sequences inflate the string's byte length, and any future
  column added after `state`, or any code path that measures the _rendered_
  state string's width (e.g. tests using `.width()` directly on the colorized
  output) would miscompute width → Mitigation: width computations always operate
  on the plain state string; coloring is applied only at the final formatting
  step, never before a width calculation.
- [Risk] Piped output (e.g. `rd list | grep working`) unexpectedly containing
  ANSI codes would break naive text matching → Mitigation: `IsTerminal`-based
  auto-detection disables color whenever stdout isn't a TTY, matching common CLI
  conventions (git, ripgrep, etc.).
- [Trade-off] No explicit `--color` flag means users who redirect to a file but
  still want color (or pipe to a pager that supports color) have no way to force
  it in this change; acceptable since it's not a stated requirement, and can be
  added later without breaking this design.

## Migration Plan

- Purely additive rendering change; no data migration required.
- Existing tests asserting exact plain-text output in non-TTY contexts
  (`format_panes` unit tests, `tests/list_sessions.rs` integration tests)
  continue to pass unmodified as long as they invoke the code path with
  `use_color = false` (the default when not attached to a TTY, which is always
  true under `cargo test`).
- Rollback: revert the color-wrapping call in `format_panes`/`run`; no persisted
  state or schema is touched.

## Open Questions

- Should a future change expose `--color=always|never|auto` for scripting
  scenarios that want forced color? Deferred as a non-goal for now.
