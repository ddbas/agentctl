## 1. status_fsm: wildcard RUN_FINISHED, active_tools gate, permission-prompt type

- [x] 1.1 Add `ToolCallType::PermissionPrompt` variant.
- [x] 1.2 Change `Event::RunFinished` to carry the reconciled `active_tools`
      count (e.g. `RunFinished { active_tools: u32 }`).
- [x] 1.3 Change `Event::ToolCallResult` to carry an optional reconciled
      `blocked_prompts` count, populated only for `PermissionPrompt`-typed
      events (e.g. add a field alongside `r#type`, `None` for other types).
- [x] 1.4 Replace the `(Working, RunFinished) => Done` transition arm with a
      wildcard: any state on `RunFinished { active_tools: 0 }` → `Done`; any
      state on `RunFinished { active_tools: n } if n > 0` → no-op (`None`).
- [x] 1.5 Add `(Working, ToolCallStart { type: PermissionPrompt }) => Blocked`.
- [x] 1.6 Add
      `(Blocked, ToolCallResult { type: PermissionPrompt, blocked_prompts: 0 }) => Working`
      and
      `(Blocked, ToolCallResult { type: PermissionPrompt, blocked_prompts: n }) if n > 0 => None`.
- [x] 1.7 Update/add unit tests in `status_fsm.rs` for: wildcard `RunFinished`
      from `Idle`/`Blocked`/`Done`; `RunFinished` no-op when `active_tools > 0`
      from each starting state; `PermissionPrompt` open/close transitions
      including the counter-gated close no-op.

## 2. state.rs: blocked_prompts field

- [x] 2.1 Add `blocked_prompts: u32` to `AgentInfo` with `#[serde(default)]`.
- [x] 2.2 Add `write_state_with_prompts_to`/equivalent helper (or extend the
      existing `write_state_with_tools_to` signature) so callers can persist
      both `active_tools` and `blocked_prompts` in one atomic write; update the
      XDG-base wrapper functions to match.
- [x] 2.3 Unit tests: round-trip `blocked_prompts`; default-zero on a state file
      written without it; legacy state file (no `blocked_prompts` key)
      deserializes as `0`.

## 3. agent_event.rs: counter bookkeeping and FSM wiring

- [x] 3.1 Read the pane's current `blocked_prompts` alongside `active_tools` at
      the top of `run()`.
- [x] 3.2 Branch `ToolCallStart`/`ToolCallResult` counter mutation on
      `tool_call_type`: `PermissionPrompt` mutates `blocked_prompts` only; every
      other type (`Custom`, `Elicitation`, `None`) mutates `active_tools` only,
      as before.
- [x] 3.3 For `RunFinished`, compute the reconciled `active_tools` value
      (existing override-or-unchanged logic) _before_ calling
      `status_fsm::apply_event`, and pass it into `Event::RunFinished`.
- [x] 3.4 For `PermissionPrompt`-typed `ToolCallResult`, compute the reconciled
      `blocked_prompts` value before calling `apply_event`, and pass it into
      `Event::ToolCallResult`.
- [x] 3.5 Persist both counters via the new `state.rs` helper from task 2.2.
- [x] 3.6 Unit tests: `permission-prompt` `TOOL_CALL_START`/`TOOL_CALL_RESULT`
      mutate `blocked_prompts` and leave `active_tools` untouched; two
      concurrent permission-prompt opens/closes behave per the spec scenarios;
      `RUN_FINISHED` with nonzero `active_tools` leaves `status` unchanged;
      `RUN_FINISHED` from `Blocked`/`Idle`/`Done` with `active_tools` zero
      drives `Done`; `SESSION_STARTED` resets `blocked_prompts` to `0`.

## 4. main.rs: CLI surface

- [x] 4.1 Add `PermissionPrompt` to `ToolCallTypeArg`
      (`--type     permission-prompt`) and its
      `From<ToolCallTypeArg> for     status_fsm::ToolCallType` mapping.
- [x] 4.2 Confirm `RUN_FINISHED`'s existing `--active-tools` flag threads
      through unchanged (no new flag needed — the reconciled value already flows
      from `EventKind::RunFinished { active_tools }` into the FSM per task 3.3).

## 5. hooks.rs: Claude hook definitions

- [x] 5.1 Update the `Notification` (`permission_prompt`) hook command from
      `--id "perm-$id" --type elicitation` to
      `--id "perm-$id-$RANDOM" --type permission-prompt` (or equivalent
      per-invocation nonce).
- [x] 5.2 Update the unmatched `PostToolUse` permission-close hook command the
      same way (`--type permission-prompt`, nonce-suffixed id).
- [x] 5.3 Update `is_agentctl_command`/superseded-entry handling if needed so
      the old `--type elicitation` perm- command strings are recognized as
      superseded and cleanly removed by `agentctl install claude`'s cleanup pass
      (verify via existing `get_single_agentctl_command`/`agentctl_lookup`
      matching — likely no code change needed since matching is by exact command
      string, and the old string will simply fall out of `AGENTCTL_CLAUDE_HOOKS`
      and be treated as superseded).
- [x] 5.4 Update unit tests in `hooks.rs` asserting on the `perm-` hook command
      strings (`--type elicitation` → `--type permission-prompt`).
- [x] 5.5 Update the doc comment above `AGENTCTL_CLAUDE_HOOKS` describing the
      eleven entries if wording references `--type elicitation` for the perm-
      pair.

## 6. Documentation

- [x] 6.1 Update `README.md`'s Claude hook table (`Notification`, `PostToolUse`
      perm- rows) to the new `--type permission-prompt` command strings.
- [x] 6.2 Update any prose in `README.md` describing the perm- pair's
      `--type elicitation` behavior.

## 7. Integration / regression

- [x] 7.1 Run `tests/agent_commands.rs`, `tests/hooks.rs`, and full unit test
      suite; update any assertions that hardcoded the old `RunFinished`
      transition (`Working`-only) or the old perm- command strings.
- [x] 7.2 Run `mise run <lint/test task>` (per project conventions) and fix any
      warnings introduced by new enum variants (e.g. exhaustive `match` arms on
      `ToolCallType`/`ToolCallTypeArg` elsewhere in the codebase).
