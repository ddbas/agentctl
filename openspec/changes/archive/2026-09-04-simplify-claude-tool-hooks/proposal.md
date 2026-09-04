## Why

`PostToolUse` only fires when a tool call succeeds, so a failed tool call
(nonzero Bash exit, a failed `Edit`, an MCP tool error, etc.) never gets its
matching `TOOL_CALL_RESULT`, leaking `active_tools` upward for the rest of the
session. Separately, the existing permission-prompt tracking (`Notification`
matcher `permission_prompt` paired with an unmatched `PostToolUse` entry)
generates its start and stop ids independently via `$RANDOM` in two different
shell subprocesses, so they can never actually match each other — the pairing
was broken from the start and only ever added unpaired noise to `events.jsonl`.
The extra `blocked_prompts` counter that was built to work around that mismatch,
and the duplicate, indistinguishable unmatched `PostToolUse` entry that
implements it, add real complexity for a mechanism that doesn't work. We want
reliable, simple tool-call tracking based only on
`PreToolUse`/`PostToolUse`/`PostToolUseFailure`, keyed on the tool's own stable
`tool_use_id`.

## What Changes

- Add `PostToolUseFailure` hook registrations (unmatched + the
  `AskUserQuestion|ExitPlanMode` matcher) mirroring the existing `PostToolUse`
  ones, each closing out the `TOOL_CALL_START` opened by `PreToolUse` with a
  `TOOL_CALL_RESULT` for the same `tool_use_id`.
- **BREAKING**: Remove the `Notification` hook registration entirely (no more
  `permission_prompt` matcher, no more `PermissionRequest`/`PermissionDenied`
  support).
- **BREAKING**: Remove the third, `perm-<session_id>-<nonce>`-keyed
  `PostToolUse` entry that paired with `Notification` — it never correctly
  matched its opening event.
- **BREAKING**: Remove the `permission-prompt` `--type` value from
  `rd agent event TOOL_CALL_START`/`TOOL_CALL_RESULT`. `elicitation` remains the
  only non-`custom` type, driving `blocked` status for
  `AskUserQuestion`/`ExitPlanMode` via ordinary `PreToolUse`/`PostToolUse`
  pairing on `tool_use_id`.
- **BREAKING**: Remove the `blocked_prompts` counter and field from `state.json`
  and `AgentInfo` — it existed solely to work around the unpairable
  permission-prompt nonce and has no other purpose once `permission-prompt` is
  gone.
- Total Claude hook entry count moves from eleven to eleven again post-change
  (drop `Notification` + the extra `PostToolUse` entry, add two
  `PostToolUseFailure` entries), but the composition changes: `PreToolUse`×2,
  `PostToolUse`×2, `PostToolUseFailure`×2 (new), plus the untouched
  `SessionStart`, `UserPromptSubmit`, `Stop`, `StopFailure`, `SessionEnd`.

## Capabilities

### Modified Capabilities

- `claude-state-hooks`: replace the `Notification`/permission-prompt
  `PostToolUse` pairing with `PostToolUseFailure` hook registrations; update the
  hook count/composition and the `rd install claude` requirement.
- `agent-events`: drop the `permission-prompt` `--type` value and its FSM
  transitions/scenarios; `TOOL_CALL_START`/`TOOL_CALL_RESULT` now only accept
  `custom` and `elicitation`.
- `agent-state`: remove the `blocked_prompts` field from `state.json` and
  `AgentInfo`.

## Impact

- `src/commands/hooks.rs`: `RADR_CLAUDE_HOOKS` table and its tests.
- `src/commands/agent*.rs` (or wherever `rd agent event`/state-machine
  transitions and `AgentInfo`/`state.json` (de)serialization live): drop
  `permission-prompt` type handling and `blocked_prompts`.
- Any `rd list`/`rd info` output that surfaces `blocked_prompts`.
- Users who already ran `rd install claude` need to re-run it to pick up the new
  `hooks/hooks.json`; existing `state.json` files with a `blocked_prompts` field
  must continue to deserialize (ignore/drop the field rather than erroring).
