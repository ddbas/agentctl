## 1. AgentState Core Types

- [ ] 1.1 Add `AgentState` enum (`Idle`, `Working`, `Done`, `Blocked`,
      `Unknown`) to the shared types module with `Display`, `Serialize`, and
      `Deserialize` derives
- [ ] 1.2 Add `AgentInfo` struct (`pane_id: String`, `harness: String`,
      `state: AgentState`, `updated_at: DateTime<Utc>`) matching the state file
      schema
- [ ] 1.3 Add `chrono` dependency to `Cargo.toml` if not already present

## 2. StateStore

- [ ] 2.1 Implement `write_state(pane_id, harness, state)` — serialise
      `AgentInfo` to JSON and atomically write via `.tmp` + rename to
      `$XDG_DATA_HOME/drovr/agents/<pane-id>/state.json`
- [ ] 2.2 Implement `delete_state(pane_id)` — remove the state file and its
      parent directory; no-op if absent
- [ ] 2.3 Implement `read_state(pane_id) -> AgentState` — read and deserialise
      state file; return `Unknown` on any error (missing file, parse failure,
      unknown variant)
- [ ] 2.4 Resolve XDG data directory correctly (`$XDG_DATA_HOME` → fallback
      `~/.local/share`)
- [ ] 2.5 Write unit tests for `write_state` / `read_state` round-trip, missing
      file, corrupt file, `delete_state` (present and absent), and the
      overwrite-on-reuse behaviour

## 3. dv list — State Column & Stale Validation

- [ ] 3.1 Update the tmux pane query to collect `#{pane_id}` and
      `#{pane_current_command}` alongside existing fields
- [ ] 3.2 Load all state files and build a `pane_id → AgentInfo` lookup map
- [ ] 3.3 Validate each state file as a last resort (only when tmux is
      reachable): delete if `pane_id` not in live pane set, or if live pane's
      `pane_current_command` doesn't match `harness`; skip all validation if
      tmux query failed
- [ ] 3.4 Add a fourth `state` column to the output, resolved from the lookup
      map; show `unknown` when no matching state file exists
- [ ] 3.5 Update column padding logic to account for the new state column
- [ ] 3.6 Update existing `dv list` integration tests to expect four columns
- [ ] 3.7 Add integration test: pane has a valid state file → state column shows
      correct value
- [ ] 3.8 Add integration test: pane has no matching state file → state column
      shows `unknown`
- [ ] 3.9 Add integration test: state file with pane_id not in live set →
      deleted after `dv list` runs
- [ ] 3.10 Add integration test: state file whose harness doesn't match
      pane_current_command → deleted after `dv list` runs

## 4. dv set-state and dv clear-state Subcommands

- [ ] 4.1 Add a `dv set-state <state>` subcommand to the CLI; accepted values
      are `idle`, `working`, `done`, `blocked`; exits 0 silently when
      `$TMUX_PANE` is unset and `--pane` is not supplied
- [ ] 4.2 Add `--harness <harness_id>` optional flag to `dv set-state`; written
      to the `harness` field in `state.json`; defaults to `"unknown"` when
      omitted
- [ ] 4.3 Add `--pane <pane_id>` optional flag to `dv set-state` to override
      `$TMUX_PANE` (useful for scripting and tests)
- [ ] 4.4 Implement the write path: resolve pane ID (flag → env var → silent
      exit), call `write_state(pane_id, harness, state)`
- [ ] 4.5 Add a `dv clear-state` subcommand that calls
      `delete_state($TMUX_PANE)`; exits 0 silently when `$TMUX_PANE` is unset or
      when no state file exists; accept an optional `--pane` flag
- [ ] 4.6 Write unit tests: `$TMUX_PANE` set → correct state written;
      `$TMUX_PANE` unset → exits 0, no changes; `--harness` flag → harness field
      populated correctly; `dv clear-state` present and absent state file

## 5. dv hooks install

- [ ] 5.1 Implement `dv hooks install <harness>` — dispatch on the harness
      argument; return a clear error for unrecognised values
- [ ] 5.2 Implement the `claude` handler — reads `~/.claude/settings.json`
      (creates if absent), merges the six drovr hook entries using
      `dv set-state <state> --harness claude` and `dv clear-state`, writes the
      file back atomically; `SessionStart` and `SessionEnd` are registered
      without `async`, all others with `async: true`
- [ ] 5.3 Ensure idempotency: if a drovr entry already exists, do not duplicate
      it
- [ ] 5.4 Preserve all existing non-drovr hook entries in the file
- [ ] 5.5 Print a confirmation message listing which hooks were added vs.
      already present
- [ ] 5.6 Write unit tests for install: clean file, idempotent re-run, merge
      with existing hooks, unrecognised harness argument returns error

## 6. Documentation

- [ ] 6.1 Update the project README with an "Agent state tracking" section
      covering the state model, `dv hooks install claude` as the one-time setup
      step, and the XDG file layout
- [ ] 6.2 Ensure `AGENTS.md` references `dv hooks install claude` as required
      setup
