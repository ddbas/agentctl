## 1. AgentState Core Types

- [x] 1.1 Add `AgentState` enum (`Idle`, `Working`, `Done`, `Blocked`,
      `Unknown`) to the shared types module with `Display`, `Serialize`, and
      `Deserialize` derives
- [x] 1.2 Add `AgentInfo` struct (`pane_id: String`, `harness: String`,
      `state: AgentState`, `updated_at: DateTime<Utc>`) matching the state file
      schema
- [x] 1.3 Add `chrono` dependency to `Cargo.toml` if not already present

## 2. StateStore

- [x] 2.1 Implement `write_state(pane_id, harness, state)` — serialise
      `AgentInfo` to JSON and atomically write via `.tmp` + rename to
      `$XDG_DATA_HOME/drovr/agents/<pane-id>/state.json`
- [x] 2.2 Implement `delete_state(pane_id)` — remove the state file and its
      parent directory; no-op if absent
- [x] 2.3 Implement `read_state(pane_id) -> AgentState` — read and deserialise
      state file; return `Unknown` on any error (missing file, parse failure,
      unknown variant)
- [x] 2.4 Resolve XDG data directory correctly (`$XDG_DATA_HOME` → fallback
      `~/.local/share`)
- [x] 2.5 Write unit tests for `write_state` / `read_state` round-trip, missing
      file, corrupt file, `delete_state` (present and absent), and the
      overwrite-on-reuse behaviour

## 3. dv list — State Column & Stale Validation

- [x] 3.1 Update the tmux pane query to collect `#{pane_id}` and
      `#{pane_current_command}` alongside existing fields
- [x] 3.2 Load all state files and build a `pane_id → AgentInfo` lookup map
- [x] 3.3 Validate each state file as a last resort (only when tmux is
      reachable): delete if `pane_id` not in live pane set, or if live pane's
      `pane_current_command` doesn't match `harness` **and** `harness` is not
      `"unknown"`; skip all validation if tmux query failed
- [x] 3.4 Add a fourth `state` column to the output, resolved from the lookup
      map; show `unknown` when no matching state file exists
- [x] 3.5 Update column padding logic to account for the new state column
- [x] 3.6 Update existing `dv list` integration tests to expect four columns
- [x] 3.7 Add integration test: pane has a valid state file → state column shows
      correct value
- [x] 3.8 Add integration test: pane has no matching state file → state column
      shows `unknown`
- [x] 3.9 Add integration test: state file with pane_id not in live set →
      deleted after `dv list` runs
- [x] 3.10 Add integration test: state file whose harness doesn't match
      pane_current_command → deleted after `dv list` runs

## 4. dv set-state and dv clear-state Subcommands

- [x] 4.1 Add a `dv set-state <state>` subcommand to the CLI; accepted values
      are `idle`, `working`, `done`, `blocked`; exits 0 silently when
      `$TMUX_PANE` is unset and `--pane` is not supplied
- [x] 4.2 Add `--harness <harness_id>` optional flag to `dv set-state`; written
      to the `harness` field in `state.json`; defaults to `"unknown"` when
      omitted
- [x] 4.3 Add `--pane <pane_id>` optional flag to `dv set-state` to override
      `$TMUX_PANE` (useful for scripting and tests)
- [x] 4.4 Implement the write path: resolve pane ID (flag → env var → silent
      exit), call `write_state(pane_id, harness, state)`
- [x] 4.5 Add a `dv clear-state` subcommand that calls
      `delete_state($TMUX_PANE)`; exits 0 silently when `$TMUX_PANE` is unset or
      when no state file exists; accept an optional `--pane` flag
- [x] 4.6 Add integration test: state file with harness `"unknown"` → not
      deleted by `dv list` (harness-match check skipped)
- [x] 4.7 Write unit tests: `$TMUX_PANE` set → correct state written;
      `$TMUX_PANE` unset → exits 0, no changes; `--harness` flag → harness field
      populated correctly; `--harness` omitted → `"unknown"` written to harness
      field; `dv clear-state` present and absent state file

## 5. dv hooks install (placeholder)

- [x] 5.1 Implement `dv hooks install <harness>` — exit 1 with the message
      `"unknown harness '<harness>'; no harnesses are currently supported"` for
      all inputs; the command exists solely to establish the subcommand name and
      argument shape for future harness changes
- [x] 5.2 Write a unit test confirming `dv hooks install <anything>` exits 1
      with the expected error message

## 6. Documentation

- [x] 6.1 Update the project README with an "Agent state tracking" section
      covering the state model, `dv set-state` / `dv clear-state` usage, and the
      XDG file layout
