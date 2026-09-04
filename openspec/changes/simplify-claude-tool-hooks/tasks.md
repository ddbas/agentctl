## 1. Claude hook registrations (`src/commands/hooks.rs`)

- [x] 1.1 Add `PostToolUseFailure` entries to `RADR_CLAUDE_HOOKS` mirroring both
      `PostToolUse` entries (unmatched untyped + `AskUserQuestion|ExitPlanMode`
      typed `elicitation`), each emitting `TOOL_CALL_RESULT --id <tool_use_id>`
- [x] 1.2 Remove the `Notification` (`permission_prompt`) hook entry
- [x] 1.3 Remove the extra unmatched `PostToolUse` entry keyed on
      `perm-<session_id>-$RANDOM`
- [x] 1.4 Update the doc comment above `RADR_CLAUDE_HOOKS` and
      `print_pi_summary`/install-summary text to describe the new eleven-entry
      composition (drop mentions of permission-prompt/Notification)
- [x] 1.5 Update unit tests in `src/commands/hooks.rs`: replace
      `install_claude_hooks_json_contains_all_eleven_entries`,
      `install_claude_posttooluse_has_three_entries`, and any other test
      asserting on `Notification`, `perm-`, or a 3-entry `PostToolUse` array,
      with tests asserting: `PostToolUse` has exactly 2 entries,
      `PostToolUseFailure` has exactly 2 entries (unmatched + matched, same
      shape as `PostToolUse`), no `Notification` key exists in `hooks.json`, and
      total entry count is eleven
- [x] 1.6 Update/add a test asserting `PostToolUseFailure`'s `async` flag is
      `true` for both entries, matching `PostToolUse`

## 2. `rd agent event` type validation (`src/commands/agent_event.rs`)

- [x] 2.1 Remove `permission-prompt` as an accepted `--type` value; only
      `custom` and `elicitation` remain
- [x] 2.2 Remove the `is_permission_prompt`/`current_blocked_prompts`/
      `new_blocked_prompts` bookkeeping and the `blocked_prompts` field on the
      dispatched event/log entry
- [x] 2.3 Update/remove tests:
      `permission_prompt_tool_call_start_increments_blocked_prompts_not_active_tools`,
      `permission_prompt_tool_call_result_decrements_blocked_prompts_not_active_tools`,
      `session_started_resets_blocked_prompts_to_zero`, and any other
      `blocked_prompts`/`permission-prompt` assertions
- [x] 2.4 Add/update a test asserting `--type permission-prompt` is now rejected
      with a usage error listing `custom`/`elicitation`

## 3. Status FSM (`src/status_fsm.rs`)

- [x] 3.1 Remove the `PermissionPrompt` tool-call type and its
      `Working`→`Blocked`/`Blocked`→`Working` transitions guarded by
      `blocked_prompts`
- [x] 3.2 Remove the `blocked_prompts: Option<u32>` field from the relevant
      event/result struct(s)
- [x] 3.3 Update tests referencing `blocked_prompts: Some(...)`/`None` to drop
      the field entirely

## 4. State schema (`src/state.rs`)

- [x] 4.1 Remove the `blocked_prompts` field from `AgentInfo` and any
      intermediate state-file struct(s)
- [x] 4.2 Confirm/adjust `serde` deserialization so old `state.json` files
      containing a leftover `blocked_prompts` key still deserialize successfully
      (ignore unknown field, no `deny_unknown_fields`)
- [x] 4.3 Update/remove tests: `write_and_read_blocked_prompts_round_trip`,
      `write_state_defaults_blocked_prompts_to_zero`,
      `write_state_with_tools_defaults_blocked_prompts_to_zero`,
      `legacy_state_file_without_blocked_prompts_deserializes_as_zero` — keep an
      equivalent "legacy field is ignored" test if none remains after removing
      the others

## 5. `rd list`/`rd info` output (`src/commands/list.rs`)

- [x] 5.1 Remove `blocked_prompts: 0` from any `AgentInfo`/state construction
      call sites
- [x] 5.2 Update any tests or golden output that reference `blocked_prompts`

## 6. Verification

- [x] 6.1 Run `mise run test` (or the project's full test task) and fix any
      remaining compile errors/test failures from the removed field/type
- [x] 6.2 `rg -n "blocked_prompts|permission-prompt|permission_prompt" src`
      returns no results (outside of comments explicitly describing removed
      history, if any are intentionally kept)
- [x] 6.3 Manually inspect `rd install claude`'s generated `hooks/hooks.json`
      (e.g. via a temp `HOME`) to confirm eleven entries: `SessionStart`,
      `UserPromptSubmit`, `PreToolUse`×2, `PostToolUse`×2,
      `PostToolUseFailure`×2, `Stop`, `StopFailure`, `SessionEnd` — and no
      `Notification` key
