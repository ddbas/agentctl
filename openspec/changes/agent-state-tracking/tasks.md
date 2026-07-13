## 1. AgentState Core Types

- [ ] 1.1 Add `AgentState` enum (`Idle`, `Working`, `Done`, `Blocked`,
      `Unknown`) to the shared types module with `Display`, `Serialize`, and
      `Deserialize` derives
- [ ] 1.2 Add `AgentInfo` struct (`agent_id: Uuid`, `harness: String`,
      `state: AgentState`, `updated_at: DateTime<Utc>`) matching the state file
      schema
- [ ] 1.3 Add `uuid` and `chrono` dependencies to `Cargo.toml` if not already
      present

## 2. StateStore

- [ ] 2.1 Implement `write_state(agent_id, harness, state)` — serialise
      `AgentInfo` to JSON and atomically write via `.tmp` + rename
- [ ] 2.2 Implement `read_state(agent_id) -> AgentState` — read and deserialise
      state file; return `Unknown` on any error (missing file, parse failure,
      unknown variant)
- [ ] 2.3 Implement `cleanup_stale(live_ids: &[Uuid])` — glob
      `$XDG_DATA_HOME/drovr/agents/*/state.json`, delete files whose `agent_id`
      is not in `live_ids`
- [ ] 2.4 Resolve XDG data directory correctly (`$XDG_DATA_HOME` → fallback
      `~/.local/share`)
- [ ] 2.5 Write unit tests for `write_state` / `read_state` round-trip, missing
      file, corrupt file, and `cleanup_stale` (keeps live, deletes stale, no-ops
      when list is empty)

## 3. Pane Spawning — UUID Injection

- [ ] 3.1 Generate a UUID v4 when `dv` creates a new agent pane
- [ ] 3.2 Set `DROVR_AGENT_ID` in the pane's tmux environment via
      `tmux set-environment -t <pane> DROVR_AGENT_ID <uuid>`
- [ ] 3.3 Call `write_state` immediately after setting the env var to persist
      the initial `Idle` state
- [ ] 3.4 Write an integration test (testcontainers) that spawns a pane and
      verifies `DROVR_AGENT_ID` is set and the state file contains
      `"state": "idle"`

## 4. dv list — State Column & Stale Cleanup

- [ ] 4.1 Update the tmux pane query to also read `DROVR_AGENT_ID` per pane via
      `tmux show-environment -t <pane> DROVR_AGENT_ID`
- [ ] 4.2 Call `cleanup_stale` with the collected live IDs after the tmux query
      succeeds; skip if the tmux query failed
- [ ] 4.3 Call `read_state` for each live pane and add a fourth `state` column
      to the output
- [ ] 4.4 Update column padding logic to account for the new state column
- [ ] 4.5 Update existing `dv list` integration tests to expect four columns
- [ ] 4.6 Add integration test: pane has valid state file → state column shows
      correct value
- [ ] 4.7 Add integration test: pane has no `DROVR_AGENT_ID` or no state file →
      state column shows `unknown`
- [ ] 4.8 Add integration test: stale state file (no matching live pane) is
      deleted after `dv list` runs

## 5. dv hook Subcommands (Claude Hook Handlers)

- [ ] 5.1 Add a `dv hook <event>` subcommand dispatcher to the CLI (routes to
      the appropriate handler based on event name)
- [ ] 5.2 Implement `dv hook session-start` — reads Claude's JSON from stdin,
      writes `Idle` state if `DROVR_AGENT_ID` is set; exits 0 silently otherwise
- [ ] 5.3 Implement `dv hook user-prompt-submit` — reads stdin, writes `Working`
      state
- [ ] 5.4 Implement `dv hook pre-tool-use` — reads stdin, writes `Working` state
- [ ] 5.5 Implement `dv hook stop` — reads stdin, writes `Done` state
- [ ] 5.6 Implement `dv hook notification` — reads stdin, checks
      `notification_type == "permission_prompt"`, writes `Blocked` state;
      ignores all other notification types
- [ ] 5.7 Write unit tests for each handler: fires with `DROVR_AGENT_ID` set →
      correct state written; fires without `DROVR_AGENT_ID` → exits 0, nothing
      written; `notification` with non-permission type → no state change

## 6. dv hooks install

- [ ] 6.1 Implement `dv hooks install` — reads `~/.claude/settings.json`
      (creates if absent), merges the five drovr hook entries, writes the file
      back atomically
- [ ] 6.2 Ensure idempotency: if a drovr entry already exists, do not duplicate
      it
- [ ] 6.3 Preserve all existing non-drovr hook entries in the file
- [ ] 6.4 Print a confirmation message listing which hooks were added vs.
      already present
- [ ] 6.5 Write unit tests for install: clean file, idempotent re-run, merge
      with existing hooks

## 7. Documentation

- [ ] 7.1 Update the project README with a "Agent state tracking" section
      covering the state model, `dv hooks install` as the one-time setup step,
      and the XDG file layout
- [ ] 7.2 Ensure `AGENTS.md` references `dv hooks install` as required setup
