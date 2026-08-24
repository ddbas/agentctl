## 1. Core `AgentState::Snoozed` variant

- [x] 1.1 Add `AgentState::Snoozed` to the enum in `src/state.rs` (serde
      `snake_case` → `"snoozed"`)
- [x] 1.2 Update the `priority()` mapping / `Ord` impl so ordering is
      `Blocked < Working < Snoozed < Done < Idle < Unknown`
- [x] 1.3 Add the `Display` impl case for `Snoozed` → `"snoozed"`
- [x] 1.4 Add unit tests in `src/state.rs`: serde round-trip for `Snoozed`,
      `Display` output, and pairwise `Ord` comparisons (`Snoozed` vs each other
      variant)
- [x] 1.5 Update `src/commands/list.rs` sort-order tests to include `Snoozed` in
      the full ascending-order assertion and any pairwise comparisons

## 2. `rd agent status` CLI support

- [x] 2.1 Update `parse_state` in `src/commands/agent_status.rs` to accept
      `"snoozed"`
- [x] 2.2 Update the error message listing accepted values to include `snoozed`
- [x] 2.3 Add a unit test verifying `rd agent status snoozed --pane <id>` writes
      `AgentState::Snoozed` to the state file

## 3. Armed-watch marker plumbing

- [x] 3.1 Design and implement a per-pane armed-watch marker file (e.g.
      `monitor.json` alongside `state.json`) with read/write/clear helpers in
      `src/state.rs`, following the existing `state_file_path_in` / atomic-write
      patterns
- [x] 3.2 Add an internal `rd` subcommand (or extend `rd agent status`) to set
      the armed-watch marker, invoked from the `Monitor` `PreToolUse` hook
- [x] 3.3 Add logic to clear the marker on a `PostToolUse` for `Monitor` whose
      response indicates the watch ended (timeout/error/denied), defaulting to
      "still armed" when the response is ambiguous
- [x] 3.4 Update `delete_state`/`rd agent clear` to also remove the armed-watch
      marker for the pane
- [x] 3.5 Add unit tests: marker set/cleared/read helpers, ambiguous response
      defaults to "still armed", marker removed by `rd agent clear`

## 4. Stop / StopFailure snoozed decision

- [x] 4.1 Update the `Stop` and `StopFailure` hook command paths to check for
      the armed-watch marker before writing state, writing `snoozed` instead of
      `done` when present
- [x] 4.2 Add unit/integration tests: `Stop` with marker present → `snoozed`;
      `Stop` with no marker → `done` (unchanged); `StopFailure` with marker
      present → `snoozed`

## 5. Transition out of Snoozed

- [x] 5.1 Verify (with a test) that a subsequent `PostToolUse`/
      `UserPromptSubmit` after a `Snoozed` state correctly overwrites it with
      `working`, so `Snoozed` is never terminal
- [x] 5.2 Add a test covering `SessionEnd` firing while a pane is `Snoozed`,
      confirming both the state file and the armed-watch marker are deleted

## 6. `rd install claude` / `rd uninstall claude` updates

- [x] 6.1 Add the `Monitor`-matched `PreToolUse` hook entry to
      `RADR_CLAUDE_HOOKS` (or an equivalent list) in `src/commands/hooks.rs`
- [x] 6.2 Verify `install_claude`/`uninstall_claude` idempotency and migration
      logic naturally covers the new entry (add a test if not)
- [x] 6.3 Add/update install and uninstall tests: new entry present after
      install, removed after uninstall, preserved through re-install

## 7. Specs and documentation

- [x] 7.1 Confirm delta specs in this change (`specs/agent-state/spec.md`,
      `specs/list-sorting/spec.md`, `specs/claude-state-hooks/spec.md`) match
      the final implementation before archiving
- [x] 7.2 Update any user-facing docs/README listing valid `rd agent status`
      values to include `snoozed`

## 8. Validation

- [x] 8.1 Run the full test suite via `mise run` (per AGENTS.md, do not invoke
      `cargo` directly) and confirm all new and existing tests pass
- [x] 8.2 Manually exercise `rd install claude` on a scratch
      `~/.claude/settings.json` copy to confirm the new hook entry format is
      valid JSON Claude Code accepts
