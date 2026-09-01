## Why

The `track-tool-calls` change (#25) added `active_tools` tracking driven by
`TOOL_CALL_START`/`TOOL_CALL_RESULT` events fired on _every_ tool call. The pi
extension got two general handlers (`tool_call`/`tool_result`) that fire for
every tool invocation, so `rd list`'s tool-count column works correctly for pi
sessions. The Claude harness was never given an equivalent general hook: its
`PreToolUse`/`PostToolUse` entries only fire `TOOL_CALL_START`/
`TOOL_CALL_RESULT` for the two interactive tools (`AskUserQuestion`,
`ExitPlanMode`), plus a same-purpose-as-before "perm-$id" pair used only to
close out permission-prompt blocking. As a result, Claude sessions never report
a nonzero active-tool count for ordinary tool calls (`Read`, `Edit`, `Bash`,
etc.) — the one thing this feature is supposed to surface — while pi sessions
do. This is the harness parity gap the user is seeing: Claude's installed hooks
reflect the pre-#25 hook set, not the current tool-call tracking design.

## What Changes

- Add a general (no-matcher) `PreToolUse` hook entry that invokes
  `rd agent event TOOL_CALL_START --id <tool_use_id> --harness claude` (async)
  for every tool call, mirroring pi's `tool_call` handler.
- Add a general (no-matcher) `PostToolUse` hook entry that invokes
  `rd agent event TOOL_CALL_RESULT --id <tool_use_id> --harness claude` (async)
  for every tool call, mirroring pi's `tool_result` handler.
- Keep the existing elicitation-typed `PreToolUse`/`PostToolUse` entries
  (matcher `AskUserQuestion|ExitPlanMode`) and the existing unmatched
  `PostToolUse` → `TOOL_CALL_RESULT --id "perm-$id" --type elicitation` /
  `Notification` → `TOOL_CALL_START --id "perm-$id" --type elicitation` pair
  used for blocked-state tracking — unchanged in behavior.
- Update `rd install claude`'s migration/cleanup logic so a settings.json
  written by a pre-change `rd` binary (missing the two new general entries) gets
  them added on the next `rd install claude`, without disturbing existing custom
  or radr-managed entries.
- Update the printed install summary, `README.md`'s Claude hook table, and the
  `claude-state-hooks` spec to describe eleven hook entries instead of nine.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `claude-state-hooks`: Add requirements for the two new general
  `PreToolUse`/`PostToolUse` hook entries that call `TOOL_CALL_START`/
  `TOOL_CALL_RESULT` (no `--type`) for every tool call, and update the existing
  hook-count/table requirements from nine to eleven entries.

## Impact

- `src/commands/hooks.rs`: `RADR_CLAUDE_HOOKS` gains two entries;
  `format_claude_summary` and any hard-coded entry counts update accordingly.
- `README.md`: Claude hook table and example install output.
- `openspec/specs/claude-state-hooks/spec.md`: two new requirements/ scenarios;
  existing "nine entries" references become "eleven entries".
- Tests in `src/commands/hooks.rs` and `tests/hooks.rs` that assert on the fixed
  hook count or enumerate all hook entries.
- No changes to `rd agent event`, the status FSM, or the state file format.
