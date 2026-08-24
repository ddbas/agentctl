## 1. Core `AgentState::Snoozed` variant

- [ ] 1.1 Add `AgentState::Snoozed` to the enum in `src/state.rs` (serde
      `snake_case` → `"snoozed"`)
- [ ] 1.2 Update the `priority()` mapping / `Ord` impl so ordering is
      `Blocked < Working < Snoozed < Done < Idle < Unknown`
- [ ] 1.3 Add the `Display` impl case for `Snoozed` → `"snoozed"`
- [ ] 1.4 Add unit tests in `src/state.rs`: serde round-trip for `Snoozed`,
      `Display` output, and pairwise `Ord` comparisons (`Snoozed` vs each other
      variant)
- [ ] 1.5 Update `src/commands/list.rs` sort-order tests to include `Snoozed` in
      the full ascending-order assertion and any pairwise comparisons

## 2. `rd agent status` CLI support

- [ ] 2.1 Update `parse_state` in `src/commands/agent_status.rs` to accept
      `"snoozed"`
- [ ] 2.2 Update the error message listing accepted values to include `snoozed`
- [ ] 2.3 Add a unit test verifying `rd agent status snoozed --pane <id>` writes
      `AgentState::Snoozed` to the state file

## 3. Armed-watch marker plumbing

- [ ] 3.1 Design and implement a per-pane armed-watch marker file (e.g.
      `monitor.json` alongside `state.json`) with read/write/clear helpers in
      `src/state.rs`, following the existing `state_file_path_in` / atomic-write
      patterns
- [ ] 3.2 Add an internal `rd` subcommand (or extend `rd agent status`) to set
      the armed-watch marker, invoked from the `Monitor` `PreToolUse` hook
- [ ] 3.3 Add logic to clear the marker on a `PostToolUse` for `Monitor` whose
      response indicates the watch ended (timeout/error/denied), defaulting to
      "still armed" when the response is ambiguous
- [ ] 3.4 Update `delete_state`/`rd agent clear` to also remove the armed-watch
      marker for the pane
- [ ] 3.5 Add unit tests: marker set/cleared/read helpers, ambiguous response
      defaults to "still armed", marker removed by `rd agent clear`

## 4. Stop / StopFailure snoozed decision

- [ ] 4.1 Update the `Stop` and `StopFailure` hook command paths to check for
      the armed-watch marker before writing state, writing `snoozed` instead of
      `done` when present
- [ ] 4.2 Add unit/integration tests: `Stop` with marker present → `snoozed`;
      `Stop` with no marker → `done` (unchanged); `StopFailure` with marker
      present → `snoozed`

## 5. Transition out of Snoozed

- [ ] 5.1 Verify (with a test) that a subsequent `PostToolUse`/
      `UserPromptSubmit` after a `Snoozed` state correctly overwrites it with
      `working`, so `Snoozed` is never terminal
- [ ] 5.2 Add a test covering `SessionEnd` firing while a pane is `Snoozed`,
      confirming both the state file and the armed-watch marker are deleted

## 6. `rd install claude` / `rd uninstall claude` updates

- [ ] 6.1 Add the `Monitor`-matched `PreToolUse` hook entry to
      `RADR_CLAUDE_HOOKS` (or an equivalent list) in `src/commands/hooks.rs`
- [ ] 6.2 Verify `install_claude`/`uninstall_claude` idempotency and migration
      logic naturally covers the new entry (add a test if not)
- [ ] 6.3 Add/update install and uninstall tests: new entry present after
      install, removed after uninstall, preserved through re-install

## 7. Specs and documentation

- [ ] 7.1 Confirm delta specs in this change (`specs/agent-state/spec.md`,
      `specs/list-sorting/spec.md`, `specs/claude-state-hooks/spec.md`) match
      the final implementation before archiving
- [ ] 7.2 Update any user-facing docs/README listing valid `rd agent status`
      values to include `snoozed`

## 8. Validation

- [ ] 8.1 Run the full test suite via `mise run` (per AGENTS.md, do not invoke
      `cargo` directly) and confirm all new and existing tests pass
- [ ] 8.2 Manually exercise `rd install claude` on a scratch
      `~/.claude/settings.json` copy to confirm the new hook entry format is
      valid JSON Claude Code accepts
