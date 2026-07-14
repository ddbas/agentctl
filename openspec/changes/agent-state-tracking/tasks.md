## 1. AgentState Core Types

- [ ] 1.1 Add `AgentState` enum (`Idle`, `Working`, `Done`, `Blocked`,
      `Unknown`) to the shared types module with `Display`, `Serialize`, and
      `Deserialize` derives
- [ ] 1.2 Add `AgentInfo` struct (`session_id: String`,
      `pane_id: Option<String>`, `harness: String`, `state: AgentState`,
      `updated_at: DateTime<Utc>`) matching the state file schema
- [ ] 1.3 Add `chrono` dependency to `Cargo.toml` if not already present

## 2. StateStore

- [ ] 2.1 Implement `write_state(session_id, pane_id, harness, state)` —
      serialise `AgentInfo` to JSON and atomically write via `.tmp` + rename
- [ ] 2.2 Implement `read_state(session_id) -> AgentState` — read and
      deserialise state file; return `Unknown` on any error (missing file, parse
      failure, unknown variant)
- [ ] 2.3 Implement `cleanup_stale(live_pane_ids: &[&str])` — glob
      `$XDG_DATA_HOME/drovr/agents/*/state.json`, delete files whose `pane_id`
      is not in `live_pane_ids`
- [ ] 2.4 Resolve XDG data directory correctly (`$XDG_DATA_HOME` → fallback
      `~/.local/share`)
- [ ] 2.5 Write unit tests for `write_state` / `read_state` round-trip, missing
      file, corrupt file, and `cleanup_stale` (keeps live, deletes stale, no-ops
      when list is empty)

## 3. dv list — State Column & Stale Cleanup

- [ ] 3.1 Update the tmux pane query to collect `#{pane_id}` alongside existing
      fields
- [ ] 3.2 Load all state files and build a `pane_id → AgentState` lookup map
- [ ] 3.3 Call `cleanup_stale` with the collected live pane IDs after the tmux
      query succeeds; skip if the tmux query failed
- [ ] 3.4 Add a fourth `state` column to the output, resolved from the lookup
      map; show `unknown` when no matching state file exists
- [ ] 3.5 Update column padding logic to account for the new state column
- [ ] 3.6 Update existing `dv list` integration tests to expect four columns
- [ ] 3.7 Add integration test: pane has a valid state file → state column shows
      correct value
- [ ] 3.8 Add integration test: pane has no matching state file → state column
      shows `unknown`
- [ ] 3.9 Add integration test: stale state file (pane_id not in live pane set)
      is deleted after `dv list` runs

## 4. dv hook Subcommands (Claude Hook Handlers)

- [ ] 4.1 Add a `dv hook <event>` subcommand dispatcher to the CLI (routes to
      the appropriate handler based on event name)
- [ ] 4.2 Implement `dv hook session-start` — reads JSON from stdin, extracts
      `session_id` and `$TMUX_PANE`; exits 0 silently if `session_id` is absent;
      otherwise writes `Idle` state
- [ ] 4.3 Implement `dv hook user-prompt-submit` — reads stdin, extracts
      `session_id`; writes `Working` state
- [ ] 4.4 Implement `dv hook pre-tool-use` — reads stdin, extracts `session_id`;
      writes `Working` state
- [ ] 4.5 Implement `dv hook stop` — reads stdin, extracts `session_id`; writes
      `Done` state
- [ ] 4.6 Implement `dv hook notification` — reads stdin, extracts `session_id`
      and `notification_type`; writes `Blocked` only when
      `notification_type == "permission_prompt"`; exits 0 silently for all other
      types
- [ ] 4.7 Write unit tests for each handler: valid payload → correct state
      written; missing `session_id` → exits 0, nothing written; `notification`
      with non-permission type → no state change

## 5. dv hooks install

- [ ] 5.1 Implement `dv hooks install` — reads `~/.claude/settings.json`
      (creates if absent), merges the five drovr hook entries, writes the file
      back atomically
- [ ] 5.2 Ensure idempotency: if a drovr entry already exists, do not duplicate
      it
- [ ] 5.3 Preserve all existing non-drovr hook entries in the file
- [ ] 5.4 Print a confirmation message listing which hooks were added vs.
      already present
- [ ] 5.5 Write unit tests for install: clean file, idempotent re-run, merge
      with existing hooks

## 6. Documentation

- [ ] 6.1 Update the project README with an "Agent state tracking" section
      covering the state model, `dv hooks install` as the one-time setup step,
      and the XDG file layout
- [ ] 6.2 Ensure `AGENTS.md` references `dv hooks install` as required setup
