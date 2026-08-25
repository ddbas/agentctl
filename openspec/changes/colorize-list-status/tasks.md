## 1. Coloring helper

- [ ] 1.1 Add a
      `colorize_state(state: AgentState, text: &str, use_color: bool) -> String`
      helper in `src/commands/list.rs` mapping `Idle` → bright black, `Working`
      → yellow, `Blocked` → red, `Done` → green, `Unknown` → unwrapped text,
      using the existing `owo-colors` dependency
- [ ] 1.2 Unit test `colorize_state` for each `AgentState` variant with
      `use_color = true` (asserts ANSI escape sequences are present and correct
      color) and `use_color = false` (asserts plain text passthrough)

## 2. Wire coloring into rendering

- [ ] 2.1 Update `format_panes` to accept a `use_color: bool` parameter and
      apply `colorize_state` to each row's state text only in the final
      formatted string, after all column widths have been computed from the
      plain state string
- [ ] 2.2 Update `run()` to compute
      `use_color = std::io::stdout().is_terminal()` and pass it to
      `format_panes`
- [ ] 2.3 Update existing `format_panes` call sites/signatures in tests to pass
      an explicit `use_color` argument

## 3. Tests

- [ ] 3.1 Add/update unit tests in `src/commands/list.rs` verifying
      `format_panes(..., use_color: true)` wraps the state column in the correct
      ANSI color codes for `idle`, `working`, `blocked`, `done`, and leaves
      `unknown` uncolored
- [ ] 3.2 Add/update unit tests verifying `format_panes(..., use_color: false)`
      produces byte-identical output to the pre-change plain-text format
- [ ] 3.3 Add/extend a test verifying column alignment is unaffected by color
      codes: rows with different-length states (e.g. `idle` vs `blocked`) still
      align when `use_color: true`, by comparing the visible (stripped) column
      start offsets
- [ ] 3.4 Review `tests/list_sessions.rs` integration tests to confirm they
      exercise the command via a non-TTY pipe (so `use_color` is false) and
      continue to assert plain text; add a comment/assertion if needed to make
      this explicit

## 4. Verification

- [ ] 4.1 Run `mise run test` (or equivalent Cargo test task) and confirm all
      `list` tests pass
- [ ] 4.2 Manually run `rd list` attached to a real terminal against a tmux
      session with panes in each of the four known states and visually confirm
      the color mapping matches idle=grey, working=yellow, blocked=red,
      done=green
- [ ] 4.3 Manually run `rd list | cat` (or redirect to a file) and confirm no
      ANSI escape codes appear in the output
